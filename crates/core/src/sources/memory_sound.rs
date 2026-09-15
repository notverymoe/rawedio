//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::sync::Arc;

use crate::{NextState, RawedioError, Sound};

/// A Sound that stores all samples on the heap.
///
/// The heap samples can be shared between multiple `MemorySounds` that can be
/// played simultaneously. Optionally the sound can repeat forever.
#[derive(Clone)]
pub struct MemorySound {
    samples: Arc<Vec<f32>>,
    channel_count: usize,
    sample_rate: usize,

    next_sample: usize,
    should_loop: bool,
}

/// A [`MetadataChanged`][NextSample::MetadataChanged] was returned while reading
/// into a [`MemorySound`] which is not currently supported.
#[derive(Debug)]
pub struct UnsupportedMetadataChangeError;

impl std::fmt::Display for UnsupportedMetadataChangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unsupported MetadataChanged encountered when consuming Sound"
        )
    }
}

impl std::error::Error for UnsupportedMetadataChangeError {}

impl MemorySound {
    /// Create a `MemorySound` be consuming another Sound and storing the samples
    /// until it returns `Finished` or `Paused`.
    ///
    /// If an Error is encountered it is returned and any already obtained
    /// samples are lost.
    ///
    /// It is not currently supported for the the originating sample to change
    /// its metadata (i.e. channel count or sample rate). If it does an
    /// `IoError` of `ErrorKind::Other` with a `UnsupportedMetadataChangeError` is
    /// returned.
    pub fn from_sound(mut orig: impl Sound) -> Result<Self, RawedioError> {
        let channel_count = orig.channel_count();
        let sample_rate = orig.sample_rate();

        let mut samples = Vec::new();

        loop {
            let from = samples.len();
            samples.resize(samples.len() + channel_count, 0.0);
            let sample = orig.fill_next_frames(&mut samples[from..])?;
            match sample {
                (_, NextState::Playing) => (),
                (count, NextState::MetadataChanged) => {
                    if orig.channel_count() != channel_count || orig.sample_rate() != sample_rate {
                        return Err(RawedioError::IoError(std::io::Error::other(
                            UnsupportedMetadataChangeError {},
                        )));
                    }
                    // Sometimes we see a MetadataChanged from a sound just to
                    // ensure that channels stay in sync. Lets ensure that here
                    // by ensuring that the next sample after MetadataChanged is
                    // for the first channel.
                    if count == 0 {
                        samples.truncate(samples.len() - channel_count);
                    } else if count != channel_count {
                        // This should be rare so lets just output 0 for the filler samples.
                        samples[from + count..].fill(0.0);
                    }
                }
                (_, NextState::Paused | NextState::Finished) => break,
            }
        }

        Ok(MemorySound {
            samples: Arc::new(samples),
            channel_count,
            sample_rate,
            next_sample: 0,
            should_loop: false,
        })
    }

    /// Create memory sound from the raw data of samples.
    ///
    /// Samples should be in the same order as they will be returned from the
    /// `next_samples` function (e.g. interleaved by channel).
    #[must_use]
    pub const fn from_samples(
        samples: Arc<Vec<f32>>,
        channel_count: usize,
        sample_rate: usize,
    ) -> MemorySound {
        MemorySound {
            samples,
            channel_count,
            sample_rate,
            next_sample: 0,
            should_loop: false,
        }
    }

    /// Instead of finishing after playing all samples, start back at the
    /// beginning and continue forever.
    pub const fn set_looping(&mut self, should_loop: bool) {
        self.should_loop = should_loop;
    }
}

impl Sound for MemorySound {
    fn channel_count(&self) -> usize {
        self.channel_count
    }

    fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    fn fill_next_frames(&mut self, buffer: &mut [f32]) -> Result<(usize, NextState), RawedioError> {
        let mut remaining = buffer.len();
        while remaining > 0 {
            let from = buffer.len() - remaining;
            let count = usize::min(remaining, self.samples.len() - self.next_sample);
            buffer[from..from + count]
                .copy_from_slice(&self.samples[self.next_sample..self.next_sample + count]);
            remaining -= count;
            self.next_sample += count;

            if self.next_sample == self.samples.len() {
                if self.should_loop && !self.samples.is_empty() {
                    self.next_sample = 0;
                } else {
                    return Ok((buffer.len() - remaining, NextState::Finished));
                }
            }
        }
        Ok((buffer.len(), NextState::Playing))
    }
}

