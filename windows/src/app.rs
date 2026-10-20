// App state, commands and keyboard handling. Mirrors ContentView.swift and AppModel.swift; views live in views.rs.
use crate::analysis::Analysis;
use crate::audio::{self, Player};
use crate::editor::Tab;
use crate::engine::{self, Message};
use crate::song;
use eframe::egui::{self, Event, Key};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Mixer,
    Tab,
}

pub enum Status {
    Idle,
    Separating(f64),
    Ready,
    Failed(String),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    OpenAudio,
    OpenFolder,
    PlayPause,
    Skip(i32),
    Bar(bool),
    LoopStart,
    LoopEnd,
    ToggleLoop,
    ClearLoop,
    Speed(i32),
    ResetSpeed,
    Pitch(i32),
    ResetPitch,
    GenerateTab,
    ToggleSynth,
    NextTone,
    ToggleEdit,
    SwitchView,
    Help,
    Palette,
    Mute(usize),
    Solo(usize),
    MoveNote(i32),
    DeleteNote,
    Undo,
    Nothing,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Trigger {
    Char(char),
    Key(Key),
    Shift(Key),
    None,
}

pub struct Command {
    pub title: String,
    pub hint: &'static str,
    pub trigger: Trigger,
    pub action: Action,
}

pub struct Palette {
    pub query: String,
    pub selected: usize,
    pub focused: bool,
}

pub struct App {
    pub player: Arc<Mutex<Player>>,
    stream: Option<cpal::Stream>,
    pub audio_error: Option<String>,
    last_frame: Instant,
    pub tab: Tab,
    pub title: Option<String>,
    pub folder: Option<PathBuf>,
    pub status: Status,
    pub notice: Option<String>,
    separation: Option<(Receiver<Message>, PathBuf, PathBuf)>,
    analysis_job: Option<(Receiver<Message>, PathBuf)>,
    tab_job: Option<(Receiver<Message>, PathBuf)>,
    pub analyzing: bool,
    pub analysis_error: Option<String>,
    pub view: View,
    pub palette: Option<Palette>,
    pub help: bool,
    pub drag_from: Option<f64>,
    synth_revision: u64,
    pub last_row: Option<usize>,
    shifted_digit: bool,
}

const DIGIT_KEYS: [Key; 9] = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9];

impl App {
    // Open the audio device only from a thread that lives as long as the process (the UI thread). cpal's WASAPI
    // backend caches its device enumerator in the COM apartment of the first thread that asks for a device, so
    // opening one from a short-lived thread, such as a test thread, leaves the next caller a dead COM object.
    pub fn new(open_audio: bool) -> App {
        let player = Arc::new(Mutex::new(Player::new(48000)));
        let (stream, audio_error) = match open_audio.then(|| audio::start_output(player.clone())) {
            Some(Ok(s)) => (Some(s), None),
            Some(Err(e)) => (None, Some(e)),
            None => (None, None),
        };
        App {
            player,
            stream,
            audio_error,
            last_frame: Instant::now(),
            tab: Tab::default(),
            title: None,
            folder: None,
            status: Status::Idle,
            notice: None,
            separation: None,
            analysis_job: None,
            tab_job: None,
            analyzing: false,
            analysis_error: None,
            view: View::Mixer,
            palette: None,
            help: false,
            drag_from: None,
            synth_revision: 0,
            last_row: None,
            shifted_digit: false,
        }
    }

    pub fn ready(&self) -> bool {
        matches!(self.status, Status::Ready)
    }

    // `urdr <song>`: a path that is not there is reported instead of leaving the app idle.
    pub fn open_argument(&mut self, path: &Path) {
        if path.exists() {
            self.open_path(path);
        } else {
            self.status = Status::Failed(format!("Could not open {}: file not found", path.display()));
        }
    }

    pub fn open_path(&mut self, path: &Path) {
        if path.is_dir() {
            self.load_folder(path, None);
        } else {
            self.open_audio(path);
        }
    }

