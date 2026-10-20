// The song's tab and its hand editing on the measure grid. Port of AppModel's tab state and TabEditor.swift.
// Every change is saved straight to tab.json.
use crate::analysis::Analysis;
use crate::score::Score;
use crate::song::{self, TabNote, FRETS, TUNING};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Selection {
    pub measure: usize,
    // Beats from the start of the measure, in sixteenth steps.
    pub slot: f64,
    pub string: i32,
}

#[derive(Default)]
pub struct Tab {
    pub notes: Vec<TabNote>,
    pub analysis: Option<Analysis>,
    pub score: Option<Score>,
    pub duration: f64,
    pub folder: Option<PathBuf>,
    pub editing: bool,
    pub selection: Option<Selection>,
    pub edited: bool,
    // A tab file exists, even an empty one, so the grid can be edited.
    pub has_tab: bool,
    // Regeneration is rewriting tab.json: editing is off and edits are refused.
    pub transcribing: bool,
    pub history: Vec<Vec<TabNote>>,
    pub pending_digit: Option<(i32, Instant, Selection)>,
    pub error: Option<String>,
    // Bumped whenever the notes change, so the synth can pick up the new ones.
    pub revision: u64,
}

impl Tab {
    pub fn set_notes(&mut self, notes: Vec<TabNote>) {
        self.notes = notes;
        self.revision += 1;
        self.rebuild_score();
    }

    pub fn set_analysis(&mut self, analysis: Option<Analysis>) {
        self.analysis = analysis;
        self.rebuild_score();
    }

    // Measures need the beat grid, so the score exists only once both the tab and the analysis do.
    fn rebuild_score(&mut self) {
        self.score = match &self.analysis {
            Some(a) if !a.downbeats.is_empty() && (self.has_tab || !self.notes.is_empty()) => Some(Score::new(&self.notes, a, self.duration)),
            _ => None,
        };
    }

    // Leaving edit mode always works; entering needs a grid and no regeneration running.
    pub fn toggle_editing(&mut self, position: f64) {
        self.pending_digit = None;
        if self.editing {
            self.editing = false;
            return;
        }
        let Some(score) = &self.score else { return };
        if self.transcribing {
            return;
        }
        self.editing = true;
        if self.selection.is_some() {
            return;
        }
        let measure = score.measure_index(position).unwrap_or(0);
        let first = score.measures.get(measure).and_then(|m| m.notes.first());
        self.selection = Some(Selection {
            measure,
            slot: first.map_or(0.0, |n| n.slot),
            string: first.map_or(1, |n| n.note.string),
        });
    }

    pub fn selected_note(&self) -> Option<TabNote> {
        let (score, sel) = (self.score.as_ref()?, self.selection?);
        score.measures.get(sel.measure)?.notes.iter().find(|n| n.slot == sel.slot && n.note.string == sel.string).map(|n| n.note)
    }

    pub fn move_selection_sixteenths(&mut self, sixteenths: i32) {
        let (Some(score), Some(mut sel)) = (&self.score, self.selection) else { return };
        let count = |m: usize| score.measures[m].beat_count() as f64;
        sel.slot += sixteenths as f64 / 4.0;
        while sel.slot < 0.0 && sel.measure > 0 {
            sel.measure -= 1;
            sel.slot += count(sel.measure);
        }
        while sel.slot >= count(sel.measure) && sel.measure + 1 < score.measures.len() {
            sel.slot -= count(sel.measure);
            sel.measure += 1;
        }
        sel.slot = sel.slot.max(0.0).min(count(sel.measure) - 0.25);
        self.selection = Some(sel);
        self.pending_digit = None;
    }

    pub fn move_selection_strings(&mut self, strings: i32) {
        if let Some(sel) = &mut self.selection {
            sel.string = (sel.string + strings).clamp(1, 6);
        }
        self.pending_digit = None;
    }

