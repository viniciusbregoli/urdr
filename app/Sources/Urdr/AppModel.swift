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

    struct TabNote: Decodable, Equatable {
        let start: Double
        let end: Double
        let midi: Int
        let string: Int
        let fret: Int
    }

    private(set) var state = State.idle
    private(set) var tabNotes: [TabNote] = [] { didSet { player.synthNotes = tabNotes } }
    private(set) var transcribing = false
    private(set) var tabError: String?
    let player = StemPlayer()
    private var folder: URL?

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
            try player.load(Self.stems(in: stemsFolder))
            self.folder = folder
            tabNotes = Self.loadTab(folder.appendingPathComponent("tab.json"))
            tabError = nil
            state = .ready(title: audio.deletingPathExtension().lastPathComponent)
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
        do {
            try await Engine.tab(audio: folder.appendingPathComponent("stems/guitar.wav"), out: out)
            tabNotes = Self.loadTab(out)
        } catch {
            tabError = error.localizedDescription
        }
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
