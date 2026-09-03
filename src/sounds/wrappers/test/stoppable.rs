use crate::{NextSample, RawedioError, Sound, sounds::wrappers::SetStopped, utils::tests::{ConstantValueSound, adapt_next_sample, adapt_next_samples_for}};

#[test]
fn set_stopped() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut first = ConstantValueSound::new(1000).stoppable();
        // starts unpaused
        assert_eq!(
            next(&mut first).unwrap(),
            crate::NextSample::Sample(1000)
        );
        first.set_stopped();
        assert_eq!(next(&mut first).unwrap(), crate::NextSample::Finished);
        assert_eq!(next(&mut first).unwrap(), crate::NextSample::Finished);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}
