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

    private(set) var state = State.idle
    let player = StemPlayer()

    // Display order; any extra stem the engine produces is appended alphabetically.
    private static let stemOrder = ["vocals", "drums", "bass", "other"]

    static let libraryRoot = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        .appendingPathComponent("Urdr/Songs")

    func open(_ audio: URL) async {
        let folder = Self.libraryRoot.appendingPathComponent(Self.folderName(for: audio))
        let stemsFolder = folder.appendingPathComponent("stems")
        do {
            // htdemucs always yields 4 stems, so fewer means an interrupted run.
            if Self.stems(in: stemsFolder).count < 4 {
                state = .separating(0)
                try await Engine.separate(audio: audio, out: stemsFolder) { fraction in
                    Task { @MainActor in self.state = .separating(fraction) }
                }
            }
            try player.load(Self.stems(in: stemsFolder))
            state = .ready(title: audio.deletingPathExtension().lastPathComponent)
        } catch {
            state = .failed(error.localizedDescription)
        }
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
