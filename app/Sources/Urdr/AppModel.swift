import Foundation
import Observation

@Observable @MainActor
final class AppModel {
    enum State: Equatable {
        case idle
        case separating(Double)
        case ready(title: String)
        case failed(String)
    }

    struct TabNote: Codable, Equatable {
        let start: Double
        let end: Double
        let midi: Int
        let string: Int
        let fret: Int
    }

    private(set) var state = State.idle
    var tabNotes: [TabNote] = [] {
        didSet {
            player.synthNotes = tabNotes
            rebuildScore()
        }
    }
    private(set) var score: Score?
    private(set) var waveforms: [String: [Float]] = [:]
    private(set) var transcribing = false
    var tabError: String?
    var analysis: Analysis? {
        didSet {
            rebuildScore()
            chordSheet = analysis.map { ChordSheet(analysis: $0, duration: player.duration) }
        }
    }
    private(set) var chordSheet: ChordSheet?
    private(set) var analyzing = false
    private(set) var analysisError: String?
    let player = StemPlayer()
    var folder: URL?

    // Tab editing state; the editing logic lives in TabEditor.swift.
    var editing = false
    var selection: Selection?
    var tabEdited = false
    // A tab file exists, even an empty one, so the grid can be edited.
    var hasTab = false { didSet { rebuildScore() } }
    var history: [[TabNote]] = []
    var pendingDigit: (value: Int, time: Date, selection: Selection)?

    // Display order; any extra stem the engine produces is appended alphabetically.
    nonisolated private static let stemOrder = ["vocals", "guitar", "drums", "bass", "piano", "other"]

    static let libraryRoot = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        .appendingPathComponent("Urdr/Songs")

    func open(_ audio: URL) async {
        let folder = Self.libraryRoot.appendingPathComponent(Self.folderName(for: audio))
        let stemsFolder = folder.appendingPathComponent("stems")
        do {
            // htdemucs_6s always yields 6 stems, so fewer means an interrupted run.
            if Self.stems(in: stemsFolder).count < 6 {
                state = .separating(0)
                try await Engine.separate(audio: audio, out: stemsFolder) { fraction in
                    Task { @MainActor in self.state = .separating(fraction) }
                }
            }
            let stems = Self.stems(in: stemsFolder)
            try player.load(stems)
            waveforms = [:]
            Task {
                let peaks = await Task.detached(priority: .userInitiated) { Waveform.peaks(for: stems) }.value
                if self.folder == folder { waveforms = peaks }
            }
            self.folder = folder
            tabNotes = Self.loadTab(folder.appendingPathComponent("tab.json"))
            tabEdited = Self.isEdited(folder.appendingPathComponent("tab.json"))
            hasTab = FileManager.default.fileExists(atPath: folder.appendingPathComponent("tab.json").path)
            history = []
            editing = false
            selection = nil
            tabError = nil
            state = .ready(title: audio.deletingPathExtension().lastPathComponent)
            analysis = Analysis.load(folder.appendingPathComponent("analysis.json"))
            analysisError = nil
            analyzing = false
            if analysis == nil { await analyze(audio, into: folder) }
        } catch {
            state = .failed(error.localizedDescription)
        }
    }

    func generateTab() async {
        guard let folder, !transcribing else { return }
        let out = folder.appendingPathComponent("tab.json")
        transcribing = true
        tabError = nil
        defer { transcribing = false }
        // Editing is off while the engine rewrites tab.json, so no edit can land between the backup and the new tab.
        editing = false
        selection = nil
        pendingDigit = nil
        do {
            // Regenerating must never cost the user their hand edits.
            if tabEdited {
                let backup = folder.appendingPathComponent("tab.edited.json")
                try? FileManager.default.removeItem(at: backup)
                try FileManager.default.copyItem(at: out, to: backup)
            }
            try await Engine.tab(audio: folder.appendingPathComponent("stems/guitar.wav"), out: out)
            // The file is already in its own song folder; only the state of a song that is still open changes.
            guard self.folder == folder else { return }
            tabNotes = Self.loadTab(out)
            hasTab = true
            tabEdited = false
            history = []
        } catch {
            guard self.folder == folder else { return }
            tabError = error.localizedDescription
        }
    }

    // Runs on the full mix while the stems play; a result for a song that is no longer open is dropped.
    private func analyze(_ audio: URL, into folder: URL) async {
        let out = folder.appendingPathComponent("analysis.json")
        analyzing = true
        do {
            try await Engine.analyze(audio: audio, out: out)
            guard self.folder == folder else { return }
            analysis = Analysis.load(out)
        } catch {
            guard self.folder == folder else { return }
            analysisError = error.localizedDescription
        }
        analyzing = false
    }

    // Measures need the beat grid, so the score exists only once both the tab and the analysis do.
    private func rebuildScore() {
        guard let analysis, !analysis.downbeats.isEmpty, hasTab || !tabNotes.isEmpty else {
            score = nil
            return
        }
        score = Score(notes: tabNotes, analysis: analysis, duration: player.duration)
    }

    nonisolated static func isEdited(_ url: URL) -> Bool {
        struct File: Decodable { let edited: Bool? }
        guard let data = try? Data(contentsOf: url) else { return false }
        return (try? JSONDecoder().decode(File.self, from: data).edited) == true
    }

    nonisolated static func loadTab(_ url: URL) -> [TabNote] {
        struct File: Decodable { let notes: [TabNote] }
        guard let data = try? Data(contentsOf: url) else { return [] }
        return (try? JSONDecoder().decode(File.self, from: data).notes) ?? []
    }

    // The path hash keeps two different files with the same name apart.
    nonisolated static func folderName(for audio: URL) -> String {
        let hash = audio.path.utf8.reduce(UInt64(5381)) { ($0 << 5) &+ $0 &+ UInt64($1) }
        return "\(audio.deletingPathExtension().lastPathComponent)-\(String(hash, radix: 16))"
    }

    nonisolated static func stems(in folder: URL) -> [(name: String, url: URL)] {
        let urls = (try? FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil)) ?? []
        let found = urls.filter { $0.pathExtension == "wav" }
            .map { (name: $0.deletingPathExtension().lastPathComponent, url: $0) }
        func rank(_ name: String) -> Int { stemOrder.firstIndex(of: name) ?? stemOrder.count }
        return found.sorted { (rank($0.name), $0.name) < (rank($1.name), $1.name) }
    }
}
