use std::sync::Arc;

use crate::{NextSample, RawedioError, Sound, sounds::{MemorySound, SoundsFromFn}, utils::tests::{adapt_next_sample, adapt_next_samples_for}};

#[test]
fn basic() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let generator = || {
            let sound = MemorySound::from_samples(Arc::new(vec![1, 2]), 2, 1000);
            let sound: Box<dyn Sound> = Box::new(sound);
            Some(sound)
        };
        let mut from_fn = SoundsFromFn::new(Box::new(generator));
        assert_eq!(from_fn.channel_count(), 2);
        assert_eq!(from_fn.sample_rate(), 1000);
        assert_eq!(next(&mut from_fn).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut from_fn).unwrap(), NextSample::Sample(2));
        assert_eq!(next(&mut from_fn).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut from_fn).unwrap(), NextSample::Sample(2));
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn changing_metadata_and_finishing() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut num = 0;
        let generator = move || {
            num += 1;
            if num == 3 {
                return None;
            } else if num > 3 {
                unreachable!("should not have been called again");
            }
            let sound = MemorySound::from_samples(Arc::new(vec![1, 2]), 2, 1000 + num);
            let sound: Box<dyn Sound> = Box::new(sound);
            Some(sound)
        };
        let mut from_fn = SoundsFromFn::new(Box::new(generator));
        assert_eq!(from_fn.channel_count(), 2);
        assert_eq!(from_fn.sample_rate(), 1001);
        assert_eq!(next(&mut from_fn).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut from_fn).unwrap(), NextSample::Sample(2));
        assert_eq!(next(&mut from_fn).unwrap(), NextSample::MetadataChanged);
        assert_eq!(from_fn.sample_rate(), 1002);
        assert_eq!(next(&mut from_fn).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut from_fn).unwrap(), NextSample::Sample(2));
        assert_eq!(next(&mut from_fn).unwrap(), NextSample::Finished);
        assert_eq!(next(&mut from_fn).unwrap(), NextSample::Finished);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}
