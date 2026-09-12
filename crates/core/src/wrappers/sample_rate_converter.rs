use crate::legacy::{NextSample, SampleBySample};
use crate::wrappers::Wrapper;
use crate::{NextState, RawedioError, Sound};

// Forked from https://github.com/RustAudio/rodio/blob/d5b9ae3467dab4316ee77b260a5b7432f74866b0/src/conversions/sample_rate.rs

/// Convert a Sound from one sample rate (number of samples per second) to
/// another.
pub struct SampleRateConverter<S: Sound> {
    /// The from Sound we are pulling samples from.
    inner: S,
    /// The output sample rate in samples per second.
    to_rate: u32,
    /// This is not the samples per second of the output but a possibly scaled
    /// down value.
    to_rate_scaled: u32,
    /// This is not the samples per second of inner but a possibly scaled down
    /// value.
    from_rate_scaled: u32,
    /// One sample per channel, extracted from `inner`.
    current_frame: Vec<i16>,
    /// The samples right after `current_frame` (one per channel), extracted
    /// from `inner`.
    next_frame: Vec<i16>,
    /// Position of `current_sample` modulo `from_rate_scaled`.
    current_frame_pos_in_chunk: u32,
    /// The position of the next sample that this sound should return, modulo
    /// `to_rate_scaled`. This counter is incremented (modulo
    /// `to_rate_scale`) every time the iterator returns a complete frame.
    next_output_frame_pos_in_chunk: u32,
    /// The buffer containing the samples waiting to be output. Never needs to
    /// contain the first channels sample. The highest channel is stored
    /// first for efficient `Vec::pop` retrieval
    output_frame: Vec<i16>,
    /// The channel count of inner and ourself
    channel_count: u16,
    /// The number of channels has changed. We need to notify the output.
    /// Note that we do not need to notify the output for sample rate changes
    /// because we ensure we always output the same output sample rate and
    /// we always output full frames.
    channel_count_changed: bool,
    /// Whether the inner channel last returned Paused or Finished
    inner_paused: bool,

    helper: SampleBySample,
}

