import SwiftUI
import UniformTypeIdentifiers

struct Command: Identifiable {
    var id: String { title }
    let title: String
    let hint: String
    let key: KeyEquivalent?
    let run: () -> Void
}

struct ContentView: View {
    @State private var model = AppModel()
    @State private var importing = false
    @State private var showPalette = false
    @State private var showHelp = false
    @FocusState private var rootFocused: Bool

    private var player: StemPlayer { model.player }

    var body: some View {
        ZStack {
            VStack(alignment: .leading, spacing: 16) {
                header
                if case .ready = model.state { transport; mixer; chordsSection; tabSection }
                Spacer(minLength: 0)
            }
            .padding(20)
            if showPalette { PaletteView(commands: commands, close: closeOverlays) }
            if showHelp { HelpView(commands: commands, close: closeOverlays) }
        }
        .frame(minWidth: 560, minHeight: 360)
        .focusable()
        .focused($rootFocused)
        .focusEffectDisabled()
        .onAppear { rootFocused = true }
        // `Urdr <song>` opens a song straight from the terminal.
        .task {
            // Requiring an existing file skips the values of AppKit flags such as `-AppleLanguages (en)`.
            if let path = CommandLine.arguments.dropFirst().first(where: { !$0.hasPrefix("-") && FileManager.default.fileExists(atPath: $0) }) {
                await model.open(URL(fileURLWithPath: path))
            }
        }
        .onKeyPress(phases: .down) { press in
            guard !showPalette, !showHelp else { return .ignored }
            if press.key == .escape { return .ignored }
            if press.key == KeyEquivalent("k"), press.modifiers == .command {
                showPalette = true
                return .handled
            }
            guard press.modifiers.isSubset(of: .shift),
                  let command = commands.first(where: { $0.key == press.key }) else { return .ignored }
            command.run()
            return .handled
        }
        .toolbar {
            Button("Open", systemImage: "folder") { importing = true }
                .keyboardShortcut("o")
        }
        .fileImporter(isPresented: $importing, allowedContentTypes: [.audio]) { result in
            if case .success(let url) = result {
                Task {
                    let scoped = url.startAccessingSecurityScopedResource()
                    defer { if scoped { url.stopAccessingSecurityScopedResource() } }
                    await model.open(url)
                }
            }
            rootFocused = true
        }
    }

    private func closeOverlays() {
        showPalette = false
        showHelp = false
        rootFocused = true
    }

    @ViewBuilder private var header: some View {
        switch model.state {
        case .idle:
            Text("Press O to open a song").font(.title3).foregroundStyle(.secondary)
        case .separating(let fraction):
            VStack(alignment: .leading) {
                Text("Separating stems").font(.headline)
                ProgressView(value: fraction)
            }
        case .ready(let title):
            Text(title).font(.title2.bold())
        case .failed(let message):
            Text(message).foregroundStyle(.red).textSelection(.enabled)
        }
    }

    private var transport: some View {
        VStack(alignment: .leading, spacing: 8) {
            Slider(value: Binding(get: { player.position }, set: { player.seek(to: $0) }),
                   in: 0...max(player.duration, 1))
            HStack {
                Text("\(format(player.position)) / \(format(player.duration))").monospacedDigit()
                Spacer()
                if let bpm = model.analysis?.bpm { Text("\(Int(bpm.rounded())) BPM") }
                Text("Speed \(Int((player.rate * 100).rounded()))%")
                Text("Pitch \(player.semitones > 0 ? "+" : "")\(player.semitones)")
                if !model.tabNotes.isEmpty {
                    Text(player.synthOn ? "Synth: \(StemPlayer.tones[player.tone].name)" : "Orig guitar")
                }
                Text(loopLabel).foregroundStyle(player.loopOn ? Color.accentColor : .secondary)
            }
            .font(.callout)
        }
    }

