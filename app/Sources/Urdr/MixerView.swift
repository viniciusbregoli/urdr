import SwiftUI

// One lane per stem on a shared timeline. A click seeks, a drag marks the loop.
struct MixerView: View {
    let player: StemPlayer
    let waveforms: [String: [Float]]
    let analysis: Analysis?
    let chordPlaceholder: String

    private let headerWidth = 248.0
    private let chordLaneHeight = 40.0
    private let spacing = 6.0
    @State private var dragRange: ClosedRange<Double>?

    var body: some View {
        GeometryReader { geometry in
            let count = Double(max(player.stems.count, 1))
            let laneHeight = max(52, (geometry.size.height - chordLaneHeight - spacing * count) / count)
            HStack(alignment: .top, spacing: 12) {
                VStack(spacing: spacing) {
                    ForEach(Array(player.stems.enumerated()), id: \.element.id) { index, stem in
                        header(stem, key: index + 1).frame(height: laneHeight)
                    }
                    Text("Chords")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(Theme.secondary)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                        .padding(.leading, 12)
                        .frame(height: chordLaneHeight)
                }
                .frame(width: headerWidth)
                timeline(laneHeight: laneHeight)
            }
        }
    }

    private func header(_ stem: StemPlayer.Stem, key: Int) -> some View {
        let tint = Theme.color(for: stem.id)
        let audible = player.isAudible(stem)
        return VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 10) {
                Image(systemName: Theme.icon(for: stem.id))
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(audible ? Color.black : Theme.secondary)
                    .frame(width: 28, height: 28)
                    .background(audible ? tint : Theme.raised, in: .circle)
                Text(stem.id.capitalized)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(audible ? Theme.text : Theme.secondary)
                Spacer(minLength: 4)
                KeyChip("\(key)")
                Button("M") { player.toggleMute(stem.id) }
                    .buttonStyle(PillToggleStyle(isOn: stem.muted, tint: Theme.text))
                    .help("Mute \(stem.id) (\(key))")
                Button("S") { player.toggleSolo(stem.id) }
                    .buttonStyle(PillToggleStyle(isOn: stem.solo, tint: tint))
                    .help("Solo \(stem.id)")
            }
            Slider(value: Binding(get: { Double(stem.volume) }, set: { player.setVolume(stem.id, Float($0)) }), in: 0...1)
                .controlSize(.mini)
                .tint(tint)
                .accessibilityLabel("\(stem.id.capitalized) volume")
        }
        .padding(.horizontal, 12)
        .frame(maxHeight: .infinity)
        .background(Theme.panel, in: .rect(cornerRadius: 12))
    }

    private func timeline(laneHeight: Double) -> some View {
        GeometryReader { geometry in
            let width = geometry.size.width
            VStack(spacing: spacing) {
                ForEach(player.stems) { stem in
                    WaveformLane(peaks: waveforms[stem.id] ?? [], tint: Theme.color(for: stem.id),
                                 audible: player.isAudible(stem), progress: player.duration > 0 ? player.position / player.duration : 0)
                        .frame(height: laneHeight)
                }
                ChordLane(analysis: analysis, placeholder: chordPlaceholder, duration: player.duration, position: player.position)
                    .frame(height: chordLaneHeight)
            }
            .overlay(alignment: .topLeading) { playheadAndLoop(width: width) }
            .contentShape(Rectangle())
            .gesture(DragGesture(minimumDistance: 0)
                .onChanged { drag in
                    guard abs(drag.translation.width) > 4 else { return }
                    let a = time(at: drag.startLocation.x, width: width), b = time(at: drag.location.x, width: width)
                    dragRange = min(a, b)...max(a, b)
                }
                .onEnded { drag in
                    if let range = dragRange, range.upperBound - range.lowerBound > 0.2 {
                        player.loopStart = range.lowerBound
                        player.loopEnd = range.upperBound
                        player.loopOn = true
                    } else {
                        player.seek(to: time(at: drag.location.x, width: width))
                    }
                    dragRange = nil
                })
        }
        .accessibilityLabel("Song timeline")
    }

    private func time(at x: Double, width: Double) -> Double {
        min(max(x / width, 0), 1) * player.duration
    }

    private func playheadAndLoop(width: Double) -> some View {
        let duration = player.duration, position = player.position
        let loop = dragRange ?? player.loopStart.map { $0...max($0, player.loopEnd ?? position) }
        let loopActive = player.loopOn || dragRange != nil
        return Canvas { context, size in
            guard duration > 0 else { return }
            func x(_ t: Double) -> Double { t / duration * width }
            if let loop, loop.upperBound > loop.lowerBound {
                let rect = CGRect(x: x(loop.lowerBound), y: 0, width: x(loop.upperBound) - x(loop.lowerBound), height: size.height)
                context.fill(Path(rect), with: .color(.white.opacity(loopActive ? 0.09 : 0.04)))
                for edge in [rect.minX, rect.maxX] {
                    context.fill(Path(CGRect(x: edge - 0.5, y: 0, width: 1, height: size.height)), with: .color(.white.opacity(0.45)))
                }
            }
            let head = x(position)
            context.fill(Path(CGRect(x: head - 1, y: 0, width: 2, height: size.height)), with: .color(.white))
            context.fill(Path(ellipseIn: CGRect(x: head - 5, y: -5, width: 10, height: 10)), with: .color(.white))
        }
        .allowsHitTesting(false)
    }
}

