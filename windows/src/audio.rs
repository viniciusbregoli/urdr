// Stem playback, port of StemPlayer.swift. The stems are mixed with their gains, then one Signalsmith stretcher
// changes speed (pitch kept) and pitch (tempo kept) and also converts the file rate to the device rate.
// The synth guitar is added after the stretcher: its notes follow the source clock and are transposed as MIDI.
use crate::song::TabNote;
use crate::synth::{Synth, TONES};
use signalsmith_stretch::Stretch;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct Stem {
    pub name: String,
    // Interleaved stereo. shortcut: whole songs sit in memory as i16 (about 60 MB per stem for 6 minutes), stream from disk if that hurts.
    samples: Vec<i16>,
    pub peaks: Vec<f32>,
    pub volume: f32,
    pub muted: bool,
    pub solo: bool,
}

pub const PEAK_BUCKETS: usize = 4096;

// Solo wins: with any stem soloed only the soloed ones play, whatever their mute state.
pub fn gains(stems: &[Stem]) -> Vec<f32> {
    let any_solo = stems.iter().any(|s| s.solo);
    stems.iter().map(|s| if (any_solo && s.solo) || (!any_solo && !s.muted) { s.volume } else { 0.0 }).collect()
}

pub struct Player {
    pub stems: Vec<Stem>,
    file_rate: u32,
    out_rate: u32,
    frames: usize,
    // Source frames already fed to the stretcher.
    cursor: usize,
    // Position at the last seek or pause; what is heard never reads earlier than this.
    anchor: f64,
    fraction: f64,
    pub playing: bool,
    pub rate: f32,
    pub semitones: i32,
    pub loop_start: Option<f64>,
    pub loop_end: Option<f64>,
    pub loop_on: bool,
    pub synth_on: bool,
    pub synth_notes: Vec<TabNote>,
    synth: Synth,
    stretch: Option<Stretch>,
    input: Vec<f32>,
    output: Vec<f32>,
}

impl Player {
    pub fn new(out_rate: u32) -> Player {
        Player {
            stems: vec![],
            file_rate: 44100,
            out_rate,
            frames: 0,
            cursor: 0,
            anchor: 0.0,
            fraction: 0.0,
            playing: false,
            rate: 1.0,
            semitones: 0,
            loop_start: None,
            loop_end: None,
            loop_on: false,
            synth_on: false,
            synth_notes: vec![],
            synth: Synth::new(out_rate),
            stretch: None,
            input: vec![],
            output: vec![],
        }
    }

    pub fn load(&mut self, stems: Vec<(String, Vec<i16>)>, file_rate: u32) {
        self.playing = false;
        self.file_rate = file_rate;
        self.frames = stems.iter().map(|s| s.1.len() / 2).min().unwrap_or(0);
        self.stems = stems
            .into_iter()
            .map(|(name, samples)| Stem { peaks: peaks(&samples, PEAK_BUCKETS), name, samples, volume: 1.0, muted: false, solo: false })
            .collect();
        self.stretch = Some(Stretch::preset_default(2, file_rate));
        self.clear_loop();
        // A song opened with the synth on keeps it on, so the new guitar stem starts muted.
        let synth_on = self.synth_on;
        if let Some(guitar) = self.stems.iter_mut().find(|s| s.name == "guitar") {
            guitar.muted = synth_on;
        }
        self.seek(0.0);
    }

    pub fn duration(&self) -> f64 {
        self.frames as f64 / self.file_rate as f64
    }

    fn ratio(&self) -> f64 {
        self.rate as f64 * self.file_rate as f64 / self.out_rate as f64
    }

    // What is coming out of the speakers now, in source seconds: the stretcher holds back some audio.
    pub fn position(&self) -> f64 {
        if !self.playing {
            return self.anchor;
        }
        let latency = self.stretch.as_ref().map_or(0.0, |s| s.input_latency() as f64 + s.output_latency() as f64 * self.ratio());
        ((self.cursor as f64 - latency) / self.file_rate as f64).max(self.anchor)
    }

    pub fn toggle_play(&mut self) {
        if self.playing {
            self.pause()
        } else {
            self.play()
        }
    }

    pub fn play(&mut self) {
        if self.stems.is_empty() || self.playing {
            return;
        }
        let at = self.anchor;
        self.seek(at);
        self.playing = true;
    }

    pub fn pause(&mut self) {
        if !self.playing {
            return;
        }
        let at = self.position();
        self.playing = false;
        self.seek(at);
    }

    // shortcut: the stretcher restarts cold after a seek, so the first few ms fade in. Pre-roll with Stretch::seek if it bothers.
    pub fn seek(&mut self, time: f64) {
        let time = time.clamp(0.0, self.duration());
        self.anchor = time;
        self.cursor = (time * self.file_rate as f64) as usize;
        self.fraction = 0.0;
        if let Some(s) = &mut self.stretch {
            s.reset();
        }
        self.synth.silence(time);
    }

