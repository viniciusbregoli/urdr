import AVFoundation

enum Waveform {
    // Peak level per bucket for each stem, scaled by the loudest stem so a quiet part also looks quiet.
    nonisolated static func peaks(for stems: [(name: String, url: URL)], buckets: Int = 1200) -> [String: [Float]] {
        var raw: [String: [Float]] = [:]
        for stem in stems {
            raw[stem.name] = try? peaks(of: stem.url, buckets: buckets)
        }
        let loudest = raw.values.flatMap { $0 }.max() ?? 0
        guard loudest > 0 else { return raw }
        return raw.mapValues { $0.map { $0 / loudest } }
    }

    nonisolated static func peaks(of url: URL, buckets: Int) throws -> [Float] {
        let file = try AVAudioFile(forReading: url)
        let perBucket = max(1, Int(file.length) / buckets)
        guard let buffer = AVAudioPCMBuffer(pcmFormat: file.processingFormat, frameCapacity: AVAudioFrameCount(perBucket)) else { return [] }
        var result: [Float] = []
        result.reserveCapacity(buckets)
        while file.framePosition < file.length {
            try file.read(into: buffer, frameCount: AVAudioFrameCount(perBucket))
            guard let channels = buffer.floatChannelData, buffer.frameLength > 0 else { break }
            var peak: Float = 0
            for channel in 0..<Int(buffer.format.channelCount) {
                for i in 0..<Int(buffer.frameLength) { peak = max(peak, abs(channels[channel][i])) }
            }
            result.append(peak)
        }
        return result
    }
}