impl<S> SampleRateConverter<S>
where S: Sound
{
    /// Create a new `SampleRateConverter` with an output sample rate of
    /// `to_rate`.
    pub fn new(inner: S, to_rate: u32) -> SampleRateConverter<S> {
        let channel_count = inner.channel_count();
        let mut new = SampleRateConverter {
            inner,
            to_rate,
            to_rate_scaled: 0,
            from_rate_scaled: 0,
            current_frame_pos_in_chunk: 0,
            next_output_frame_pos_in_chunk: 0,
            current_frame: Vec::new(),
            next_frame: Vec::new(),
            output_frame: Vec::new(),
            channel_count,
            channel_count_changed: false,
            inner_paused: false,
            helper: SampleBySample::new(),
        };
        new.init();
        new
    }

    fn init(&mut self) {
        let channel_count = self.inner.channel_count();
        if self.channel_count != channel_count {
            self.channel_count_changed = true;
            self.channel_count = channel_count;
        }
        let from_rate = self.inner.sample_rate();
        assert!(from_rate >= 1);
        assert!(self.to_rate >= 1);

        // finding greatest common divisor
        let gcd = {
            #[inline]
            fn gcd(a: u32, b: u32) -> u32 {
                if b == 0 {
                    a
                } else {
                    gcd(b, a % b)
                }
            }

            gcd(from_rate, self.to_rate)
        };

        // These will get filled on the first or next call to next_sample
        self.current_frame = Vec::new();
        self.next_frame = Vec::new();

        self.to_rate_scaled = self.to_rate / gcd;
        self.from_rate_scaled = from_rate / gcd;
        self.current_frame_pos_in_chunk = 0;
        self.next_output_frame_pos_in_chunk = 0;
        self.output_frame = Vec::with_capacity(channel_count as usize - 1);
    }

    fn fill_frames(&mut self) -> Result<bool, RawedioError> {
        let (first_samples, next_samples) = if self.from_rate_scaled == self.to_rate_scaled {
            (Vec::new(), Vec::new())
        } else {
            let mut collect_frame = || match self.helper.next_frame(&mut self.inner) {
                Ok(f) => Ok(f),
                Err(special) => match special {
                    Ok(NextSample::Sample(_)) => unreachable!(),
                    Ok(NextSample::MetadataChanged) => Err(None),
                    Err(e) => Err(Some(e)),
                    Ok(NextSample::Paused) => {
                        self.inner_paused = true;
                        Ok(Vec::new())
                    }
                    Ok(NextSample::Finished) => {
                        self.inner_paused = false;
                        Ok(Vec::new())
                    }
                },
            };
            let first = match collect_frame() {
                Ok(o) => o,
                Err(Some(e)) => return Err(e),
                Err(None) => return Ok(false),
            };
            let next = match collect_frame() {
                Ok(o) => o,
                Err(Some(e)) => return Err(e),
                Err(None) => return Ok(false),
            };
            (first, next)
        };
        self.current_frame = first_samples;
        self.next_frame = next_samples;
        Ok(true)
    }

    fn next_input_frame(&mut self) -> Result<bool, RawedioError> {
        self.current_frame_pos_in_chunk += 1;

        std::mem::swap(&mut self.current_frame, &mut self.next_frame);
        self.next_frame.clear();
        let specials = self
            .helper
            .append_next_frame_to(&mut self.inner, &mut self.next_frame);
        match specials {
            Ok(()) => (),
            Err(Ok(NextSample::Sample(_))) => unreachable!(),
            Err(Ok(NextSample::MetadataChanged)) => {
                return Ok(false);
            }
            // We handle not having any more samples left outside this function
            Err(Ok(NextSample::Paused)) => self.inner_paused = true,
            Err(Ok(NextSample::Finished)) => self.inner_paused = false,
            Err(Err(e)) => return Err(e),
        }
        Ok(true)
    }

    /// Unwrap the inner Sound.
    ///
    /// It is guaranteed that the inner Sound is at the start of a Frame.
    /// (i.e. the inner sound has not been partially incremented inside a frame)
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S> Sound for SampleRateConverter<S>
where S: Sound
{
    fn channel_count(&self) -> u16 {
        self.inner.channel_count()
    }

    fn sample_rate(&self) -> u32 {
        self.to_rate
    }

    fn on_start_of_batch(&mut self) {
        self.inner.on_start_of_batch();
    }

    fn fill_next_frames(&mut self, buffer: &mut [i16]) -> Result<(usize, NextState), RawedioError> {
        for (i, dst) in buffer.iter_mut().enumerate() {
            match self.next_sample() {
                Ok(NextSample::Sample(s)) => *dst = s,
                Ok(NextSample::MetadataChanged) => return Ok((i, NextState::MetadataChanged)),
                Ok(NextSample::Finished) => return Ok((i, NextState::Finished)),
                Ok(NextSample::Paused) => return Ok((i, NextState::Paused)),
                Err(e) => return Err(e),
            }
        }

        Ok((buffer.len(), NextState::Playing))
    }
}

impl<S> SampleRateConverter<S>
where S: Sound
{
    // TODO OPT `next_samples_for`

    fn next_sample(&mut self) -> Result<NextSample, RawedioError> {
        if self.channel_count_changed {
            self.channel_count_changed = false;
            return Ok(NextSample::MetadataChanged);
        }
        // the algorithm below doesn't work if `self.from_rate_scaled ==
        // self.to_rate_scaled`
        if self.from_rate_scaled == self.to_rate_scaled {
            debug_assert_eq!(self.from_rate_scaled, 1);
            let next = self.helper.next_sample(&mut self.inner)?;
            match next {
                NextSample::Sample(_) | NextSample::Paused | NextSample::Finished => {
                    return Ok(next);
                }
                NextSample::MetadataChanged => {
                    if self.inner.sample_rate() != self.to_rate {
                        self.init();
                    }
                    return Ok(NextSample::MetadataChanged);
                }
            }
        }

        // Short circuit if there are some samples waiting in the already processed
        // frame
        if let Some(sample) = self.output_frame.pop() {
            return Ok(NextSample::Sample(sample));
        }

        // Coming back from being paused or first run. Refill our frames.
        if self.current_frame.is_empty() && !self.fill_frames()? {
            self.init();
            return self.next_sample();
        }

        // The frame we are going to return from this function will be a linear
        // interpolation between `self.current_frame` and `self.next_frame`.

        if self.next_output_frame_pos_in_chunk == self.to_rate_scaled {
            // If we jump to the next frame, we reset the whole state.
            self.next_output_frame_pos_in_chunk = 0;

            if !self.next_input_frame()? {
                self.init();
                return self.next_sample();
            }
            while self.current_frame_pos_in_chunk != self.from_rate_scaled {
                if !self.next_input_frame()? {
                    self.init();
                    return self.next_sample();
                }
            }
            self.current_frame_pos_in_chunk = 0;
        } else {
            // Finding the position of the first sample of the linear interpolation.
            let req_left_sample = (self.from_rate_scaled * self.next_output_frame_pos_in_chunk
                / self.to_rate_scaled)
                % self.from_rate_scaled;

            // Advancing `self.current_frame`, `self.next_frame` and
            // `self.current_frame_pos_in_chunk` until the latter variable
            // matches `req_left_sample`.
            while self.current_frame_pos_in_chunk != req_left_sample {
                if !self.next_input_frame()? {
                    self.init();
                    return self.next_sample();
                }
                debug_assert!(self.current_frame_pos_in_chunk < self.from_rate_scaled);
            }
        }

        // Merging `self.current_frame` and `self.next_frame` into `self.output_frame`.
        // Note that `self.output_frame` can be truncated if there is not enough data in
        // `self.next_frame`.
        let mut result = None;
        let numerator =
            (self.from_rate_scaled * self.next_output_frame_pos_in_chunk) % self.to_rate_scaled;
        // If we are coming back from a pause where the next frame was empty,
        // lets fill both frames
        if self.current_frame.is_empty() && !self.next_frame.is_empty() {
            let has_next = self.next_input_frame()?;
            if !has_next {
                self.init();
                return self.next_sample();
            }
        }
        for (index, (cur, next)) in self
            .current_frame
            .iter()
            .zip(self.next_frame.iter())
            .enumerate()
            // push frames in reverse for efficient retrieval
            .rev()
        {
            let sample = linear_interpolation(*cur, *next, numerator, self.to_rate_scaled);

            if index == 0 {
                result = Some(sample);
            } else {
                self.output_frame.push(sample);
            }
        }

        // Incrementing the counter for the next iteration.
        self.next_output_frame_pos_in_chunk += 1;

        if let Some(sample) = result {
            Ok(NextSample::Sample(sample))
        } else {
            // If there are no more samples for next_frame we still want to send
            // current_frame to the output
            if self.current_frame.is_empty() {
                // Set things up so we will attempt to pull for more frames again
                self.current_frame_pos_in_chunk = 0;
                self.next_output_frame_pos_in_chunk = 0;
                if self.inner_paused {
                    Ok(NextSample::Paused)
                } else {
                    Ok(NextSample::Finished)
                }
            } else {
                self.current_frame.reverse();
                let r = NextSample::Sample(self.current_frame.pop().unwrap());
                std::mem::swap(&mut self.output_frame, &mut self.current_frame);
                debug_assert!(self.current_frame.is_empty());
                Ok(r)
            }
        }
    }
}

impl<S: Sound> Wrapper for SampleRateConverter<S> {
    type Inner = S;

    fn inner(&self) -> &S {
        &self.inner
    }

    fn inner_mut(&mut self) -> &mut Self::Inner {
        &mut self.inner
    }

    fn into_inner(self) -> S {
        self.inner
    }
}

const fn linear_interpolation(first: i16, second: i16, numerator: u32, denominator: u32) -> i16 {
    (first as i64 + (second as i64 - first as i64) * numerator as i64 / denominator as i64) as i16
}

#[cfg(test)]
mod tests {
    use crate::utils::test::Sawtooth;
    use crate::wrappers::{SampleRateConverter, SetPaused, SetSpeed, Wrapper};
    use crate::{NextState, Sound};

    #[test]
    fn test_no_conversion() {
        let mut buffer = [0, 0];
        let sound = Sawtooth::new(2, 1000).pausable();
        let mut converted = SampleRateConverter::new(sound, 1000);
        assert_eq!(converted.channel_count(), 2);
        assert_eq!(converted.sample_rate(), 1000);

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [0, 0]);

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [1, 1]);

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [2, 2]);

        converted.inner_mut().set_paused(true);
        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Paused)
        );

        converted.inner_mut().set_paused(false);
        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [3, 3]);
    }

    #[test]
    fn test_four_times() {
        let mut buffer = [0, 0];
        let sound = Sawtooth::new(2, 1000).pausable();
        let mut converted = SampleRateConverter::new(sound, 250);
        assert_eq!(converted.channel_count(), 2);
        assert_eq!(converted.sample_rate(), 250);

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [0, 0]);

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [4, 4]);

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [8, 8]);

        converted.inner_mut().set_paused(true);
        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Paused)
        );

        converted.inner_mut().set_paused(false);
        // The pause interrupted our conversion so we come back in
        // a different offset
        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [10, 10]);

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [14, 14]);
    }

    #[test]
    fn test_div_4() {
        let mut buffer = [0, 0];
        let sound = Sawtooth::new(2, 1000).pausable();
        let mut converted = SampleRateConverter::new(sound, 4000);
        assert_eq!(converted.channel_count(), 2);
        assert_eq!(converted.sample_rate(), 4000);

        for _ in 0..4 {
            assert_eq!(
                converted.fill_next_frames(&mut buffer).unwrap(),
                (2, NextState::Playing)
            );
            assert_eq!(buffer, [0, 0]);
        }
        for _ in 0..4 {
            assert_eq!(
                converted.fill_next_frames(&mut buffer).unwrap(),
                (2, NextState::Playing)
            );
            assert_eq!(buffer, [1, 1]);
        }
        for _ in 0..3 {
            assert_eq!(
                converted.fill_next_frames(&mut buffer).unwrap(),
                (2, NextState::Playing)
            );
            assert_eq!(buffer, [2, 2]);
        }

        converted.inner_mut().set_paused(true);
        // It can take some time for us to flush buffers before we
        // see the pause from the inner item. This is fine.
        for _ in 0..1 {
            assert_eq!(
                converted.fill_next_frames(&mut buffer).unwrap(),
                (2, NextState::Playing)
            );
            assert_eq!(buffer, [2, 2]);
        }

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (2, NextState::Playing)
        );
        assert_eq!(buffer, [3, 3]);

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Paused)
        );
        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Paused)
        );

        converted.inner_mut().set_paused(false);
        // Coming back from being paused we can lose samples

        for _ in 0..4 {
            assert_eq!(
                converted.fill_next_frames(&mut buffer).unwrap(),
                (2, NextState::Playing)
            );
            assert_eq!(buffer, [4, 4]);
        }
    }

    #[test]
    fn catch_metadata_changed_when_passing_through() {
        let mut buffer = [0];
        let sound = Sawtooth::new(1, 1000).with_adjustable_speed();
        let mut converted = SampleRateConverter::new(sound, 1000);
        assert_eq!(converted.channel_count(), 1);
        assert_eq!(converted.sample_rate(), 1000);

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [0]);

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [1]);

        converted.inner_mut().set_speed(2.0);
        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::MetadataChanged)
        );

        assert_eq!(converted.sample_rate(), 1000);
        assert_eq!(converted.inner_mut().sample_rate(), 2000);

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [2]);

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [4]);

        assert_eq!(
            converted.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [6]);

        assert_eq!(converted.channel_count(), 1);
    }
}
