import XCTest
@testable import Urdr

final class ChordSheetTests: XCTestCase {
    // 120 BPM, bars every 2 s from 1 s, with a 1 s pickup before the first downbeat.
    private let analysis = Analysis(
        bpm: 120, beats: stride(from: 1.0, to: 9, by: 0.5).map { $0 }, downbeats: [1, 3, 5, 7],
        chords: [.init(start: 0, end: 1, label: "N"), .init(start: 1, end: 4, label: "A:min"),
                 .init(start: 4, end: 5, label: "E"), .init(start: 5, end: 7, label: "E"),
                 .init(start: 7, end: 9, label: "C")])

    func testBarsListTheirChords() {
        let sheet = ChordSheet(analysis: analysis, duration: 9)
        XCTAssertEqual(sheet.bars.map { $0.chords.map(\.name) }, [["N.C."], ["Am"], ["Am", "E"], ["E"], ["C"]])
        // E arrives halfway through bar 3 (4 s in a 3-5 s bar).
        XCTAssertEqual(sheet.bars[2].chords.map(\.at), [0, 0.5])
        XCTAssertEqual(sheet.bars.map(\.start), [0, 1, 3, 5, 7])
        XCTAssertEqual(sheet.bars[1].beats, [1, 1.5, 2, 2.5])
        XCTAssertEqual(sheet.barIndex(at: 4.2), 2)
        XCTAssertNil(sheet.barIndex(at: 9.5))
    }

    func testChangeJustAfterTheDownbeatOwnsTheBar() {
        // The previous chord leaks 0.1 s past the downbeat at 6.5 s, as in real tracker output.
        let jittery = Analysis(bpm: 72, beats: [3.2, 6.5, 9.8], downbeats: [3.2, 6.5, 9.8],
                               chords: [.init(start: 4.8, end: 6.6, label: "D"), .init(start: 6.6, end: 9.6, label: "F"),
                                        .init(start: 9.6, end: 12, label: "G")])
        let bar = ChordSheet(analysis: jittery, duration: 13).bars.first { $0.start == 6.5 }
        XCTAssertEqual(bar?.chords.map(\.name), ["F", "G"])
    }

    func testUpcomingSkipsRepeatsAndNoChord() {
        XCTAssertEqual(ChordSheet.upcoming(analysis, at: 0.5, count: 3).map(\.name), ["Am", "E", "C"])
        XCTAssertEqual(ChordSheet.upcoming(analysis, at: 4.5, count: 3).map(\.name), ["E", "C"])
    }

    func testShapes() {
        XCTAssertEqual(ChordShape.shape(for: "C")?.frets, [nil, 3, 2, 0, 1, 0])
        XCTAssertEqual(ChordShape.shape(for: "A:min")?.frets, [nil, 0, 2, 2, 1, 0])
        XCTAssertEqual(ChordShape.shape(for: "F")?.frets, [1, 3, 3, 2, 1, 1])
        XCTAssertEqual(ChordShape.shape(for: "F")?.baseFret, 1)
        // Bb sits lower as an A-shape barre at 1 than as an E-shape at 6.
        XCTAssertEqual(ChordShape.shape(for: "Bb")?.frets, [nil, 1, 3, 3, 3, 1])
        XCTAssertEqual(ChordShape.shape(for: "F#:min")?.frets, [2, 4, 4, 2, 2, 2])
        XCTAssertEqual(ChordShape.shape(for: "C#")?.baseFret, 4)
        XCTAssertNil(ChordShape.shape(for: "N"))
    }
}
