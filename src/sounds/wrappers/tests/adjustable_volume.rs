//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::{
    sounds::wrappers::SetVolume,
    utils::test::{adapt_next_sample, adapt_next_samples_for, ConstantValueSound},
    NextSample, RawedioError, Sound,
};

#[test]
fn adjust_down() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut first = ConstantValueSound::new(1000).with_adjustable_volume();
        first.set_volume(0.5);
        assert_eq!(next(&mut first).unwrap(), crate::NextSample::Sample(500));
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn adjust_up() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut first = ConstantValueSound::new(1000).with_adjustable_volume();
        first.set_volume(5.0);
        assert_eq!(next(&mut first).unwrap(), crate::NextSample::Sample(5000));
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn test_saturation() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut first = ConstantValueSound::new(1000).with_adjustable_volume();
        first.set_volume(1000.0);
        assert_eq!(
            next(&mut first).unwrap(),
            crate::NextSample::Sample(i16::MAX)
        );
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}
