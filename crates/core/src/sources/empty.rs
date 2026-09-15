//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::{NextState, RawedioError, Sound};

/// A Sound that immediately and always returns Finished
pub struct Empty {
    channel_count: usize,
    sample_rate: usize,
}

impl Empty {
    /// Create a new sound that will immediately and always returns Finished
    #[must_use]
    pub const fn new(channel_count: usize, sample_rate: usize) -> Empty {
        Empty {
            channel_count,
            sample_rate,
        }
    }
}

impl Sound for Empty {
    fn channel_count(&self) -> usize {
        self.channel_count
    }

    fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    fn fill_next_frames(
        &mut self,
        _buffer: &mut [f32],
    ) -> Result<(usize, NextState), RawedioError> {
        Ok((0, NextState::Finished))
    }
}
