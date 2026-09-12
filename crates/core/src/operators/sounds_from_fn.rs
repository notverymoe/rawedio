//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::{NextState, RawedioError, Sound};

type SoundGenerator = Box<dyn FnMut() -> Option<Box<dyn Sound>> + Send>;

/// Play sounds produced by a function returning sounds one after the other.
///
/// The generator function is called after each previously produced sound has
/// returned finished. After `SoundsFromFn` returns None
/// this sound returns Finished. If an Error is returned from `next_sound`
/// that sound is dropped and the Error is returned. If `next_sound` is called
/// again `SoundsFromFn` is called again.
///
/// This can be used to create sounds that loop forever without storing all
/// samples in memory.
pub struct SoundsFromFn {
    generator: SoundGenerator,
    current: Option<Box<dyn Sound>>,
    current_channel_count: u16,
    current_sample_rate: u32,
}

impl SoundsFromFn {
    /// Call `generator` to generate Sounds that will be played to completion.
    /// If `generator` returns None, this Sound will be Finished and `generator`
    /// will no longer be called.
    ///
    /// ## Examples
    /// Play an audio file forever.
    ///
    /// ```rust,no_run
    /// use rawedio::operators::SoundsFromFn;
    /// use rawedio::sources::open_file;
    ///
    /// let generator = || Some(open_file("test.wav").unwrap());
    /// let forever_sound = SoundsFromFn::new(Box::new(generator));
    /// ```
    #[must_use]
    pub fn new(mut generator: SoundGenerator) -> Self {
        let current = generator();
        let mut to_return = Self {
            generator,
            current,
            current_channel_count: 0,
            current_sample_rate: 0,
        };
        to_return.update_metadata();
        to_return
    }

    fn update_metadata(&mut self) {
        self.current_channel_count = self.channel_count();
        self.current_sample_rate = self.sample_rate();
    }
}

impl Sound for SoundsFromFn {
    fn channel_count(&self) -> u16 {
        self.current.as_ref().map_or(1, Sound::channel_count)
    }

    fn sample_rate(&self) -> u32 {
        self.current.as_ref().map_or(1000, Sound::sample_rate)
    }

    fn on_start_of_batch(&mut self) {
        if let Some(current) = &mut self.current {
            current.on_start_of_batch();
        }
    }

    fn fill_next_frames(&mut self, buffer: &mut [i16]) -> Result<(usize, NextState), RawedioError> {
        loop {
            let Some(current) = &mut self.current else {
                return Ok((0, NextState::Finished));
            };
            let sample = current.fill_next_frames(buffer);
            let sample = match sample {
                Ok(s) => s,
                Err(e) => {
                    self.current = None;
                    self.current = (self.generator)();
                    self.update_metadata();
                    return Err(e);
                }
            };
            match sample {
                (_, NextState::MetadataChanged) => {
                    self.update_metadata();
                    return Ok(sample);
                }
                (_, NextState::Playing | NextState::Paused) => return Ok(sample),
                (count, NextState::Finished) => {
                    let old_channel_count = self.current_channel_count;
                    let old_sample_rate = self.current_sample_rate;
                    self.current = None;
                    self.current = (self.generator)();
                    self.update_metadata();
                    if self.current.is_none() {
                        return Ok((count, NextState::Finished));
                    }
                    if old_sample_rate != self.sample_rate()
                        || old_channel_count != self.channel_count()
                    {
                        return Ok((count, NextState::MetadataChanged));
                    }
                    if count > 0 {
                        return Ok((count, NextState::Playing));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use crate::operators::SoundsFromFn;
    use crate::sources::MemorySound;
    use crate::{NextState, Sound};

    #[test]
    fn basic() {
        let generator = || {
            let sound = MemorySound::from_samples(Arc::new(vec![1, 2]), 2, 1000);
            let sound: Box<dyn Sound> = Box::new(sound);
            Some(sound)
        };
        let mut buffer = [0, 0];
        let mut from_fn = SoundsFromFn::new(Box::new(generator));
        assert_eq!(from_fn.channel_count(), 2);
        assert_eq!(from_fn.sample_rate(), 1000);

        assert_eq!(
            from_fn.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [1, 2]);

        assert_eq!(
            from_fn.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [1, 2]);
    }

    #[test]
    fn changing_metadata_and_finishing() {
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
        let mut buffer = [0, 0];
        let mut from_fn = SoundsFromFn::new(Box::new(generator));

        assert_eq!(from_fn.channel_count(), 2);
        assert_eq!(from_fn.sample_rate(), 1001);

        assert_eq!(
            from_fn.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::MetadataChanged)
        );
        assert_eq!(buffer, [1, 2]);

        assert_eq!(from_fn.sample_rate(), 1002);
        assert_eq!(
            from_fn.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Finished)
        );
        assert_eq!(buffer, [1, 2]);

        assert_eq!(
            from_fn.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
    }
}
