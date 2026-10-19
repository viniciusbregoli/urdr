"""Full mix to beats, downbeats and major/minor chords: Beat This! for the beat grid, madmom's CNN + CRF for chords."""

from pathlib import Path

import numpy as np


def chord_label(label: str) -> str:
    # madmom writes majors as "C:maj"; lead sheets just say "C".
    return label.removesuffix(":maj")


def bpm(beats: list[float]) -> float:
    if len(beats) < 2:
        return 0.0
    # Averaging intervals near the median drops missed or doubled beats and the 20 ms frame rounding of each beat.
    gaps = np.diff(beats)
    median = np.median(gaps)
    return round(60 / float(gaps[abs(gaps - median) < 0.2 * median].mean()), 1)


def analyze(audio: Path) -> dict:
    from beat_this.inference import Audio2Beats
    from madmom.audio.signal import Signal
    from madmom.features.chords import CNNChordFeatureProcessor, CRFChordRecognitionProcessor

    signal = Signal(str(audio), sample_rate=44100, num_channels=1, dtype=np.float32)
    beats, downbeats = Audio2Beats(checkpoint_path="final0", device="cpu")(signal, signal.sample_rate)
    segments = CRFChordRecognitionProcessor()(CNNChordFeatureProcessor()(signal))
    return {
        "bpm": bpm(beats),
        "beats": [round(float(t), 3) for t in beats],
        "downbeats": [round(float(t), 3) for t in downbeats],
        "chords": [{"start": round(float(s), 3), "end": round(float(e), 3), "label": chord_label(str(l))}
                   for s, e, l in segments],
    }


if __name__ == "__main__":
    # Self-check, run with: uv run python analysis.py
    assert [chord_label(l) for l in ["C:maj", "A:min", "F#:maj", "N"]] == ["C", "A:min", "F#", "N"]
    assert bpm([0.0, 0.5, 1.0, 1.5, 2.0]) == 120.0
    assert bpm([0.0, 0.5, 1.0, 1.25, 1.5, 2.0, 2.5]) == 120.0, "one extra beat must not move the tempo"
    assert bpm([1.0]) == 0.0
    print("ok")
