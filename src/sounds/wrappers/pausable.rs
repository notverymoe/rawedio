//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use super::{SetSpeed, SetVolume};
use crate::{sounds::wrappers::SetStopped, NextSample, NextSampleBuffer, RawedioError, Sound};

/// A Sound which can be paused.
pub trait SetPaused {
    /// Pause or unpause the sound.
    fn set_paused(&mut self, paused: bool);
}

/// A wrapper to make a Sound pausable.
pub struct Pausable<S: Sound> {
    inner: S,
    paused: bool,
}

impl<S> Pausable<S>
where S: Sound
{
    /// Wrap `inner` and allow it to be paused via
    /// [`set_paused`][SetPaused::set_paused].
    pub const fn new(inner: S) -> Self {
        Pausable {
            inner,
            paused: false,
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

impl<S> Sound for Pausable<S>
where S: Sound
{
    fn channel_count(&self) -> u16 {
        self.inner.channel_count()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn next_samples_for(&mut self, buffer: &mut [i16]) -> Result<NextSampleBuffer, RawedioError> {
        if self.paused {
            return Ok(NextSampleBuffer::Paused(0));
        }
        self.inner.next_samples_for(buffer)
    }

    fn next_sample(&mut self) -> Result<NextSample, RawedioError> {
        if self.paused {
            return Ok(NextSample::Paused);
        }
        self.inner.next_sample()
    }

    fn on_start_of_batch(&mut self, count: usize) {
        self.inner.on_start_of_batch(count);
    }
}

impl<S> Pausable<S>
where S: Sound
{
    /// Return if the Sound is currently being paused.
    pub const fn paused(&self) -> bool {
        self.paused
    }
}

impl<S> SetPaused for Pausable<S>
where S: Sound
{
    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }
}

impl<S> SetStopped for Pausable<S>
where S: Sound + SetStopped
{
    fn set_stopped(&mut self) {
        self.inner.set_stopped();
    }
}

impl<S> SetVolume for Pausable<S>
where S: Sound + SetVolume
{
    fn set_volume(&mut self, multiplier: f32) {
        self.inner.set_volume(multiplier);
    }
}

impl<S> SetSpeed for Pausable<S>
where S: Sound + SetSpeed
{
    fn set_speed(&mut self, multiplier: f32) {
        self.inner.set_speed(multiplier);
    }
}
