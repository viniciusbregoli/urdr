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

    func testStemsAreOrderedForDisplay() throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        for name in ["bass", "other", "vocals", "drums", "guitar"] {
            FileManager.default.createFile(atPath: folder.appendingPathComponent("\(name).wav").path, contents: Data())
        }
        XCTAssertEqual(AppModel.stems(in: folder).map(\.name), ["vocals", "drums", "bass", "other", "guitar"])
    }
}