    fn open_audio(&mut self, audio: &Path) {
        let folder = song::library_root().join(song::folder_name(audio));
        let stems = folder.join("stems");
        // htdemucs_6s always yields 6 stems, so fewer means an interrupted run.
        if song::stems(&stems).len() >= 6 {
            self.load_folder(&folder, Some(audio));
            return;
        }
        if let Err(e) = std::fs::create_dir_all(&stems) {
            self.status = Status::Failed(format!("Could not create {}: {e}", stems.display()));
            return;
        }
        self.player.lock().unwrap().pause();
        self.status = Status::Separating(0.0);
        let rx = engine::run("separate", audio, &stems, &stems.join("engine.log"));
        self.separation = Some((rx, folder, audio.to_path_buf()));
    }

    pub fn load_folder(&mut self, folder: &Path, audio: Option<&Path>) {
        let found = song::stems(&folder.join("stems"));
        if found.is_empty() {
            self.status = Status::Failed(format!(
                "No stems in {}. A song folder holds stems/*.wav, analysis.json and tab.json, as the Mac app makes them.",
                folder.join("stems").display()
            ));
            return;
        }
        let mut stems = vec![];
        let mut rate = 0;
        for (name, path) in found {
            match audio::read_wav(&path) {
                Ok((samples, r)) => {
                    rate = r;
                    stems.push((name, samples));
                }
                Err(e) => {
                    self.status = Status::Failed(format!("Could not read a stem: {e}"));
                    return;
                }
            }
        }
        let duration = {
            let mut player = self.player.lock().unwrap();
            player.load(stems, rate);
            player.duration()
        };
        self.folder = Some(folder.to_path_buf());
        let tab_path = folder.join("tab.json");
        self.tab = Tab {
            duration,
            folder: Some(folder.to_path_buf()),
            edited: song::is_edited(&tab_path),
            has_tab: tab_path.exists(),
            // A regeneration still running for the previous song keeps edits off, like the Mac's transcribing flag.
            transcribing: self.tab_job.is_some(),
            // Keeps counting across songs, so the synth always notices the new song's notes.
            revision: self.tab.revision,
            ..Default::default()
        };
        self.tab.set_notes(song::load_tab(&tab_path));
        self.tab.set_analysis(Analysis::load(&folder.join("analysis.json")));
        self.title = Some(audio.and_then(|a| a.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| song::title_for_folder(folder)));
        self.analysis_error = None;
        self.analyzing = false;
        self.analysis_job = None;
        self.last_row = None;
        self.status = Status::Ready;
        if self.tab.analysis.is_none() {
            match audio {
                // Runs on the full mix while the stems play; a result for a song that is no longer open is dropped.
                Some(audio) => {
                    let out = folder.join("analysis.json");
                    self.analysis_job = Some((engine::run("analyze", audio, &out, &folder.join("analysis.log")), folder.to_path_buf()));
                    self.analyzing = true;
                }
                None => self.analysis_error = Some("this folder has no analysis.json, and making one needs the engine and the original audio".into()),
            }
        }
    }

    fn generate_tab(&mut self) {
        let Some(folder) = self.folder.clone() else { return };
        if self.tab_job.is_some() {
            return;
        }
        if let Err(e) = song::backup_edited_tab(&folder) {
            self.tab.error = Some(format!("Could not back up the edited tab: {e}"));
            return;
        }
        self.tab.error = None;
        // Editing is off while the engine rewrites tab.json, so no edit can land between the backup and the new tab.
        self.tab.editing = false;
        self.tab.selection = None;
        self.tab.pending_digit = None;
        self.tab.transcribing = true;
        let rx = engine::run("tab", &folder.join("stems").join("guitar.wav"), &folder.join("tab.json"), &folder.join("tab.log"));
        self.tab_job = Some((rx, folder));
    }

