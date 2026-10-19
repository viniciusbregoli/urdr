import XCTest
@testable import Urdr

final class UrdrTests: XCTestCase {
    func testEngineEventParsing() {
        XCTAssertEqual(EngineEvent(line: #"{"event": "progress", "stage": "separate", "fraction": 0.5}"#), .progress(0.5))
        XCTAssertEqual(EngineEvent(line: #"{"event": "done", "stems": ["bass", "drums"]}"#), .done(["bass", "drums"]))
        XCTAssertEqual(EngineEvent(line: #"{"event": "error", "message": "boom"}"#), .error("boom"))
        XCTAssertNil(EngineEvent(line: "Warning: not json"))
        XCTAssertNil(EngineEvent(line: #"{"event": "unknown"}"#))
    }

    func testFolderNameSeparatesSameNamedFiles() {
        let a = AppModel.folderName(for: URL(fileURLWithPath: "/a/song.mp3"))
        let b = AppModel.folderName(for: URL(fileURLWithPath: "/b/song.mp3"))
        XCTAssertNotEqual(a, b)
        XCTAssertTrue(a.hasPrefix("song-"))
    }

    func testLoadTabReadsEngineOutput() throws {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("\(UUID().uuidString).json")
        defer { try? FileManager.default.removeItem(at: url) }
        try Data(#"{"tuning": [64], "notes": [{"start": 1.5, "end": 2, "midi": 69, "amp": 0.4, "string": 1, "fret": 5}]}"#.utf8).write(to: url)
        XCTAssertEqual(AppModel.loadTab(url), [AppModel.TabNote(start: 1.5, end: 2, midi: 69, string: 1, fret: 5)])
        XCTAssertEqual(AppModel.loadTab(url.appendingPathExtension("missing")), [])
    }

    func testStemsAreOrderedForDisplay() throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        for name in ["bass", "other", "vocals", "drums", "guitar", "zither"] {
            FileManager.default.createFile(atPath: folder.appendingPathComponent("\(name).wav").path, contents: Data())
        }
        XCTAssertEqual(AppModel.stems(in: folder).map(\.name), ["vocals", "guitar", "drums", "bass", "other", "zither"])
    }

    func testSynthWindowTriggersEachNoteOnce() {
        let notes = [0.0, 0.5, 1.0].map { AppModel.TabNote(start: $0, end: $0 + 0.4, midi: 60, string: 2, fret: 1) }
        let ticks = [0.0, 0.3, 0.5, 0.9, 1.2]
        let triggered = zip(ticks, ticks.dropFirst()).flatMap { StemPlayer.notesStarting(in: $0..<$1, notes) }
        XCTAssertEqual(triggered.map(\.start), [0.0, 0.5, 1.0])
    }
}
