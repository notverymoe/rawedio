//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::{
    sounds::wrappers::SetStopped,
    utils::test::{adapt_next_sample, adapt_next_samples_for, ConstantValueSound},
    NextSample, RawedioError, Sound,
};

#[test]
fn set_stopped() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut first = ConstantValueSound::new(1000).stoppable();
        // starts unpaused
        assert_eq!(next(&mut first).unwrap(), NextSample::Sample(1000));
        first.set_stopped();
        assert_eq!(next(&mut first).unwrap(), NextSample::Finished);
        assert_eq!(next(&mut first).unwrap(), NextSample::Finished);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}