    pub fn skip(&mut self, seconds: f64) {
        self.seek(self.position() + seconds);
    }

    pub fn set_loop_start(&mut self) {
        self.loop_start = Some(self.position());
    }

    pub fn set_loop_end(&mut self) {
        self.loop_end = Some(self.position());
        self.loop_on = self.loop_start.is_some();
    }

    pub fn set_loop(&mut self, a: f64, b: f64) {
        self.loop_start = Some(a.min(b));
        self.loop_end = Some(a.max(b));
        self.loop_on = true;
    }

    pub fn clear_loop(&mut self) {
        self.loop_start = None;
        self.loop_end = None;
        self.loop_on = false;
    }

    pub fn change_rate(&mut self, delta: f32) {
        // Steps of 5 % stay exact instead of drifting by float error.
        self.rate = (((self.rate + delta) * 20.0).round() / 20.0).clamp(0.5, 2.0);
    }

    pub fn change_pitch(&mut self, delta: i32) {
        self.semitones = (self.semitones + delta).clamp(-12, 12);
    }

    pub fn toggle_mute(&mut self, index: usize) {
        if let Some(s) = self.stems.get_mut(index) {
            s.muted = !s.muted;
        }
    }

    pub fn toggle_solo(&mut self, index: usize) {
        if let Some(s) = self.stems.get_mut(index) {
            s.solo = !s.solo;
        }
    }

    // The synth replaces the original guitar, like Songsterr's ORIG/SYNTH switch.
    pub fn toggle_synth(&mut self) {
        self.synth_on = !self.synth_on;
        let on = self.synth_on;
        if let Some(guitar) = self.stems.iter_mut().find(|s| s.name == "guitar") {
            guitar.muted = on;
        }
        self.synth.silence(self.position());
    }

    pub fn tone_name(&self) -> &'static str {
        TONES[self.synth.tone]
    }

    pub fn cycle_tone(&mut self) {
        self.synth.tone = (self.synth.tone + 1) % TONES.len();
        self.synth.silence(self.position());
    }

    fn mix(&mut self, frames: usize) {
        let gains = gains(&self.stems);
        self.input.clear();
        self.input.resize(frames * 2, 0.0);
        let end = (self.cursor + frames).min(self.frames);
        for (stem, gain) in self.stems.iter().zip(gains) {
            if gain == 0.0 || self.cursor >= end {
                continue;
            }
            let source = &stem.samples[self.cursor * 2..end * 2];
            for (out, &s) in self.input.iter_mut().zip(source) {
                *out += s as f32 * (gain / 32768.0);
            }
        }
    }

    // Fills an interleaved device buffer and advances the clock; the audio callback calls this.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        out.fill(0.0);
        if !self.playing || self.stretch.is_none() || channels == 0 {
            return;
        }
        let frames = out.len() / channels;
        self.fraction += frames as f64 * self.ratio();
        let consumed = self.fraction.floor() as usize;
        self.fraction -= consumed as f64;
        self.mix(consumed);
        self.cursor += consumed;
        self.output.clear();
        self.output.resize(frames * 2, 0.0);
        let transpose = 2f32.powf(self.semitones as f32 / 12.0) * self.file_rate as f32 / self.out_rate as f32;
        let stretch = self.stretch.as_mut().unwrap();
        stretch.set_transpose_factor(transpose, None);
        stretch.process(&self.input, &mut self.output);
        for (frame, stereo) in out.chunks_mut(channels).zip(self.output.chunks(2)) {
            if channels == 1 {
                frame[0] = 0.5 * (stereo[0] + stereo[1]);
            } else {
                frame[0] = stereo[0];
                frame[1] = stereo[1];
            }
        }
        let now = self.position();
        if self.synth_on {
            self.synth.advance(now, &self.synth_notes, self.semitones);
            self.synth.render(out, channels);
        }
        // shortcut: the loop and the song end are checked once per device buffer (about 10 ms), not per sample.
        match (self.loop_on, self.loop_start, self.loop_end) {
            (true, Some(start), Some(end)) if start < end && now >= end => self.seek(start),
            _ if now >= self.duration() => {
                self.playing = false;
                self.seek(0.0);
            }
            _ => {}
        }
    }
}

// Max absolute level of the mono mix per bucket, 0...1.
pub fn peaks(samples: &[i16], buckets: usize) -> Vec<f32> {
    let frames = samples.len() / 2;
    if frames == 0 {
        return vec![0.0; buckets];
    }
    (0..buckets)
        .map(|b| {
            let (from, to) = (b * frames / buckets, ((b + 1) * frames / buckets).max(b * frames / buckets + 1).min(frames));
            samples[from * 2..to * 2].chunks(2).map(|f| ((f[0] as i32 + f[1] as i32).abs() / 2) as f32).fold(0.0, f32::max) / 32768.0
        })
        .collect()
}