    @ViewBuilder private var chordsSection: some View {
        if let analysis = model.analysis {
            ChordsView(analysis: analysis, position: player.position)
        } else if model.analyzing {
            ProgressView("Finding chords and beats").controlSize(.small)
        } else if let error = model.analysisError {
            Text("Chord analysis failed: \(error)").foregroundStyle(.red)
        }
    }

    private func jumpBar(forward: Bool) {
        if let time = model.analysis?.downbeat(from: player.position, forward: forward) { player.seek(to: time) }
    }

    @ViewBuilder private var tabSection: some View {
        if model.transcribing {
            ProgressView("Transcribing guitar").controlSize(.small)
        } else if let score = model.score {
            ScoreView(score: score, position: player.position) { player.seek(to: $0) }
        } else if !model.tabNotes.isEmpty {
            TabView(notes: model.tabNotes, position: player.position)
        } else {
            Text(model.tabError.map { "Tab failed: \($0)" } ?? "Press T to transcribe the guitar into tab")
                .foregroundStyle(model.tabError == nil ? Color.secondary : .red)
        }
    }

    private var loopLabel: String {
        guard let start = player.loopStart else { return "Loop off" }
        return "Loop \(format(start)) - \(player.loopEnd.map(format) ?? "...")"
    }

    private var mixer: some View {
        VStack(spacing: 10) {
            ForEach(Array(player.stems.enumerated()), id: \.element.id) { index, stem in
                HStack {
                    Text("\(index + 1)").foregroundStyle(.secondary).frame(width: 14)
                    Text(stem.id.capitalized).frame(width: 70, alignment: .leading)
                    Toggle("M", isOn: Binding(get: { stem.muted }, set: { _ in player.toggleMute(stem.id) }))
                        .toggleStyle(.button)
                    Toggle("S", isOn: Binding(get: { stem.solo }, set: { _ in player.toggleSolo(stem.id) }))
                        .toggleStyle(.button)
                    Slider(value: Binding(get: { Double(stem.volume) }, set: { player.setVolume(stem.id, Float($0)) }),
                           in: 0...1)
                }
            }
        }
    }

    private func format(_ seconds: Double) -> String {
        let total = Int(seconds)
        return String(format: "%d:%02d", total / 60, total % 60)
    }

    private var commands: [Command] {
        var list = [
            Command(title: "Open song", hint: "O", key: KeyEquivalent("o")) { importing = true },
            Command(title: "Play / pause", hint: "Space", key: .space) { player.togglePlay() },
            Command(title: "Back 5 seconds", hint: "Left", key: .leftArrow) { player.skip(-5) },
            Command(title: "Forward 5 seconds", hint: "Right", key: .rightArrow) { player.skip(5) },
            Command(title: "Previous bar", hint: "Up", key: .upArrow) { jumpBar(forward: false) },
            Command(title: "Next bar", hint: "Down", key: .downArrow) { jumpBar(forward: true) },
            Command(title: "Set loop start", hint: "[", key: KeyEquivalent("[")) { player.setLoopStart() },
            Command(title: "Set loop end", hint: "]", key: KeyEquivalent("]")) { player.setLoopEnd() },
            Command(title: "Toggle loop", hint: "L", key: KeyEquivalent("l")) { player.loopOn.toggle() },
            Command(title: "Clear loop", hint: "\\", key: KeyEquivalent("\\")) { player.clearLoop() },
            Command(title: "Slower", hint: ",", key: KeyEquivalent(",")) { player.rate = max(0.5, player.rate - 0.05) },
            Command(title: "Faster", hint: ".", key: KeyEquivalent(".")) { player.rate = min(2, player.rate + 0.05) },
            Command(title: "Reset speed", hint: "/", key: KeyEquivalent("/")) { player.rate = 1 },
            Command(title: "Pitch down", hint: "-", key: KeyEquivalent("-")) { player.semitones = max(-12, player.semitones - 1) },
            Command(title: "Pitch up", hint: "=", key: KeyEquivalent("=")) { player.semitones = min(12, player.semitones + 1) },
            Command(title: "Reset pitch", hint: "0", key: KeyEquivalent("0")) { player.semitones = 0 },
            Command(title: "Generate guitar tab", hint: "T", key: KeyEquivalent("t")) { Task { await model.generateTab() } },
            Command(title: player.synthOn ? "Original guitar" : "Synth guitar", hint: "Y", key: KeyEquivalent("y")) { player.toggleSynth() },
            Command(title: "Next synth tone", hint: "U", key: KeyEquivalent("u")) { player.cycleTone() },
            Command(title: "Show shortcuts", hint: "?", key: KeyEquivalent("?")) { showHelp = true },
        ]
        for (index, stem) in player.stems.prefix(9).enumerated() {
            list.append(Command(title: "Mute \(stem.id)", hint: "\(index + 1)",
                                key: KeyEquivalent(Character("\(index + 1)"))) { player.toggleMute(stem.id) })
            list.append(Command(title: "Solo \(stem.id)", hint: "", key: nil) { player.toggleSolo(stem.id) })
        }
        return list
    }
}