// Mirrored peak bars; the played part is drawn at full strength by masking a second copy.
struct WaveformLane: View {
    let peaks: [Float]
    let tint: Color
    let audible: Bool
    let progress: Double

    var body: some View {
        ZStack(alignment: .leading) {
            bars(opacity: audible ? 0.38 : 0.12)
            bars(opacity: audible ? 1 : 0.22)
                .mask(alignment: .leading) {
                    GeometryReader { geometry in Rectangle().frame(width: geometry.size.width * progress) }
                }
        }
        .padding(.vertical, 8)
        .background(Theme.panel, in: .rect(cornerRadius: 12))
        .overlay {
            if peaks.isEmpty { ProgressView().controlSize(.small) }
        }
    }

    private func bars(opacity: Double) -> some View {
        Canvas { context, size in
            guard !peaks.isEmpty else { return }
            let step = 3.0
            let count = Int(size.width / step)
            let mid = size.height / 2
            var path = Path()
            for i in 0..<count {
                let peak = Double(peaks[min(peaks.count - 1, i * peaks.count / max(count, 1))])
                let half = max(0.75, peak * mid)
                path.addRoundedRect(in: CGRect(x: Double(i) * step, y: mid - half, width: 2, height: half * 2), cornerSize: CGSize(width: 1, height: 1))
            }
            context.fill(path, with: .color(tint.opacity(opacity)))
        }
    }
}

// Chord segments on the same timeline as the lanes; the sounding chord is filled white.
struct ChordLane: View {
    let analysis: Analysis?
    let placeholder: String
    let duration: Double
    let position: Double

    var body: some View {
        Canvas { context, size in
            guard let analysis, duration > 0 else { return }
            for chord in analysis.chords where chord.label != "N" {
                let x0 = chord.start / duration * size.width, x1 = chord.end / duration * size.width
                let rect = CGRect(x: x0 + 1, y: 4, width: max(x1 - x0 - 2, 1), height: size.height - 8)
                let current = chord.start <= position && position < chord.end
                context.fill(Path(roundedRect: rect, cornerRadius: 6), with: .color(current ? Theme.text : Theme.raised))
                let label = context.resolve(Text(chord.name)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(current ? Color.black : Theme.text))
                if label.measure(in: size).width + 10 < rect.width {
                    context.draw(label, at: CGPoint(x: rect.minX + 7, y: rect.midY), anchor: .leading)
                }
            }
        }
        .background(Theme.panel, in: .rect(cornerRadius: 12))
        .overlay {
            if analysis == nil {
                Text(placeholder)
                    .font(.system(size: 12))
                    .foregroundStyle(Theme.secondary)
            }
        }
    }
}
