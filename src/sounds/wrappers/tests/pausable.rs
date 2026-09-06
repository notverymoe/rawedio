//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::{
    sounds::wrappers::SetPaused,
    utils::test::{adapt_next_sample, adapt_next_samples_for, ConstantValueSound},
    NextSample, RawedioError, Sound,
};

#[test]
fn set_paused_and_unpause() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut first = ConstantValueSound::new(1000).pausable();
        // starts unpaused
        assert_eq!(next(&mut first).unwrap(), NextSample::Sample(1000));
        first.set_paused(true);
        assert_eq!(next(&mut first).unwrap(), NextSample::Paused);
        first.set_paused(false);
        assert_eq!(next(&mut first).unwrap(), NextSample::Sample(1000));
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}
