use crate::{
    NextSample, RawedioError, Sound, sounds::{
        SoundList,
        SoundMixer,
        wrappers::AddSound
    }, utils::tests::{
        ConstantValueSound, DEFAULT_CHANNEL_COUNT, DEFAULT_SAMPLE_RATE, adapt_next_sample, adapt_next_samples_for
    }
};

#[test]
fn additional_silent_sounds_do_not_affect_first() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let first = ConstantValueSound::new(5);
        let second = ConstantValueSound::new(0);
        let mut mixer = SoundMixer::new(DEFAULT_CHANNEL_COUNT, DEFAULT_SAMPLE_RATE);
        mixer.add(Box::new(first));
        mixer.add(Box::new(second));
        assert_eq!(next(&mut mixer).unwrap(), NextSample::Sample(5));
        assert_eq!(next(&mut mixer).unwrap(), NextSample::Sample(5));
        assert_eq!(next(&mut mixer).unwrap(), NextSample::Sample(5));
        let third = ConstantValueSound::new(0);
        mixer.add(Box::new(third));
        assert_eq!(next(&mut mixer).unwrap(), NextSample::Sample(5));
        assert_eq!(next(&mut mixer).unwrap(), NextSample::Sample(5));
        assert_eq!(next(&mut mixer).unwrap(), NextSample::Sample(5));
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn two_sounds_add_together() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        let first = ConstantValueSound::new(5);
        let second = ConstantValueSound::new(7);
        let mut mixer = SoundMixer::new(DEFAULT_CHANNEL_COUNT, DEFAULT_SAMPLE_RATE);
        mixer.add(Box::new(first));
        mixer.add(Box::new(second));
        assert_eq!(next(&mut mixer).unwrap(), NextSample::Sample(12));
        assert_eq!(next(&mut mixer).unwrap(), NextSample::Sample(12));
        assert_eq!(next(&mut mixer).unwrap(), NextSample::Sample(12));
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}

#[test]
fn empty_sound_list_not_same_sample_rate() {
    fn run_with(mut next: impl FnMut(&mut dyn Sound) -> Result<NextSample, RawedioError>) {
        // Reproducing issue when SoundMixer matches audio but goes through SoundList
        // with different sample rate
        let mut mixer = SoundMixer::new(2, 40000);
        let (sound, mut controller) = SoundList::new().controllable();
        mixer.add(Box::new(sound));
        mixer.on_start_of_batch(2);
        assert_eq!(NextSample::Paused, next(&mut mixer).unwrap());
        let mut sound = ConstantValueSound::new(5);

        sound.set_channel_count(2);
        sound.set_sample_rate(40000);
        controller.add(Box::new(sound));

        assert_eq!(NextSample::Paused, next(&mut mixer).unwrap());

        mixer.on_start_of_batch(1);
        assert_eq!(NextSample::Sample(5), next(&mut mixer).unwrap());
    }

    run_with(adapt_next_sample);
    run_with(adapt_next_samples_for);
}
