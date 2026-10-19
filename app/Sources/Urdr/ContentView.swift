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
    enum Pane: String { case mixer = "Mixer", tab = "Tab" }

    @State private var model = AppModel()
    @State private var pane = Pane.mixer
    @State private var importing = false
    @State private var showPalette = false
    @State private var showHelp = false
    @FocusState private var rootFocused: Bool

    private var player: StemPlayer { model.player }

    var body: some View {
        ZStack {
            Theme.ground.ignoresSafeArea()
            VStack(spacing: 0) {
                Group {
                    switch model.state {
                    case .idle: idle
                    case .separating(let fraction): separating(fraction)
                    case .failed(let message): failed(message)
                    case .ready: pane == .mixer ? AnyView(mixer) : AnyView(tabPane)
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                if case .ready = model.state {
                    TransportBar(player: player, analysis: model.analysis, hasTab: !model.tabNotes.isEmpty)
                }
            }
            if showPalette { PaletteView(commands: commands, close: closeOverlays) }
            if showHelp { HelpView(commands: commands, close: closeOverlays) }
        }
        .toolbar { toolbar }
        .toolbarBackground(Theme.ground, for: .windowToolbar)
        .toolbarBackground(.visible, for: .windowToolbar)
        .preferredColorScheme(.dark)
        .tint(Theme.text)
        .frame(minWidth: 900, minHeight: 600)
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
            if model.editing, let result = editKey(press) { return result }
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
        .fileImporter(isPresented: $importing, allowedContentTypes: [.audio]) { result in
            if case .success(let url) = result { open(url) }
            rootFocused = true
        }
    }

    private func open(_ url: URL) {
        Task {
            let scoped = url.startAccessingSecurityScopedResource()
            defer { if scoped { url.stopAccessingSecurityScopedResource() } }
            pane = .mixer
            await model.open(url)
        }
    }

    private func closeOverlays() {
        showPalette = false
        showHelp = false
        rootFocused = true
    }

    // Lives in the native toolbar so it shares the row with the traffic lights.
    @ToolbarContentBuilder private var toolbar: some ToolbarContent {
        ToolbarItem(placement: .navigation) {
            HStack(spacing: 10) {
                if case .ready(let title) = model.state {
                    Text(title).font(.system(size: 14, weight: .semibold)).foregroundStyle(Theme.text).lineLimit(1)
                    if let bpm = model.analysis?.bpm {
                        Text("\(Int(bpm.rounded())) BPM").font(.system(size: 12, weight: .medium).monospacedDigit()).foregroundStyle(Theme.secondary)
                    }
                } else {
                    Text("Urdr").font(.system(size: 14, weight: .semibold)).foregroundStyle(Theme.text)
                }
            }
            .padding(.horizontal, 6)
        }
        ToolbarItem(placement: .principal) {
            if case .ready = model.state {
                HStack(spacing: 8) {
                    Picker("View", selection: $pane) {
                        Text("Mixer").tag(Pane.mixer)
                        Text("Tab").tag(Pane.tab)
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    .frame(width: 150)
                    .help("Switch view (V)")
                    KeyChip("V")
                }
            }
        }
        ToolbarItemGroup(placement: .primaryAction) {
            Button { importing = true } label: { Label("Open", systemImage: "folder") }
                .keyboardShortcut("o")
                .help("Open a song (O)")
            Button { showPalette = true } label: { Label("Commands", systemImage: "command") }
                .help("Command palette (Cmd+K)")
            Button { showHelp = true } label: { Label("Shortcuts", systemImage: "keyboard") }
                .help("Keyboard shortcuts (?)")
        }
    }

    private var idle: some View {
        VStack(spacing: 18) {
            Image(systemName: "waveform").font(.system(size: 44, weight: .light)).foregroundStyle(Theme.secondary)
            Text("Open a song to split it into stems").font(.system(size: 22, weight: .semibold)).foregroundStyle(Theme.text)
            Text("Vocals, guitar, drums, bass, piano and the rest, each on its own track.")
                .font(.system(size: 14)).foregroundStyle(Theme.secondary)
            HStack(spacing: 8) {
                Button { importing = true } label: { Label("Choose a song", systemImage: "folder") }
                    .buttonStyle(ChromeButtonStyle(isOn: true))
                KeyChip("O")
            }
            Text("or drop an audio file here").font(.system(size: 12)).foregroundStyle(Theme.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .contentShape(Rectangle())
        .dropDestination(for: URL.self) { urls, _ in
            guard let url = urls.first else { return false }
            open(url)
            return true
        }
    }

    private func separating(_ fraction: Double) -> some View {
        VStack(spacing: 14) {
            Text("Separating stems").font(.system(size: 20, weight: .semibold)).foregroundStyle(Theme.text)
            ProgressView(value: fraction).frame(width: 360)
            Text("\(Int(fraction * 100))%  This runs once per song; it takes about as long as the song.")
                .font(.system(size: 12).monospacedDigit()).foregroundStyle(Theme.secondary)
        }
    }

    private func failed(_ message: String) -> some View {
        VStack(spacing: 14) {
            Text("This song could not be opened").font(.system(size: 20, weight: .semibold)).foregroundStyle(Theme.text)
            Text(message).font(.system(size: 12, design: .monospaced)).foregroundStyle(Theme.failure)
                .textSelection(.enabled).frame(maxWidth: 560)
            Button { importing = true } label: { Label("Open another song", systemImage: "folder") }
                .buttonStyle(ChromeButtonStyle(isOn: true))
        }
    }

    private var mixer: some View {
        MixerView(player: player, waveforms: model.waveforms, analysis: model.analysis,
                  chordPlaceholder: model.analyzing ? "Finding chords and beats"
                      : model.analysisError.map { "Chord analysis failed: \($0)" } ?? "")
            .padding(16)
    }

    private var tabPane: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 10) {
                Image(systemName: "guitars").foregroundStyle(Theme.color(for: "guitar"))
                Text("Guitar tab").font(.system(size: 14, weight: .semibold)).foregroundStyle(Theme.text)
                if model.tabEdited { Text("edited").font(.system(size: 12)).foregroundStyle(Theme.secondary) }
                Spacer()
                if model.editing {
                    Text("Type a fret, arrows move, Shift+Up/Down changes string, Delete removes, Cmd+Z undoes")
                        .font(.system(size: 12)).foregroundStyle(Theme.secondary).lineLimit(1)
                }
                if model.score != nil {
                    Button { model.toggleEditing() } label: { Label(model.editing ? "Done" : "Edit", systemImage: "pencil") }
                        .buttonStyle(ChromeButtonStyle(isOn: model.editing))
                        .help(model.editing ? "Stop editing (Esc or E)" : "Edit the tab (E)")
                    KeyChip("E")
                }
            }
            tabContent
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Theme.panel, in: .rect(cornerRadius: 12))
        }
        .padding(16)
    }

    @ViewBuilder private var tabContent: some View {
        if model.transcribing {
            VStack(spacing: 10) {
                ProgressView().controlSize(.small)
                Text("Transcribing the guitar stem").font(.system(size: 13)).foregroundStyle(Theme.secondary)
            }
        } else if let score = model.score {
            if let error = model.tabError {
                Text(error).foregroundStyle(.red).textSelection(.enabled)
            }
            ScoreView(score: score, position: player.position, selection: model.editing ? model.selection : nil,
                      seek: { player.seek(to: $0) },
                      select: model.editing ? { model.selection = $0 } : nil)
                .padding(.vertical, 8)
        } else if !model.tabNotes.isEmpty {
            TabView(notes: model.tabNotes, position: player.position).padding()
        } else {
            VStack(spacing: 12) {
                Text(model.tabError.map { "Tab failed: \($0)" } ?? "No tab yet for this song")
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(model.tabError == nil ? Theme.text : Theme.failure)
                Text("Urdr transcribes the guitar stem into tab. Fix anything it gets wrong with Edit.")
                    .font(.system(size: 13)).foregroundStyle(Theme.secondary)
                HStack(spacing: 8) {
                    Button { generateTab() } label: { Label("Transcribe guitar", systemImage: "wand.and.stars") }
                        .buttonStyle(ChromeButtonStyle(isOn: true))
                    KeyChip("T")
                }
            }
        }
    }

    private func generateTab() {
        pane = .tab
        Task { await model.generateTab() }
    }

    private func jumpBar(forward: Bool) {
        if let time = model.analysis?.downbeat(from: player.position, forward: forward) { player.seek(to: time) }
    }

    // Edit mode takes over digits, arrows and delete; everything else falls through to the normal shortcuts.
    private func editKey(_ press: KeyPress) -> KeyPress.Result? {
        let shift = press.modifiers.contains(.shift)
        if press.modifiers == .command, press.key == KeyEquivalent("z") {
            model.undoEdit()
            return .handled
        }
        // Keypad digits carry the numericPad flag and should type frets too.
        if press.modifiers.subtracting(.numericPad).isEmpty, let digit = press.characters.first?.wholeNumberValue {
            model.typeDigit(digit)
            return .handled
        }
        switch press.key {
        case .leftArrow: model.moveSelection(sixteenths: shift ? -4 : -1)
        case .rightArrow: model.moveSelection(sixteenths: shift ? 4 : 1)
        case .upArrow: shift ? model.moveSelectedNote(strings: -1) : model.moveSelection(strings: -1)
        case .downArrow: shift ? model.moveSelectedNote(strings: 1) : model.moveSelection(strings: 1)
        case .delete, .deleteForward: model.deleteSelectedNote()
        case .escape: model.toggleEditing()
        default: return nil
        }
        return .handled
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
            Command(title: pane == .mixer ? "Show the tab" : "Show the mixer", hint: "V", key: KeyEquivalent("v")) { pane = pane == .mixer ? .tab : .mixer },
            Command(title: "Generate guitar tab", hint: "T", key: KeyEquivalent("t")) { generateTab() },
            Command(title: player.synthOn ? "Original guitar" : "Synth guitar", hint: "Y", key: KeyEquivalent("y")) { player.toggleSynth() },
            Command(title: "Next synth tone", hint: "U", key: KeyEquivalent("u")) { player.cycleTone() },
            Command(title: model.editing ? "Stop editing tab" : "Edit tab", hint: "E", key: KeyEquivalent("e")) {
                pane = .tab
                model.toggleEditing()
            },
            Command(title: "Show shortcuts", hint: "?", key: KeyEquivalent("?")) { showHelp = true },
        ]
        for (index, stem) in player.stems.prefix(9).enumerated() {
            list.append(Command(title: "Mute \(stem.id)", hint: "\(index + 1)",
                                key: KeyEquivalent(Character("\(index + 1)"))) { player.toggleMute(stem.id) })
            list.append(Command(title: "Solo \(stem.id)", hint: "", key: nil) { player.toggleSolo(stem.id) })
        }
        if model.editing {
            // Handled by editKey; listed here so the help overlay and palette show them.
            list += [
                Command(title: "Type a fret at the cursor", hint: "0-22", key: nil) {},
                Command(title: "Move the edit cursor", hint: "Arrows", key: nil) {},
                Command(title: "Move the cursor a beat", hint: "Shift+Left/Right", key: nil) {},
                Command(title: "Move note to the string above", hint: "Shift+Up", key: nil) { model.moveSelectedNote(strings: -1) },
                Command(title: "Move note to the string below", hint: "Shift+Down", key: nil) { model.moveSelectedNote(strings: 1) },
                Command(title: "Delete note", hint: "Delete", key: nil) { model.deleteSelectedNote() },
                Command(title: "Undo edit", hint: "Cmd+Z", key: nil) { model.undoEdit() },
            ]
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
                    .background(index == selection ? Color.white.opacity(0.1) : .clear, in: .rect(cornerRadius: 6))
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

// Dim backdrop plus a centered card; Esc or a click outside closes it.
struct Overlay<Content: View>: View {
    let close: () -> Void
    @ViewBuilder let content: Content

    var body: some View {
        ZStack {
            Color.black.opacity(0.55).ignoresSafeArea().onTapGesture(perform: close)
            content
                .foregroundStyle(Theme.text)
                .padding(18)
                .frame(width: 420)
                .background(Theme.raised, in: .rect(cornerRadius: 14))
                .overlay(RoundedRectangle(cornerRadius: 14).stroke(Theme.stroke))
                .shadow(color: .black.opacity(0.5), radius: 30, y: 12)
        }
        .onExitCommand(perform: close)
    }
}
