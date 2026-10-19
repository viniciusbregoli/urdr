import Foundation

// The engine's analysis.json: tempo, beat grid and chord segments of the full mix.
struct Analysis: Decodable, Equatable {
    struct Chord: Decodable, Equatable {
        let start: Double
        let end: Double
        let label: String

        // "A:min" reads as "Am" on a lead sheet; "N" is the engine's no-chord label.
        var name: String { label == "N" ? "N.C." : label.replacingOccurrences(of: ":min", with: "m") }
    }

    let bpm: Double
    let beats: [Double]
    let downbeats: [Double]
    let chords: [Chord]

    static func load(_ url: URL) -> Analysis? {
        guard let data = try? Data(contentsOf: url) else { return nil }
        return try? JSONDecoder().decode(Analysis.self, from: data)
    }

    func chordIndex(at time: Double) -> Int? {
        chords.firstIndex { $0.start <= time && time < $0.end }
    }

    // Going back skips a downbeat that was just passed, so repeated presses while playing keep moving.
    func downbeat(from time: Double, forward: Bool) -> Double? {
        forward ? downbeats.first { $0 > time + 0.01 } : downbeats.last { $0 < time - 0.25 }
    }
}
