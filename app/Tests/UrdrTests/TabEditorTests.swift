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

    private func makeModel(_ notes: [AppModel.TabNote]) throws -> (AppModel, URL) {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let model = AppModel()
        model.folder = folder
        model.analysis = Analysis(bpm: 120, beats: stride(from: 0.0, to: 6, by: 0.5).map { $0 }, downbeats: [0, 2, 4], chords: [])
        model.tabNotes = notes
        return (model, folder)
    }

    func testTwoDigitFretIsOneUndoStepAndUndoClearsThePendingDigit() throws {
        let (model, folder) = try makeModel([AppModel.TabNote(start: 0, end: 0.5, midi: 62, string: 3, fret: 7)])
        defer { try? FileManager.default.removeItem(at: folder) }
        model.toggleEditing()
        model.moveSelection(sixteenths: 2)
        let now = Date()
        model.typeDigit(1, at: now)
        model.typeDigit(2, at: now.addingTimeInterval(0.3))
        XCTAssertEqual(model.selectedNote?.fret, 12)
        // One undo removes the whole new note, not just its second digit.
        model.undoEdit()
        XCTAssertNil(model.selectedNote)
        XCTAssertEqual(model.tabNotes.count, 1)

        model.typeDigit(1, at: now)
        model.undoEdit()
        model.typeDigit(2, at: now.addingTimeInterval(0.3))
        XCTAssertEqual(model.selectedNote?.fret, 2)

        // A deletion between the digits ends the entry too: without it, 1, Delete, 2 would make fret 12.
        let later = now.addingTimeInterval(10)
        model.typeDigit(1, at: later)
        model.deleteSelectedNote()
        model.typeDigit(2, at: later.addingTimeInterval(0.3))
        XCTAssertEqual(model.selectedNote?.fret, 2)
    }

    func testStringMoveRefusesToLandOnAChordNote() throws {
        // A double-stop: E on the high e (open) and B on the B string (open) at the same slot.
        let chord = [AppModel.TabNote(start: 0, end: 0.5, midi: 64, string: 1, fret: 0),
                     AppModel.TabNote(start: 0, end: 0.5, midi: 59, string: 2, fret: 0)]
        let (model, folder) = try makeModel(chord)
        defer { try? FileManager.default.removeItem(at: folder) }
        model.toggleEditing()
        model.selection = AppModel.Selection(measure: 0, slot: 0, string: 1)
        model.moveSelectedNote(strings: 1)
        XCTAssertEqual(model.tabNotes, chord)
        XCTAssertEqual(model.selection?.string, 1)
    }

    func testEmptyTabCanBeEditedAndLeft() throws {
        let (model, folder) = try makeModel([])
        defer { try? FileManager.default.removeItem(at: folder) }
        model.hasTab = true
        model.toggleEditing()
        XCTAssertTrue(model.editing)
        model.typeDigit(5)
        XCTAssertEqual(model.tabNotes.count, 1)
        model.deleteSelectedNote()
        XCTAssertTrue(model.tabNotes.isEmpty)
        // With the last note gone the grid stays, so E (or Esc) still leaves edit mode.
        XCTAssertNotNil(model.score)
        model.toggleEditing()
        XCTAssertFalse(model.editing)
    }
}
