//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::time::Duration;

use crate::wrappers::Wrapper;
use crate::{NextState, RawedioError, Sound};

/// Play the  first part of an inner Sound measured in seconds then stop even
/// if the inner sound still has samples remaining. Finishes early if the inner
/// sound finishes before duration. Any time the inner sample is paused does not
/// count against the duration (i.e. duration only includes time of samples).
pub struct FinishAfter<S: Sound> {
    inner: S,
    fames_remaining: u64,
    total_duration: Duration,
    current_channel_count: u16,
    current_sample_rate: u32,
}

impl<S> FinishAfter<S>
where S: Sound
{
    /// Only play the first `duration` of inner before finishing.
    pub fn new(inner: S, duration: Duration) -> Self {
        let current_channel_count = inner.channel_count();
        let current_sample_rate = inner.sample_rate();
        let fames_remaining = num_frames(duration, current_sample_rate);
        FinishAfter {
            inner,
            fames_remaining,
            total_duration: duration,
            current_channel_count,
            current_sample_rate,
        }
    }

    /// Get a reference to the wrapped inner Sound.
    pub const fn inner(&self) -> &S {
        &self.inner
    }

    /// Get a mutable reference to the wrapped inner Sound.
    pub const fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Unwrap and return the previously wrapped Sound.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S> Sound for FinishAfter<S>
where S: Sound
{
    fn channel_count(&self) -> u16 {
        self.inner.channel_count()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn on_start_of_batch(&mut self) {
        self.inner.on_start_of_batch();
    }

    fn fill_next_frames(&mut self, buffer: &mut [i16]) -> Result<(usize, NextState), RawedioError> {
        if self.fames_remaining == 0 {
            return Ok((0, NextState::Finished));
        }

        let samples_max = usize::min(
            self.fames_remaining as usize * self.current_channel_count as usize,
            buffer.len(),
        );
        let next = self.inner.fill_next_frames(&mut buffer[..samples_max])?;
        match next {
            (_, NextState::Playing) => {
                self.fames_remaining -= (samples_max / self.current_channel_count as usize) as u64;
            }
            (_, NextState::MetadataChanged) => {
                let total_old_frames = num_frames(self.total_duration, self.current_sample_rate);
                let num_frames_played = total_old_frames - self.fames_remaining;
                let seconds_played = num_frames_played as f64 / self.current_sample_rate as f64;
                let duration_played = Duration::from_secs_f64(seconds_played);
                let duration_remaining = self.total_duration.checked_sub(duration_played).unwrap();
                self.current_channel_count = self.inner.channel_count();
                self.current_sample_rate = self.inner.sample_rate();
                self.fames_remaining = num_frames(duration_remaining, self.current_sample_rate);
            }
            (_, NextState::Paused) => (),
            (_, NextState::Finished) => (),
        }
        Ok(next)
    }
}

pub const fn num_frames(duration: Duration, num_samples: u32) -> u64 {
    const MICROS_PER_SEC: u64 = 1_000_000;
    let micros = duration.as_secs() * MICROS_PER_SEC + duration.subsec_micros() as u64;
    micros * num_samples as u64 / MICROS_PER_SEC
}

impl<S: Sound> Wrapper for FinishAfter<S> {
    type Inner = S;

    fn inner(&self) -> &Self::Inner {
        &self.inner
    }

    fn inner_mut(&mut self) -> &mut Self::Inner {
        &mut self.inner
    }

    fn into_inner(self) -> Self::Inner {
        self.inner
    }
}

#[cfg(test)]
mod tests {

    use std::assert_matches;
    use std::time::Duration;

    use crate::utils::test::ConstantValueSound;
    use crate::wrappers::SetPaused;
    use crate::{NextState, Sound};

    #[test]
    fn test_simple() {
        let mut buffer = [0, 0];
        let mut sound = ConstantValueSound::new(1000).finish_after(Duration::from_millis(100));
        for _ in 0..(44100 / 10) {
            assert_matches!(
                sound.fill_next_frames(&mut buffer).unwrap(),
                (2, NextState::Playing | NextState::Finished)
            );
        }
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
    }

    #[test]
    fn test_pausing_does_not_count() {
        let mut buffer = [0, 0];
        let mut sound = ConstantValueSound::new(1000)
            .pausable()
            .finish_after(Duration::from_millis(100));
        for s in 0..(44100 / 10) {
            if s % 15 == 0 {
                sound.set_paused(true);
                assert_eq!(
                    sound.fill_next_frames(&mut buffer).unwrap(),
                    (0, NextState::Paused)
                );
                sound.set_paused(false);
            }
            assert_matches!(
                sound.fill_next_frames(&mut buffer).unwrap(),
                (2, NextState::Playing | NextState::Finished)
            );
            assert_eq!(buffer, [1000, 1000]);
        }
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
    }

    #[test]
    fn test_metadata_change_beginning() {
        let mut buffer = [0];
        let mut sound = ConstantValueSound::new(1000).finish_after(Duration::from_millis(100));
        sound.inner_mut().set_sample_rate(22050);
        sound.inner_mut().set_channel_count(1);
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::MetadataChanged)
        );
        for _ in 0..(22050 / 10) {
            assert_matches!(
                sound.fill_next_frames(&mut buffer).unwrap(),
                (1, NextState::Playing | NextState::Finished)
            );
            assert_eq!(buffer, [1000]);
        }
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
    }

    #[test]
    fn test_metadata_change_halfway() {
        let mut buffer = [0, 0];
        let mut sound = ConstantValueSound::new(1000).finish_after(Duration::from_millis(100));
        for _ in 0..(44100 / 20) {
            assert_matches!(
                sound.fill_next_frames(&mut buffer).unwrap(),
                (2, NextState::Playing | NextState::Finished)
            );
            assert_eq!(buffer, [1000, 1000]);
        }
        sound.inner_mut().set_sample_rate(88200);
        sound.inner_mut().set_channel_count(4);
        let mut buffer = [0, 0, 0, 0];
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::MetadataChanged)
        );
        for _ in 0..(88200 / 20) {
            assert_matches!(
                sound.fill_next_frames(&mut buffer).unwrap(),
                (4, NextState::Playing | NextState::Finished)
            );
            assert_eq!(buffer, [1000, 1000, 1000, 1000]);
        }
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
    }

    #[test]
    fn test_metadata_change_end() {
        let mut buffer = [0, 0];
        let mut sound = ConstantValueSound::new(1000).finish_after(Duration::from_millis(100));
        for _ in 0..(44100 / 10) {
            assert_matches!(
                sound.fill_next_frames(&mut buffer).unwrap(),
                (2, NextState::Playing | NextState::Finished)
            );
            assert_eq!(buffer, [1000, 1000]);
        }
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
        sound.inner_mut().set_sample_rate(22050);
        sound.inner_mut().set_channel_count(1);
        let mut buffer = [0];
        assert_eq!(
            sound.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
    }
}
