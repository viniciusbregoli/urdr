import SwiftUI

// Six-line tab that scrolls under a fixed playhead; notes sounding at the playhead are highlighted.
struct TabView: View {
    let notes: [AppModel.TabNote]
    let position: Double
    private let pointsPerSecond = 250.0
    private let lineGap = 18.0

    var body: some View {
        Canvas { context, size in
            let playhead = size.width * 0.25
            let from = position - playhead / pointsPerSecond
            let to = position + (size.width - playhead) / pointsPerSecond
            func y(_ string: Int) -> Double { lineGap * Double(string) }

            for string in 1...6 {
                var line = Path()
                line.move(to: CGPoint(x: 0, y: y(string)))
                line.addLine(to: CGPoint(x: size.width, y: y(string)))
                context.stroke(line, with: .color(.secondary.opacity(0.5)), lineWidth: 1)
            }
            for note in notes where note.end >= from && note.start <= to {
                let sounding = note.start <= position && position < note.end
                let label = context.resolve(Text("\(note.fret)")
                    .font(.system(size: 12, weight: .semibold, design: .monospaced))
                    .foregroundStyle(sounding ? Color.accentColor : .primary))
                let labelSize = label.measure(in: size)
                let x = playhead + (note.start - position) * pointsPerSecond
                // Paint over the string line so the fret number stays readable.
                context.fill(Path(CGRect(x: x - 2, y: y(note.string) - labelSize.height / 2,
                                         width: labelSize.width + 4, height: labelSize.height)),
                             with: .color(Color(nsColor: .windowBackgroundColor)))
                context.draw(label, at: CGPoint(x: x, y: y(note.string)), anchor: .leading)
            }
            var head = Path()
            head.move(to: CGPoint(x: playhead, y: lineGap / 2))
            head.addLine(to: CGPoint(x: playhead, y: lineGap * 6.5))
            context.stroke(head, with: .color(.accentColor), lineWidth: 1.5)
        }
        .frame(height: lineGap * 7)
        .accessibilityLabel("Guitar tab, \(notes.count) notes")
    }
}
