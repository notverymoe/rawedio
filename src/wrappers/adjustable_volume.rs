//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::wrappers::{SetPaused, SetSpeed, SetStopped};
use crate::{NextState, RawedioError, Sound};

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
where S: Sound
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
        let next = self.inner.fill_next_frames(buffer)?;
        let count = match next {
            (_, NextState::Playing) => buffer.len(),
            (count, NextState::MetadataChanged | NextState::Paused | NextState::Finished) => count,
        };
        buffer[..count]
            .iter_mut()
            .for_each(|s| *s = ((*s as f32) * self.volume_adjustment) as i16);
        Ok(next)
    }
}

impl<S> AdjustableVolume<S>
where S: Sound
{
    /// Return the current gain multiplier. 1.0 is the default multiplier.
    pub const fn volume(&self) -> f32 {
        self.volume_adjustment
    }
}

impl<S> SetVolume for AdjustableVolume<S>
where S: Sound
{
    fn set_volume(&mut self, new: f32) {
        self.volume_adjustment = new;
    }
}

impl<S> SetPaused for AdjustableVolume<S>
where S: Sound + SetPaused
{
    fn set_paused(&mut self, paused: bool) {
        self.inner.set_paused(paused);
    }
}

impl<S> SetStopped for AdjustableVolume<S>
where S: Sound + SetStopped
{
    fn set_stopped(&mut self) {
        self.inner.set_stopped();
    }
}

impl<S> SetSpeed for AdjustableVolume<S>
where S: Sound + SetSpeed
{
    fn set_speed(&mut self, multiplier: f32) {
        self.inner.set_speed(multiplier);
    }
}

#[cfg(test)]
mod tests {

    use crate::utils::test::ConstantValueSound;
    use crate::wrappers::SetVolume;
    use crate::{NextState, Sound};

    #[test]
    fn adjust_down() {
        let mut buffer = [0];
        let mut first = ConstantValueSound::new(1000).with_adjustable_volume();
        first.set_volume(0.5);
        assert_eq!(
            first.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [500]);
    }

    #[test]
    fn adjust_up() {
        let mut buffer = [0];
        let mut first = ConstantValueSound::new(1000).with_adjustable_volume();
        first.set_volume(5.0);
        assert_eq!(
            first.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [5000]);
    }

    #[test]
    fn test_saturation() {
        let mut buffer = [0];
        let mut first = ConstantValueSound::new(1000).with_adjustable_volume();
        first.set_volume(1000.0);
        assert_eq!(
            first.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [32767]);
    }
}