    // Two quick digits make a two-digit fret, so "1" then "2" is fret 12.
    pub fn type_digit(&mut self, digit: i32, now: Instant) {
        let Some(sel) = self.selection else { return };
        if let Some((value, time, at)) = self.pending_digit {
            if at == sel && now.duration_since(time) < Duration::from_millis(800) && value > 0 && value * 10 + digit <= FRETS {
                // One undo step for the whole two-digit fret: drop the entry the first digit pushed.
                let before_first_digit = self.history.pop();
                let depth = self.history.len();
                self.set_fret(value * 10 + digit);
                if let Some(before) = before_first_digit {
                    if self.history.len() > depth {
                        *self.history.last_mut().unwrap() = before;
                    } else {
                        self.history.push(before);
                    }
                }
                self.pending_digit = None;
                return;
            }
        }
        self.set_fret(digit);
        self.pending_digit = Some((digit, now, sel));
    }

    pub fn set_fret(&mut self, fret: i32) {
        let (Some(score), Some(sel)) = (&self.score, self.selection) else { return };
        if !(0..=FRETS).contains(&fret) {
            return;
        }
        let midi = TUNING[sel.string as usize - 1] + fret;
        let mut notes = self.notes.clone();
        if let Some(index) = self.selected_note().and_then(|old| notes.iter().position(|n| *n == old)) {
            let old = notes[index];
            notes[index] = TabNote { midi, fret, ..old };
        } else {
            let measure = &score.measures[sel.measure];
            let start = measure.time_at_beat(sel.slot);
            let end = measure.time_at_beat((sel.slot + 1.0).min(measure.beat_count() as f64));
            notes.push(TabNote { start, end, midi, string: sel.string, fret });
        }
        self.commit(notes);
    }

    pub fn delete_selected_note(&mut self) {
        let Some(old) = self.selected_note() else { return };
        let notes = self.notes.iter().copied().filter(|n| *n != old).collect();
        self.commit(notes);
    }

    // Same pitch on a neighbouring string: the fix the generated tab needs most often.
    pub fn move_selected_note(&mut self, strings: i32) {
        let Some(old) = self.selected_note() else { return };
        let Some(index) = self.notes.iter().position(|n| *n == old) else { return };
        let string = old.string + strings;
        if !(1..=6).contains(&string) {
            return;
        }
        let fret = old.midi - TUNING[string as usize - 1];
        if !(0..=FRETS).contains(&fret) {
            return;
        }
        // A chord can already hold a note on the target string at this slot.
        let (Some(score), Some(sel)) = (&self.score, self.selection) else { return };
        if score.measures[sel.measure].notes.iter().any(|n| n.slot == sel.slot && n.note.string == string) {
            return;
        }
        let mut notes = self.notes.clone();
        notes[index] = TabNote { string, fret, ..old };
        self.commit(notes);
        if let Some(sel) = &mut self.selection {
            sel.string = string;
        }
    }

    pub fn undo(&mut self) {
        let Some(previous) = self.history.pop() else { return };
        self.pending_digit = None;
        self.set_notes(previous);
        self.save();
    }

    fn commit(&mut self, mut notes: Vec<TabNote>) {
        if self.transcribing {
            return;
        }
        // Any edit ends a two-digit entry; type_digit sets the pending digit again after its own commit.
        self.pending_digit = None;
        self.history.push(self.notes.clone());
        if self.history.len() > 200 {
            self.history.remove(0);
        }
        notes.sort_by(|a, b| a.start.total_cmp(&b.start));
        self.set_notes(notes);
        self.edited = true;
        self.save();
    }

