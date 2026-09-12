//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::wrappers::{SetPaused, SetSpeed, SetVolume};
use crate::{NextState, RawedioError, Sound};

/// A Sound which can be stopped.
pub trait SetStopped {
    /// Stop the sound
    fn set_stopped(&mut self);
}

/// A wrapper to make a Sound stoppable.
/// A stopped sound will always return `NextSample::Finished`.
pub struct Stoppable<S: Sound> {
    inner: S,
    stopped: bool,
}

impl<S> Stoppable<S>
where S: Sound
{
    /// Wrap `inner` and allow it to be stopped via
    /// [`set_stopped`][SetStopped::set_stopped].
    #[must_use]
    pub const fn new(inner: S) -> Self {
        Stoppable {
            inner,
            stopped: false,
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

impl<S> Sound for Stoppable<S>
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
        if self.stopped {
            return Ok((0, NextState::Finished));
        }
        self.inner.fill_next_frames(buffer)
    }
}

impl<S> Stoppable<S>
where S: Sound
{
    /// Return if the Sound is stopped.
    pub const fn stopped(&self) -> bool {
        self.stopped
    }
}

impl<S> SetStopped for Stoppable<S>
where S: Sound
{
    fn set_stopped(&mut self) {
        self.stopped = true;
    }
}

impl<S> SetPaused for Stoppable<S>
where S: Sound + SetPaused
{
    fn set_paused(&mut self, paused: bool) {
        self.inner.set_paused(paused);
    }
}

impl<S> SetVolume for Stoppable<S>
where S: Sound + SetVolume
{
    fn set_volume(&mut self, multiplier: f32) {
        self.inner.set_volume(multiplier);
    }
}

impl<S> SetSpeed for Stoppable<S>
where S: Sound + SetSpeed
{
    fn set_speed(&mut self, multiplier: f32) {
        self.inner.set_speed(multiplier);
    }
}

#[cfg(test)]
mod tests {

    use crate::utils::test::ConstantValueSound;
    use crate::wrappers::SetStopped;
    use crate::{NextState, Sound};

    #[test]
    fn set_stopped() {
        let mut buffer = [0];
        let mut first = ConstantValueSound::new(1000).stoppable();
        // starts unpaused
        assert_eq!(
            first.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [1000]);
        first.set_stopped();
        assert_eq!(
            first.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
        assert_eq!(
            first.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
    }
}
