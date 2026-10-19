import SwiftUI

// Current/next chord readout over a chord and beat lane that scrolls under the same playhead as TabView.
struct ChordsView: View {
    let analysis: Analysis
    let position: Double
    // Same playhead and scale as TabView so the two lanes line up.
    private let pointsPerSecond = 250.0
    private let playheadFraction = 0.25
    private let laneHeight = 44.0

    private var current: Int? { analysis.chordIndex(at: position) }

    private var next: Analysis.Chord? {
        let after = current.map { analysis.chords[$0].end } ?? position
        return analysis.chords.first { $0.start >= after && $0.label != "N" }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline, spacing: 16) {
                Text(current.map { analysis.chords[$0].name } ?? "-")
                    .font(.system(size: 44, weight: .bold))
                Text(next.map { "then \($0.name)" } ?? "")
                    .font(.title2)
                    .foregroundStyle(.secondary)
            }
            lane
        }
        .accessibilityElement(children: .combine)
    }

    private var lane: some View {
        Canvas { context, size in
            let playhead = size.width * playheadFraction
            let from = position - playhead / pointsPerSecond
            let to = position + (size.width - playhead) / pointsPerSecond
            func x(_ time: Double) -> Double { playhead + (time - position) * pointsPerSecond }

            let downbeats = Set(analysis.downbeats)
            for beat in analysis.beats where beat >= from && beat <= to {
                let strong = downbeats.contains(beat)
                var tick = Path()
                tick.move(to: CGPoint(x: x(beat), y: laneHeight))
                tick.addLine(to: CGPoint(x: x(beat), y: laneHeight - (strong ? 16 : 6)))
                context.stroke(tick, with: .color(.secondary.opacity(strong ? 0.9 : 0.5)), lineWidth: strong ? 2 : 1)
            }
            for (index, chord) in analysis.chords.enumerated() where chord.end >= from && chord.start <= to {
                let label = context.resolve(Text(chord.name)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(index == current ? Color.accentColor : chord.label == "N" ? .secondary : .primary))
                context.draw(label, at: CGPoint(x: x(chord.start) + 3, y: 2), anchor: .topLeading)
                var edge = Path()
                edge.move(to: CGPoint(x: x(chord.start), y: 0))
                edge.addLine(to: CGPoint(x: x(chord.start), y: laneHeight))
                context.stroke(edge, with: .color(.secondary.opacity(0.3)), lineWidth: 1)
            }
            var head = Path()
            head.move(to: CGPoint(x: playhead, y: 0))
            head.addLine(to: CGPoint(x: playhead, y: laneHeight))
            context.stroke(head, with: .color(.accentColor), lineWidth: 1.5)
        }
        .frame(height: laneHeight)
    }
}
