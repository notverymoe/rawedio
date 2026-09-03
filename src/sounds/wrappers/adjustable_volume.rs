//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::{NextSampleBuffer, Sound};

use super::{SetPaused, SetSpeed, SetStopped};

/// A sound multiplied by a linear gain adjustment.
pub trait SetVolume {
    /// Change the gain multiplier.
    ///
    /// The samples are multiplied by `multiplier` so 1.0 would leave the Sound
    /// unchanged. 0.5 would reduce the sample values by half and 2.0 would
    /// double them (saturating if larger than the max value).
    ///
    /// These changes linear and 0.5 will not sound half as loud since
    fn set_volume(&mut self, multiplier: f32);
}

/// A wrapper that adjusts the gain of the inner sound.
pub struct AdjustableVolume<S: Sound> {
    inner: S,
    volume_adjustment: f32,
}

impl<S> AdjustableVolume<S>
where
    S: Sound,
{
    /// Wrap `inner` such that its gain can be adjusted.
    ///
    /// The value is set to 1.0 so no adjustment is made.
    ///
    /// See `set_volume`.
    pub const fn new(inner: S) -> Self {
        AdjustableVolume {
            inner,
            volume_adjustment: 1.0,
        }
    }

    /// Wrap `inner` such that its volume can be adjusted and set an initial
    /// adjustment.
    ///
    /// See `set_volume`.
    pub const fn new_with_volume(inner: S, volume_adjustment: f32) -> Self {
        AdjustableVolume {
            inner,
            volume_adjustment,
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

impl<S> Sound for AdjustableVolume<S>
where
    S: Sound,
{
    fn channel_count(&self) -> u16 {
        self.inner.channel_count()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn next_samples_for(
        &mut self,
        buffer: &mut [i16],
    ) -> Result<NextSampleBuffer, crate::RawedioError> {
        let next = self.inner.next_samples_for(buffer)?;
        let count = match next {
            NextSampleBuffer::Continue => buffer.len(),
            NextSampleBuffer::MetadataChanged(count)
            | NextSampleBuffer::Paused(count)
            | NextSampleBuffer::Finished(count) => count,
        };
        buffer[..count]
            .iter_mut()
            .for_each(|s| *s = ((*s as f32) * self.volume_adjustment) as i16);
        Ok(next)
    }

    fn next_sample(&mut self) -> Result<crate::NextSample, crate::RawedioError> {
        let next = self.inner.next_sample()?;
        Ok(match next {
            crate::NextSample::Sample(s) => {
                // Since Rust 1.45, the `as` keyword performs a *saturating cast*
                // when casting from float to int.
                let adjusted = (s as f32 * self.volume_adjustment) as i16;
                crate::NextSample::Sample(adjusted)
            }
            crate::NextSample::MetadataChanged
            | crate::NextSample::Paused
            | crate::NextSample::Finished => next,
        })
    }

    fn on_start_of_batch(&mut self, count: usize) {
        self.inner.on_start_of_batch(count);
    }

    fn into_memory_sound(self) -> Result<crate::sounds::MemorySound, crate::RawedioError>
    where
        Self: Sized,
    {
        crate::sounds::MemorySound::from_sound(self)
    }

    fn loop_from_memory(self) -> Result<crate::sounds::MemorySound, crate::RawedioError>
    where
        Self: Sized,
    {
        let mut to_return = crate::sounds::MemorySound::from_sound(self)?;
        to_return.set_looping(true);
        Ok(to_return)
    }

    fn controllable(self) -> (super::Controllable<Self>, super::Controller<Self>)
    where
        Self: Sized,
    {
        super::Controllable::new(self)
    }

    fn with_async_completion_notifier(
        self,
    ) -> (
        crate::sounds::wrappers::AsyncCompletionNotifier<Self>,
        tokio::sync::oneshot::Receiver<()>,
    )
    where
        Self: Sized,
    {
        crate::sounds::wrappers::AsyncCompletionNotifier::new(self)
    }

    fn with_completion_notifier(
        self,
    ) -> (
        crate::sounds::wrappers::CompletionNotifier<Self>,
        std::sync::mpsc::Receiver<()>,
    )
    where
        Self: Sized,
    {
        crate::sounds::wrappers::CompletionNotifier::new(self)
    }

    fn with_adjustable_volume(self) -> AdjustableVolume<Self>
    where
        Self: Sized,
    {
        AdjustableVolume::new(self)
    }

    fn with_adjustable_volume_of(self, volume_adjustment: f32) -> AdjustableVolume<Self>
    where
        Self: Sized,
    {
        AdjustableVolume::new_with_volume(self, volume_adjustment)
    }

    fn with_adjustable_speed(self) -> super::AdjustableSpeed<Self>
    where
        Self: Sized,
    {
        super::AdjustableSpeed::new(self)
    }

    fn with_adjustable_speed_of(self, speed_adjustment: f32) -> super::AdjustableSpeed<Self>
    where
        Self: Sized,
    {
        super::AdjustableSpeed::new_with_speed(self, speed_adjustment)
    }

    fn pausable(self) -> super::Pausable<Self>
    where
        Self: Sized,
    {
        super::Pausable::new(self)
    }

    fn paused(self) -> super::Pausable<Self>
    where
        Self: Sized,
    {
        let mut to_return = super::Pausable::new(self);
        to_return.set_paused(true);
        to_return
    }

    fn stoppable(self) -> super::Stoppable<Self>
    where
        Self: Sized,
    {
        super::Stoppable::new(self)
    }

    fn finish_after(self, duration: std::time::Duration) -> super::FinishAfter<Self>
    where
        Self: Sized,
    {
        super::FinishAfter::new(self, duration)
    }

    fn skip(&mut self, duration: std::time::Duration) -> Result<bool, crate::RawedioError> {
        let mut current_channel_count = self.channel_count();
        let mut current_sample_rate = self.sample_rate();
        let mut num_samples_remaining = crate::utils::duration_to_num_samples(
            duration,
            current_channel_count,
            current_sample_rate,
        );

        while num_samples_remaining > 0 {
            let next = self.next_sample()?;
            match next {
                crate::NextSample::Sample(_) => {
                    num_samples_remaining -= 1;
                }
                crate::NextSample::MetadataChanged => {
                    let new_channel_count = self.channel_count();
                    let new_sample_rate = self.sample_rate();
                    if new_channel_count != current_channel_count
                        || new_sample_rate != current_sample_rate
                    {
                        num_samples_remaining = crate::utils::convert_num_samples(
                            num_samples_remaining,
                            current_channel_count,
                            current_sample_rate,
                            new_channel_count,
                            new_sample_rate,
                        );
                        current_channel_count = new_channel_count;
                        current_sample_rate = new_sample_rate;
                    }
                }
                crate::NextSample::Paused => return Ok(false),
                crate::NextSample::Finished => return Ok(false),
            }
        }
        Ok(true)
    }
}

impl<S> AdjustableVolume<S>
where
    S: Sound,
{
    /// Return the current gain multiplier. 1.0 is the default multiplier.
    pub const fn volume(&self) -> f32 {
        self.volume_adjustment
    }
}

impl<S> SetVolume for AdjustableVolume<S>
where
    S: Sound,
{
    fn set_volume(&mut self, new: f32) {
        self.volume_adjustment = new;
    }
}

impl<S> SetPaused for AdjustableVolume<S>
where
    S: Sound + SetPaused,
{
    fn set_paused(&mut self, paused: bool) {
        self.inner.set_paused(paused);
    }
}

impl<S> SetStopped for AdjustableVolume<S>
where
    S: Sound + SetStopped,
{
    fn set_stopped(&mut self) {
        self.inner.set_stopped();
    }
}

impl<S> SetSpeed for AdjustableVolume<S>
where
    S: Sound + SetSpeed,
{
    fn set_speed(&mut self, multiplier: f32) {
        self.inner.set_speed(multiplier);
    }
}
