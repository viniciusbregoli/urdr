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

    @ObservationIgnored private let engine = AVAudioEngine()
    @ObservationIgnored private let mixer = AVAudioMixerNode()
    @ObservationIgnored private let timePitch = AVAudioUnitTimePitch()
    @ObservationIgnored private var players: [String: AVAudioPlayerNode] = [:]
    @ObservationIgnored private var files: [String: AVAudioFile] = [:]
    @ObservationIgnored private var sampleRate = 44100.0
    @ObservationIgnored private var startPosition = 0.0
    @ObservationIgnored private var ticker: Task<Void, Never>?

    init() {
        engine.attach(mixer)
        engine.attach(timePitch)
        engine.connect(timePitch, to: engine.mainMixerNode, format: nil)
        ticker = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .milliseconds(30))
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
    }

    func seek(to time: Double) {
        position = min(max(time, 0), duration)
        if isPlaying { schedule(from: position) }
    }

    func skip(_ seconds: Double) {
        seek(to: (isPlaying ? currentPosition() : position) + seconds)
    }

    func setLoopStart() { loopStart = currentPosition() }
    func setLoopEnd() { loopEnd = currentPosition(); loopOn = loopStart != nil }

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
        position = currentPosition()
        // shortcut: a loop wrap re-schedules the players, which leaves an audible gap of a few ms.
        if loopOn, let start = loopStart, let end = loopEnd, start < end, position >= end {
            seek(to: start)
        } else if position >= duration {
            pause()
            position = 0
        }
    }
}
