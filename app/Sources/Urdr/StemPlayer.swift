import AVFoundation
import Observation

@Observable @MainActor
final class StemPlayer {
    struct Stem: Identifiable {
        let id: String
        var volume: Float = 1
        var muted = false
        var solo = false
    }

    private(set) var stems: [Stem] = []
    private(set) var isPlaying = false
    private(set) var duration = 0.0
    private(set) var position = 0.0
    var loopStart: Double?
    var loopEnd: Double?
    var loopOn = false
    var rate: Float = 1 { didSet { timePitch.rate = rate } }
    var semitones = 0 { didSet { timePitch.pitch = Float(semitones) * 100 } }
    private(set) var synthOn = false
    private(set) var tone = 0
    var synthNotes: [AppModel.TabNote] = []

    static let tones: [(name: String, program: UInt8)] = [("Overdriven", 29), ("Clean", 27), ("Acoustic", 25)]

    @ObservationIgnored private let engine = AVAudioEngine()
    @ObservationIgnored private let mixer = AVAudioMixerNode()
    @ObservationIgnored private let timePitch = AVAudioUnitTimePitch()
    @ObservationIgnored private var players: [String: AVAudioPlayerNode] = [:]
    @ObservationIgnored private var files: [String: AVAudioFile] = [:]
    @ObservationIgnored private var sampleRate = 44100.0
    @ObservationIgnored private var startPosition = 0.0
    @ObservationIgnored private var ticker: Task<Void, Never>?
    @ObservationIgnored private let sampler = AVAudioUnitSampler()
    @ObservationIgnored private var synthClock = 0.0
    @ObservationIgnored private var sounding: [(midi: UInt8, end: Double)] = []

    init() {
        engine.attach(mixer)
        engine.attach(timePitch)
        engine.connect(timePitch, to: engine.mainMixerNode, format: nil)
        // The synth bypasses the time-pitch unit: tempo follows the source clock and transposing shifts the MIDI notes.
        engine.attach(sampler)
        engine.connect(sampler, to: engine.mainMixerNode, format: nil)
        loadTone()
        ticker = Task { [weak self] in
            while !Task.isCancelled {
                // 10 ms keeps synth note onsets tight; the observable position is throttled in tick().
                try? await Task.sleep(for: .milliseconds(10))
                self?.tick()
            }
        }
    }

    func load(_ urls: [(name: String, url: URL)]) throws {
        pause()
        engine.stop()
        for player in players.values {
            engine.detach(player)
        }
        players = [:]
        files = [:]
        for (name, url) in urls {
            files[name] = try AVAudioFile(forReading: url)
        }
        guard let format = files.values.first?.processingFormat else { return }
        sampleRate = format.sampleRate
        duration = Double(files.values.map(\.length).min() ?? 0) / sampleRate
        engine.connect(mixer, to: timePitch, format: format)
        for (name, _) in urls {
            let player = AVAudioPlayerNode()
            engine.attach(player)
            engine.connect(player, to: mixer, format: format)
            players[name] = player
        }
        stems = urls.map { Stem(id: $0.name) }
        position = 0
        loopStart = nil
        loopEnd = nil
        loopOn = false
        applyGains()
        try engine.start()
    }

    func togglePlay() {
        isPlaying ? pause() : play()
    }

    func play() {
        guard !players.isEmpty, !isPlaying else { return }
        schedule(from: position)
        isPlaying = true
    }

    func pause() {
        guard isPlaying else { return }
        position = currentPosition()
        players.values.forEach { $0.stop() }
        isPlaying = false
        silenceSynth()
    }

    func seek(to time: Double) {
        position = min(max(time, 0), duration)
        silenceSynth()
        if isPlaying { schedule(from: position) }
    }

    func skip(_ seconds: Double) {
        seek(to: (isPlaying ? currentPosition() : position) + seconds)
    }

    func setLoopStart() { (loopStart, loopEnd) = Self.ordered(currentPosition(), loopEnd) }

    func setLoopEnd() {
        (loopStart, loopEnd) = Self.ordered(loopStart, currentPosition())
        loopOn = loopStart != nil
    }

    // Marking the ends in either order still gives a playable loop.
    nonisolated static func ordered(_ start: Double?, _ end: Double?) -> (Double?, Double?) {
        if let start, let end, end < start { return (end, start) }
        return (start, end)
    }

    func clearLoop() {
        loopStart = nil
        loopEnd = nil
        loopOn = false
    }

