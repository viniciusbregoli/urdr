// Tab notes laid out in measures from the beat grid, the way Songsterr shows them. Port of Score.swift.
use crate::analysis::Analysis;
use crate::song::TabNote;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub note: TabNote,
    // Position in beats from the start of the measure, snapped to sixteenths.
    pub slot: f64,
    // Length in sixteenths up to the next note, which drives the rhythm stems and beams.
    pub sixteenths: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chord {
    pub slot: f64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
    pub number: usize,
    // Beat onsets inside the measure followed by the measure end, so there are beat_count + 1 entries.
    pub beats: Vec<f64>,
    pub notes: Vec<Note>,
    pub chords: Vec<Chord>,
}

impl Measure {
    pub fn start(&self) -> f64 {
        self.beats[0]
    }
    pub fn end(&self) -> f64 {
        self.beats[self.beats.len() - 1]
    }
    pub fn beat_count(&self) -> usize {
        self.beats.len() - 1
    }

    pub fn beat_position(&self, time: f64) -> f64 {
        let Some(j) = (0..self.beats.len() - 1).rev().find(|&j| self.beats[j] <= time) else { return 0.0 };
        j as f64 + ((time - self.beats[j]) / (self.beats[j + 1] - self.beats[j])).clamp(0.0, 1.0)
    }

    pub fn time_at_beat(&self, position: f64) -> f64 {
        let j = (position as i64).clamp(0, self.beat_count() as i64 - 1) as usize;
        self.beats[j] + (position - j as f64) * (self.beats[j + 1] - self.beats[j])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    pub measures: Vec<Measure>,
}

impl Score {
    pub fn new(notes: &[TabNote], analysis: &Analysis, duration: f64) -> Score {
        let beat_length = if analysis.bpm > 0.0 { 60.0 / analysis.bpm } else { 0.5 };
        let mut bounds = analysis.downbeats.clone();
        // Notes before the first downbeat go in a pickup measure.
        if let Some(&first) = bounds.first() {
            if first > 0.05 && notes.iter().any(|n| n.start < first) {
                bounds.insert(0, (first - beat_length * 4.0).max(0.0));
            }
        }
        bounds.push(duration.max(bounds.last().copied().unwrap_or(0.0) + beat_length));
        let mut measures: Vec<Measure> = bounds
            .windows(2)
            .enumerate()
            .map(|(index, pair)| {
                let mut beats = vec![pair[0]];
                beats.extend(analysis.beats.iter().copied().filter(|&b| b > pair[0] + 0.02 && b < pair[1] - 0.02));
                beats.push(pair[1]);
                Measure { number: index + 1, beats, notes: vec![], chords: vec![] }
            })
            .collect();
        if measures.is_empty() {
            return Score { measures };
        }

        let locate = |measures: &[Measure], time: f64| -> (usize, f64) {
            let m = measures.iter().rposition(|x| x.start() <= time).unwrap_or(0);
            let count = measures[m].beat_count() as f64;
            let slot = (measures[m].beat_position(time) * 4.0).round() / 4.0;
            // Snapping can push a late note onto the next downbeat.
            if slot >= count && m + 1 < measures.len() {
                return (m + 1, 0.0);
            }
            (m, slot.min(count - 0.25))
        };

        let mut sorted = notes.to_vec();
        sorted.sort_by(|a, b| a.start.total_cmp(&b.start));
        let placed: Vec<(TabNote, (usize, f64))> = sorted.into_iter().map(|n| (n, locate(&measures, n.start))).collect();
        // Global sixteenth index of each note, used to measure the gap to the next one.
        let mut offsets = vec![0i64];
        for m in &measures {
            offsets.push(offsets.last().unwrap() + m.beat_count() as i64 * 4);
        }
        let indices: Vec<i64> = placed.iter().map(|(_, (m, slot))| offsets[*m] + (slot * 4.0) as i64).collect();
        for (i, (note, (m, slot))) in placed.iter().enumerate() {
            // Notes sharing a slot (a double-stop) measure to the next distinct onset, not to each other.
            let gap = indices[i + 1..].iter().find(|&&x| x > indices[i]).map_or(4, |x| x - indices[i]) as i32;
            let own = ((note.end - note.start) / beat_length * 4.0 + 0.5) as i32;
            let sixteenths = gap.min(own.max(1)).min(16).max(1);
            measures[*m].notes.push(Note { note: *note, slot: *slot, sixteenths });
        }
        for chord in analysis.chords.iter().filter(|c| c.label != "N") {
            let (m, slot) = locate(&measures, chord.start);
            measures[m].chords.push(Chord { slot, name: chord.name() });
        }
        Score { measures }
    }

