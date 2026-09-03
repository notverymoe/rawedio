//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

//! Misc utilities

use std::time::Duration;

/// Convert a number of samples at an old sample rate and channel count to a
/// new number of samples at a different channel rate or sample count such that
/// the new number of samples would be at the same time offset as the old number
/// of samples.
///
/// Any fractional samples are truncated.
///
/// Overflow occurs if `old_num_samples * old_channel_count * old_sample_rate`
/// does not fit into a u64.
#[must_use]
pub const fn convert_num_samples(
    old_num_samples: u64,
    old_channel_count: u16,
    old_sample_rate: u32,
    new_channel_count: u16,
    new_sample_rate: u32,
) -> u64 {
    old_num_samples * new_channel_count as u64 * new_sample_rate as u64
        / (old_channel_count as u64 * old_sample_rate as u64)
}

/// Return the number of samples that happen within `duration` amount of time
/// (truncates).
#[must_use]
pub fn duration_to_num_samples(duration: Duration, channel_count: u16, sample_rate: u32) -> u64 {
    convert_num_samples(
        duration
            .as_micros()
            .try_into()
            .expect("duration in microseconds is too large to fit into a u64"),
        1,
        1_000_000,
        channel_count,
        sample_rate,
    )
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub mod test {
    #![allow(missing_docs)]

    //! Common utilities for tests.

    use crate::{NextSample, NextSampleBuffer, Sound};

    pub const DEFAULT_SAMPLE_RATE: u32 = 44100;
    pub const DEFAULT_CHANNEL_COUNT: u16 = 2;

    pub fn adapt_next_sample(s: &mut dyn Sound) -> Result<NextSample, crate::RawedioError> {
        s.next_sample()
    }

    pub fn adapt_next_samples_for(s: &mut dyn Sound) -> Result<NextSample, crate::RawedioError> {
        let mut scratch = [0];
        match s.next_samples_for(&mut scratch) {
            Ok(NextSampleBuffer::Continue) => Ok(NextSample::Sample(scratch[0])),
            Ok(NextSampleBuffer::MetadataChanged(count)) => {
                if count > 0 {
                    Ok(NextSample::Sample(scratch[0]))
                } else {
                    Ok(NextSample::MetadataChanged)
                }
            }
            Ok(NextSampleBuffer::Paused(count)) => {
                if count > 0 {
                    Ok(NextSample::Sample(scratch[0]))
                } else {
                    Ok(NextSample::Paused)
                }
            }
            Ok(NextSampleBuffer::Finished(count)) => {
                if count > 0 {
                    Ok(NextSample::Sample(scratch[0]))
                } else {
                    Ok(NextSample::Finished)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Only useful for tests as a constant offset makes no hearable sound.
    pub struct ConstantValueSound {
        pub value: i16,
        pub channel_count: u16,
        pub sample_rate: u32,
        pub metadata_changed: bool,
    }

    impl ConstantValueSound {
        #[must_use]
        pub fn new(value: i16) -> ConstantValueSound {
            ConstantValueSound {
                value,
                channel_count: DEFAULT_CHANNEL_COUNT,
                sample_rate: DEFAULT_SAMPLE_RATE,
                metadata_changed: false,
            }
        }
    }

    impl Sound for ConstantValueSound {
        fn channel_count(&self) -> u16 {
            self.channel_count
        }

        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }

        fn next_samples_for(
            &mut self,
            buffer: &mut [i16],
        ) -> Result<NextSampleBuffer, crate::RawedioError> {
            if self.metadata_changed {
                self.metadata_changed = false;
                return Ok(NextSampleBuffer::MetadataChanged(0));
            }
            buffer.fill(self.value);
            Ok(NextSampleBuffer::Continue)
        }

        fn next_sample(&mut self) -> Result<NextSample, crate::RawedioError> {
            if self.metadata_changed {
                self.metadata_changed = false;
                return Ok(NextSample::MetadataChanged);
            }
            Ok(NextSample::Sample(self.value))
        }
    }

    impl ConstantValueSound {
        pub fn set_channel_count(&mut self, new_count: u16) {
            self.channel_count = new_count;
            self.metadata_changed = true;
        }

        pub fn set_sample_rate(&mut self, new_rate: u32) {
            self.sample_rate = new_rate;
            self.metadata_changed = true;
        }
    }

    /// Start at 0, increment by 1 until MAX value then jump to MIN value and
    /// increment by 1 again
    pub struct Sawtooth {
        pub value: i16,
        pub channel_count: u16,
        pub channel_idx: u16,
        pub sample_rate: u32,
    }

    impl Sawtooth {
        #[must_use]
        pub fn new(channel_count: u16, sample_rate: u32) -> Sawtooth {
            Sawtooth {
                value: 0,
                channel_count,
                channel_idx: 0,
                sample_rate,
            }
        }
    }

    impl Sound for Sawtooth {
        fn channel_count(&self) -> u16 {
            self.channel_count
        }

        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }

        fn next_samples_for(
            &mut self,
            buffer: &mut [i16],
        ) -> Result<NextSampleBuffer, crate::RawedioError> {
            for dst in buffer {
                *dst = self.value;
                self.channel_idx += 1;
                if self.channel_idx == self.channel_count {
                    self.channel_idx = 0;
                    self.value = self.value.wrapping_add(1);
                }
            }
            Ok(NextSampleBuffer::Continue)
        }

        fn next_sample(&mut self) -> Result<NextSample, crate::RawedioError> {
            let to_return = NextSample::Sample(self.value);
            self.channel_idx += 1;
            if self.channel_idx == self.channel_count {
                self.channel_idx = 0;
                self.value = self.value.wrapping_add(1);
            }
            Ok(to_return)
        }
    }
}
