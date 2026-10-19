import SwiftUI

// Bottom bar: chord now and next, play controls with the clock, then the practice controls, each with its key.
struct TransportBar: View {
    let player: StemPlayer
    let analysis: Analysis?
    let hasTab: Bool

    var body: some View {
        HStack(spacing: 24) {
            chord.frame(width: 220, alignment: .leading)
            Spacer(minLength: 0)
            playControls
            Spacer(minLength: 0)
            practiceControls
        }
        .padding(.horizontal, 20)
        .frame(height: 84)
        .background(Theme.panel)
        .overlay(alignment: .top) { Rectangle().fill(Theme.stroke).frame(height: 1) }
    }

    private var currentChord: Analysis.Chord? {
        analysis?.chordIndex(at: player.position).map { analysis!.chords[$0] }
    }

    private var nextChord: Analysis.Chord? {
        let after = currentChord?.end ?? player.position
        return analysis?.chords.first { $0.start >= after && $0.label != "N" }
    }

    private var chord: some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            Text(currentChord?.name ?? "-")
                .font(.system(size: 34, weight: .bold))
                .foregroundStyle(Theme.text)
                .contentTransition(.numericText())
            if let next = nextChord {
                Text("then \(next.name)")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(Theme.secondary)
            }
        }
        .accessibilityElement(children: .combine)
    }

    private var playControls: some View {
        HStack(spacing: 18) {
            Button { player.skip(-5) } label: { Image(systemName: "gobackward.5") }
                .buttonStyle(.plain).font(.system(size: 18)).foregroundStyle(Theme.text)
                .help("Back 5 seconds (Left)")
            Button { player.togglePlay() } label: {
                Image(systemName: player.isPlaying ? "pause.fill" : "play.fill")
                    .font(.system(size: 20, weight: .bold))
                    .foregroundStyle(Color.black)
                    .frame(width: 48, height: 48)
                    .background(Theme.text, in: .circle)
            }
            .buttonStyle(.plain)
            .help(player.isPlaying ? "Pause (Space)" : "Play (Space)")
            Button { player.skip(5) } label: { Image(systemName: "goforward.5") }
                .buttonStyle(.plain).font(.system(size: 18)).foregroundStyle(Theme.text)
                .help("Forward 5 seconds (Right)")
            Text("\(clock(player.position)) / \(clock(player.duration))")
                .font(.system(size: 13, weight: .medium).monospacedDigit())
                .foregroundStyle(Theme.secondary)
                .frame(width: 92, alignment: .leading)
        }
    }

    private var practiceControls: some View {
        HStack(spacing: 14) {
            stepper("Speed", value: "\(Int((player.rate * 100).rounded()))%", keys: ", .",
                    down: { player.rate = max(0.5, player.rate - 0.05) }, reset: { player.rate = 1 },
                    up: { player.rate = min(2, player.rate + 0.05) })
            stepper("Pitch", value: player.semitones == 0 ? "0" : String(format: "%+d", player.semitones), keys: "- =",
                    down: { player.semitones = max(-12, player.semitones - 1) }, reset: { player.semitones = 0 },
                    up: { player.semitones = min(12, player.semitones + 1) })
            control("Loop", keys: "L") {
                Button { player.loopOn.toggle() } label: {
                    Label(loopLabel, systemImage: "repeat").labelStyle(.titleAndIcon)
                }
                .buttonStyle(ChromeButtonStyle(isOn: player.loopOn))
                .disabled(player.loopStart == nil)
                .help("Toggle loop (L). Set it with [ and ] or by dragging across the lanes.")
            }
            if hasTab {
                control("Guitar", keys: "Y U") {
                    Button { player.toggleSynth() } label: {
                        Label(player.synthOn ? StemPlayer.tones[player.tone].name : "Original", systemImage: "guitars")
                    }
                    .buttonStyle(ChromeButtonStyle(isOn: player.synthOn))
                    .help("Play the tab through the synth instead of the original guitar (Y). Next tone (U).")
                }
            }
        }
    }

    private var loopLabel: String {
        guard let start = player.loopStart else { return "Off" }
        return "\(clock(start))-\(player.loopEnd.map(clock) ?? "...")"
    }

    private func stepper(_ title: String, value: String, keys: String,
                         down: @escaping () -> Void, reset: @escaping () -> Void, up: @escaping () -> Void) -> some View {
        control(title, keys: keys) {
            HStack(spacing: 2) {
                Button(action: down) { Image(systemName: "minus") }.help("\(title) down")
                Button(action: reset) { Text(value).monospacedDigit().frame(minWidth: 40) }.help("Reset \(title.lowercased())")
                Button(action: up) { Image(systemName: "plus") }.help("\(title) up")
            }
            .buttonStyle(ChromeButtonStyle())
        }
    }

    private func control(_ title: String, keys: String, @ViewBuilder content: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack(spacing: 6) {
                Text(title).font(.system(size: 11, weight: .semibold)).foregroundStyle(Theme.secondary)
                KeyChip(keys)
            }
            content()
        }
    }

    private func clock(_ seconds: Double) -> String {
        let total = Int(seconds)
        return String(format: "%d:%02d", total / 60, total % 60)
    }
}