struct PaletteView: View {
    let commands: [Command]
    let close: () -> Void
    @State private var query = ""
    @State private var selection = 0
    @FocusState private var focused: Bool

    private var matches: [Command] {
        query.isEmpty ? commands : commands.filter { $0.title.localizedCaseInsensitiveContains(query) }
    }

    var body: some View {
        Overlay(close: close) {
            VStack(alignment: .leading, spacing: 8) {
                TextField("Type a command", text: $query)
                    .textFieldStyle(.plain)
                    .font(.title3)
                    .focused($focused)
                    .onSubmit {
                        guard matches.indices.contains(selection) else { return }
                        let command = matches[selection]
                        close()
                        command.run()
                    }
                    .onChange(of: query) { selection = 0 }
                Divider()
                ForEach(Array(matches.enumerated()), id: \.element.id) { index, command in
                    HStack {
                        Text(command.title)
                        Spacer()
                        KeyChip(command.hint)
                    }
                    .padding(.vertical, 2).padding(.horizontal, 6)
                    .background(index == selection ? Color.accentColor.opacity(0.25) : .clear, in: .rect(cornerRadius: 5))
                }
            }
        }
        .onAppear { focused = true }
        .onKeyPress(.upArrow) { selection = max(0, selection - 1); return .handled }
        .onKeyPress(.downArrow) { selection = min(matches.count - 1, selection + 1); return .handled }
    }
}

struct HelpView: View {
    let commands: [Command]
    let close: () -> Void

    var body: some View {
        Overlay(close: close) {
            VStack(alignment: .leading, spacing: 6) {
                Text("Shortcuts").font(.headline)
                ForEach(commands.filter { !$0.hint.isEmpty }) { command in
                    HStack {
                        Text(command.title)
                        Spacer()
                        KeyChip(command.hint)
                    }
                }
                HStack { Text("Command palette"); Spacer(); KeyChip("Cmd+K") }
                HStack { Text("Close this"); Spacer(); KeyChip("Esc") }
            }
        }
    }
}

struct KeyChip: View {
    let text: String
    init(_ text: String) { self.text = text }

    var body: some View {
        if !text.isEmpty {
            Text(text)
                .font(.caption.monospaced())
                .padding(.horizontal, 6).padding(.vertical, 1)
                .background(.quaternary, in: .rect(cornerRadius: 4))
        }
    }
}

// Dim backdrop plus a centered card; Esc or a click outside closes it.
struct Overlay<Content: View>: View {
    let close: () -> Void
    @ViewBuilder let content: Content

    var body: some View {
        ZStack {
            Color.black.opacity(0.35).onTapGesture(perform: close)
            content
                .padding(16)
                .frame(width: 380)
                .background(.regularMaterial, in: .rect(cornerRadius: 10))
        }
        .onExitCommand(perform: close)
    }
}
