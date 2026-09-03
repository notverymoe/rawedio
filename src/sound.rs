//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::ops::{Deref, DerefMut};
use std::time::Duration;

use crate::sources::MemorySound;
#[cfg(feature = "async")]
use crate::wrappers::AsyncCompletionNotifier;
use crate::wrappers::{
    AdjustableSpeed, AdjustableVolume, CompletionNotifier, Controllable, Controller, FinishAfter,
    Pausable, SetPaused, Stoppable,
};
use crate::{utils, RawedioError};

/// A provider of audio samples.
///
/// This is the foundational trait of this crate. A `Box<dyn Sound>` can be
/// played on a [Manager][crate::manager::Manager]. Sounds can be wrapped to
/// modify the inner sound, often by using helper functions of this trait
/// (e.g. [pausable][Sound::pausable]).
pub trait Sound: Send {
    /// Returns the number of channels.
    fn channel_count(&self) -> u16;

    /// Returns the number of samples per second for each channel for this sound
    /// (e.g. 48,000).
    fn sample_rate(&self) -> u32;

    /// Called whenever a new batch of audio samples is requested by the
    /// backend, where `count` is the number of samples in the batch.
    ///
    /// This is a good place to put code that needs to run fairly frequently,
    /// but not for every single audio sample.
    fn on_start_of_batch(&mut self) {}

    /// Fill the given buffer with the next samples, until the next
    /// notification. Return the number of samples written and the
    /// message.
    ///
    /// The contents of the buffer are initialized, but may not be
    /// set to 0. The length of the buffer will always be a multiple
    /// of the frame size (ie. `channel_count`), including a length of
    /// zero. Samples for each channel should be interleaved, and the
    /// first sample should always be for the first channel.
    ///
    /// The result
    fn fill_next_frames(&mut self, buffer: &mut [i16]) -> Result<(usize, NextState), RawedioError>;

    /// Read the entire sound into memory. `MemorySound` can be cloned for
    /// efficient reuse. See [`MemorySound::from_sound`].
    fn into_memory_sound(self) -> Result<MemorySound, RawedioError>
    where Self: Sized {
        MemorySound::from_sound(self)
    }

    /// Read the entire sound into memory and loop indefinitely.
    ///
    /// If you do not want to read the entire sound into memory see
    /// [`SoundsFromFn`][crate::sounds::SoundsFromFn] as an alternative.
    fn loop_from_memory(self) -> Result<MemorySound, RawedioError>
    where Self: Sized {
        let mut to_return = MemorySound::from_sound(self)?;
        to_return.set_looping(true);
        Ok(to_return)
    }

    /// Allow this sound to be controlled after it has started playing with a
    /// [`Controller`].
    ///
    /// What can be controlled depends on the Sound type (e.g. `set_volume`).
    fn controllable(self) -> (Controllable<Self>, Controller<Self>)
    where Self: Sized {
        Controllable::new(self)
    }

    /// Get notified via a [`tokio::sync::oneshot::Receiver`] when this sound
    /// has Finished.
    #[cfg(feature = "async")]
    fn with_async_completion_notifier(
        self,
    ) -> (
        AsyncCompletionNotifier<Self>,
        tokio::sync::oneshot::Receiver<()>,
    )
    where Self: Sized {
        AsyncCompletionNotifier::new(self)
    }

    /// Get notified via a [`std::sync::mpsc::Receiver`] when this sound
    /// has Finished.
    fn with_completion_notifier(self) -> (CompletionNotifier<Self>, std::sync::mpsc::Receiver<()>)
    where Self: Sized {
        CompletionNotifier::new(self)
    }

    /// Allow the volume of the sound to be adjustable with `set_volume`.
    fn with_adjustable_volume(self) -> AdjustableVolume<Self>
    where Self: Sized {
        AdjustableVolume::new(self)
    }

    /// Allow the volume of the sound to be adjustable with `set_volume` and set
    /// the initial volume adjustment.
    fn with_adjustable_volume_of(self, volume_adjustment: f32) -> AdjustableVolume<Self>
    where Self: Sized {
        AdjustableVolume::new_with_volume(self, volume_adjustment)
    }

    /// Allow the speed of the sound to be adjustable with `set_speed`.
    ///
    /// This adjusts both speed and pitch.
    fn with_adjustable_speed(self) -> AdjustableSpeed<Self>
    where Self: Sized {
        AdjustableSpeed::new(self)
    }

    /// Allow the speed of the sound to be adjustable with `set_speed` and set
    /// the initial speed adjustment.
    ///
    /// This adjusts both speed and pitch.
    fn with_adjustable_speed_of(self, speed_adjustment: f32) -> AdjustableSpeed<Self>
    where Self: Sized {
        AdjustableSpeed::new_with_speed(self, speed_adjustment)
    }

    /// Allow for the sound to be pausable with `set_paused`. Starts unpaused.
    fn pausable(self) -> Pausable<Self>
    where Self: Sized {
        Pausable::new(self)
    }

    /// Allow for the sound to be pausable with `set_paused`. Starts paused.
    fn paused(self) -> Pausable<Self>
    where Self: Sized {
        let mut to_return = Pausable::new(self);
        to_return.set_paused(true);
        to_return
    }

    /// Allow for the sound to be stoppable with `set_stopped`.
    /// A stopped sound returns `Finished`.
    fn stoppable(self) -> Stoppable<Self>
    where Self: Sized {
        Stoppable::new(self)
    }