    fn poll_jobs(&mut self) {
        if let Some((rx, folder, audio)) = &self.separation {
            let mut finished = None;
            for m in rx.try_iter() {
                match m {
                    Message::Progress(f) => self.status = Status::Separating(f),
                    Message::Finished(r) => finished = Some(r),
                }
            }
            if let Some(result) = finished {
                let (folder, audio) = (folder.clone(), audio.clone());
                self.separation = None;
                match result {
                    Ok(()) => self.load_folder(&folder, Some(&audio)),
                    Err(e) => {
                        self.status = Status::Failed(format!(
                            "Stem separation needs the Urdr engine, which does not run on Windows yet ({e}). \
                             Song folders made on the Mac open directly: press Shift+O or use Open folder."
                        ))
                    }
                }
            }
        }
        if let Some((rx, folder)) = &self.analysis_job {
            if let Some(result) = rx.try_iter().find_map(|m| if let Message::Finished(r) = m { Some(r) } else { None }) {
                let folder = folder.clone();
                self.analysis_job = None;
                if self.folder.as_deref() == Some(folder.as_path()) {
                    self.analyzing = false;
                    match result {
                        Ok(()) => self.tab.set_analysis(Analysis::load(&folder.join("analysis.json"))),
                        Err(e) => self.analysis_error = Some(format!("needs the Urdr engine ({e})")),
                    }
                }
            }
        }
        if let Some((rx, folder)) = &self.tab_job {
            if let Some(result) = rx.try_iter().find_map(|m| if let Message::Finished(r) = m { Some(r) } else { None }) {
                let folder = folder.clone();
                self.tab_job = None;
                self.tab.transcribing = false;
                // The file is already in its own song folder; only the state of a song that is still open changes.
                if self.folder.as_deref() != Some(folder.as_path()) {
                    return;
                }
                match result {
                    Ok(()) => {
                        self.tab.has_tab = true;
                        self.tab.set_notes(song::load_tab(&folder.join("tab.json")));
                        self.tab.edited = false;
                        self.tab.history.clear();
                    }
                    Err(e) => self.tab.error = Some(format!("Transcription needs the Urdr engine, which does not run on Windows yet ({e})")),
                }
            }
        }
    }

    #[cfg(windows)]
    fn pick(&mut self, folder: bool) -> Option<PathBuf> {
        let dialog = rfd::FileDialog::new();
        if folder {
            dialog.set_directory(song::library_root()).set_title("Open song folder").pick_folder()
        } else {
            dialog.set_title("Open audio file").add_filter("Audio", &["wav", "mp3", "m4a", "flac", "aiff", "aif", "ogg"]).pick_file()
        }
    }

    #[cfg(not(windows))]
    fn pick(&mut self, _folder: bool) -> Option<PathBuf> {
        self.notice = Some("This build has no file dialog; pass a song folder or audio file on the command line.".into());
        None
    }

    pub fn run(&mut self, action: Action) {
        let position = self.player.lock().unwrap().position();
        match action {
            Action::OpenAudio => {
                if let Some(p) = self.pick(false) {
                    self.open_path(&p)
                }
            }
            Action::OpenFolder => {
                if let Some(p) = self.pick(true) {
                    self.load_folder(&p, None)
                }
            }
            Action::GenerateTab => self.generate_tab(),
            Action::ToggleEdit => self.tab.toggle_editing(position),
            Action::SwitchView => self.view = if self.view == View::Mixer { View::Tab } else { View::Mixer },
            Action::Help => self.help = true,
            Action::Palette => self.palette = Some(Palette { query: String::new(), selected: 0, focused: false }),
            Action::MoveNote(d) => self.tab.move_selected_note(d),
            Action::DeleteNote => self.tab.delete_selected_note(),
            Action::Undo => self.tab.undo(),
            Action::Bar(forward) => {
                if let Some(t) = self.tab.analysis.as_ref().and_then(|a| a.downbeat(position, forward)) {
                    self.player.lock().unwrap().seek(t)
                }
            }
            Action::Nothing => {}
            _ => {
                let mut p = self.player.lock().unwrap();
                match action {
                    Action::PlayPause => p.toggle_play(),
                    Action::Skip(s) => p.skip(s as f64),
                    Action::LoopStart => p.set_loop_start(),
                    Action::LoopEnd => p.set_loop_end(),
                    Action::ToggleLoop => p.loop_on = !p.loop_on,
                    Action::ClearLoop => p.clear_loop(),
                    Action::Speed(d) => p.change_rate(d as f32 * 0.05),
                    Action::ResetSpeed => p.rate = 1.0,
                    Action::Pitch(d) => p.change_pitch(d),
                    Action::ResetPitch => p.semitones = 0,
                    Action::ToggleSynth => p.toggle_synth(),
                    Action::NextTone => p.cycle_tone(),
                    Action::Mute(i) => p.toggle_mute(i),
                    Action::Solo(i) => p.toggle_solo(i),
                    _ => {}
                }
            }
        }
    }

