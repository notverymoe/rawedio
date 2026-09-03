//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::wrappers::{AddSound, ClearSounds};
use crate::{NextState, RawedioError, Sound};

/// Play Sounds sequentially one after the other.
///
/// Only after a Sound has returned `NextSample::Finished` will the next Sound
/// start playing.
///
/// If an Error is returned from a Sound it is dropped and the error is
/// propagated to the caller. Calling `next_sound` again would continue
/// with the next Sound in the list.
pub struct SoundList {
    sounds: Vec<Box<dyn Sound>>,
    was_empty: bool,
}

impl SoundList {
    /// Create a new empty `SoundList`.
    #[must_use]
    pub fn new() -> Self {
        SoundList {
            sounds: Vec::new(),
            was_empty: false,
        }
    }

    /// Add a Sound to be played after any existing sounds have `Finished`.
    pub fn add(&mut self, sound: Box<dyn Sound>) {
        if self.sounds.is_empty() {
            self.was_empty = true;
        }
        self.sounds.push(sound);
    }

    /// Inserts a sound at position `index`, shifting all elements after it to
    /// the right.
    ///
    /// Panics
    ///
    /// Panics if `index > len`.
    pub fn insert(&mut self, index: usize, sound: Box<dyn Sound>) {
        if self.sounds.is_empty() {
            self.was_empty = true;
        }
        self.sounds.insert(index, sound);
    }

    /// Stop all sounds including the currently playing one.
    pub fn clear(&mut self) {
        self.sounds.clear();
    }

    /// Returns the number of sounds currently in the list.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sounds.len()
    }

    /// Returns `true` if the list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }
}

impl From<Vec<Box<dyn Sound>>> for SoundList {
    fn from(sounds: Vec<Box<dyn Sound>>) -> Self {
        let was_empty = sounds.is_empty();
        SoundList { sounds, was_empty }
    }
}

impl From<SoundList> for Vec<Box<dyn Sound>> {
    fn from(list: SoundList) -> Self {
        list.sounds
    }
}

impl FromIterator<Box<dyn Sound>> for SoundList {
    fn from_iter<T: IntoIterator<Item = Box<dyn Sound>>>(iter: T) -> Self {
        let vec: Vec<_> = iter.into_iter().collect();
        vec.into()
    }
}

// Returned only when no sounds exist so they shouldn't be used in practice.
const DEFAULT_CHANNEL_COUNT: u16 = 2;
const DEFAULT_SAMPLE_RATE: u32 = 48000;

impl Sound for SoundList {
    fn channel_count(&self) -> u16 {
        self.sounds
            .first()
            .map_or(DEFAULT_CHANNEL_COUNT, Sound::channel_count)
    }

    fn sample_rate(&self) -> u32 {
        self.sounds
            .first()
            .map_or(DEFAULT_SAMPLE_RATE, Sound::sample_rate)
    }

    fn on_start_of_batch(&mut self) {
        for sound in &mut self.sounds {
            sound.on_start_of_batch();
        }
    }

    fn fill_next_frames(&mut self, buffer: &mut [i16]) -> Result<(usize, NextState), RawedioError> {
        let Some(next_sound) = self.sounds.first_mut() else {
            return Ok((0, NextState::Finished));
        };

        if self.was_empty {
            self.was_empty = false;
            return Ok((0, NextState::MetadataChanged));
        }

        let next_sample = match next_sound.fill_next_frames(buffer) {
            Ok(s) => s,
            Err(e) => {
                self.sounds.remove(0);
                return Err(e);
            }
        };

        let ret = match next_sample {
            (_, NextState::Playing | NextState::MetadataChanged | NextState::Paused) => next_sample,
            (count, NextState::Finished) => {
                self.sounds.remove(0);
                if self.sounds.is_empty() {
                    (count, NextState::Finished)
                } else {
                    // The next sample might have different metadata. Instead of
                    // normalizing here let downstream normalize.
                    (count, NextState::MetadataChanged)
                }
            }
        };
        Ok(ret)
    }
}

impl AddSound for SoundList {
    fn add(&mut self, sound: Box<dyn Sound>) {
        SoundList::add(self, sound);
    }
}

impl ClearSounds for SoundList {
    fn clear(&mut self) {
        self.clear();
    }
}

impl Default for SoundList {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SoundList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SoundList")
            .field("sounds", &format!("{} sounds", self.sounds.len()))
            .field("was_empty", &self.was_empty)
            .finish()
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use crate::operators::SoundList;
    use crate::sources::MemorySound;
    use crate::{NextState, Sound};

    #[test]
    fn empty_gives_metadata_changed_on_next() {
        let mut buffer = [0, 0, 0, 0];
        let mut list = SoundList::new();
        assert_eq!(
            list.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );

        let first = MemorySound::from_samples(Arc::new(vec![1, 2, 3, 4]), 4, 1000);
        list.add(Box::new(first));
        let second = MemorySound::from_samples(Arc::new(vec![5, 6]), 2, 8000);
        list.add(Box::new(second));
        assert_eq!(
            list.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::MetadataChanged)
        );
        assert_eq!(list.channel_count(), 4);
        assert_eq!(list.sample_rate(), 1000);

        assert_eq!(
            list.fill_next_frames(&mut buffer).unwrap(),
            (4, NextState::MetadataChanged)
        );
        assert_eq!(buffer, [1, 2, 3, 4]);
        assert_eq!(list.channel_count(), 2);
        assert_eq!(list.sample_rate(), 8000);

        assert_eq!(
            list.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Finished)
        );
        assert_eq!(buffer[..2], [5, 6]);
    }
}
