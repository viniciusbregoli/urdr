// Song folder layout shared with the Mac app: <library>/Songs/<name>-<hash>/{stems/*.wav, analysis.json, tab.json}.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TabNote {
    pub start: f64,
    pub end: f64,
    pub midi: i32,
    pub string: i32,
    pub fret: i32,
}

// Mirrors engine/tab.py TUNING: open-string MIDI pitches, string 1 is the high e.
pub const TUNING: [i32; 6] = [64, 59, 55, 50, 45, 40];
pub const FRETS: i32 = 22;

// Display order; any extra stem the engine produces is appended alphabetically.
pub const STEM_ORDER: [&str; 6] = ["vocals", "guitar", "drums", "bass", "piano", "other"];

pub fn library_root() -> PathBuf {
    match std::env::var_os("APPDATA") {
        Some(appdata) => PathBuf::from(appdata).join("Urdr").join("Songs"),
        None => PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share/Urdr/Songs"),
    }
}

// The path hash keeps two different files with the same name apart. Same djb2 as AppModel.folderName.
pub fn folder_name(audio: &Path) -> String {
    let path = audio.to_string_lossy();
    let hash = path.bytes().fold(5381u64, |h, b| (h << 5).wrapping_add(h).wrapping_add(b as u64));
    let stem = audio.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    format!("{stem}-{hash:x}")
}

// "jazz-solo-1ceb9390f01a9699" shows as "jazz-solo".
pub fn title_for_folder(folder: &Path) -> String {
    let name = folder.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    match name.rsplit_once('-') {
        Some((title, hash)) if !title.is_empty() && !hash.is_empty() && hash.chars().all(|c| c.is_ascii_hexdigit()) => title.into(),
        _ => name,
    }
}

pub fn stems(folder: &Path) -> Vec<(String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("wav")))
        .filter_map(|p| Some((p.file_stem()?.to_string_lossy().into_owned(), p)))
        .collect();
    let rank = |name: &str| STEM_ORDER.iter().position(|s| *s == name).unwrap_or(STEM_ORDER.len());
    found.sort_by(|a, b| (rank(&a.0), &a.0).cmp(&(rank(&b.0), &b.0)));
    found
}

pub fn load_tab(path: &Path) -> Vec<TabNote> {
    #[derive(Deserialize)]
    struct File {
        notes: Vec<TabNote>,
    }
    std::fs::read(path)
        .ok()
        .and_then(|data| serde_json::from_slice::<File>(&data).ok())
        .map(|f| f.notes)
        .unwrap_or_default()
}

pub fn is_edited(path: &Path) -> bool {
    #[derive(Deserialize)]
    struct File {
        edited: Option<bool>,
    }
    std::fs::read(path)
        .ok()
        .and_then(|data| serde_json::from_slice::<File>(&data).ok())
        .and_then(|f| f.edited)
        == Some(true)
}

pub fn save_tab(path: &Path, notes: &[TabNote]) -> std::io::Result<()> {
    #[derive(Serialize)]
    struct File<'a> {
        tuning: [i32; 6],
        notes: &'a [TabNote],
        edited: bool,
    }
    let data = serde_json::to_vec(&File { tuning: TUNING, notes, edited: true })?;
    // Write then rename, like the Mac's atomic write, so a crash never leaves half a tab.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(tmp, path)
}

// Regenerating must never cost the user their hand edits.
pub fn backup_edited_tab(folder: &Path) -> std::io::Result<()> {
    let tab = folder.join("tab.json");
    if is_edited(&tab) {
        std::fs::copy(&tab, folder.join("tab.edited.json"))?;
    }
    Ok(())
}

#[cfg(test)]
pub fn temp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!("urdr-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_name_separates_same_named_files() {
        let a = folder_name(Path::new("/a/song.mp3"));
        let b = folder_name(Path::new("/b/song.mp3"));
        assert_ne!(a, b);
        assert!(a.starts_with("song-"));
        // Same djb2 as the Mac app, so the value is pinned.
        let hash = "/a/song.mp3".bytes().fold(5381u64, |h, c| h.wrapping_mul(33).wrapping_add(c as u64));
        assert_eq!(a, format!("song-{hash:x}"));
        assert_eq!(title_for_folder(Path::new("/x/jazz-solo-1ceb9390f01a9699")), "jazz-solo");
        assert_eq!(title_for_folder(Path::new("/x/my-song")), "my-song");
    }

    #[test]
    fn load_tab_reads_engine_output() {
        let dir = temp_dir("tab");
        let path = dir.join("tab.json");
        std::fs::write(&path, r#"{"tuning": [64], "notes": [{"start": 1.5, "end": 2, "midi": 69, "amp": 0.4, "string": 1, "fret": 5}]}"#).unwrap();
        assert_eq!(load_tab(&path), vec![TabNote { start: 1.5, end: 2.0, midi: 69, string: 1, fret: 5 }]);
        assert!(load_tab(&dir.join("missing.json")).is_empty());
        assert!(!is_edited(&path));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn saved_tab_is_marked_edited_and_backed_up() {
        let dir = temp_dir("save");
        let path = dir.join("tab.json");
        let notes = vec![TabNote { start: 0.25, end: 0.75, midi: 76, string: 1, fret: 12 }];
        backup_edited_tab(&dir).unwrap();
        assert!(!dir.join("tab.edited.json").exists(), "nothing to back up yet");
        save_tab(&path, &notes).unwrap();
        assert!(is_edited(&path));
        assert_eq!(load_tab(&path), notes);
        let json: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(json["tuning"], serde_json::json!([64, 59, 55, 50, 45, 40]));
        backup_edited_tab(&dir).unwrap();
        assert_eq!(load_tab(&dir.join("tab.edited.json")), notes);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn stems_are_ordered_for_display() {
        let dir = temp_dir("stems");
        for name in ["bass", "other", "vocals", "drums", "guitar", "zither"] {
            std::fs::write(dir.join(format!("{name}.wav")), b"").unwrap();
        }
        std::fs::write(dir.join("engine.log"), b"").unwrap();
        let names: Vec<String> = stems(&dir).into_iter().map(|s| s.0).collect();
        assert_eq!(names, ["vocals", "guitar", "drums", "bass", "other", "zither"]);
        std::fs::remove_dir_all(dir).ok();
    }
}
