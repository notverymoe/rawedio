//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//


use crate::manager::BackendSource;
use crate::utils::NoHashIndexMap;
use crate::wrappers::{AddSound, ChannelCountConverter, ClearSounds, SampleRateConverter, SoundId, SoundRegistry};
use crate::{NextState, RawedioError, Sound};

type MixedSound = SampleRateConverter<ChannelCountConverter<Box<dyn Sound>>>;

/// Mix multiple sounds together to be played simultaneously.
///
/// The [Manager][crate::manager::Manager] contains a `SoundMixer` so you might
/// not need to crate one yourself but instead add multiple sounds on the
/// Manager.
///
/// If a Sound returns an Error from `next_sample`, the error is logged and the
/// Sound is dropped but other sounds keep playing.
pub struct SoundMixer {
    sounds: NoHashIndexMap<SoundId, MixedSound>,
    paused_sounds: NoHashIndexMap<SoundId, MixedSound>,
    next_sound_id: u32,
    output_channel_count: u16,
    output_sample_rate: u32,
    metadata_changed: bool,
    next_output_channel_idx: u16,
    scratch_buffer: Vec<i16>,
    to_remove: Vec<(SoundId, bool)>,
}

impl SoundMixer {
    /// Create a new empty sound mixer with an output channel count and sample
    /// rate that all added sounds will be converted to.
    #[must_use]
    pub fn new(output_channel_count: u16, output_sample_rate: u32) -> Self {
        SoundMixer {
            sounds: NoHashIndexMap::default(),
            paused_sounds: NoHashIndexMap::default(),
            next_sound_id: 0,
            output_channel_count,
            output_sample_rate,
            metadata_changed: false,
            next_output_channel_idx: 0,
            scratch_buffer: Vec::new(),
            to_remove: Vec::new(),
        }
    }
}

impl BackendSource for SoundMixer {
    /// Set the output channel count and sample rate.
    /// Added sounds will be converted to the output values. Must only be called
    /// when the next sample is for the first channel in the frame.
    fn set_output_channel_count_and_sample_rate(
        &mut self,
        output_channel_count: u16,
        output_sample_rate: u32,
    ) {
        self.metadata_changed = true;

        self.output_channel_count = output_channel_count;
        self.output_sample_rate = output_sample_rate;

        // Now re-wrap all the sounds with the new values.
        for mixed_sound in self.sounds.values_mut().chain(self.paused_sounds.values_mut()) {
            replace_with::replace_with_or_abort(mixed_sound, |tmp| {
                let inner = tmp.into_inner().into_inner();
                SampleRateConverter::new(
                    ChannelCountConverter::new(inner, self.output_channel_count),
                    self.output_sample_rate,
                )
            });
        }
    }
}

impl Sound for SoundMixer {
    fn channel_count(&self) -> u16 {
        self.output_channel_count
    }

    fn sample_rate(&self) -> u32 {
        self.output_sample_rate
    }

    fn on_start_of_batch(&mut self) {
        // Attempt to grab from paused sounds again
        self.sounds.reserve(self.paused_sounds.len());
        for (id, paused_sound) in self.paused_sounds.drain(..) {
            self.sounds.insert(id, paused_sound);
        }

        for sound in self.sounds.values_mut() {
            sound.on_start_of_batch();
        }
    }

    /// Guaranteed to not return an Error.
    #[allow(clippy::panic_in_result_fn)]
    fn fill_next_frames(&mut self, buffer: &mut [i16]) -> Result<(usize, NextState), RawedioError> {
        if self.metadata_changed {
            assert_eq!(self.next_output_channel_idx, 0); // TODO debug assert instead? error?
            self.metadata_changed = false;
            return Ok((0, NextState::MetadataChanged));
        }

        self.scratch_buffer
            .resize(usize::max(self.scratch_buffer.len(), buffer.len()), 0);
        buffer.fill(0);

        let mut max_written = 0;

        for (&id, sound) in &mut self.sounds {
            let count = loop {
                let next = sound.fill_next_frames(&mut self.scratch_buffer[..buffer.len()]);
                match next {
                    Ok((count, NextState::Playing)) => {
                        break count;
                    }
                    Ok((count, NextState::MetadataChanged)) => {
                        // We know that the channel_count and sample_rate haven't changed because
                        // we have wrapped the sound in converters. It is possible that the
                        // MetadataChanged implies we need to start over at the first channel.
                        // Normally however Metadata only change on the first sample of a frame
                        // so handle that by looping around and calling next_sample again
                        // immediately
                        if count != 0 {
                            // In the rare case we see MetadataChange not on
                            // the first channel, lets pause the sound until the
                            // next batch to avoid de-syncing the channels.
                            self.to_remove.push((id, true));
                            break 0;
                        }

                        // continue
                    }
                    Ok((count, NextState::Paused)) => {
                        self.to_remove.push((id, true));
                        break count;
                    }
                    Ok((count, NextState::Finished)) => {
                        self.to_remove.push((id, false));
                        break count;
                    }
                    Err(e) => {
                        // TODO probably want to let applications subscribe to be notified of these
                        // errors
                        log::error!("dropping sound in SoundMixer which returned error: {e}");
                        self.to_remove.push((id, false));
                        break 0;
                    }
                }
            };

            max_written = usize::max(max_written, count);

            if count > 0 {
                // Accumulate
                buffer[..count]
                    .iter_mut()
                    .zip(self.scratch_buffer[..count].iter())
                    .for_each(|(dst, src)| *dst = dst.saturating_add(*src));
            }
        }

        for (id, paused) in self.to_remove.drain(..).rev() {
            let sound = self.sounds.swap_remove(&id).unwrap();
            if paused {
                self.paused_sounds.insert(id, sound);
            }
            // otherwise drop finished sound
        }

        self.next_output_channel_idx = ((self.next_output_channel_idx as usize + max_written)
            % (self.output_channel_count as usize)) as u16;

        match (self.sounds.is_empty(), self.paused_sounds.is_empty()) {
            // We assume that we are finished since this sound has been handed
            // off to the Manager so new sounds can't be added without a
            // Controllable. If this is wrapped in a Controllable, the Finished
            // is changed to a Paused by the wrapper.
            (true, true) => Ok((max_written, NextState::Finished)),
            (true, false) => Ok((max_written, NextState::Paused)),
            (false, _) => Ok((buffer.len(), NextState::Playing)),
        }
    }
}

