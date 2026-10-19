import Foundation

// Tab notes laid out in measures from the beat grid, the way Songsterr shows them.
struct Score: Equatable {
    struct Note: Equatable {
        let note: AppModel.TabNote
        // Position in beats from the start of the measure, snapped to sixteenths.
        let slot: Double
        // Length in sixteenths up to the next note, which drives the rhythm stems and beams.
        let sixteenths: Int
    }

    struct Chord: Equatable {
        let slot: Double
        let name: String
    }

    struct Measure: Equatable {
        let number: Int
        // Beat onsets inside the measure followed by the measure end, so there are beatCount + 1 entries.
        let beats: [Double]
        var notes: [Note] = []
        var chords: [Chord] = []

        var start: Double { beats[0] }
        var end: Double { beats[beats.count - 1] }
        var beatCount: Int { beats.count - 1 }

        func beatPosition(at time: Double) -> Double {
            guard let j = beats.indices.dropLast().last(where: { beats[$0] <= time }) else { return 0 }
            return Double(j) + min(max((time - beats[j]) / (beats[j + 1] - beats[j]), 0), 1)
        }

        func time(atBeat position: Double) -> Double {
            let j = min(max(Int(position), 0), beatCount - 1)
            return beats[j] + (position - Double(j)) * (beats[j + 1] - beats[j])
        }
    }

    let measures: [Measure]

    init(notes: [AppModel.TabNote], analysis: Analysis, duration: Double) {
        let beatLength = analysis.bpm > 0 ? 60 / analysis.bpm : 0.5
        var bounds = analysis.downbeats
        // Notes before the first downbeat go in a pickup measure.
        if let first = bounds.first, first > 0.05, notes.contains(where: { $0.start < first }) {
            bounds.insert(max(0, first - beatLength * 4), at: 0)
        }
        bounds.append(max(duration, (bounds.last ?? 0) + beatLength))
        var measures: [Measure] = zip(bounds, bounds.dropFirst()).enumerated().map { index, pair in
            let inside = analysis.beats.filter { $0 > pair.0 + 0.02 && $0 < pair.1 - 0.02 }
            return Measure(number: index + 1, beats: [pair.0] + inside + [pair.1])
        }
        guard !measures.isEmpty else {
            self.measures = []
            return
        }

        func locate(_ time: Double) -> (measure: Int, slot: Double) {
            let m = measures.lastIndex(where: { $0.start <= time }) ?? 0
            var slot = (measures[m].beatPosition(at: time) * 4).rounded() / 4
            // Snapping can push a late note onto the next downbeat.
            if slot >= Double(measures[m].beatCount), m + 1 < measures.count { return (m + 1, 0) }
            slot = min(slot, Double(measures[m].beatCount) - 0.25)
            return (m, slot)
        }

        let placed = notes.sorted { $0.start < $1.start }.map { ($0, locate($0.start)) }
        // Global sixteenth index of each note, used to measure the gap to the next one.
        let offsets = measures.reduce(into: [0]) { $0.append($0.last! + $1.beatCount * 4) }
        let indices = placed.map { offsets[$0.1.measure] + Int($0.1.slot * 4) }
        for (i, (note, where_)) in placed.enumerated() {
            let gap = i + 1 < indices.count ? indices[i + 1] - indices[i] : 4
            let own = Int((note.end - note.start) / beatLength * 4 + 0.5)
            measures[where_.measure].notes.append(Note(note: note, slot: where_.slot, sixteenths: max(1, min(gap, max(own, 1), 16))))
        }
        for chord in analysis.chords where chord.label != "N" {
            let where_ = locate(chord.start)
            measures[where_.measure].chords.append(Chord(slot: where_.slot, name: chord.name))
        }
        self.measures = measures
    }

    func measureIndex(at time: Double) -> Int? {
        measures.lastIndex { $0.start <= time && time < $0.end }
    }
}
