"""Score tab.py against GuitarSet solo annotations.

uv run python eval_guitarset.py <guitarset dir with annotation/ and audio/> [limit]
"""

import json
import sys
from pathlib import Path

import numpy as np
from mir_eval.transcription import match_notes, precision_recall_f1_overlap

import tab


def ground_truth(jams: Path) -> list[dict]:
    notes = []
    for annotation in json.loads(jams.read_text())["annotations"]:
        if annotation["namespace"] != "note_midi":
            continue
        # data_source 0 is the low E string; tab.py numbers strings from the high e.
        string = 6 - int(annotation["annotation_metadata"]["data_source"])
        for obs in annotation["data"]:
            midi = round(obs["value"])
            notes.append({"start": obs["time"], "end": obs["time"] + obs["duration"], "midi": midi,
                          "string": string, "fret": midi - tab.TUNING[string - 1]})
    return sorted(notes, key=lambda n: n["start"])


def arrays(notes: list[dict]) -> tuple[np.ndarray, np.ndarray]:
    intervals = np.array([[n["start"], max(n["end"], n["start"] + 0.01)] for n in notes]).reshape(-1, 2)
    hz = np.array([440.0 * 2 ** ((n["midi"] - 69) / 12) for n in notes])
    return intervals, hz


def main() -> None:
    root = Path(sys.argv[1])
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else None
    files = sorted((root / "annotation").glob("*_solo.jams"))[:limit]
    f1s, tab_hits, matched, oracle_hits, oracle_total = [], 0, 0, 0, 0
    for jams in files:
        truth = ground_truth(jams)
        predicted = tab.tab(root / "audio" / jams.name.replace(".jams", "_mic.wav"))
        ref, est = arrays(truth), arrays(predicted)
        _, _, f1, _ = precision_recall_f1_overlap(*ref, *est, onset_tolerance=0.05, offset_ratio=None)
        pairs = match_notes(*ref, *est, onset_tolerance=0.05, offset_ratio=None)
        hits = sum((truth[r]["string"], truth[r]["fret"]) == (predicted[e]["string"], predicted[e]["fret"])
                   for r, e in pairs)
        oracle = tab.assign_frets([{k: n[k] for k in ("start", "end", "midi")} for n in truth])
        o_hits = sum(t["string"] == o["string"] for t, o in zip(truth, oracle))
        f1s.append(f1)
        tab_hits, matched = tab_hits + hits, matched + len(pairs)
        oracle_hits, oracle_total = oracle_hits + o_hits, oracle_total + len(truth)
        print(f"{jams.stem:32s} note F1 {f1:.2f}  fingering {hits}/{len(pairs)}  oracle {o_hits}/{len(truth)}", flush=True)
    print(f"\n{len(files)} solos: mean note F1 {np.mean(f1s):.3f}, "
          f"fingering on correct notes {tab_hits / max(matched, 1):.3f}, "
          f"fingering from true pitches {oracle_hits / max(oracle_total, 1):.3f}")


if __name__ == "__main__":
    main()
