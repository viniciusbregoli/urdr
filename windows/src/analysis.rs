// The engine's analysis.json: tempo, beat grid and chord segments of the full mix.
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Chord {
    pub start: f64,
    pub end: f64,
    pub label: String,
}

impl Chord {
    // "A:min" reads as "Am" on a lead sheet; "N" is the engine's no-chord label.
    pub fn name(&self) -> String {
        if self.label == "N" {
            "N.C.".into()
        } else {
            self.label.replace(":min", "m")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Analysis {
    pub bpm: f64,
    pub beats: Vec<f64>,
    pub downbeats: Vec<f64>,
    pub chords: Vec<Chord>,
}

impl Analysis {
    pub fn load(path: &Path) -> Option<Analysis> {
        serde_json::from_slice(&std::fs::read(path).ok()?).ok()
    }

    pub fn chord_index(&self, time: f64) -> Option<usize> {
        self.chords.iter().position(|c| c.start <= time && time < c.end)
    }

    pub fn next_chord(&self, time: f64) -> Option<&Chord> {
        let after = self.chord_index(time).map_or(time, |i| self.chords[i].end);
        self.chords.iter().find(|c| c.start >= after && c.label != "N")
    }

    // Going back skips a downbeat that was just passed, so repeated presses while playing keep moving.
    pub fn downbeat(&self, time: f64, forward: bool) -> Option<f64> {
        if forward {
            self.downbeats.iter().copied().find(|&d| d > time + 0.01)
        } else {
            self.downbeats.iter().copied().rev().find(|&d| d < time - 0.25)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_analysis_reads_engine_output() {
        let dir = std::env::temp_dir().join(format!("urdr-analysis-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("analysis.json");
        std::fs::write(&path, r#"{"bpm": 120.0, "beats": [0.0, 0.5, 1.0, 1.5, 2.0, 2.5], "downbeats": [0.0, 2.0], "chords": [{"start": 0.0, "end": 2.0, "label": "C"}, {"start": 2.0, "end": 3.0, "label": "A:min"}, {"start": 3.0, "end": 4.0, "label": "N"}]}"#).unwrap();
        let a = Analysis::load(&path).unwrap();
        assert_eq!(a.bpm, 120.0);
        assert_eq!(a.downbeats, vec![0.0, 2.0]);
        assert_eq!(a.chords.iter().map(Chord::name).collect::<Vec<_>>(), ["C", "Am", "N.C."]);
        assert_eq!(a.chord_index(2.5), Some(1));
        assert_eq!(a.chord_index(5.0), None);
        assert_eq!(a.next_chord(0.5).map(Chord::name), Some("Am".into()));
        assert_eq!(a.downbeat(0.5, true), Some(2.0));
        assert_eq!(a.downbeat(2.1, false), Some(0.0), "a downbeat just passed is skipped");
        assert_eq!(a.downbeat(2.5, false), Some(2.0));
        assert_eq!(a.downbeat(2.0, true), None);
        assert!(Analysis::load(&dir.join("missing.json")).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }
}
