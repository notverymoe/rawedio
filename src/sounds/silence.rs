//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::{NextSample, NextSampleBuffer, RawedioError, Sound};

/// A forever stream of samples of value 0 (creating no sound).
pub struct Silence {
    channel_count: u16,
    sample_rate: u32,
}

impl Silence {
    /// Create a new sound that will return 0 samples forever and
    /// return the specified channel count and sample rate from
    /// their respective methods.
    #[must_use]
    pub const fn new(channel_count: u16, sample_rate: u32) -> Silence {
        Silence {
            channel_count,
            sample_rate,
        }
    }
}

impl Sound for Silence {
    fn channel_count(&self) -> u16 {
        self.channel_count
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn next_samples_for(&mut self, buffer: &mut [i16]) -> Result<NextSampleBuffer, RawedioError> {
        buffer.fill(0);
        Ok(NextSampleBuffer::Continue)
    }

    fn next_sample(&mut self) -> Result<NextSample, RawedioError> {
        Ok(NextSample::Sample(0))
    }
}