    fn save(&mut self) {
        let Some(folder) = &self.folder else { return };
        match song::save_tab(&folder.join("tab.json"), &self.notes) {
            Ok(()) => {
                self.has_tab = true;
                self.error = None;
            }
            Err(e) => self.error = Some(format!("Could not save the tab, so the latest edits exist only until the song is closed: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editing_session() {
        let folder = song::temp_dir("edit");
        let mut tab = Tab { folder: Some(folder.clone()), duration: 6.0, ..Default::default() };
        // 120 BPM in 4/4: beats every 0.5 s, bars every 2 s. One note: D on the G string, fret 7.
        tab.set_analysis(Some(Analysis { bpm: 120.0, beats: (0..12).map(|i| i as f64 * 0.5).collect(), downbeats: vec![0.0, 2.0, 4.0], chords: vec![] }));
        tab.set_notes(vec![TabNote { start: 0.0, end: 0.5, midi: 62, string: 3, fret: 7 }]);

        tab.toggle_editing(0.0);
        assert_eq!(tab.selection, Some(Selection { measure: 0, slot: 0.0, string: 3 }));

        // Same pitch one string lower: D string, fret 12.
        tab.move_selected_note(1);
        assert_eq!(tab.notes.iter().map(|n| [n.string, n.fret, n.midi]).collect::<Vec<_>>(), [[4, 12, 62]]);
        assert_eq!(tab.selection.unwrap().string, 4);

        // Two quick digits make fret 12 on the high e, an eighth note after the first note.
        tab.move_selection_sixteenths(2);
        tab.move_selection_strings(-3);
        let now = Instant::now();
        tab.type_digit(1, now);
        tab.type_digit(2, now + Duration::from_millis(300));
        assert_eq!(tab.notes.len(), 2);
        assert_eq!(tab.selected_note(), Some(TabNote { start: 0.25, end: 0.75, midi: 76, string: 1, fret: 12 }));

        // A slow second digit replaces the fret instead of extending it.
        tab.type_digit(3, now + Duration::from_secs(5));
        assert_eq!(tab.selected_note().unwrap().fret, 3);

        tab.delete_selected_note();
        assert_eq!(tab.notes.len(), 1);
        tab.undo();
        assert_eq!(tab.selected_note().unwrap().fret, 3);

        let saved = folder.join("tab.json");
        assert!(song::is_edited(&saved));
        assert_eq!(song::load_tab(&saved), tab.notes);

        // The cursor walks across the bar line.
        tab.selection = Some(Selection { measure: 0, slot: 3.75, string: 1 });
        tab.move_selection_sixteenths(1);
        assert_eq!(tab.selection, Some(Selection { measure: 1, slot: 0.0, string: 1 }));
        tab.move_selection_sixteenths(-1);
        assert_eq!(tab.selection, Some(Selection { measure: 0, slot: 3.75, string: 1 }));
        // Shift+Left/Right moves a whole beat.
        tab.move_selection_sixteenths(4);
        assert_eq!(tab.selection, Some(Selection { measure: 1, slot: 0.75, string: 1 }));
        std::fs::remove_dir_all(folder).ok();
    }

    fn tab_with(notes: Vec<TabNote>) -> (Tab, PathBuf) {
        let folder = song::temp_dir("edit");
        let mut tab = Tab { folder: Some(folder.clone()), duration: 6.0, ..Default::default() };
        tab.set_analysis(Some(Analysis { bpm: 120.0, beats: (0..12).map(|i| i as f64 * 0.5).collect(), downbeats: vec![0.0, 2.0, 4.0], chords: vec![] }));
        tab.set_notes(notes);
        (tab, folder)
    }

    #[test]
    fn two_digit_fret_is_one_undo_step_and_undo_clears_the_pending_digit() {
        let (mut tab, folder) = tab_with(vec![TabNote { start: 0.0, end: 0.5, midi: 62, string: 3, fret: 7 }]);
        tab.toggle_editing(0.0);
        tab.move_selection_sixteenths(2);
        let now = Instant::now();
        tab.type_digit(1, now);
        tab.type_digit(2, now + Duration::from_millis(300));
        assert_eq!(tab.selected_note().unwrap().fret, 12);
        // One undo removes the whole new note, not just its second digit.
        tab.undo();
        assert!(tab.selected_note().is_none());
        assert_eq!(tab.notes.len(), 1);

        tab.type_digit(1, now);
        tab.undo();
        tab.type_digit(2, now + Duration::from_millis(300));
        assert_eq!(tab.selected_note().unwrap().fret, 2);

        // A deletion between the digits ends the entry too: without it, 1, Delete, 2 would make fret 12.
        let later = now + Duration::from_secs(10);
        tab.type_digit(1, later);
        tab.delete_selected_note();
        tab.type_digit(2, later + Duration::from_millis(300));
        assert_eq!(tab.selected_note().unwrap().fret, 2);
        std::fs::remove_dir_all(folder).ok();
    }

    #[test]
    fn string_move_refuses_to_land_on_a_chord_note() {
        // A double-stop: E on the high e (open) and B on the B string (open) at the same slot.
        let chord = vec![TabNote { start: 0.0, end: 0.5, midi: 64, string: 1, fret: 0 }, TabNote { start: 0.0, end: 0.5, midi: 59, string: 2, fret: 0 }];
        let (mut tab, folder) = tab_with(chord.clone());
        tab.toggle_editing(0.0);
        tab.selection = Some(Selection { measure: 0, slot: 0.0, string: 1 });
        tab.move_selected_note(1);
        assert_eq!(tab.notes, chord);
        assert_eq!(tab.selection.unwrap().string, 1);
        std::fs::remove_dir_all(folder).ok();
    }

    #[test]
    fn empty_tab_can_be_edited_and_left() {
        let (mut tab, folder) = tab_with(vec![]);
        assert!(tab.score.is_none(), "no tab file, no grid");
        tab.has_tab = true;
        tab.set_notes(vec![]);
        tab.toggle_editing(0.0);
        assert!(tab.editing);
        tab.type_digit(5, Instant::now());
        assert_eq!(tab.notes.len(), 1);
        tab.delete_selected_note();
        assert!(tab.notes.is_empty());
        // With the last note gone the grid stays, so E (or Esc) still leaves edit mode.
        assert!(tab.score.is_some());
        tab.toggle_editing(0.0);
        assert!(!tab.editing);
        std::fs::remove_dir_all(folder).ok();
    }

    #[test]
    fn regeneration_keeps_editing_off_and_refuses_edits() {
        let (mut tab, folder) = tab_with(vec![TabNote { start: 0.0, end: 0.5, midi: 62, string: 3, fret: 7 }]);
        tab.transcribing = true;
        tab.toggle_editing(0.0);
        assert!(!tab.editing);
        tab.selection = Some(Selection { measure: 0, slot: 0.0, string: 3 });
        tab.set_fret(9);
        tab.delete_selected_note();
        assert_eq!(tab.notes[0].fret, 7);
        assert!(tab.history.is_empty());
        assert!(!folder.join("tab.json").exists());
        std::fs::remove_dir_all(folder).ok();
    }

    #[test]
    fn save_errors_are_reported() {
        let (mut tab, folder) = tab_with(vec![TabNote { start: 0.0, end: 0.5, midi: 62, string: 3, fret: 7 }]);
        tab.folder = Some(folder.join("gone"));
        tab.toggle_editing(0.0);
        tab.set_fret(9);
        assert!(tab.error.as_deref().unwrap().starts_with("Could not save the tab"));
        tab.folder = Some(folder.clone());
        tab.set_fret(10);
        assert!(tab.error.is_none(), "a later save that works clears the error");
        std::fs::remove_dir_all(folder).ok();
    }

    #[test]
    fn fret_out_of_range_on_the_new_string_is_refused() {
        let mut tab = Tab { duration: 4.0, ..Default::default() };
        tab.set_analysis(Some(Analysis { bpm: 120.0, beats: (0..8).map(|i| i as f64 * 0.5).collect(), downbeats: vec![0.0, 2.0], chords: vec![] }));
        // Open low E cannot move to the A string.
        tab.set_notes(vec![TabNote { start: 0.0, end: 0.5, midi: 40, string: 6, fret: 0 }]);
        tab.toggle_editing(0.0);
        tab.move_selected_note(-1);
        assert_eq!(tab.notes[0].string, 6);
        assert!(tab.history.is_empty());
    }
}