    // Same list and keys as ContentView.commands, with Cmd mapped to Ctrl.
    pub fn commands(&self) -> Vec<Command> {
        let p = self.player.lock().unwrap();
        let c = |title: &str, hint: &'static str, trigger: Trigger, action: Action| Command { title: title.into(), hint, trigger, action };
        let mut list = vec![
            c("Open audio file", "O", Trigger::Char('o'), Action::OpenAudio),
            c("Open song folder", "Shift+O", Trigger::Char('O'), Action::OpenFolder),
            c("Play / pause", "Space", Trigger::Key(Key::Space), Action::PlayPause),
            c("Back 5 seconds", "Left", Trigger::Key(Key::ArrowLeft), Action::Skip(-5)),
            c("Forward 5 seconds", "Right", Trigger::Key(Key::ArrowRight), Action::Skip(5)),
            c("Previous bar", "Up", Trigger::Key(Key::ArrowUp), Action::Bar(false)),
            c("Next bar", "Down", Trigger::Key(Key::ArrowDown), Action::Bar(true)),
            c("Set loop start", "[", Trigger::Char('['), Action::LoopStart),
            c("Set loop end", "]", Trigger::Char(']'), Action::LoopEnd),
            c("Toggle loop", "L", Trigger::Char('l'), Action::ToggleLoop),
            c("Clear loop", "\\", Trigger::Char('\\'), Action::ClearLoop),
            c("Slower", ",", Trigger::Char(','), Action::Speed(-1)),
            c("Faster", ".", Trigger::Char('.'), Action::Speed(1)),
            c("Reset speed", "/", Trigger::Char('/'), Action::ResetSpeed),
            c("Pitch down", "-", Trigger::Char('-'), Action::Pitch(-1)),
            c("Pitch up", "=", Trigger::Char('='), Action::Pitch(1)),
            c("Reset pitch", "0", Trigger::Char('0'), Action::ResetPitch),
            c("Generate guitar tab", "T", Trigger::Char('t'), Action::GenerateTab),
            c(if p.synth_on { "Original guitar" } else { "Synth guitar" }, "Y", Trigger::Char('y'), Action::ToggleSynth),
            c("Next synth tone", "U", Trigger::Char('u'), Action::NextTone),
            c(if self.tab.editing { "Stop editing tab" } else { "Edit tab" }, "E", Trigger::Char('e'), Action::ToggleEdit),
            c(if self.view == View::Mixer { "Show tab" } else { "Show mixer" }, "V", Trigger::Char('v'), Action::SwitchView),
            c("Show shortcuts", "?", Trigger::Char('?'), Action::Help),
        ];
        const DIGITS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];
        const SHIFT_DIGITS: [&str; 9] = ["Shift+1", "Shift+2", "Shift+3", "Shift+4", "Shift+5", "Shift+6", "Shift+7", "Shift+8", "Shift+9"];
        for (i, stem) in p.stems.iter().take(9).enumerate() {
            list.push(c(&format!("Mute {}", stem.name), DIGITS[i], Trigger::Char((b'1' + i as u8) as char), Action::Mute(i)));
            list.push(c(&format!("Solo {}", stem.name), SHIFT_DIGITS[i], Trigger::Shift(DIGIT_KEYS[i]), Action::Solo(i)));
        }
        if self.tab.editing {
            // Handled by edit_key; listed here so the help overlay and palette show them.
            list.extend([
                c("Type a fret at the cursor", "0-22", Trigger::None, Action::Nothing),
                c("Move the edit cursor", "Arrows", Trigger::None, Action::Nothing),
                c("Move the cursor a beat", "Shift+Left/Right", Trigger::None, Action::Nothing),
                c("Move note to the string above", "Shift+Up", Trigger::None, Action::MoveNote(-1)),
                c("Move note to the string below", "Shift+Down", Trigger::None, Action::MoveNote(1)),
                c("Delete note", "Delete", Trigger::None, Action::DeleteNote),
                c("Undo edit", "Ctrl+Z", Trigger::None, Action::Undo),
            ]);
        }
        list
    }

    pub fn palette_matches(&self) -> Vec<Command> {
        let query = self.palette.as_ref().map(|p| p.query.to_lowercase()).unwrap_or_default();
        self.commands().into_iter().filter(|c| c.title.to_lowercase().contains(&query)).collect()
    }

    // Takes the key events this app owns out of the frame's input, so widgets never see them twice.
    pub fn handle_keys(&mut self, ctx: &egui::Context) {
        // A clicked button keeps focus in egui, and Space would press it again: only the palette's field may keep focus.
        if self.palette.is_none() || !ctx.wants_keyboard_input() {
            if let Some(id) = ctx.memory(|m| m.focused()) {
                ctx.memory_mut(|m| m.surrender_focus(id));
            }
        }
        let typing = ctx.wants_keyboard_input();
        let events = ctx.input_mut(|i| std::mem::take(&mut i.events));
        let mut rest = Vec::with_capacity(events.len());
        for event in events {
            if !self.handle_event(&event, typing) {
                rest.push(event);
            }
        }
        ctx.input_mut(|i| i.events = rest);
    }

    fn handle_event(&mut self, event: &Event, typing: bool) -> bool {
        if self.palette.is_some() {
            let Event::Key { key, pressed: true, .. } = event else { return false };
            let matches: Vec<Action> = self.palette_matches().iter().map(|c| c.action).collect();
            let palette = self.palette.as_mut().unwrap();
            match key {
                Key::ArrowDown => palette.selected = (palette.selected + 1).min(matches.len().saturating_sub(1)),
                Key::ArrowUp => palette.selected = palette.selected.saturating_sub(1),
                Key::Escape => self.palette = None,
                Key::Enter => {
                    let action = matches.get(palette.selected).copied();
                    self.palette = None;
                    if let Some(a) = action {
                        self.run(a);
                    }
                }
                _ => return false,
            }
            return true;
        }
        if self.help {
            match event {
                Event::Key { key: Key::Escape, pressed: true, .. } => self.help = false,
                Event::Text(t) if t == "?" => self.help = false,
                Event::Key { .. } | Event::Text(_) => {}
                _ => return false,
            }
            return true;
        }
        if typing {
            return false;
        }
        match event {
            Event::Key { key, pressed: true, modifiers, .. } => {
                if modifiers.command {
                    let action = match key {
                        Key::K => Action::Palette,
                        Key::O if modifiers.shift => Action::OpenFolder,
                        Key::O => Action::OpenAudio,
                        Key::Z if self.tab.editing => Action::Undo,
                        _ => return false,
                    };
                    self.run(action);
                    return true;
                }
                // Shift+digit solos by key, since the character it types depends on the keyboard layout.
                let trigger = if modifiers.shift && DIGIT_KEYS.contains(key) {
                    self.shifted_digit = true;
                    Trigger::Shift(*key)
                } else {
                    Trigger::Key(*key)
                };
                if self.tab.editing && trigger == Trigger::Key(*key) && self.edit_key(*key, modifiers.shift) {
                    return true;
                }
                let found = self.commands().into_iter().find(|c| c.trigger == trigger).map(|c| c.action);
                match found {
                    Some(action) => {
                        self.run(action);
                        true
                    }
                    None => false,
                }
            }
            Event::Text(text) => {
                // The character a Shift+digit types depends on the layout; the key event already soloed.
                if std::mem::take(&mut self.shifted_digit) {
                    return true;
                }
                for ch in text.chars() {
                    // Space arrives as a key event too.
                    if ch == ' ' {
                        continue;
                    }
                    if self.tab.editing {
                        if let Some(d) = ch.to_digit(10) {
                            self.tab.type_digit(d as i32, Instant::now());
                            continue;
                        }
                    }
                    let commands = self.commands();
                    let found = commands
                        .iter()
                        .find(|c| c.trigger == Trigger::Char(ch))
                        .or_else(|| commands.iter().find(|c| c.trigger == Trigger::Char(ch.to_ascii_lowercase())))
                        .map(|c| c.action);
                    if let Some(action) = found {
                        self.run(action);
                    }
                }
                true
            }
            _ => false,
        }
    }

    // Edit mode takes over arrows, delete and Esc; everything else falls through to the normal shortcuts.
    fn edit_key(&mut self, key: Key, shift: bool) -> bool {
        match key {
            Key::ArrowLeft => self.tab.move_selection_sixteenths(if shift { -4 } else { -1 }),
            Key::ArrowRight => self.tab.move_selection_sixteenths(if shift { 4 } else { 1 }),
            Key::ArrowUp if shift => self.tab.move_selected_note(-1),
            Key::ArrowUp => self.tab.move_selection_strings(-1),
            Key::ArrowDown if shift => self.tab.move_selected_note(1),
            Key::ArrowDown => self.tab.move_selection_strings(1),
            Key::Delete | Key::Backspace => self.tab.delete_selected_note(),
            Key::Escape => self.tab.toggle_editing(0.0),
            _ => return false,
        }
        true
    }

    // Per-frame upkeep: jobs, synth notes, and a clock for machines without an audio device.
    pub fn tick(&mut self) {
        self.poll_jobs();
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame).as_secs_f64().min(0.25);
        self.last_frame = now;
        let mut p = self.player.lock().unwrap();
        if self.tab.revision != self.synth_revision {
            p.synth_notes = self.tab.notes.clone();
            self.synth_revision = self.tab.revision;
        }
        if self.stream.is_none() && p.playing {
            // shortcut: without a device the player is rendered into nothing to keep time; it is what Wine in CI gets.
            let mut scratch = vec![0.0f32; ((elapsed * 48000.0) as usize).max(1) * 2];
            p.render(&mut scratch, 2);
        }
    }

    pub fn busy(&self) -> bool {
        self.separation.is_some() || self.analysis_job.is_some() || self.tab_job.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(t: &str) -> Event {
        Event::Text(t.into())
    }

    fn key(key: Key, modifiers: egui::Modifiers) -> Event {
        Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
    }

    fn app_with_song() -> App {
        let mut app = App::new(false);
        app.load_folder(&song_folder(), None);
        app
    }

    #[test]
    fn regeneration_result_for_another_song_is_dropped() {
        let mut app = app_with_song();
        let first = app.folder.clone().unwrap();
        app.tab.toggle_editing(0.0);
        // The engine is not installed here, so the job fails; its error must not land on the next song.
        app.run(Action::GenerateTab);
        assert!(!app.tab.editing, "regeneration turns editing off");
        app.run(Action::ToggleEdit);
        assert!(!app.tab.editing, "and keeps it off while it runs");
        app.load_folder(&song_folder(), None);
        assert_ne!(app.folder.as_ref(), Some(&first));
        while app.tab_job.is_some() {
            std::thread::sleep(std::time::Duration::from_millis(10));
            app.poll_jobs();
        }
        assert!(app.tab.error.is_none());
        assert!(!app.tab.transcribing);
        assert_eq!(app.tab.notes.len(), 1);
    }

    #[test]
    fn shift_digit_solos_a_stem() {
        let mut app = app_with_song();
        app.handle_event(&key(Key::Num2, egui::Modifiers::SHIFT), false);
        app.handle_event(&text("@"), false);
        let p = app.player.lock().unwrap();
        assert!(p.stems[1].solo);
        assert!(!p.stems[1].muted, "the typed character does not also mute");
        assert_eq!(crate::audio::gains(&p.stems), [0.0, 1.0]);
    }

    #[test]
    fn missing_command_line_path_is_reported() {
        let mut app = App::new(false);
        let missing = song::temp_dir("arg").join("nope.wav");
        app.open_argument(&missing);
        match &app.status {
            Status::Failed(m) => assert_eq!(m, &format!("Could not open {}: file not found", missing.display())),
            _ => panic!("a missing path must show an error"),
        }
        app.open_argument(&song_folder());
        assert!(app.ready());
    }

    #[test]
    fn synth_follows_the_newly_opened_song() {
        let mut app = app_with_song();
        app.tick();
        assert_eq!(app.player.lock().unwrap().synth_notes[0].fret, 7);
        let second = song_folder();
        std::fs::write(second.join("tab.json"), r#"{"notes":[{"start":1,"end":1.5,"midi":69,"string":1,"fret":5}]}"#).unwrap();
        app.load_folder(&second, None);
        app.tick();
        let p = app.player.lock().unwrap();
        assert_eq!(p.synth_notes.len(), 1);
        assert_eq!(p.synth_notes[0].fret, 5, "the synth plays the new song's tab");
    }

    fn song_folder() -> PathBuf {
        let dir = song::temp_dir("app");
        std::fs::create_dir_all(dir.join("stems")).unwrap();
        let spec = hound::WavSpec { channels: 2, sample_rate: 44100, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        for name in ["vocals", "guitar"] {
            let mut w = hound::WavWriter::create(dir.join("stems").join(format!("{name}.wav")), spec).unwrap();
            for _ in 0..44100 * 8 * 2 {
                w.write_sample(0i16).unwrap();
            }
            w.finalize().unwrap();
        }
        std::fs::write(dir.join("analysis.json"), r#"{"bpm":120,"beats":[0,0.5,1,1.5,2,2.5,3,3.5,4,4.5,5,5.5],"downbeats":[0,2,4],"chords":[{"start":0,"end":2,"label":"A:min"}]}"#).unwrap();
        std::fs::write(dir.join("tab.json"), r#"{"tuning":[64,59,55,50,45,40],"notes":[{"start":0,"end":0.5,"midi":62,"string":3,"fret":7}]}"#).unwrap();
        dir
    }

    #[test]
    fn opening_a_mac_song_folder_loads_everything() {
        let app = app_with_song();
        assert!(app.ready());
        let p = app.player.lock().unwrap();
        assert_eq!(p.stems.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["vocals", "guitar"]);
        assert!((p.duration() - 8.0).abs() < 1e-9);
        assert_eq!(app.tab.score.as_ref().unwrap().measures.len(), 3);
        assert!(app.analysis_error.is_none());
    }

    #[test]
    fn single_key_shortcuts_follow_the_mac() {
        let mut app = app_with_song();
        let none = egui::Modifiers::NONE;
        app.handle_event(&key(Key::Space, none), false);
        assert!(app.player.lock().unwrap().playing);
        app.handle_event(&text(" "), false);
        assert!(app.player.lock().unwrap().playing, "the Space text event does not toggle twice");
        app.handle_event(&key(Key::Space, none), false);
        app.handle_event(&key(Key::ArrowRight, none), false);
        assert_eq!(app.player.lock().unwrap().position(), 5.0);
        app.handle_event(&key(Key::ArrowUp, none), false);
        assert_eq!(app.player.lock().unwrap().position(), 4.0, "previous bar");
        for t in [".", ".", "=", "2", "["] {
            app.handle_event(&text(t), false);
        }
        {
            let p = app.player.lock().unwrap();
            assert!((p.rate - 1.1).abs() < 1e-6);
            assert_eq!(p.semitones, 1);
            assert!(p.stems[1].muted);
            assert_eq!(p.loop_start, Some(4.0));
        }
        app.handle_event(&text("v"), false);
        assert_eq!(app.view, View::Tab);
        // Typing in a text field leaves single keys alone.
        assert!(!app.handle_event(&text("v"), true));
        assert_eq!(app.view, View::Tab);
        app.handle_event(&key(Key::K, egui::Modifiers::COMMAND), false);
        assert!(app.palette.is_some());
        app.palette.as_mut().unwrap().query = "reset sp".into();
        app.handle_event(&key(Key::Enter, none), false);
        assert!(app.palette.is_none());
        assert_eq!(app.player.lock().unwrap().rate, 1.0);
        app.handle_event(&text("?"), false);
        assert!(app.help);
        app.handle_event(&key(Key::Escape, none), false);
        assert!(!app.help);
    }

    #[test]
    fn edit_mode_takes_digits_and_arrows() {
        let mut app = app_with_song();
        let none = egui::Modifiers::NONE;
        app.handle_event(&text("e"), false);
        assert!(app.tab.editing);
        app.handle_event(&key(Key::ArrowDown, egui::Modifiers::SHIFT), false);
        assert_eq!((app.tab.notes[0].string, app.tab.notes[0].fret), (4, 12));
        app.handle_event(&text("5"), false);
        assert_eq!(app.tab.notes[0].fret, 5);
        assert!(!app.player.lock().unwrap().stems.get(4).is_some_and(|s| s.muted), "digits type frets, not mutes");
        app.handle_event(&key(Key::Z, egui::Modifiers::COMMAND), false);
        assert_eq!(app.tab.notes[0].fret, 12);
        app.handle_event(&key(Key::Escape, none), false);
        assert!(!app.tab.editing);
        assert!(song::is_edited(&app.folder.as_ref().unwrap().join("tab.json")));
    }
}
