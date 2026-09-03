//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::{
    sounds::wrappers::SetSpeed,
    utils::test::{
        adapt_next_sample, adapt_next_samples_for, ConstantValueSound, DEFAULT_SAMPLE_RATE,
    },
    NextSample, RawedioError, Sound,
};

#[test]
fn adjust_down() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut first = ConstantValueSound::new(1000).with_adjustable_speed_of(0.5);
        assert_eq!(next(&mut first).unwrap(), crate::NextSample::Sample(1000));
        assert_eq!(first.sample_rate(), 22050);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn adjust_up() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut first = ConstantValueSound::new(1000).with_adjustable_speed_of(5.0);
        assert_eq!(next(&mut first).unwrap(), crate::NextSample::Sample(1000));
        assert_eq!(first.sample_rate(), 44100 * 5);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn test_real_fast() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut first = ConstantValueSound::new(1000).with_adjustable_speed_of(1000.0);
        assert_eq!(next(&mut first).unwrap(), crate::NextSample::Sample(1000));
        assert_eq!(first.sample_rate(), 44100 * 1000);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn test_max_saturation() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut first = ConstantValueSound::new(1000).with_adjustable_speed_of(1_000_000.0);
        assert_eq!(next(&mut first).unwrap(), crate::NextSample::Sample(1000));
        assert_eq!(first.sample_rate(), u32::MAX);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn test_min_saturation() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut first = ConstantValueSound::new(1000).with_adjustable_speed();
        first.set_speed(0.000_000_000_1);
        assert_eq!(first.sample_rate(), 1);
        assert_eq!(
            next(&mut first).unwrap(),
            crate::NextSample::MetadataChanged
        );
        assert_eq!(next(&mut first).unwrap(), crate::NextSample::Sample(1000));
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn metadata_changed_notification() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut first = ConstantValueSound::new(1000).with_adjustable_speed();
        assert_eq!(first.sample_rate(), DEFAULT_SAMPLE_RATE);
        assert_eq!(next(&mut first).unwrap(), crate::NextSample::Sample(1000));
        first.set_speed(0.50);
        assert_eq!(first.sample_rate(), 22050);
        assert_eq!(
            next(&mut first).unwrap(),
            crate::NextSample::MetadataChanged
        );
        assert_eq!(next(&mut first).unwrap(), crate::NextSample::Sample(1000));
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}
