use std::sync::Arc;

use crate::{NextSample, RawedioError, Sound, sounds::{MemorySound, SoundList}, utils::tests::{adapt_next_sample, adapt_next_samples_for}};

#[test]
fn empty_gives_metadata_changed_on_next() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut list = SoundList::new();
        assert_eq!(next(&mut list).unwrap(), NextSample::Finished);

        let first = MemorySound::from_samples(Arc::new(vec![1, 2, 3, 4]), 4, 1000);
        list.add(Box::new(first));
        let second = MemorySound::from_samples(Arc::new(vec![5, 6]), 2, 8000);
        list.add(Box::new(second));
        assert_eq!(next(&mut list).unwrap(), NextSample::MetadataChanged);
        assert_eq!(list.channel_count(), 4);
        assert_eq!(list.sample_rate(), 1000);
        assert_eq!(next(&mut list).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut list).unwrap(), NextSample::Sample(2));
        assert_eq!(next(&mut list).unwrap(), NextSample::Sample(3));
        assert_eq!(next(&mut list).unwrap(), NextSample::Sample(4));
        assert_eq!(next(&mut list).unwrap(), NextSample::MetadataChanged);
        assert_eq!(list.channel_count(), 2);
        assert_eq!(list.sample_rate(), 8000);
        assert_eq!(next(&mut list).unwrap(), NextSample::Sample(5));
        assert_eq!(next(&mut list).unwrap(), NextSample::Sample(6));
        assert_eq!(next(&mut list).unwrap(), NextSample::Finished);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}
