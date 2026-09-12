//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::{NextState, RawedioError, Sound};

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

    fn fill_next_frames(&mut self, buffer: &mut [i16]) -> Result<(usize, NextState), RawedioError> {
        buffer.fill(0);
        Ok((buffer.len(), NextState::Playing))
    }
}
