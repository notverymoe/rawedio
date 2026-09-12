//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

//! Items that are deprecated

#![deprecated]

use crate::{NextState, RawedioError, Sound};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NextSample {
    /// A sample for one channel. Channels are interleaved. The first sample is
    /// for the first channel and so forth and repeats (e.g. L-R-L-R-L-R).
    Sample(i16),

    /// The number of channels or the sample rate has changed. Continue to
    /// retrieve samples afterward. The next sample will always be for the
    /// first track regardless of what track was next
    // before this value was returned.
    MetadataChanged,

    /// No more samples for now. More might come later. It is expected that the
    /// Sound will not be pulled again during this batch of samples.
    Paused,

    /// All samples have been retrieved and no more will come.
    Finished,
}

impl From<NextSample> for NextState {
    fn from(value: NextSample) -> Self {
        match value {
            NextSample::Sample(_) => NextState::Playing,
            NextSample::MetadataChanged => NextState::MetadataChanged,
            NextSample::Paused => NextState::Paused,
            NextSample::Finished => NextState::Finished,
        }
    }
}

pub struct SampleBySample {
    buffer: Vec<i16>,
    response: NextState,
}

impl SampleBySample {
    pub const fn new() -> Self {
        Self {
            buffer: vec![],
            response: NextState::Playing,
        }
    }

    fn refill(&mut self, other: &mut dyn Sound) -> Result<NextState, RawedioError> {
        self.buffer.resize(other.channel_count() as usize, 0);
        let (count, next) = other.fill_next_frames(&mut self.buffer)?;
        self.buffer.truncate(count);
        Ok(std::mem::replace(&mut self.response, next))
    }

    pub fn next_sample(&mut self, other: &mut dyn Sound) -> Result<NextSample, RawedioError> {
        if let Some(s) = self.buffer.pop() {
            Ok(NextSample::Sample(s))
        } else {
            match std::mem::replace(&mut self.response, NextState::Playing) {
                NextState::Playing => {
                    self.refill(other)?;
                    self.next_sample(other)
                }
                NextState::MetadataChanged => Ok(NextSample::MetadataChanged),
                NextState::Paused => Ok(NextSample::Paused),
                NextState::Finished => Ok(NextSample::Finished),
            }
        }
    }

    pub fn next_frame(
        &mut self,
        other: &mut dyn Sound,
    ) -> Result<Vec<i16>, Result<NextSample, RawedioError>> {
        let mut samples = Vec::with_capacity(other.channel_count() as usize);
        self.append_next_frame_to(other, &mut samples)?;
        Ok(samples)
    }

    pub fn append_next_frame_to(
        &mut self,
        other: &mut dyn Sound,
        samples: &mut Vec<i16>,
    ) -> Result<(), Result<NextSample, RawedioError>> {
        for _ in 0..other.channel_count() {
            let next = self.next_sample(other);
            match next {
                Ok(NextSample::Sample(s)) => samples.push(s),
                Ok(NextSample::MetadataChanged | NextSample::Paused | NextSample::Finished)
                | Err(_) => return Err(next),
            }
        }
        Ok(())
    }
}