// Any PCM or float WAV, mono or more channels, as interleaved stereo i16.
pub fn read_wav(path: &Path) -> Result<(Vec<i16>, u32), String> {
    let reader = hound::WavReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let spec = reader.spec();
    let channels = spec.channels as usize;
    let mono: Vec<i16> = match spec.sample_format {
        hound::SampleFormat::Float => reader.into_samples::<f32>().map(|s| s.map(|v| (v.clamp(-1.0, 1.0) * 32767.0) as i16)).collect::<Result<_, _>>(),
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => reader.into_samples::<i16>().collect::<Result<_, _>>(),
        hound::SampleFormat::Int => {
            let shift = spec.bits_per_sample - 16;
            reader.into_samples::<i32>().map(|s| s.map(|v| (v >> shift) as i16)).collect::<Result<_, _>>()
        }
    }
    .map_err(|e| format!("{}: {e}", path.display()))?;
    let stereo = mono.chunks(channels).flat_map(|f| [f[0], if channels > 1 { f[1] } else { f[0] }]).collect();
    Ok((stereo, spec.sample_rate))
}

// Opens the default output device. The callback only locks the player for one buffer.
pub fn start_output(player: Arc<Mutex<Player>>) -> Result<cpal::Stream, String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let device = cpal::default_host().default_output_device().ok_or("no audio output device")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let config: cpal::StreamConfig = supported.config();
    let channels = config.channels as usize;
    player.lock().unwrap().out_rate_changed(config.sample_rate.0);
    let on_error = |e| eprintln!("audio stream error: {e}");
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config,
            move |data: &mut [f32], _| player.lock().unwrap().render(data, channels),
            on_error,
            None,
        ),
        cpal::SampleFormat::I16 => {
            let mut buffer = vec![];
            device.build_output_stream(
                &config,
                move |data: &mut [i16], _| {
                    buffer.resize(data.len(), 0.0);
                    player.lock().unwrap().render(&mut buffer, channels);
                    for (d, s) in data.iter_mut().zip(&buffer) {
                        *d = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                    }
                },
                on_error,
                None,
            )
        }
        other => return Err(format!("unsupported sample format {other}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

impl Player {
    fn out_rate_changed(&mut self, rate: u32) {
        self.out_rate = rate;
        self.synth = Synth::new(rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::tests::pitch;

    fn sine(hz: f64, seconds: f64, rate: u32) -> Vec<i16> {
        (0..(seconds * rate as f64) as usize)
            .flat_map(|i| {
                let v = ((i as f64 / rate as f64 * hz * std::f64::consts::TAU).sin() * 16000.0) as i16;
                [v, v]
            })
            .collect()
    }

    fn constant(value: i16, frames: usize) -> Vec<i16> {
        vec![value; frames * 2]
    }

    fn player(stems: Vec<(String, Vec<i16>)>, file_rate: u32, out_rate: u32) -> Player {
        let mut p = Player::new(out_rate);
        p.load(stems, file_rate);
        p
    }

    // Renders in 10 ms device buffers and returns the left channel.
    fn play_for(p: &mut Player, seconds: f64) -> Vec<f32> {
        let chunk = (p.out_rate / 100) as usize;
        let mut left = vec![];
        for _ in 0..(seconds * 100.0) as usize {
            let mut out = vec![0.0; chunk * 2];
            p.render(&mut out, 2);
            left.extend(out.iter().step_by(2));
        }
        left
    }

    #[test]
    fn gains_apply_mute_and_solo_wins() {
        let mut p = player(vec![("vocals".into(), constant(1000, 10)), ("guitar".into(), constant(2000, 10)), ("drums".into(), constant(4000, 10))], 44100, 44100);
        p.stems[0].volume = 0.5;
        assert_eq!(gains(&p.stems), [0.5, 1.0, 1.0]);
        p.toggle_mute(1);
        assert_eq!(gains(&p.stems), [0.5, 0.0, 1.0]);
        p.mix(4);
        assert!((p.input[0] - (500.0 + 4000.0) / 32768.0).abs() < 1e-6);
        // Solo wins over mute: the soloed muted guitar plays, everything else goes quiet.
        p.toggle_solo(1);
        assert_eq!(gains(&p.stems), [0.0, 1.0, 0.0]);
        p.toggle_solo(2);
        assert_eq!(gains(&p.stems), [0.0, 1.0, 1.0]);
        p.mix(4);
        assert!((p.input[1] - 6000.0 / 32768.0).abs() < 1e-6);
    }

    #[test]
    fn synth_mutes_the_guitar_stem() {
        let mut p = player(vec![("vocals".into(), constant(1, 10)), ("guitar".into(), constant(1, 10))], 44100, 44100);
        p.toggle_synth();
        assert_eq!(gains(&p.stems), [1.0, 0.0]);
        p.toggle_synth();
        assert_eq!(gains(&p.stems), [1.0, 1.0]);
    }

    #[test]
    fn half_speed_doubles_the_length_and_keeps_the_pitch() {
        let mut p = player(vec![("other".into(), sine(440.0, 3.0, 44100))], 44100, 44100);
        p.rate = 0.5;
        p.play();
        let out = play_for(&mut p, 2.0);
        // Two seconds out at half speed is one second of the song.
        assert_eq!(p.cursor, 44100);
        let hz = pitch(&out[44100..66150], 44100.0, 440.0);
        assert!((hz / 440.0).log2().abs() * 1200.0 < 10.0, "measured {hz} Hz");
    }

    #[test]
    fn double_speed_halves_the_length() {
        let mut p = player(vec![("other".into(), sine(440.0, 4.0, 44100))], 44100, 44100);
        p.rate = 2.0;
        p.play();
        let out = play_for(&mut p, 1.0);
        assert_eq!(p.cursor, 88200);
        assert!((pitch(&out[22050..33075], 44100.0, 440.0) / 440.0 - 1.0).abs() < 0.01);
    }

    #[test]
    fn pitch_shift_keeps_the_tempo_and_device_rate_is_converted() {
        // A 44.1 kHz file on a 48 kHz device, an octave up.
        let mut p = player(vec![("other".into(), sine(440.0, 3.0, 44100))], 44100, 48000);
        p.semitones = 12;
        p.play();
        let out = play_for(&mut p, 1.0);
        assert_eq!(p.cursor, 44100, "one second of output is one second of song");
        let hz = pitch(&out[24000..36000], 48000.0, 880.0);
        assert!((hz / 880.0).log2().abs() * 1200.0 < 10.0, "measured {hz} Hz");
    }

    #[test]
    fn loop_wraps_back_to_its_start() {
        let mut p = player(vec![("other".into(), sine(220.0, 4.0, 44100))], 44100, 44100);
        p.seek(0.5);
        p.set_loop_start();
        p.seek(1.0);
        p.set_loop_end();
        assert!(p.loop_on);
        p.seek(0.5);
        p.play();
        let mut wrapped = false;
        let mut last = p.position();
        for _ in 0..200 {
            play_for(&mut p, 0.01);
            let now = p.position();
            assert!((0.5..1.011).contains(&now), "position {now} left the loop");
            wrapped |= now < last;
            last = now;
        }
        assert!(wrapped);
        assert!(p.playing);
    }

    #[test]
    fn song_end_stops_and_rewinds() {
        let mut p = player(vec![("other".into(), sine(220.0, 0.5, 44100))], 44100, 44100);
        p.play();
        play_for(&mut p, 1.0);
        assert!(!p.playing);
        assert_eq!(p.position(), 0.0);
    }

    #[test]
    fn pause_keeps_the_heard_position() {
        let mut p = player(vec![("other".into(), sine(220.0, 3.0, 44100))], 44100, 44100);
        p.seek(1.0);
        p.play();
        play_for(&mut p, 0.5);
        let heard = p.position();
        assert!(heard > 1.0 && heard <= 1.5, "heard {heard}");
        p.pause();
        assert_eq!(p.position(), heard);
        p.skip(-5.0);
        assert_eq!(p.position(), 0.0);
    }

    #[test]
    fn speed_and_pitch_steppers_clamp() {
        let mut p = Player::new(44100);
        for _ in 0..30 {
            p.change_rate(-0.05);
        }
        assert_eq!(p.rate, 0.5);
        for _ in 0..40 {
            p.change_rate(0.05);
        }
        assert_eq!(p.rate, 2.0);
        p.change_pitch(20);
        assert_eq!(p.semitones, 12);
        p.change_pitch(-30);
        assert_eq!(p.semitones, -12);
    }

    #[test]
    fn wav_reading_and_peaks() {
        let dir = crate::song::temp_dir("wav");
        let path = dir.join("mono.wav");
        let spec = hound::WavSpec { channels: 1, sample_rate: 22050, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for v in [0i16, 16384, -32768, 100] {
            w.write_sample(v).unwrap();
        }
        w.finalize().unwrap();
        let (samples, rate) = read_wav(&path).unwrap();
        assert_eq!(rate, 22050);
        assert_eq!(samples, [0, 0, 16384, 16384, -32768, -32768, 100, 100]);
        assert_eq!(peaks(&samples, 2), [0.5, 1.0]);
        std::fs::remove_dir_all(dir).ok();
    }
}
