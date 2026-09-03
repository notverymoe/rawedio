//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::utils::test::{adapt_next_sample, adapt_next_samples_for, ConstantValueSound, Sawtooth};
use crate::{NextSample, RawedioError, Sound};

#[test]
fn test_constant_value_sound_basic() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut sound = ConstantValueSound::new(42);
        assert_eq!(sound.channel_count(), 2);
        assert_eq!(sound.sample_rate(), 44100);

        // First sample should be the constant value
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(42));
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn test_constant_value_sound_metadata_changes() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut sound = ConstantValueSound::new(42);

        // Change sample rate
        sound.set_sample_rate(48000);
        assert_eq!(sound.sample_rate(), 48000);
        assert_eq!(next(&mut sound).unwrap(), NextSample::MetadataChanged);
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(42));

        // Change channel count
        sound.set_channel_count(1);
        assert_eq!(sound.channel_count(), 1);
        assert_eq!(next(&mut sound).unwrap(), NextSample::MetadataChanged);
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(42));

        // Multiple changes before sampling
        sound.set_sample_rate(96000);
        sound.set_channel_count(4);
        assert_eq!(next(&mut sound).unwrap(), NextSample::MetadataChanged);
        assert_eq!(sound.sample_rate(), 96000);
        assert_eq!(sound.channel_count(), 4);
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(42));
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn test_sawtooth_basic() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut sound = Sawtooth::new(1, 44100);

        // Mono sawtooth should increment each sample
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(0));
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(2));
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn test_sawtooth_stereo() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut sound = Sawtooth::new(2, 44100);

        // Stereo sawtooth should increment every other sample
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(0)); // L
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(0)); // R
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(1)); // L
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(1)); // R
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn test_sawtooth_wrap_around() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut sound = Sawtooth::new(1, 44100);
        sound.value = i16::MAX - 1;

        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(i16::MAX - 1));
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(i16::MAX));
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(i16::MIN));
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn test_sawtooth_sample_rate() {
    let sound = Sawtooth::new(1, 48000);
    assert_eq!(sound.sample_rate(), 48000);
}
