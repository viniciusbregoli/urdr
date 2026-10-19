import Foundation

// Hand editing of the generated tab on the measure grid. Every change is saved straight to tab.json.
extension AppModel {
    struct Selection: Equatable {
        var measure: Int
        // Beats from the start of the measure, in sixteenth steps.
        var slot: Double
        var string: Int
    }

    // Mirrors engine/tab.py TUNING: open-string MIDI pitches, string 1 is the high e.
    nonisolated static let tuning = [64, 59, 55, 50, 45, 40]
    nonisolated static let frets = 22

    func toggleEditing() {
        pendingDigit = nil
        if editing {
            editing = false
            return
        }
        guard let score, !transcribing else { return }
        editing = true
        guard selection == nil else { return }
        let measure = score.measureIndex(at: player.position) ?? 0
        let first = score.measures.indices.contains(measure) ? score.measures[measure].notes.first : nil
        selection = Selection(measure: measure, slot: first?.slot ?? 0, string: first?.note.string ?? 1)
    }

    var selectedNote: TabNote? {
        guard let score, let selection, score.measures.indices.contains(selection.measure) else { return nil }
        return score.measures[selection.measure].notes
            .first { $0.slot == selection.slot && $0.note.string == selection.string }?.note
    }

    func moveSelection(sixteenths: Int) {
        guard let score, var selection else { return }
        selection.slot += Double(sixteenths) / 4
        while selection.slot < 0, selection.measure > 0 {
            selection.measure -= 1
            selection.slot += Double(score.measures[selection.measure].beatCount)
        }
        while selection.slot >= Double(score.measures[selection.measure].beatCount), selection.measure + 1 < score.measures.count {
            selection.slot -= Double(score.measures[selection.measure].beatCount)
            selection.measure += 1
        }
        selection.slot = min(max(selection.slot, 0), Double(score.measures[selection.measure].beatCount) - 0.25)
        self.selection = selection
        pendingDigit = nil
    }

    func moveSelection(strings: Int) {
        guard let current = selection?.string else { return }
        selection?.string = min(max(current + strings, 1), 6)
        pendingDigit = nil
    }

    // Two quick digits make a two-digit fret, so "1" then "2" is fret 12.
    func typeDigit(_ digit: Int, at now: Date = .now) {
        guard let selection else { return }
        if let pending = pendingDigit, pending.selection == selection, now.timeIntervalSince(pending.time) < 0.8,
           pending.value > 0, pending.value * 10 + digit <= Self.frets {
            // One undo step for the whole two-digit fret: drop the entry the first digit pushed.
            let beforeFirstDigit = history.popLast()
            let depth = history.count
            setFret(pending.value * 10 + digit)
            if let beforeFirstDigit {
                if history.count > depth { history[history.count - 1] = beforeFirstDigit } else { history.append(beforeFirstDigit) }
            }
            pendingDigit = nil
        } else {
            setFret(digit)
            pendingDigit = (digit, now, selection)
        }
    }

    func setFret(_ fret: Int) {
        guard let score, let selection, (0...Self.frets).contains(fret) else { return }
        let midi = Self.tuning[selection.string - 1] + fret
        var notes = tabNotes
        if let old = selectedNote, let index = notes.firstIndex(of: old) {
            notes[index] = TabNote(start: old.start, end: old.end, midi: midi, string: old.string, fret: fret)
        } else {
            let measure = score.measures[selection.measure]
            let start = measure.time(atBeat: selection.slot)
            let end = measure.time(atBeat: min(selection.slot + 1, Double(measure.beatCount)))
            notes.append(TabNote(start: start, end: end, midi: midi, string: selection.string, fret: fret))
        }
        commit(notes)
    }

    func deleteSelectedNote() {
        guard let old = selectedNote else { return }
        commit(tabNotes.filter { $0 != old })
    }

    // Same pitch on a neighbouring string: the fix the generated tab needs most often.
    func moveSelectedNote(strings: Int) {
        guard let old = selectedNote, let index = tabNotes.firstIndex(of: old) else { return }
        let string = old.string + strings
        guard (1...6).contains(string) else { return }
        let fret = old.midi - Self.tuning[string - 1]
        guard (0...Self.frets).contains(fret), let score, let selection else { return }
        // A chord can already hold a note on the target string at this slot.
        guard !score.measures[selection.measure].notes.contains(where: { $0.slot == selection.slot && $0.note.string == string }) else { return }
        var notes = tabNotes
        notes[index] = TabNote(start: old.start, end: old.end, midi: old.midi, string: string, fret: fret)
        commit(notes)
        self.selection?.string = string
    }

    func undoEdit() {
        guard let previous = history.popLast() else { return }
        pendingDigit = nil
        tabNotes = previous
        save()
    }

    private func commit(_ notes: [TabNote]) {
        guard !transcribing else { return }
        // Any edit ends a two-digit entry; typeDigit sets the pending digit again after its own commit.
        pendingDigit = nil
        history.append(tabNotes)
        if history.count > 200 { history.removeFirst() }
        tabNotes = notes.sorted { $0.start < $1.start }
        tabEdited = true
        save()
    }

    private func save() {
        guard let folder else { return }
        struct File: Encodable { let tuning: [Int]; let notes: [TabNote]; let edited: Bool }
        do {
            let data = try JSONEncoder().encode(File(tuning: Self.tuning, notes: tabNotes, edited: true))
            try data.write(to: folder.appendingPathComponent("tab.json"), options: .atomic)
            hasTab = true
            tabError = nil
        } catch {
            tabError = "Could not save the tab, so the latest edits exist only until the song is closed: \(error.localizedDescription)"
        }
    }
}
