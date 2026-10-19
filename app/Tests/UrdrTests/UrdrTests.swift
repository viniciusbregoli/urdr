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

    func testLoadAnalysisReadsEngineOutput() throws {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("\(UUID().uuidString).json")
        defer { try? FileManager.default.removeItem(at: url) }
        try Data(#"{"bpm": 120.0, "beats": [0.0, 0.5, 1.0, 1.5, 2.0, 2.5], "downbeats": [0.0, 2.0], "chords": [{"start": 0.0, "end": 2.0, "label": "C"}, {"start": 2.0, "end": 3.0, "label": "A:min"}, {"start": 3.0, "end": 4.0, "label": "N"}]}"#.utf8).write(to: url)
        let analysis = try XCTUnwrap(Analysis.load(url))
        XCTAssertEqual(analysis.bpm, 120)
        XCTAssertEqual(analysis.downbeats, [0, 2])
        XCTAssertEqual(analysis.chords.map(\.name), ["C", "Am", "N.C."])
        XCTAssertEqual(analysis.chordIndex(at: 2.5), 1)
        XCTAssertNil(analysis.chordIndex(at: 5))
        XCTAssertEqual(analysis.downbeat(from: 0.5, forward: true), 2)
        XCTAssertEqual(analysis.downbeat(from: 2.1, forward: false), 0, "a downbeat just passed is skipped")
        XCTAssertEqual(analysis.downbeat(from: 2.5, forward: false), 2)
        XCTAssertNil(analysis.downbeat(from: 2, forward: true))
        XCTAssertNil(Analysis.load(url.appendingPathExtension("missing")))
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
}
