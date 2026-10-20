"""Full mix to beats, downbeats and major/minor chords: Beat This! for the beat grid, madmom's CNN + CRF for chords."""

from pathlib import Path

import numpy as np


def chord_label(label: str) -> str:
    # madmom writes majors as "C:maj"; lead sheets just say "C".
    return label.removesuffix(":maj")


SHARPS = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
FLATS = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"]
# Diatonic triads of a major key as (root offset from the tonic, is minor).
DIATONIC = {(0, False), (2, True), (4, True), (5, False), (7, False), (9, True)}
MAJOR_SCALE = {0, 2, 4, 5, 7, 9, 11}
FLAT_KEYS = {5, 10, 3, 8, 1}  # F, Bb, Eb, Ab, Db major and their relative minors


def spell_chords(chords: list[dict]) -> list[dict]:
    """Spell chord roots the way a lead sheet in the song's key would.

    In-key roots follow the key signature. Out-of-key roots are borrowed chords: in a major key
    they are flat degrees (bII, bIII, bV, bVI, bVII: Bb and Eb in C, never A# and D#); in a minor
    key they are raised degrees (the leading tone G# in A minor) except the Neapolitan bII.
    """

    def parse(label: str) -> tuple[int, bool] | None:
        root, _, quality = label.partition(":")
        return (SHARPS.index(root), quality == "min") if root in SHARPS else None

    scores = [0.0] * 12
    for chord in chords:
        if (parsed := parse(chord["label"])) is None:
            continue
        root, minor = parsed
        for tonic in range(12):
            if ((root - tonic) % 12, minor) in DIATONIC:
                scores[tonic] += chord["end"] - chord["start"]
    if max(scores, default=0) == 0:
        return chords
    major = scores.index(max(scores))

    def time_on(root: int, minor: bool) -> float:
        return sum(c["end"] - c["start"] for c in chords if parse(c["label"]) == (root, minor))

    # The relative minor is the key when its tonic chord sounds longer than the major one.
    minor_key = time_on((major + 9) % 12, True) > time_on(major, False)

    def name(root: int) -> str:
        if (root - major) % 12 in MAJOR_SCALE:
            return (FLATS if major in FLAT_KEYS else SHARPS)[root]
        degree = (root - (major + 9 if minor_key else major)) % 12
        return SHARPS[root] if minor_key and degree != 1 else FLATS[root]

    out = []
    for chord in chords:
        root, sep, quality = chord["label"].partition(":")
        out.append({**chord, "label": (name(SHARPS.index(root)) if root in SHARPS else root) + sep + quality})
    return out


def regular_downbeats(beats: list[float], downbeats: list[float], beats_per_bar: int = 4) -> list[float]:
    """Keep the tracker's bars unless they are implausibly short, as on drumless clips; then bar every 4 beats."""
    if len(downbeats) < 2 or len(beats) < 2 * beats_per_bar:
        return downbeats
    counts = [sum(1 for b in beats if start - 0.02 <= b < end - 0.02) for start, end in zip(downbeats, downbeats[1:])]
    if np.median(counts) >= 3:
        return downbeats
    first = int(np.argmin([abs(b - downbeats[0]) for b in beats]))
    return beats[first::beats_per_bar]


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
    beats = [round(float(t), 3) for t in beats]
    return {
        "bpm": bpm(beats),
        "beats": beats,
        "downbeats": regular_downbeats(beats, [round(float(t), 3) for t in downbeats]),
        "chords": spell_chords([{"start": round(float(s), 3), "end": round(float(e), 3), "label": chord_label(str(l))}
                                for s, e, l in segments]),
    }


if __name__ == "__main__":
    # Self-check, run with: uv run python analysis.py
    assert [chord_label(l) for l in ["C:maj", "A:min", "F#:maj", "N"]] == ["C", "A:min", "F#", "N"]
    assert bpm([0.0, 0.5, 1.0, 1.5, 2.0]) == 120.0
    assert bpm([0.0, 0.5, 1.0, 1.25, 1.5, 2.0, 2.5]) == 120.0, "one extra beat must not move the tempo"
    assert bpm([1.0]) == 0.0

    def seq(*labels):
        return [{"start": float(i), "end": float(i + 1), "label": l} for i, l in enumerate(labels)]

    # F major: A#/D# become Bb/Eb, minor qualities and "N" survive.
    flat = [c["label"] for c in spell_chords(seq("F", "D:min", "A#", "C", "D#", "G:min", "N"))]
    assert flat == ["F", "D:min", "Bb", "C", "Eb", "G:min", "N"], flat
    # C major with borrowed chords: bVII and bIII are Bb and Eb, never A# and D#.
    borrowed = [c["label"] for c in spell_chords(seq("C", "F", "G", "A#", "C", "D#", "G", "C"))]
    assert borrowed == ["C", "F", "G", "Bb", "C", "Eb", "G", "C"], borrowed
    # A minor: the leading-tone G# stays sharp, the Neapolitan bII is Bb.
    minor = [c["label"] for c in spell_chords(seq("A:min", "G#", "A:min", "D:min", "E", "A:min", "A#", "A:min"))]
    assert minor == ["A:min", "G#", "A:min", "D:min", "E", "A:min", "Bb", "A:min"], minor
    assert spell_chords([]) == []

    # Drumless clip: the tracker marks a downbeat every 1-2 beats, so bars are rebuilt every 4 beats.
    beats = [i * 0.5 for i in range(16)]
    assert regular_downbeats(beats, [0.0, 0.5, 1.5, 2.0, 3.0, 3.5]) == [0.0, 2.0, 4.0, 6.0]
    # A plausible 4/4 grid is kept as it is.
    assert regular_downbeats(beats, [0.0, 2.0, 4.0, 6.0]) == [0.0, 2.0, 4.0, 6.0]
    print("ok")