    /// Play the first `duration` of the sound, then finish even if samples
    /// remain.
    ///
    /// See [`FinishAfter`].
    fn finish_after(self, duration: Duration) -> FinishAfter<Self>
    where Self: Sized {
        FinishAfter::new(self, duration)
    }

    /// Skip the next `duration` of samples.
    ///
    /// This is done by calling `next_sample` repeatedly.
    ///
    /// Returns true if all samples were successfully skipped, false if a Paused
    /// or Finished were encountered first. `MetadataChanged` events are handled
    /// correctly but are not returned.
    fn skip(&mut self, duration: Duration) -> Result<bool, RawedioError> {
        let mut current_channel_count = self.channel_count();
        let mut current_sample_rate = self.sample_rate();
        let mut num_frames_remaining = utils::duration_to_num_frames(duration, current_sample_rate);

        let mut scratch = vec![0; current_channel_count as usize];
        while num_frames_remaining > 0 {
            let next = self.fill_next_frames(&mut scratch)?;
            match next {
                (_, NextState::Playing) => {
                    num_frames_remaining -= 1;
                }
                (_, NextState::MetadataChanged) => {
                    let new_channel_count = self.channel_count();
                    let new_sample_rate = self.sample_rate();
                    if new_channel_count != current_channel_count
                        || new_sample_rate != current_sample_rate
                    {
                        num_frames_remaining = utils::convert_num_frames(
                            num_frames_remaining,
                            current_sample_rate,
                            new_sample_rate,
                        );
                        current_channel_count = new_channel_count;
                        current_sample_rate = new_sample_rate;
                        scratch.clear();
                        scratch.resize(current_channel_count as usize, 0);
                    }
                }
                (_, NextState::Paused) => return Ok(false),
                (_, NextState::Finished) => return Ok(false),
            }
        }
        Ok(true)
    }
}

/// The result of [`Sound::fill_next_frames`]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NextState {
    /// The sound is continuing to play.
    Playing,

    /// The number of channels or the sample rate has changed. The
    /// samples written to the buffer are the remaining samples for
    /// the previous metadata. Future calls to `Sound::fill_next_frames`
    /// should adjust their destination buffer for the new format.
    MetadataChanged,

    /// The sound is pausing playback, no new samples will be returned
    /// until the sound starts playback again. The samples written to
    /// the buffer are the remaining samples before the sound paused.
    ///
    /// It is expected that the Sound will not be pulled again during
    /// this batch of samples. Future calls to `Sound::fill_next_frames`
    /// should return no written samples until unpaused. The caller
    /// should determine how to handle the silence (fade, insert 0s).
    Paused,

    /// The sound has finished playback and will never resume playback,
    /// The caller should free the sound, or stop its own playback.
    Finished,
}

impl Sound for Box<dyn Sound> {
    fn channel_count(&self) -> u16 {
        self.deref().channel_count()
    }

    fn sample_rate(&self) -> u32 {
        self.deref().sample_rate()
    }

    fn on_start_of_batch(&mut self) {
        self.deref_mut().on_start_of_batch();
    }

    fn fill_next_frames(&mut self, buffer: &mut [i16]) -> Result<(usize, NextState), RawedioError> {
        self.deref_mut().fill_next_frames(buffer)
    }
}

#[cfg(test)]
mod tests {

    use crate::utils::test::{ConstantValueSound, Sawtooth};
    use crate::{NextState, Sound};

    #[test]
    fn test_constant_value_sound_basic() {
        let mut buffer = [0, 0];
        let mut sound = ConstantValueSound::new(42);
        assert_eq!(sound.channel_count(), 2);
        assert_eq!(sound.sample_rate(), 44100);

        // First sample should be the constant value
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [42, 42]);
    }

    #[test]
    fn test_constant_value_sound_metadata_changes() {
        let mut buffer = [0, 0];
        let mut sound = ConstantValueSound::new(42);

        // Change sample rate
        sound.set_sample_rate(48000);
        assert_eq!(sound.sample_rate(), 48000);
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::MetadataChanged)
        );
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [42, 42]);

        // Change channel count
        sound.set_channel_count(1);
        assert_eq!(sound.channel_count(), 1);
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::MetadataChanged)
        );
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [42, 42]);

        // Multiple changes before sampling
        sound.set_sample_rate(96000);
        sound.set_channel_count(4);
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::MetadataChanged)
        );
        assert_eq!(sound.sample_rate(), 96000);
        assert_eq!(sound.channel_count(), 4);
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [42, 42]);
    }

    #[test]
    fn test_sawtooth_basic() {
        let mut buffer = [0];
        let mut sound = Sawtooth::new(1, 44100);

        // Mono sawtooth should increment each sample
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [1]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [2]);
    }

    #[test]
    fn test_sawtooth_stereo() {
        let mut buffer = [0, 0];
        let mut sound = Sawtooth::new(2, 44100);

        // Stereo sawtooth should increment every other sample
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [0, 0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [1, 1]);
    }

    #[test]
    fn test_sawtooth_wrap_around() {
        let mut buffer = [0];
        let mut sound = Sawtooth::new(1, 44100);
        sound.value = i16::MAX - 1;

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [i16::MAX - 1]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [i16::MAX]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [i16::MIN]);
    }

    #[test]
    fn test_sawtooth_sample_rate() {
        let sound = Sawtooth::new(1, 48000);
        assert_eq!(sound.sample_rate(), 48000);
    }
}
