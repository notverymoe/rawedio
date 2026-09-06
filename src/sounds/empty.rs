//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::{NextSample, NextSampleBuffer, RawedioError, Sound};

/// A Sound that immediately and always returns Finished
pub struct Empty {
    channel_count: u16,
    sample_rate: u32,
}

impl Empty {
    /// Create a new sound that will immediately and always returns Finished
    #[must_use]
    pub const fn new(channel_count: u16, sample_rate: u32) -> Empty {
        Empty {
            channel_count,
            sample_rate,
        }
    }
}

impl Sound for Empty {
    fn channel_count(&self) -> u16 {
        self.channel_count
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn next_samples_for(&mut self, _buffer: &mut [i16]) -> Result<NextSampleBuffer, RawedioError> {
        Ok(NextSampleBuffer::Finished(0))
    }

    fn next_sample(&mut self) -> Result<NextSample, RawedioError> {
        Ok(NextSample::Finished)
    }
}
