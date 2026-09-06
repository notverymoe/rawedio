//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::time::Duration;

use crate::{
    sounds::wrappers::SetPaused,
    utils::test::{adapt_next_sample, adapt_next_samples_for, ConstantValueSound},
    NextSample, RawedioError, Sound,
};

#[test]
fn test_simple() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut sound = ConstantValueSound::new(1000).finish_after(Duration::from_millis(100));
        for _ in 0..(44100 * 2 / 10) {
            assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(1000));
        }
        assert_eq!(next(&mut sound).unwrap(), NextSample::Finished);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn test_pausing_does_not_count() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut sound = ConstantValueSound::new(1000)
            .pausable()
            .finish_after(Duration::from_millis(100));
        for s in 0..(44100 * 2 / 10) {
            if s % 15 == 0 {
                sound.set_paused(true);
                assert_eq!(next(&mut sound).unwrap(), NextSample::Paused);
                sound.set_paused(false);
            }
            assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(1000));
        }
        assert_eq!(next(&mut sound).unwrap(), NextSample::Finished);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn test_metadata_change_beginning() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut sound = ConstantValueSound::new(1000).finish_after(Duration::from_millis(100));
        sound.inner_mut().set_sample_rate(22050);
        sound.inner_mut().set_channel_count(1);
        assert_eq!(next(&mut sound).unwrap(), NextSample::MetadataChanged);
        for _ in 0..(44100 / 2 / 10) {
            assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(1000));
        }
        assert_eq!(next(&mut sound).unwrap(), NextSample::Finished);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn test_metadata_change_halfway() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut sound = ConstantValueSound::new(1000).finish_after(Duration::from_millis(100));
        for _ in 0..(44100 / 2 / 5) {
            assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(1000));
        }
        sound.inner_mut().set_sample_rate(88200);
        sound.inner_mut().set_channel_count(4);
        assert_eq!(next(&mut sound).unwrap(), NextSample::MetadataChanged);
        for _ in 0..(88200 * 4 / 20) {
            assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(1000));
        }
        assert_eq!(next(&mut sound).unwrap(), NextSample::Finished);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn test_metadata_change_end() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut sound = ConstantValueSound::new(1000).finish_after(Duration::from_millis(100));
        for _ in 0..(44100 * 2 / 10) {
            assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(1000));
        }
        assert_eq!(next(&mut sound).unwrap(), NextSample::Finished);
        sound.inner_mut().set_sample_rate(22050);
        sound.inner_mut().set_channel_count(1);
        assert_eq!(next(&mut sound).unwrap(), NextSample::Finished);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}
