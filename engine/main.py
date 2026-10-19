"""Urdr engine: one subcommand per job, progress as JSON lines on stdout."""

import argparse
import json
import sys
from pathlib import Path


def emit(event: str, **fields) -> None:
    print(json.dumps({"event": event, **fields}), flush=True)


def separate(audio: Path, out: Path) -> None:
    import torch
    from demucs.api import Separator, save_audio

    device = "mps" if torch.backends.mps.is_available() else "cpu"

    def on_segment(info: dict) -> None:
        # Fires per segment; segment_offset and audio_length are both in samples.
        if info["state"] == "end":
            emit("progress", stage="separate", fraction=min(info["segment_offset"] / info["audio_length"], 1.0))

    # The 6-source model adds guitar and piano stems; the guitar stem feeds the tab command.
    separator = Separator(model="htdemucs_6s", device=device, callback=on_segment)
    _, stems = separator.separate_audio_file(audio)
    out.mkdir(parents=True, exist_ok=True)
    for name, wav in stems.items():
        save_audio(wav, out / f"{name}.wav", samplerate=separator.samplerate)
    emit("done", stems=sorted(stems))


def tab(audio: Path, out: Path) -> None:
    import tab as tablature

    emit("progress", stage="tab", fraction=0.0)
    notes = tablature.tab(audio)
    out.write_text(json.dumps({"tuning": tablature.TUNING, "notes": notes}))
    emit("done", notes=len(notes))


def analyze(audio: Path, out: Path) -> None:
    import analysis

    emit("progress", stage="analyze", fraction=0.0)
    result = analysis.analyze(audio)
    out.write_text(json.dumps(result))
    emit("done", bpm=result["bpm"], chords=len(result["chords"]))


def main() -> int:
    parser = argparse.ArgumentParser(prog="urdr-engine")
    sub = parser.add_subparsers(dest="command", required=True)
    sep = sub.add_parser("separate")
    sep.add_argument("audio", type=Path)
    sep.add_argument("out", type=Path)
    tab_parser = sub.add_parser("tab")
    tab_parser.add_argument("audio", type=Path)
    tab_parser.add_argument("out", type=Path)
    analyze_parser = sub.add_parser("analyze")
    analyze_parser.add_argument("audio", type=Path)
    analyze_parser.add_argument("out", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "separate":
            separate(args.audio, args.out)
        elif args.command == "tab":
            tab(args.audio, args.out)
        elif args.command == "analyze":
            analyze(args.audio, args.out)
    except Exception as exc:  # the app shows this message; the traceback goes to stderr
        emit("error", message=str(exc))
        raise
    return 0


if __name__ == "__main__":
    sys.exit(main())
