import Foundation

// The song as a lead sheet: one cell per bar with the chords played in it, as in Moises' chord view.
struct ChordSheet: Equatable {
    struct Chord: Equatable {
        let name: String
        // Where in the bar it starts, from 0 (the downbeat) to 1.
        let at: Double
    }

    struct Bar: Equatable {
        let number: Int
        let start: Double
        let end: Double
        // Beat onsets inside the bar, the first one being the downbeat.
        let beats: [Double]
        // The chord sounding at the downbeat first, then each change inside the bar.
        let chords: [Chord]
    }

    let bars: [Bar]

    init(analysis: Analysis, duration: Double) {
        var bounds = analysis.downbeats
        // A pickup before the first downbeat gets its own cell so the opening chord is not lost.
        if let first = bounds.first, first > 0.5 { bounds.insert(0, at: 0) }
        bounds.append(max(duration, (bounds.last ?? 0) + 0.1))
        bars = zip(bounds, bounds.dropFirst()).enumerated().map { index, pair in
            let (start, end) = pair
            let beats = [start] + analysis.beats.filter { $0 > start + 0.02 && $0 < end - 0.02 }
            // Chord and beat timings jitter by a frame or two, so a change within 150 ms of a downbeat belongs to it.
            let snap = 0.15
            var chords: [Chord] = []
            if let sounding = analysis.chordIndex(at: start + snap) { chords.append(Chord(name: analysis.chords[sounding].name, at: 0)) }
            for chord in analysis.chords where chord.start > start + snap && chord.start < end - snap {
                chords.append(Chord(name: chord.name, at: (chord.start - start) / (end - start)))
            }
            chords = chords.reduce(into: []) { if $0.last?.name != $1.name { $0.append($1) } }
            if chords.count > 1 { chords.removeAll { $0.name == "N.C." } }
            return Bar(number: index + 1, start: start, end: end, beats: beats, chords: chords)
        }
    }

    func barIndex(at time: Double) -> Int? {
        bars.lastIndex { $0.start <= time && time < $0.end }
    }

    // The sounding chord followed by the next distinct ones, skipping "no chord" stretches.
    static func upcoming(_ analysis: Analysis, at time: Double, count: Int) -> [Analysis.Chord] {
        let from = analysis.chordIndex(at: time) ?? analysis.chords.firstIndex { $0.start > time } ?? analysis.chords.count
        var result: [Analysis.Chord] = []
        for chord in analysis.chords[from...] where chord.label != "N" {
            if result.last?.name != chord.name { result.append(chord) }
            if result.count == count { break }
        }
        return result
    }
}

// A guitar fingering for a major or minor chord, strings from low E to high e; nil is a muted string.
struct ChordShape: Equatable {
    let frets: [Int?]

    // The fret the diagram starts at: 1 for chords near the nut, otherwise the lowest fretted note.
    var baseFret: Int {
        let fretted = frets.compactMap { $0 }.filter { $0 > 0 }
        return (fretted.max() ?? 0) <= 4 ? 1 : fretted.min()!
    }

    private static let roots = ["C": 0, "C#": 1, "Db": 1, "D": 2, "D#": 3, "Eb": 3, "E": 4, "F": 5, "F#": 6,
                                "Gb": 6, "G": 7, "G#": 8, "Ab": 8, "A": 9, "A#": 10, "Bb": 10, "B": 11]
    private static let open: [String: [Int?]] = [
        "C": [nil, 3, 2, 0, 1, 0], "D": [nil, nil, 0, 2, 3, 2], "E": [0, 2, 2, 1, 0, 0],
        "G": [3, 2, 0, 0, 0, 3], "A": [nil, 0, 2, 2, 2, 0],
        "Am": [nil, 0, 2, 2, 1, 0], "Dm": [nil, nil, 0, 2, 3, 1], "Em": [0, 2, 2, 0, 0, 0],
    ]

    // Takes the engine label ("Bb", "F#:min"); open shapes where guitarists use them, otherwise the lower barre.
    static func shape(for label: String) -> ChordShape? {
        let parts = label.split(separator: ":")
        guard let first = parts.first, let root = roots[String(first)] else { return nil }
        let minor = parts.count > 1 && parts[1] == "min"
        if let frets = open[String(first) + (minor ? "m" : "")] { return ChordShape(frets: frets) }
        let eFret = (root - 4 + 12) % 12, aFret = (root - 9 + 12) % 12
        if eFret <= aFret {
            let f = eFret
            return ChordShape(frets: minor ? [f, f + 2, f + 2, f, f, f] : [f, f + 2, f + 2, f + 1, f, f])
        }
        let f = aFret
        return ChordShape(frets: minor ? [nil, f, f + 2, f + 2, f + 1, f] : [nil, f, f + 2, f + 2, f + 2, f])
    }
}
