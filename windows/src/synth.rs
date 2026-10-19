// Plucked-string guitar (Karplus-Strong) that plays the tab. Stands in for the Mac's General MIDI sampler.
use crate::song::TabNote;

pub const TONES: [&str; 3] = ["Overdriven", "Clean", "Acoustic"];
const MAX_VOICES: usize = 24;

// Half-open on the left so a note is never triggered twice across consecutive ticks.
pub fn notes_starting(from: f64, to: f64, notes: &[TabNote]) -> impl Iterator<Item = &TabNote> {
    notes.iter().filter(move |n| n.start >= from && n.start < to)
}

struct Voice {
    line: Vec<f32>,
    index: usize,
    // First-order allpass for the fractional part of the delay, so high notes stay in tune.
    coefficient: f32,
    allpass_in: f32,
    allpass_out: f32,
    damping: f32,
    end: f64,
    released: bool,
    level: f32,
}

pub struct Synth {
    rate: f32,
    voices: Vec<Voice>,
    seed: u32,
    pub tone: usize,
    clock: f64,
}

impl Synth {
    pub fn new(rate: u32) -> Synth {
        Synth { rate: rate as f32, voices: vec![], seed: 22222, tone: 0, clock: 0.0 }
    }

    pub fn silence(&mut self, now: f64) {
        self.voices.clear();
        self.clock = now;
    }

    // Starts the notes whose onset passed since the last call. A backwards or long jump is a seek or loop wrap:
    // the clock restarts instead of replaying skipped notes.
    pub fn advance(&mut self, now: f64, notes: &[TabNote], semitones: i32) {
        for v in &mut self.voices {
            if !v.released && v.end <= now {
                v.released = true;
                v.damping = 0.9;
            }
        }
        if now >= self.clock && now - self.clock < 0.5 {
            let starting: Vec<TabNote> = notes_starting(self.clock, now, notes).copied().collect();
            for note in starting {
                self.pluck(note.midi + semitones, note.end);
            }
        }
        self.clock = now;
    }

    fn noise(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.seed >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }

    fn pluck(&mut self, midi: i32, end: f64) {
        let frequency = 440.0 * 2f32.powf((midi - 69) as f32 / 12.0);
        // Averaging a slot with the one written a sample later shortens the loop by half a sample.
        let delay = (self.rate / frequency + 0.5).max(2.0);
        let length = delay.floor() as usize;
        let fraction = delay - length as f32;
        let (damping, darkness) = match self.tone {
            0 => (0.998, 0.2),
            1 => (0.996, 0.0),
            _ => (0.993, 0.6),
        };
        let mut line = Vec::with_capacity(length);
        let mut previous = 0.0;
        for _ in 0..length {
            let n = self.noise();
            previous = darkness * previous + (1.0 - darkness) * n;
            line.push(previous);
        }
        if self.voices.len() >= MAX_VOICES {
            self.voices.remove(0);
        }
        self.voices.push(Voice {
            line,
            index: 0,
            coefficient: (1.0 - fraction) / (1.0 + fraction),
            allpass_in: 0.0,
            allpass_out: 0.0,
            damping,
            end,
            released: false,
            level: 1.0,
        });
    }

    // Adds the synth to an interleaved buffer.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        let overdriven = self.tone == 0;
        for frame in out.chunks_mut(channels) {
            let mut sum = 0.0;
            for v in &mut self.voices {
                let n = v.line.len();
                let current = v.line[v.index];
                let next = v.line[(v.index + 1) % n];
                let averaged = v.damping * 0.5 * (current + next);
                let shifted = v.coefficient * averaged + v.allpass_in - v.coefficient * v.allpass_out;
                v.allpass_in = averaged;
                v.allpass_out = shifted;
                v.line[v.index] = shifted;
                v.index = (v.index + 1) % n;
                v.level = 0.999 * v.level + 0.001 * current.abs();
                sum += current;
            }
            let sample = if overdriven { (sum * 3.0).tanh() * 0.25 } else { sum * 0.3 };
            for s in frame.iter_mut().take(2) {
                *s += sample;
            }
        }
        self.voices.retain(|v| !(v.released && v.level < 1e-4));
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn synth_window_triggers_each_note_once() {
        let notes: Vec<TabNote> = [0.0, 0.5, 1.0].iter().map(|&s| TabNote { start: s, end: s + 0.4, midi: 60, string: 2, fret: 1 }).collect();
        let ticks = [0.0, 0.3, 0.5, 0.9, 1.2];
        let triggered: Vec<f64> = ticks.windows(2).flat_map(|w| notes_starting(w[0], w[1], &notes).map(|n| n.start)).collect();
        assert_eq!(triggered, [0.0, 0.5, 1.0]);
    }

    // Autocorrelation peak near the expected period, refined with a parabola.
    pub fn pitch(signal: &[f32], rate: f64, expected: f64) -> f64 {
        let period = rate / expected;
        let score = |lag: usize| signal.iter().zip(&signal[lag..]).map(|(a, b)| a * b).sum::<f32>() as f64;
        let lags = (period * 0.7) as usize..(period * 1.4) as usize;
        let best = lags.max_by(|&a, &b| score(a).total_cmp(&score(b))).unwrap();
        let (l, c, r) = (score(best - 1), score(best), score(best + 1));
        rate / (best as f64 + 0.5 * (l - r) / (l - 2.0 * c + r))
    }

    #[test]
    fn plucked_note_sounds_at_its_pitch_and_dies_after_release() {
        let rate = 48000;
        let mut synth = Synth::new(rate);
        let notes = [TabNote { start: 0.0, end: 0.5, midi: 69, string: 1, fret: 5 }];
        for (tone, semitones, expected) in [(1, 0, 440.0), (2, 0, 440.0), (0, 12, 880.0), (1, 17, 1174.66)] {
            synth.tone = tone;
            synth.silence(0.0);
            synth.advance(0.0, &notes, semitones);
            synth.advance(0.01, &notes, semitones);
            let mut out = vec![0.0f32; rate as usize];
            synth.render(&mut out, 2);
            let left: Vec<f32> = out.iter().step_by(2).copied().collect();
            let hz = pitch(&left[4800..14400], rate as f64, expected);
            // Within 5 cents.
            assert!((hz / expected).log2().abs() * 1200.0 < 5.0, "tone {tone}: measured {hz} Hz, wanted {expected}");
        }
        // Past the note end the voice is released and goes away.
        synth.advance(1.0, &notes, 0);
        let mut out = vec![0.0f32; rate as usize];
        synth.render(&mut out, 2);
        assert!(synth.voices.is_empty());
    }
}
