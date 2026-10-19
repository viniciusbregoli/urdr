import XCTest
@testable import Urdr

@MainActor
final class TabEditorTests: XCTestCase {
    func testEditingSession() throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }

        let model = AppModel()
        model.folder = folder
        // 120 BPM in 4/4: beats every 0.5 s, bars every 2 s. One note: D on the G string, fret 7.
        model.analysis = Analysis(bpm: 120, beats: stride(from: 0.0, to: 6, by: 0.5).map { $0 }, downbeats: [0, 2, 4], chords: [])
        model.tabNotes = [AppModel.TabNote(start: 0, end: 0.5, midi: 62, string: 3, fret: 7)]

        model.toggleEditing()
        XCTAssertEqual(model.selection, AppModel.Selection(measure: 0, slot: 0, string: 3))

        // Same pitch one string lower: D string, fret 12.
        model.moveSelectedNote(strings: 1)
        XCTAssertEqual(model.tabNotes.map { [$0.string, $0.fret, $0.midi] }, [[4, 12, 62]])
        XCTAssertEqual(model.selection?.string, 4)

        // Two quick digits make fret 12 on the high e, an eighth note after the first note.
        model.moveSelection(sixteenths: 2)
        model.moveSelection(strings: -3)
        let now = Date()
        model.typeDigit(1, at: now)
        model.typeDigit(2, at: now.addingTimeInterval(0.3))
        XCTAssertEqual(model.tabNotes.count, 2)
        XCTAssertEqual(model.selectedNote, AppModel.TabNote(start: 0.25, end: 0.75, midi: 76, string: 1, fret: 12))

        // A slow second digit replaces the fret instead of extending it.
        model.typeDigit(3, at: now.addingTimeInterval(5))
        XCTAssertEqual(model.selectedNote?.fret, 3)

        model.deleteSelectedNote()
        XCTAssertEqual(model.tabNotes.count, 1)
        model.undoEdit()
        XCTAssertEqual(model.selectedNote?.fret, 3)

        let saved = folder.appendingPathComponent("tab.json")
        XCTAssertTrue(AppModel.isEdited(saved))
        XCTAssertEqual(AppModel.loadTab(saved), model.tabNotes)

        // The cursor walks across the bar line.
        model.selection = AppModel.Selection(measure: 0, slot: 3.75, string: 1)
        model.moveSelection(sixteenths: 1)
        XCTAssertEqual(model.selection, AppModel.Selection(measure: 1, slot: 0, string: 1))
        model.moveSelection(sixteenths: -1)
        XCTAssertEqual(model.selection, AppModel.Selection(measure: 0, slot: 3.75, string: 1))
    }
}
