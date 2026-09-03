//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::assert_matches;
use std::f32::consts::TAU;
use std::sync::Arc;

use crate::sources::MemorySound;
use crate::{NextState, RawedioError, Sound};

/// A constant pitch sound of infinite length.
pub struct SineWave {
    freq: f32,
    sample_rate: u32,
    sample_num: u32,
    reset_num: u32,
}

impl SineWave {
    /// A constant pitch sound with a default sample rate of 48,000.
    #[must_use]
    pub fn new(freq: f32) -> SineWave {
        Self::with_sample_rate(freq, 48000)
    }

    /// A constant pitch sound with `sample_rate`.
    ///
    /// freq must be greater than 0 Hz.
    #[must_use]
    pub fn with_sample_rate(freq: f32, sample_rate: u32) -> SineWave {
        assert!(freq > 0.0);
        let reset_num = find_reset_num(freq, sample_rate);

        SineWave {
            freq,
            sample_rate,
            sample_num: 0,
            reset_num,
        }
    }

    /// Precompute samples into a looping memory sound
    #[must_use]
    pub fn as_memory_sound(freq: f32, sample_rate: u32) -> MemorySound {
        let mut sine_wave = SineWave::with_sample_rate(freq, sample_rate);
        let mut samples = vec![0; (sine_wave.reset_num + 1) as usize];
        assert_matches!(
            sine_wave.fill_next_frames(&mut samples),
            Ok((_, NextState::Playing))
        );
        let mut sound = MemorySound::from_samples(Arc::new(samples), 1, sample_rate);
        sound.set_looping(true);
        sound
    }

    fn advance(&mut self) -> i16 {
        if self.sample_num == self.reset_num {
            self.sample_num = 0;
        } else {
            self.sample_num += 1;
        }
        sample_for(self.sample_num as f32, self.freq, self.sample_rate as f32)
    }
}

/// find a sample number where we can reset to 0 where the value will
/// be close to 0 to minimize distortion when resetting.
///
/// We want to minimize the sample number though because large numbers
/// cause distortions and also minimize memory usage for `as_memory_sound`.
fn find_reset_num(freq: f32, sample_rate: u32) -> u32 {
    let mut best_error = i16::MAX;
    let mut best_reset_num = 0;

    for multiple in 1..=500 {
        let reset_num = sample_rate as f64 * (multiple as f64) / freq as f64;
        let actual_sample = sample_for(reset_num.round() as f32, freq, sample_rate as f32);
        let error = actual_sample.abs(); // 0 is our ideal sample
        if error < best_error {
            best_error = error;
            best_reset_num = reset_num.round() as u32;
            if error < 100 {
                // if the error is less than 0.3% lets go with it
                break;
            }
        }
    }

    // We start at 0 so we don't want to end at 0 as it would duplicate 0 so we
    // subtract 1
    best_reset_num - 1
}

impl Sound for SineWave {
    fn channel_count(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn fill_next_frames(&mut self, buffer: &mut [i16]) -> Result<(usize, NextState), RawedioError> {
        buffer.fill_with(|| self.advance());
        Ok((buffer.len(), NextState::Playing))
    }
}

fn sample_for(sample_num: f32, freq: f32, sample_rate: f32) -> i16 {
    let value = sample_num * freq * TAU / sample_rate;
    (value.sin() * i16::MAX as f32) as i16
}

#[cfg(test)]
mod test {
    use crate::sources::SineWave;
    use crate::{NextState, Sound};

    #[test]
    fn high_freq_wav() {
        let mut buffer = [0];
        let mut wav = SineWave::with_sample_rate(12000.0, 48000);
        assert_eq!(wav.sample_rate(), 48000);
        assert_eq!(wav.channel_count(), 1);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [i16::MAX]);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [0]);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [i16::MIN + 1]);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [0]);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [i16::MAX]);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [0]);
    }

    #[test]
    fn one_khz_wav() {
        let mut buffer = [0];
        let mut wav = SineWave::with_sample_rate(1000.0, 44100);
        assert_eq!(wav.sample_rate(), 44100);
        assert_eq!(wav.channel_count(), 1);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [4652]); // 1

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [9211]); // 2

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [13582]); // 3

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [17679]); // 4

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [21417]); // 5

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [24721]); // 6

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [27525]); // 7

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [29770]); // 8

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [31412]); // 9

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [32418]); // 10

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [32766]); // 11

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [32451]); // 12
    }

    #[test]
    fn high_freq_wav_memory_sound() {
        let mut buffer = [0];
        let mut wav = SineWave::as_memory_sound(12000.0, 48000);
        assert_eq!(wav.as_ref().len(), 4);
        assert_eq!(wav.sample_rate(), 48000);
        assert_eq!(wav.channel_count(), 1);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [i16::MAX]);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [0]);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [i16::MIN + 1]);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [0]);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [i16::MAX]);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [0]);
    }

    #[test]
    fn low_freq_wav_memory_sound() {
        let mut buffer = [0];
        let mut wav = SineWave::as_memory_sound(20.0, 48000);
        assert_eq!(wav.as_ref().len(), 2400);
        assert_eq!(wav.sample_rate(), 48000);
        assert_eq!(wav.channel_count(), 1);

        assert_eq!(
            wav.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [85]);
    }

    #[test]
    fn max_as_memory_sound_size() {
        let mut max_num_samples = 0;
        for hz in 20..20_000 {
            let wav = SineWave::as_memory_sound(hz as f32, 48000);
            let num_samples = wav.as_ref().len();
            if num_samples > max_num_samples {
                max_num_samples = num_samples;
            }
        }
        assert_eq!(max_num_samples, 2400);
    }
}