impl AsRef<[f32]> for MemorySound {
    fn as_ref(&self) -> &[f32] {
        &self.samples
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use crate::operators::SoundList;
    use crate::sources::MemorySound;
    use crate::{assert_float_all_ulp_eq, NextState, Sound};

    #[test]
    fn metadata_change_two_off_does_not_cause_desync() {
        let first =
            MemorySound::from_samples(Arc::new(vec![1.0, 2.0, 3.0, 4.0, 11.0, 12.0]), 4, 1000);
        assert_eq!(first.as_ref(), [1.0, 2.0, 3.0, 4.0, 11.0, 12.0]);

        let second = MemorySound::from_samples(Arc::new(vec![21.0, 22.0, 23.0, 24.0]), 4, 1000);
        assert_eq!(second.as_ref(), [21.0, 22.0, 23.0, 24.0]);

        let mut list = SoundList::new();
        list.add(Box::new(first));
        list.add(Box::new(second));

        let mut buffer = [0.0, 0.0, 0.0, 0.0];
        let mut sound = MemorySound::from_sound(list).unwrap();
        assert_eq!(sound.as_ref(), [
            1.0, 2.0, 3.0, 4.0, 11.0, 12.0, 0.0, 0.0, 21.0, 22.0, 23.0, 24.0
        ]);

        assert_eq!(sound.sample_rate(), 1000);
        assert_eq!(sound.channel_count(), 4);
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (4, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [1.0, 2.0, 3.0, 4.0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (4, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [11.0, 12.0, 0.0, 0.0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (4, NextState::Finished)
        );
        assert_float_all_ulp_eq!(buffer, [21.0, 22.0, 23.0, 24.0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
    }

    #[test]
    fn metadata_change_one_off_does_not_cause_desync() {
        let first = MemorySound::from_samples(Arc::new(vec![1.0, 2.0, 3.0, 4.0, 11.0]), 4, 1000);
        let second = MemorySound::from_samples(Arc::new(vec![21.0, 22.0, 23.0, 24.0]), 4, 1000);
        let mut list = SoundList::new();
        list.add(Box::new(first));
        list.add(Box::new(second));

        let mut buffer = [0.0, 0.0, 0.0, 0.0];
        let mut sound = MemorySound::from_sound(list).unwrap();
        assert_eq!(sound.sample_rate(), 1000);
        assert_eq!(sound.channel_count(), 4);
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (4, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [1.0, 2.0, 3.0, 4.0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (4, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [11.0, 0.0, 0.0, 0.0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (4, NextState::Finished)
        );
        assert_float_all_ulp_eq!(buffer, [21.0, 22.0, 23.0, 24.0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
    }

    #[test]
    fn metadata_change_in_sync() {
        let first = MemorySound::from_samples(Arc::new(vec![1.0, 2.0, 3.0, 4.0]), 4, 1000);
        let second = MemorySound::from_samples(Arc::new(vec![11.0, 12.0, 13.0, 14.0]), 4, 1000);
        let mut list = SoundList::new();
        list.add(Box::new(first));
        list.add(Box::new(second));

        let mut buffer = [0.0, 0.0, 0.0, 0.0];
        let mut sound = MemorySound::from_sound(list).unwrap();

        assert_eq!(sound.sample_rate(), 1000);
        assert_eq!(sound.channel_count(), 4);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (4, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [1.0, 2.0, 3.0, 4.0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (4, NextState::Finished)
        );
        assert_float_all_ulp_eq!(buffer, [11.0, 12.0, 13.0, 14.0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
    }

    #[test]
    fn loop_forever() {
        let mut buffer = [0.0, 0.0];
        let mut sound = MemorySound::from_samples(Arc::new(vec![1.0, 2.0]), 2, 1000);
        sound.set_looping(true);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [1.0, 2.0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [1.0, 2.0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [1.0, 2.0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [1.0, 2.0]);

        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [1.0, 2.0]);
    }
}
