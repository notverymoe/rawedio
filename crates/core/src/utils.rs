//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

//! Misc utilities

use std::{hash::BuildHasherDefault, time::Duration};

use indexmap::IndexMap;
use nohash_hasher::NoHashHasher;

/// Provides a version of the fast-iteration associative `IndexMap` that applies no
/// hasher to the `std::hash::Hash` result of the key, providing improved lookup speed.
pub type NoHashIndexMap<K, V> = IndexMap<K, V, BuildHasherDefault<NoHashHasher<K>>>;

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
pub const fn convert_num_frames(
    old_num_samples: u64,
    old_sample_rate: u32,
    new_sample_rate: u32,
) -> u64 {
    old_num_samples * new_sample_rate as u64 / (old_sample_rate as u64)
}

/// Return the number of samples that happen within `duration` amount of time
/// (truncates).
#[must_use]
pub fn duration_to_num_frames(duration: Duration, sample_rate: u32) -> u64 {
    convert_num_frames(
        duration
            .as_micros()
            .try_into()
            .expect("duration in microseconds is too large to fit into a u64"),
        1_000_000,
        sample_rate,
    )
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
pub mod test {
    #![allow(missing_docs)]

    //! Common utilities for tests.

    use crate::{NextState, RawedioError, Sound};

    pub const DEFAULT_SAMPLE_RATE: u32 = 44100;
    pub const DEFAULT_CHANNEL_COUNT: u16 = 2;

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

        fn fill_next_frames(
            &mut self,
            buffer: &mut [i16],
        ) -> Result<(usize, NextState), RawedioError> {
            if self.metadata_changed {
                self.metadata_changed = false;
                return Ok((0, NextState::MetadataChanged));
            }
            buffer.fill(self.value);
            Ok((buffer.len(), NextState::Playing))
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

        fn next_sample(&mut self) -> i16 {
            let to_return = self.value;
            self.channel_idx += 1;
            if self.channel_idx == self.channel_count {
                self.channel_idx = 0;
                self.value = self.value.wrapping_add(1);
            }
            to_return
        }
    }

    impl Sound for Sawtooth {
        fn channel_count(&self) -> u16 {
            self.channel_count
        }

        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }

        fn fill_next_frames(
            &mut self,
            buffer: &mut [i16],
        ) -> Result<(usize, NextState), RawedioError> {
            for dst in buffer.iter_mut() {
                *dst = self.next_sample();
            }

            Ok((buffer.len(), NextState::Playing))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::utils::{convert_num_samples, duration_to_num_samples};

    #[test]
    fn test_convert_num_samples() {
        assert_eq!(convert_num_samples(1000, 2, 44100, 2, 44100), 1000);
        assert_eq!(convert_num_samples(1000, 2, 44100, 2, 22050), 500);
        assert_eq!(convert_num_samples(1000, 2, 22050, 2, 44100), 2000);
        assert_eq!(convert_num_samples(1000, 2, 44100, 1, 44100), 500);
        // Truncates fractional samples
        assert_eq!(convert_num_samples(4, 1, 44100, 100, 1), 0);
    }

    #[test]
    fn test_duration_to_num_samples() {
        assert_eq!(
            duration_to_num_samples(Duration::from_millis(1_000), 1, 44100),
            44100
        );
        assert_eq!(
            duration_to_num_samples(Duration::from_millis(1), 1, 44100),
            44
        );
        assert_eq!(
            duration_to_num_samples(Duration::from_millis(10), 1, 44100),
            441
        );
        // 277 hours
        assert_eq!(
            duration_to_num_samples(Duration::from_secs(1_000_000), 6, 44100),
            264_600_000_000
        );
    }
}
