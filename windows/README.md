# Urdr for Windows

A Rust port of the Mac app (`app/`): six-stem mixer with waveforms, chord lane, speed and pitch, A-B loop, the Songsterr-style tab with hand editing, and a plucked-string synth guitar. It reads and writes the same song folders as the Mac app, so a folder copied from `~/Library/Application Support/Urdr/Songs/` into `%APPDATA%\Urdr\Songs\` opens and works as is.

Built with egui (eframe, OpenGL), cpal (WASAPI output), hound (WAV) and signalsmith-stretch (speed and pitch).

## Build and test

On Windows, with Rust and LLVM installed (bindgen needs libclang for the stretcher):

```
cargo test
cargo build --release      # target\release\urdr.exe
```

From any Linux machine with Docker, without installing Rust (the cross build targets `x86_64-pc-windows-gnu` and links libstdc++ statically, so the exe needs no extra DLLs):

```
./build.sh test            # unit tests, native Linux target
./build.sh exe             # dist/urdr.exe
./build.sh linux           # dist/urdr-linux, same code on X11
```

The cargo registry and target dir live in the Docker volumes `urdr-cargo` and `urdr-target`, so rebuilds are incremental.

## Use

- `O` opens an audio file, `Shift+O` opens a song folder, `urdr.exe <file or folder>` opens one from the command line.
- `?` lists every shortcut, `Ctrl+K` opens the command palette. The keys match the Mac app with Cmd mapped to Ctrl.
- Separating stems, transcribing the tab and finding chords run the Python engine (`uv run --project <engine> python <engine>/main.py ...`) from `URDR_ENGINE_DIR` or an `engine` folder next to the exe. The engine only runs on macOS for now, so on Windows open song folders made on the Mac.
