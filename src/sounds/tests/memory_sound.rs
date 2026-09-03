//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::sync::Arc;

use crate::{
    sounds::{MemorySound, SoundList},
    utils::test::{adapt_next_sample, adapt_next_samples_for},
    NextSample, RawedioError, Sound,
};

#[test]
fn metadata_change_two_off_does_not_cause_desync() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let first = MemorySound::from_samples(Arc::new(vec![1, 2, 3, 4, 1, 2]), 4, 1000);
        let second = MemorySound::from_samples(Arc::new(vec![1, 2, 3, 4]), 4, 1000);
        let mut list = SoundList::new();
        list.add(Box::new(first));
        list.add(Box::new(second));
        let mut combined = list.into_memory_sound().unwrap();
        assert_eq!(combined.sample_rate(), 1000);
        assert_eq!(combined.channel_count(), 4);
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(2));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(3));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(4));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(2));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(0));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(0));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(2));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(3));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(4));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Finished);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn metadata_change_one_off_does_not_cause_desync() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let first = MemorySound::from_samples(Arc::new(vec![1, 2, 3, 4, 1]), 4, 1000);
        let second = MemorySound::from_samples(Arc::new(vec![1, 2, 3, 4]), 4, 1000);
        let mut list = SoundList::new();
        list.add(Box::new(first));
        list.add(Box::new(second));
        let mut combined = MemorySound::from_sound(list).unwrap();
        assert_eq!(combined.sample_rate(), 1000);
        assert_eq!(combined.channel_count(), 4);
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(2));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(3));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(4));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(0));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(0));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(0));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(2));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(3));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(4));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Finished);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn metadata_change_in_sync() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let first = MemorySound::from_samples(Arc::new(vec![1, 2, 3, 4]), 4, 1000);
        let second = MemorySound::from_samples(Arc::new(vec![1, 2, 3, 4]), 4, 1000);
        let mut list = SoundList::new();
        list.add(Box::new(first));
        list.add(Box::new(second));
        let mut combined = MemorySound::from_sound(list).unwrap();
        assert_eq!(combined.sample_rate(), 1000);
        assert_eq!(combined.channel_count(), 4);
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(2));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(3));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(4));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(2));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(3));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Sample(4));
        assert_eq!(next(&mut combined).unwrap(), NextSample::Finished);
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn loop_forever() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let mut sound = MemorySound::from_samples(Arc::new(vec![1, 2]), 2, 1000);
        sound.set_looping(true);
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(2));
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(1));
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(2));
        assert_eq!(next(&mut sound).unwrap(), NextSample::Sample(1));
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}
