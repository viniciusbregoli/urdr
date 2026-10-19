import SwiftUI

// Songsterr-style tab: rows of measures with chords above, rhythm below and a cursor that follows playback.
struct ScoreView: View {
    let score: Score
    let position: Double
    let seek: (Double) -> Void

    private let lineGap = 13.0
    private let staffTop = 36.0
    private let rowHeight = 140.0
    private let labelWidth = 22.0
    private let minMeasureWidth = 260.0

    var body: some View {
        GeometryReader { geometry in
            let perRow = max(1, Int((geometry.size.width - labelWidth) / minMeasureWidth))
            let measureWidth = (geometry.size.width - labelWidth) / Double(perRow)
            let rows = stride(from: 0, to: score.measures.count, by: perRow).map { start in
                Array(score.measures[start..<min(start + perRow, score.measures.count)])
            }
            let currentRow = score.measureIndex(at: position).map { $0 / perRow }
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(rows.indices, id: \.self) { index in
                            row(rows[index], measureWidth: measureWidth).id(index)
                        }
                    }
                }
                .onChange(of: currentRow) { _, row in
                    if let row { withAnimation { proxy.scrollTo(row, anchor: .center) } }
                }
            }
        }
        .accessibilityLabel("Guitar tab score, \(score.measures.count) measures")
    }

    private func padding(_ measure: Score.Measure) -> Double { measure.number == 1 ? 34 : 14 }

    private func x(_ beat: Double, in measure: Score.Measure, at index: Int, width: Double) -> Double {
        let start = labelWidth + Double(index) * width + padding(measure)
        return start + beat / Double(measure.beatCount) * (width - padding(measure) - 10)
    }

    private func y(_ string: Int) -> Double { staffTop + Double(string - 1) * lineGap }

    private func row(_ measures: [Score.Measure], measureWidth: Double) -> some View {
        Canvas { context, size in
            let end = labelWidth + Double(measures.count) * measureWidth
            for (string, name) in ["e", "B", "G", "D", "A", "E"].enumerated() {
                context.draw(Text(name).font(.caption2.monospaced()).foregroundStyle(.secondary),
                             at: CGPoint(x: labelWidth / 2, y: y(string + 1)))
                stroke(&context, from: CGPoint(x: labelWidth, y: y(string + 1)), to: CGPoint(x: end, y: y(string + 1)), .secondary.opacity(0.5))
            }
            for (index, measure) in measures.enumerated() {
                draw(measure, at: index, width: measureWidth, in: &context)
            }
            stroke(&context, from: CGPoint(x: end, y: y(1)), to: CGPoint(x: end, y: y(6)), .secondary)
        }
        .frame(height: rowHeight)
        .onTapGesture { location in
            let index = Int((location.x - labelWidth) / measureWidth)
            guard measures.indices.contains(index) else { return }
            let measure = measures[index]
            let left = x(0, in: measure, at: index, width: measureWidth)
            let right = x(Double(measure.beatCount), in: measure, at: index, width: measureWidth)
            let beat = (location.x - left) / (right - left) * Double(measure.beatCount)
            seek(measure.time(atBeat: min(max(beat, 0), Double(measure.beatCount))))
        }
    }

    private func draw(_ measure: Score.Measure, at index: Int, width: Double, in context: inout GraphicsContext) {
        let left = labelWidth + Double(index) * width
        stroke(&context, from: CGPoint(x: left, y: y(1)), to: CGPoint(x: left, y: y(6)), .secondary)
        context.draw(Text("\(measure.number)").font(.caption2).foregroundStyle(.secondary),
                     at: CGPoint(x: left + 3, y: staffTop - 26), anchor: .leading)
        if measure.number == 1 {
            for (row, digit) in ["\(score.beatsPerBar)", "4"].enumerated() {
                context.draw(Text(digit).font(.system(size: 26, weight: .bold, design: .serif)),
                             at: CGPoint(x: left + 16, y: y(2) + Double(row) * lineGap * 2.5))
            }
        }
        for chord in measure.chords {
            context.draw(Text(chord.name).font(.callout.bold()),
                         at: CGPoint(x: x(chord.slot, in: measure, at: index, width: width), y: staffTop - 14), anchor: .leading)
        }

        let sounding = measure.start <= position && position < measure.end
        if sounding {
            let cursor = x(measure.beatPosition(at: position), in: measure, at: index, width: width)
            let capsule = Path(roundedRect: CGRect(x: cursor - 4, y: y(1) - 9, width: 22, height: lineGap * 5 + 18), cornerRadius: 11)
            context.stroke(capsule, with: .color(.green), lineWidth: 2)
        }
        for placed in measure.notes {
            let label = context.resolve(Text("\(placed.note.fret)")
                .font(.system(size: 12, weight: .semibold, design: .monospaced))
                .foregroundStyle(placed.note.start <= position && position < placed.note.end ? Color.green : .primary))
            let labelSize = label.measure(in: CGSize(width: 40, height: 20))
            let noteX = x(placed.slot, in: measure, at: index, width: width)
            context.fill(Path(CGRect(x: noteX - 1, y: y(placed.note.string) - labelSize.height / 2,
                                     width: labelSize.width + 2, height: labelSize.height)),
                         with: .color(Color(nsColor: .windowBackgroundColor)))
            context.draw(label, at: CGPoint(x: noteX, y: y(placed.note.string)), anchor: .leading)
        }
        drawRhythm(measure, at: index, width: width, in: &context)
    }

    // shortcut: beams group notes per beat and use one beam level for the whole group, not per-note subdivisions.
    private func drawRhythm(_ measure: Score.Measure, at index: Int, width: Double, in context: inout GraphicsContext) {
        let top = y(6) + 12
        let bottom = top + 18
        let slots = Dictionary(grouping: measure.notes, by: \.slot).mapValues { $0.map(\.sixteenths).min()! }
        let byBeat = Dictionary(grouping: slots.keys.sorted(), by: { Int($0) })
        for (_, beatSlots) in byBeat {
            let xs = beatSlots.map { x($0, in: measure, at: index, width: width) + 4 }
            for stemX in xs {
                stroke(&context, from: CGPoint(x: stemX, y: top), to: CGPoint(x: stemX, y: bottom), .secondary)
            }
            let shortest = beatSlots.map { slots[$0]! }.min()!
            guard shortest < 4 else { continue }
            for level in 0..<(shortest == 1 ? 2 : 1) {
                let beamY = bottom - Double(level) * 4
                if xs.count > 1 {
                    stroke(&context, from: CGPoint(x: xs.first!, y: beamY), to: CGPoint(x: xs.last!, y: beamY), .secondary, width: 2.5)
                } else {
                    stroke(&context, from: CGPoint(x: xs[0], y: beamY), to: CGPoint(x: xs[0] + 6, y: beamY - 5), .secondary, width: 1.5)
                }
            }
        }
    }

    private func stroke(_ context: inout GraphicsContext, from: CGPoint, to: CGPoint, _ color: Color, width: Double = 1) {
        var path = Path()
        path.move(to: from)
        path.addLine(to: to)
        context.stroke(path, with: .color(color), lineWidth: width)
    }
}
