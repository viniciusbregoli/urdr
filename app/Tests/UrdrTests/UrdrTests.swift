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

    func testSynthWindowTriggersEachNoteOnce() {
        let notes = [0.0, 0.5, 1.0].map { AppModel.TabNote(start: $0, end: $0 + 0.4, midi: 60, string: 2, fret: 1) }
        let ticks = [0.0, 0.3, 0.5, 0.9, 1.2]
        let triggered = zip(ticks, ticks.dropFirst()).flatMap { StemPlayer.notesStarting(in: $0..<$1, notes) }
        XCTAssertEqual(triggered.map(\.start), [0.0, 0.5, 1.0])
    }

    func testScoreSnapsNotesIntoMeasures() {
        // 120 BPM in 4/4: beats every 0.5 s, bars every 2 s.
        let analysis = Analysis(bpm: 120, beats: stride(from: 0.0, to: 4, by: 0.5).map { $0 },
                                downbeats: [0, 2], chords: [.init(start: 0, end: 2, label: "A:min"), .init(start: 2, end: 4, label: "N")])
        func note(_ start: Double, _ end: Double) -> AppModel.TabNote {
            AppModel.TabNote(start: start, end: end, midi: 69, string: 1, fret: 5)
        }
        let score = Score(notes: [note(0, 0.25), note(0.26, 0.37), note(0.37, 0.5), note(1.99, 3)], analysis: analysis, duration: 4)

        XCTAssertEqual(score.measures.map(\.beatCount), [4, 4])
        XCTAssertEqual(score.measures[0].notes.map(\.slot), [0, 0.5, 0.75])
        XCTAssertEqual(score.measures[0].notes.map(\.sixteenths), [2, 1, 1])
        // 1.99 s is a hair before the second downbeat and belongs on it.
        XCTAssertEqual(score.measures[1].notes.map(\.slot), [0])
        XCTAssertEqual(score.measures[1].notes[0].sixteenths, 4)
        XCTAssertEqual(score.measures[0].chords, [Score.Chord(slot: 0, name: "Am")])
        XCTAssertEqual(score.measures[1].chords, [])

        let measure = score.measures[1]
        XCTAssertEqual(measure.beatPosition(at: 2.75), 1.5, accuracy: 1e-9)
        XCTAssertEqual(measure.time(atBeat: 1.5), 2.75, accuracy: 1e-9)
        XCTAssertEqual(score.measureIndex(at: 3.9), 1)

        // A one-beat pickup before the first downbeat must not become the time signature.
        let pickup = Score(notes: [note(0.1, 0.4), note(0.6, 0.9)],
                           analysis: Analysis(bpm: 120, beats: [0.5, 1, 1.5, 2, 2.5, 3, 3.5, 4, 4.5], downbeats: [0.5, 2.5], chords: []),
                           duration: 4.5)
        XCTAssertEqual(pickup.measures.first?.number, 1)
        XCTAssertEqual(pickup.beatsPerBar, 4)

        // A double-stop on beat 1 followed by a note on beat 2 is two quarter notes, not sixteenths.
        let doubleStop = Score(notes: [note(0, 0.5), AppModel.TabNote(start: 0, end: 0.5, midi: 64, string: 2, fret: 5), note(0.5, 1)],
                               analysis: analysis, duration: 4)
        XCTAssertEqual(doubleStop.measures[0].notes.map(\.sixteenths), [4, 4, 4])

        // The tempo drifts to 0.6 s beats while the average BPM says 0.5 s: a one-beat note is still a quarter.
        let drifting = Score(notes: [note(0, 0.6)],
                             analysis: Analysis(bpm: 120, beats: [0, 0.6, 1.2, 1.8, 2.4, 3.0], downbeats: [0, 2.4], chords: []),
                             duration: 3)
        XCTAssertEqual(drifting.measures[0].notes.map(\.sixteenths), [4])
    }

    func testLoopEndsAreOrdered() {
        XCTAssertTrue(StemPlayer.ordered(8, 3) == (3, 8))
        XCTAssertTrue(StemPlayer.ordered(3, 8) == (3, 8))
        XCTAssertTrue(StemPlayer.ordered(nil, 5) == (nil, 5))
        XCTAssertTrue(StemPlayer.ordered(5, nil) == (5, nil))
    }
}