    func setVolume(_ name: String, _ volume: Float) {
        guard let index = stems.firstIndex(where: { $0.id == name }) else { return }
        stems[index].volume = volume
        applyGains()
    }

    func toggleMute(_ name: String) {
        guard let index = stems.firstIndex(where: { $0.id == name }) else { return }
        stems[index].muted.toggle()
        applyGains()
    }

    func toggleSolo(_ name: String) {
        guard let index = stems.firstIndex(where: { $0.id == name }) else { return }
        stems[index].solo.toggle()
        applyGains()
    }

    // Synth replaces the original guitar, like Songsterr's ORIG/SYNTH switch.
    func toggleSynth() {
        synthOn.toggle()
        if let index = stems.firstIndex(where: { $0.id == "guitar" }) {
            stems[index].muted = synthOn
            applyGains()
        }
        silenceSynth()
    }

    func cycleTone() {
        tone = (tone + 1) % Self.tones.count
        silenceSynth()
        loadTone()
    }

    private func loadTone() {
        let bank = URL(fileURLWithPath: "/System/Library/Components/CoreAudio.component/Contents/Resources/gs_instruments.dls")
        try? sampler.loadSoundBankInstrument(at: bank, program: Self.tones[tone].program,
                                             bankMSB: UInt8(kAUSampler_DefaultMelodicBankMSB),
                                             bankLSB: UInt8(kAUSampler_DefaultBankLSB))
    }

    private func silenceSynth() {
        sounding.forEach { sampler.stopNote($0.midi, onChannel: 0) }
        sounding = []
        synthClock = position
    }

    private func updateSynth(_ now: Double) {
        guard synthOn else { return }
        sounding.removeAll { note in
            if note.end <= now { sampler.stopNote(note.midi, onChannel: 0) }
            return note.end <= now
        }
        // A backwards or long jump is a seek or loop wrap: restart the clock instead of replaying skipped notes.
        let window = now >= synthClock && now - synthClock < 0.5 ? synthClock..<now : now..<now
        for note in Self.notesStarting(in: window, synthNotes) {
            let midi = UInt8(clamping: note.midi + semitones)
            sampler.startNote(midi, withVelocity: 100, onChannel: 0)
            sounding.append((midi, note.end))
        }
        synthClock = now
    }

    // Half-open on the left so a note is never triggered twice across consecutive ticks.
    nonisolated static func notesStarting(in window: Range<Double>, _ notes: [AppModel.TabNote]) -> [AppModel.TabNote] {
        notes.filter { $0.start >= window.lowerBound && $0.start < window.upperBound }
    }

    func isAudible(_ stem: Stem) -> Bool {
        stems.contains(where: \.solo) ? stem.solo : !stem.muted
    }

    private func applyGains() {
        let anySolo = stems.contains(where: \.solo)
        for stem in stems {
            let audible = anySolo ? stem.solo : !stem.muted
            players[stem.id]?.volume = audible ? stem.volume : 0
        }
    }

    // All players are started at one shared host time so the stems stay sample-aligned.
    private func schedule(from time: Double) {
        let frame = AVAudioFramePosition(time * sampleRate)
        for (name, player) in players {
            guard let file = files[name], frame < file.length else { continue }
            player.stop()
            player.scheduleSegment(file, startingFrame: frame,
                                   frameCount: AVAudioFrameCount(file.length - frame), at: nil)
        }
        let start = AVAudioTime(hostTime: mach_absolute_time() + AVAudioTime.hostTime(forSeconds: 0.1))
        players.values.forEach { $0.play(at: start) }
        startPosition = time
    }

    private func currentPosition() -> Double {
        guard isPlaying, let player = players.values.first,
              let render = player.lastRenderTime, let node = player.playerTime(forNodeTime: render)
        else { return position }
        // The player sits before the time-pitch unit, so its clock counts source time whatever the rate.
        return startPosition + max(0, Double(node.sampleTime)) / sampleRate
    }

    private func tick() {
        guard isPlaying else { return }
        let now = currentPosition()
        updateSynth(now)
        if abs(now - position) >= 1.0 / 30 { position = now }
        // shortcut: a loop wrap re-schedules the players, which leaves an audible gap of a few ms.
        if loopOn, let start = loopStart, let end = loopEnd, start < end, now >= end {
            seek(to: start)
        } else if now >= duration {
            pause()
            position = 0
        }
    }
}
