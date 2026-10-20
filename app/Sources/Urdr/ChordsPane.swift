import SwiftUI

// Play-along chord view: what to play now and next with fingerings, over a bar grid that follows the song.
struct ChordsPane: View {
    let analysis: Analysis
    let sheet: ChordSheet
    let position: Double
    let seek: (Double) -> Void

    private let columns = 4

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            upcoming
            grid
        }
        .padding(16)
    }

    private var upcoming: some View {
        let chords = ChordSheet.upcoming(analysis, at: position, count: 7)
        return HStack(alignment: .bottom, spacing: 14) {
            if chords.isEmpty {
                Text("No chords detected here").font(.system(size: 15)).foregroundStyle(Theme.secondary)
                    .frame(maxWidth: .infinity, minHeight: 170)
            }
            ForEach(Array(chords.enumerated()), id: \.offset) { index, chord in
                ChordCard(chord: chord, current: index == 0 && chord.start <= position)
            }
            Spacer(minLength: 0)
        }
        .frame(height: 210, alignment: .bottom)
    }

    private var grid: some View {
        let current = sheet.barIndex(at: position)
        return ScrollViewReader { proxy in
            ScrollView {
                LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: columns), spacing: 8) {
                    ForEach(sheet.bars, id: \.number) { bar in
                        BarCell(bar: bar, position: position, current: bar.number - 1 == current)
                            .id(bar.number)
                            .onTapGesture { seek(bar.start) }
                    }
                }
                .padding(2)
            }
            .onChange(of: current.map { $0 / columns }) { _, row in
                guard let row, sheet.bars.indices.contains(row * columns) else { return }
                withAnimation(.easeOut(duration: 0.25)) { proxy.scrollTo(sheet.bars[row * columns].number, anchor: .center) }
            }
        }
    }
}

private struct ChordCard: View {
    let chord: Analysis.Chord
    let current: Bool

    var body: some View {
        VStack(spacing: 8) {
            Text(chord.name)
                .font(.system(size: current ? 34 : 20, weight: .bold))
                .foregroundStyle(current ? Theme.text : Theme.secondary)
            if let shape = ChordShape.shape(for: chord.label) {
                ChordDiagram(shape: shape, tint: current ? Theme.mint : Theme.secondary)
                    .frame(width: current ? 112 : 76, height: current ? 132 : 90)
            }
        }
        .padding(.horizontal, current ? 18 : 12).padding(.vertical, 12)
        .background(Theme.panel, in: .rect(cornerRadius: 12))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(current ? Theme.mint : .clear, lineWidth: 1.5))
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(current ? "Now \(chord.name)" : "Next \(chord.name)")
    }
}

private struct BarCell: View {
    let bar: ChordSheet.Bar
    let position: Double
    let current: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("\(bar.number)").font(.system(size: 10, weight: .medium).monospacedDigit()).foregroundStyle(Theme.secondary)
            // Names and beat dots sit where they fall in the bar, so a chord lines up with the beat it changes on.
            GeometryReader { geometry in
                let span = bar.end - bar.start
                ZStack(alignment: .topLeading) {
                    // Quick changes would collide, so each name starts at least 44 pt after the previous one.
                    let xs = bar.chords.reduce(into: [Double]()) { $0.append(max($1.at * geometry.size.width, ($0.last ?? -44) + 44)) }
                    ForEach(Array(bar.chords.enumerated()), id: \.offset) { index, chord in
                        Text(chord.name)
                            .font(.system(size: 20, weight: .semibold))
                            .foregroundStyle(position >= bar.end ? Theme.secondary : Theme.text)
                            .fixedSize()
                            .offset(x: xs[index])
                    }
                    if bar.chords.isEmpty { Text("%").font(.system(size: 20)).foregroundStyle(Theme.secondary) }
                    ForEach(Array(bar.beats.enumerated()), id: \.offset) { index, beat in
                        let next = index + 1 < bar.beats.count ? bar.beats[index + 1] : bar.end
                        Circle()
                            .fill(current && beat <= position && position < next ? Theme.mint : Theme.raised)
                            .frame(width: 6, height: 6)
                            .offset(x: (beat - bar.start) / span * geometry.size.width, y: 32)
                    }
                }
            }
            .frame(height: 38)
        }
        .padding(10)
        .frame(maxWidth: .infinity, minHeight: 78, alignment: .topLeading)
        .background(current ? Theme.raised : Theme.panel, in: .rect(cornerRadius: 10))
        .overlay(RoundedRectangle(cornerRadius: 10).stroke(current ? Theme.mint : .clear, lineWidth: 1.5))
        .contentShape(Rectangle())
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Bar \(bar.number), \(bar.chords.map(\.name).joined(separator: " then "))")
        .accessibilityAddTraits(.isButton)
    }
}

// Standard chord box: strings left (low E) to right (high e), nut or base fret on top.
struct ChordDiagram: View {
    let shape: ChordShape
    let tint: Color

    var body: some View {
        Canvas { context, size in
            let frets = 4, top = 16.0, left = 14.0
            let width = size.width - left - 6, height = size.height - top - 4
            let stringGap = width / 5, fretGap = height / Double(frets)
            let base = shape.baseFret
            func x(_ string: Int) -> Double { left + Double(string) * stringGap }
            func line(_ a: CGPoint, _ b: CGPoint, _ w: Double) {
                var p = Path(); p.move(to: a); p.addLine(to: b)
                context.stroke(p, with: .color(Theme.secondary.opacity(0.7)), lineWidth: w)
            }
            for s in 0..<6 { line(CGPoint(x: x(s), y: top), CGPoint(x: x(s), y: top + height), 1) }
            for f in 0...frets {
                line(CGPoint(x: left, y: top + Double(f) * fretGap), CGPoint(x: left + width, y: top + Double(f) * fretGap),
                     f == 0 && base == 1 ? 3 : 1)
            }
            if base > 1 {
                context.draw(Text("\(base)").font(.system(size: 10, weight: .semibold)).foregroundStyle(Theme.secondary),
                             at: CGPoint(x: 5, y: top + fretGap / 2))
            }
            let dot = min(stringGap, fretGap) * 0.62
            for (s, fret) in shape.frets.enumerated() {
                guard let fret else {
                    context.draw(Text("×").font(.system(size: 11, weight: .bold)).foregroundStyle(Theme.secondary), at: CGPoint(x: x(s), y: top - 8))
                    continue
                }
                if fret == 0 {
                    context.stroke(Path(ellipseIn: CGRect(x: x(s) - 3.5, y: top - 11.5, width: 7, height: 7)),
                                   with: .color(Theme.secondary), lineWidth: 1.2)
                    continue
                }
                let row = Double(fret - base) + 0.5
                context.fill(Path(ellipseIn: CGRect(x: x(s) - dot / 2, y: top + row * fretGap - dot / 2, width: dot, height: dot)),
                             with: .color(tint))
            }
        }
        .accessibilityHidden(true)
    }
}
