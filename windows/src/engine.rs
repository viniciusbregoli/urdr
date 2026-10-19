// Runs the Python engine like Engine.swift: `uv run --project <dir> python <dir>/main.py <command> <in> <out>`,
// reading JSON lines from stdout and sending stderr to a log file.
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver};

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Progress(f64),
    Done(Vec<String>),
    Error(String),
}

pub fn parse_event(line: &str) -> Option<Event> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    match value.get("event")?.as_str()? {
        "progress" => Some(Event::Progress(value.get("fraction").and_then(|f| f.as_f64()).unwrap_or(0.0))),
        "done" => Some(Event::Done(
            value
                .get("stems")
                .and_then(|s| s.as_array())
                .map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        )),
        "error" => Some(Event::Error(value.get("message").and_then(|m| m.as_str()).unwrap_or("unknown error").into())),
        _ => None,
    }
}

pub enum Message {
    Progress(f64),
    Finished(Result<(), String>),
}

// shortcut: the engine is run from a checkout next to the exe or URDR_ENGINE_DIR, bundle it if the app is ever packaged.
pub fn directory() -> PathBuf {
    if let Some(dir) = std::env::var_os("URDR_ENGINE_DIR") {
        return dir.into();
    }
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default();
    exe_dir.join("engine")
}

// Starts the engine in the background; the receiver gets progress and then exactly one Finished.
pub fn run(command: &str, input: &Path, out: &Path, log: &Path) -> Receiver<Message> {
    let (tx, rx) = channel();
    let dir = directory();
    let mut cmd = Command::new("uv");
    cmd.arg("run").arg("--project").arg(&dir).arg("python").arg(dir.join("main.py")).arg(command).arg(input).arg(out);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let log = log.to_path_buf();
    std::thread::spawn(move || {
        let result = (|| {
            let stderr = std::fs::File::create(&log).map_err(|e| format!("cannot write {}: {e}", log.display()))?;
            let mut child = cmd
                .stdout(Stdio::piped())
                .stderr(stderr)
                .stdin(Stdio::null())
                .spawn()
                .map_err(|e| format!("could not start uv: {e}"))?;
            let mut failure = None;
            for line in BufReader::new(child.stdout.take().unwrap()).lines().map_while(Result::ok) {
                match parse_event(&line) {
                    Some(Event::Progress(f)) => {
                        let _ = tx.send(Message::Progress(f));
                    }
                    Some(Event::Error(m)) => failure = Some(m),
                    _ => {}
                }
            }
            let status = child.wait().map_err(|e| e.to_string())?;
            if status.success() {
                Ok(())
            } else {
                Err(failure.unwrap_or_else(|| format!("engine exited with {status}, see {}", log.display())))
            }
        })();
        let _ = tx.send(Message::Finished(result));
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_event_parsing() {
        assert_eq!(parse_event(r#"{"event": "progress", "stage": "separate", "fraction": 0.5}"#), Some(Event::Progress(0.5)));
        assert_eq!(parse_event(r#"{"event": "done", "stems": ["bass", "drums"]}"#), Some(Event::Done(vec!["bass".into(), "drums".into()])));
        assert_eq!(parse_event(r#"{"event": "error", "message": "boom"}"#), Some(Event::Error("boom".into())));
        assert_eq!(parse_event("Warning: not json"), None);
        assert_eq!(parse_event(r#"{"event": "unknown"}"#), None);
    }

    #[test]
    fn missing_engine_finishes_with_an_error() {
        let dir = crate::song::temp_dir("engine");
        std::env::set_var("URDR_ENGINE_DIR", dir.join("nowhere"));
        let rx = run("tab", &dir.join("in.wav"), &dir.join("tab.json"), &dir.join("tab.log"));
        let finished = rx.iter().find_map(|m| match m {
            Message::Finished(r) => Some(r),
            Message::Progress(_) => None,
        });
        assert!(matches!(finished, Some(Err(_))));
        std::fs::remove_dir_all(dir).ok();
    }
}