impl SoundRegistry for SoundMixer {

    fn add(&mut self, sound: Box<dyn Sound>) -> SoundId {
        let id = self.next_sound_id;
        self.next_sound_id += 1;
        let id = SoundId::from_inner(id);

        self.insert(id, sound);

        id
    }
    
    fn insert(&mut self, id: SoundId, sound: Box<dyn Sound>) -> Option<Box<dyn Sound>> {
        self.sounds
            .insert(
                id, 
                SampleRateConverter::new(
                    ChannelCountConverter::new(sound, self.output_channel_count),
                    self.output_sample_rate,
                )
            )
            .map(|v| v.into_inner().into_inner())
    }
    
    fn remove(&mut self, id: SoundId) -> Option<Box<dyn Sound>> {
        self.sounds
            .swap_remove(&id)
            .map(|v| v.into_inner().into_inner())
    }
    
}

impl AddSound for SoundMixer {
    fn add(&mut self, sound: Box<dyn Sound>) {
        SoundRegistry::add(self, sound);
    }
}

impl ClearSounds for SoundMixer {
    /// Remove all audio sounds.
    fn clear(&mut self) {
        self.sounds.clear();
        self.paused_sounds.clear();
    }
}

#[cfg(test)]
mod tests {

    use crate::operators::{SoundList, SoundMixer};
    use crate::utils::test::{ConstantValueSound, DEFAULT_CHANNEL_COUNT, DEFAULT_SAMPLE_RATE};
    use crate::wrappers::AddSound;
    use crate::{NextState, Sound};

    #[test]
    fn additional_silent_sounds_do_not_affect_first() {
        let mut buffer = [0, 0];
        let first = ConstantValueSound::new(5);
        let second = ConstantValueSound::new(0);
        let mut mixer = SoundMixer::new(DEFAULT_CHANNEL_COUNT, DEFAULT_SAMPLE_RATE);
        mixer.add(Box::new(first));
        mixer.add(Box::new(second));

        assert_eq!(
            mixer.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [5, 5]);

        assert_eq!(
            mixer.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [5, 5]);

        assert_eq!(
            mixer.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [5, 5]);

        let third = ConstantValueSound::new(0);
        mixer.add(Box::new(third));

        assert_eq!(
            mixer.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [5, 5]);

        assert_eq!(
            mixer.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [5, 5]);

        assert_eq!(
            mixer.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [5, 5]);
    }

    #[test]
    fn two_sounds_add_together() {
        let mut buffer = [0, 0];
        let first = ConstantValueSound::new(5);
        let second = ConstantValueSound::new(7);
        let mut mixer = SoundMixer::new(DEFAULT_CHANNEL_COUNT, DEFAULT_SAMPLE_RATE);
        mixer.add(Box::new(first));
        mixer.add(Box::new(second));
        assert_eq!(
            mixer.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [12, 12]);

        assert_eq!(
            mixer.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [12, 12]);

        assert_eq!(
            mixer.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [12, 12]);
    }

    #[test]
    fn empty_sound_list_not_same_sample_rate() {
        // Reproducing issue when SoundMixer matches audio but goes through SoundList
        // with different sample rate
        let mut buffer = [0, 0];
        let mut mixer = SoundMixer::new(2, 40000);
        let (sound, mut controller) = SoundList::new().controllable();
        mixer.add(Box::new(sound));
        mixer.on_start_of_batch();
        assert_eq!(
            mixer.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Paused)
        );

        let mut sound = ConstantValueSound::new(5);
        sound.set_channel_count(2);
        sound.set_sample_rate(40000);
        controller.add(Box::new(sound));

        assert_eq!(
            mixer.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Paused)
        );

        mixer.on_start_of_batch();
        assert_eq!(
            mixer.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [5, 5]);
    }
}