    // The most common bar length, so a pickup bar does not set the time signature. Ties go to the shorter bar.
    pub fn beats_per_bar(&self) -> usize {
        let mut counts: HashMap<usize, usize> = HashMap::new();
        for m in &self.measures {
            *counts.entry(m.beat_count()).or_default() += 1;
        }
        counts.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))).map_or(4, |(k, _)| k)
    }

    pub fn measure_index(&self, time: f64) -> Option<usize> {
        self.measures.iter().rposition(|m| m.start() <= time && time < m.end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::Chord as AChord;

    fn note(start: f64, end: f64) -> TabNote {
        TabNote { start, end, midi: 69, string: 1, fret: 5 }
    }

    fn grid(beats: Vec<f64>, downbeats: Vec<f64>, chords: Vec<AChord>) -> Analysis {
        Analysis { bpm: 120.0, beats, downbeats, chords }
    }

    #[test]
    fn score_snaps_notes_into_measures() {
        // 120 BPM in 4/4: beats every 0.5 s, bars every 2 s.
        let chords = vec![
            AChord { start: 0.0, end: 2.0, label: "A:min".into() },
            AChord { start: 2.0, end: 4.0, label: "N".into() },
        ];
        let analysis = grid((0..8).map(|i| i as f64 * 0.5).collect(), vec![0.0, 2.0], chords);
        let score = Score::new(&[note(0.0, 0.25), note(0.26, 0.37), note(0.37, 0.5), note(1.99, 3.0)], &analysis, 4.0);

        assert_eq!(score.measures.iter().map(Measure::beat_count).collect::<Vec<_>>(), [4, 4]);
        assert_eq!(score.measures[0].notes.iter().map(|n| n.slot).collect::<Vec<_>>(), [0.0, 0.5, 0.75]);
        assert_eq!(score.measures[0].notes.iter().map(|n| n.sixteenths).collect::<Vec<_>>(), [2, 1, 1]);
        // 1.99 s is a hair before the second downbeat and belongs on it.
        assert_eq!(score.measures[1].notes.iter().map(|n| n.slot).collect::<Vec<_>>(), [0.0]);
        assert_eq!(score.measures[1].notes[0].sixteenths, 4);
        assert_eq!(score.measures[0].chords, [Chord { slot: 0.0, name: "Am".into() }]);
        assert!(score.measures[1].chords.is_empty());

        let measure = &score.measures[1];
        assert!((measure.beat_position(2.75) - 1.5).abs() < 1e-9);
        assert!((measure.time_at_beat(1.5) - 2.75).abs() < 1e-9);
        assert_eq!(score.measure_index(3.9), Some(1));
    }

    #[test]
    fn pickup_bar_does_not_set_the_time_signature() {
        let analysis = grid(vec![0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5], vec![0.5, 2.5], vec![]);
        let pickup = Score::new(&[note(0.1, 0.4), note(0.6, 0.9)], &analysis, 4.5);
        assert_eq!(pickup.measures.first().map(|m| m.number), Some(1));
        assert_eq!(pickup.measures[0].beat_count(), 1, "the pickup holds one beat");
        assert_eq!(pickup.beats_per_bar(), 4);
    }

    #[test]
    fn double_stop_measures_to_the_next_distinct_onset() {
        let analysis = grid((0..8).map(|i| i as f64 * 0.5).collect(), vec![0.0, 2.0], vec![]);
        let notes = [note(0.0, 0.5), TabNote { start: 0.0, end: 0.5, midi: 64, string: 2, fret: 5 }, note(0.5, 1.0)];
        let score = Score::new(&notes, &analysis, 4.0);
        assert_eq!(score.measures[0].notes.iter().map(|n| n.sixteenths).collect::<Vec<_>>(), [4, 4, 4]);
    }
}
