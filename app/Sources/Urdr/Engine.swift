import Foundation

enum EngineEvent: Equatable {
    case progress(Double)
    case done([String])
    case error(String)

    init?(line: String) {
        guard let object = try? JSONSerialization.jsonObject(with: Data(line.utf8)) as? [String: Any],
              let event = object["event"] as? String else { return nil }
        switch event {
        case "progress": self = .progress(object["fraction"] as? Double ?? 0)
        case "done": self = .done(object["stems"] as? [String] ?? [])
        case "error": self = .error(object["message"] as? String ?? "unknown error")
        default: return nil
        }
    }
}

struct EngineError: LocalizedError {
    let message: String
    var errorDescription: String? { message }
}

enum Engine {
    // shortcut: the engine is run from the source checkout, replace with a bundled copy if the app is ever packaged.
    static let directory = URL(fileURLWithPath: ProcessInfo.processInfo.environment["URDR_ENGINE_DIR"]
        ?? NSHomeDirectory() + "/.t3/projects/urdr/engine")

    static func separate(audio: URL, out: URL, onProgress: @escaping @Sendable (Double) -> Void) async throws {
        try FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)
        try await run(["separate", audio.path, out.path], log: out.appendingPathComponent("engine.log"), onProgress: onProgress)
    }

    static func tab(audio: URL, out: URL) async throws {
        try await run(["tab", audio.path, out.path], log: out.deletingLastPathComponent().appendingPathComponent("tab.log")) { _ in }
    }

    static func analyze(audio: URL, out: URL) async throws {
        try await run(["analyze", audio.path, out.path], log: out.deletingLastPathComponent().appendingPathComponent("analysis.log")) { _ in }
    }

    private static func run(_ command: [String], log: URL, onProgress: @escaping @Sendable (Double) -> Void) async throws {
        FileManager.default.createFile(atPath: log.path, contents: nil)

        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = ["uv", "run", "--project", directory.path, "python",
                             directory.appendingPathComponent("main.py").path] + command
        // GUI apps start with a minimal PATH that lacks Homebrew, where uv lives.
        var environment = ProcessInfo.processInfo.environment
        environment["PATH"] = "/opt/homebrew/bin:/usr/local/bin:" + (environment["PATH"] ?? "/usr/bin:/bin")
        process.environment = environment
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = try FileHandle(forWritingTo: log)
        try process.run()

        var failure: String?
        for try await line in pipe.fileHandleForReading.bytes.lines {
            switch EngineEvent(line: line) {
            case .progress(let fraction): onProgress(fraction)
            case .error(let message): failure = message
            default: break
            }
        }
        process.waitUntilExit()
        if process.terminationStatus != 0 {
            throw EngineError(message: failure ?? "engine exited with status \(process.terminationStatus), see \(log.path)")
        }
    }
}
