"""Audio to guitar tab: Basic Pitch notes, the top melodic line, then a playable string/fret path."""

from pathlib import Path

# Open-string MIDI pitches, tab order: string 1 is high e, string 6 is low E (standard tuning).
TUNING = [64, 59, 55, 50, 45, 40]
FRETS = 22
OPEN_PENALTY = 1.5
STRETCH = 0.5
MOVE = 0.3


def transcribe(audio: Path) -> list[dict]:
    from basic_pitch.inference import predict

    # 75-1400 Hz spans a 22-fret guitar in standard tuning.
    _, _, events = predict(audio, minimum_frequency=75, maximum_frequency=1400)
    return [{"start": float(s), "end": float(e), "midi": int(p), "amp": float(a)} for s, e, p, a, _ in events]


def top_line(notes: list[dict], chord_window: float = 0.03) -> list[dict]:
    """Keep the highest note of each onset cluster and cut it off where the next one starts.

    A solo is usually the top voice, so this drops rhythm parts and harmonics under it.
    """
    line: list[dict] = []
    for note in sorted(notes, key=lambda n: n["start"]):
        if line and note["start"] - line[-1]["start"] < chord_window:
            if note["midi"] > line[-1]["midi"]:
                line[-1] = dict(note)
            continue
        # A lower note that starts while the previous one still rings is an inner voice.
        if line and note["start"] < line[-1]["end"] and note["midi"] < line[-1]["midi"]:
            continue
        if line:
            line[-1]["end"] = min(line[-1]["end"], note["start"])
        line.append(dict(note))
    return line


def positions(midi: int) -> list[tuple[int, int]]:
    return [(s + 1, midi - open_) for s, open_ in enumerate(TUNING) if 0 <= midi - open_ <= FRETS]


def assign_frets(notes: list[dict]) -> list[dict]:
    """Viterbi over fretting-hand positions: notes inside a 4-fret span are free, moving the hand costs."""
    hands = range(1, FRETS - 2)

    def fingering(midi: int, hand: int) -> tuple[float, tuple[int, int]] | None:
        options = []
        for string, fret in positions(midi):
            if fret == 0:
                options.append((OPEN_PENALTY, (string, fret)))
            elif hand <= fret <= hand + 3:
                options.append((0.0, (string, fret)))
            elif fret in (hand - 1, hand + 4):
                options.append((STRETCH, (string, fret)))
        return min(options, default=None)

    notes = [n for n in notes if positions(n["midi"])]
    if not notes:
        return []
    inf = float("inf")
    # A small bias towards lower positions breaks ties the way most players finger a phrase.
    best = {h: (f[0] + 0.01 * h if (f := fingering(notes[0]["midi"], h)) else inf) for h in hands}
    back: list[dict[int, int]] = []
    for note in notes[1:]:
        step, pointers = {}, {}
        for h in hands:
            f = fingering(note["midi"], h)
            prev = min(hands, key=lambda p: best[p] + MOVE * abs(p - h))
            step[h] = best[prev] + MOVE * abs(prev - h) + (f[0] + 0.01 * h if f else inf)
            pointers[h] = prev
        best = step
        back.append(pointers)
    hand = min(hands, key=best.__getitem__)
    path = [hand]
    for pointers in reversed(back):
        hand = pointers[hand]
        path.append(hand)
    path.reverse()
    out = []
    for note, hand in zip(notes, path):
        string, fret = fingering(note["midi"], hand)[1]
        out.append({**note, "string": string, "fret": fret})
    return out


def tab(audio: Path) -> list[dict]:
    return assign_frets(top_line(transcribe(audio)))


if __name__ == "__main__":
    # Self-check, run with: uv run python tab.py
    chord = [{"start": 0.0, "end": 1.0, "midi": 60}, {"start": 0.01, "end": 1.0, "midi": 64},
             {"start": 0.5, "end": 0.8, "midi": 55}, {"start": 1.0, "end": 1.5, "midi": 62}]
    line = top_line(chord)
    assert [n["midi"] for n in line] == [64, 62], line
    assert line[0]["end"] == 1.0

    # A minor pentatonic box at the 5th fret should stay in position instead of jumping to open strings.
    box = [45, 48, 50, 52, 55, 57, 60, 62, 64, 67, 69, 72]
    frets = assign_frets([{"start": i, "end": i + 1, "midi": m} for i, m in enumerate(box)])
    assert all(5 <= n["fret"] <= 8 for n in frets), [(n["string"], n["fret"]) for n in frets]
    assert assign_frets([{"start": 0, "end": 1, "midi": 30}]) == []
    print("ok")
