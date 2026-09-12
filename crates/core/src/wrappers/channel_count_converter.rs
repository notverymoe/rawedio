//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use crate::wrappers::Wrapper;
use crate::{NextState, RawedioError, Sound};

/// Convert a Sound to have a specified number of output channels.
/// For example convert a mono sound to stereo or vice versa.
pub struct ChannelCountConverter<S: Sound> {
    inner: S,
    to_count: u16,
    converter_type: ConverterType,
    scratch: Vec<i16>,
}

enum ConverterType {
    PassThrough,
    MonoToStereo,
    StereoToMono,
}

impl<S> ChannelCountConverter<S>
where S: Sound
{
    /// Wrap `inner` such that it will output `to_count` channels.
    pub fn new(inner: S, to_count: u16) -> ChannelCountConverter<S> {
        let converter_type = Self::get_type(inner.channel_count(), to_count);

        ChannelCountConverter {
            inner,
            to_count,
            converter_type,
            scratch: Vec::new(),
        }
    }

    fn get_type(from_count: u16, to_count: u16) -> ConverterType {
        if from_count == to_count {
            ConverterType::PassThrough
        } else if from_count == 1 && to_count == 2 {
            ConverterType::MonoToStereo
        } else if from_count == 2 && to_count == 1 {
            ConverterType::StereoToMono
        } else {
            // Can implement more conversions like
            // https://developer.mozilla.org/en-US/docs/Web/API/Web_Audio_API/Basic_concepts_behind_Web_Audio_API#up-mixing_and_down-mixing
            todo!(
                "ChannelCountConverter for {} to {} channels not implemented.",
                from_count,
                to_count
            );
        }
    }

    // We could save the metadata of the inner Source and only return MetadataChange
    // if the metadata change is something we can't handle (i.e. a Rate Change).
    fn handle_channel_count_change(&mut self) {
        let from_count = self.inner.channel_count();
        self.converter_type = Self::get_type(from_count, self.to_count);
    }

    /// Unwrap the inner Sound.
    ///
    /// It is guaranteed that the inner Sound is at the start of a Frame.
    /// (i.e. the inner sound has not been partially incremented inside a frame)
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S> Sound for ChannelCountConverter<S>
where S: Sound
{
    fn channel_count(&self) -> u16 {
        self.to_count
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn on_start_of_batch(&mut self) {
        self.inner.on_start_of_batch();
    }

    fn fill_next_frames(
        &mut self,
        buffer: &mut [i16],
    ) -> Result<(usize, crate::NextState), RawedioError> {
        match self.converter_type {
            ConverterType::PassThrough => {
                let next = self.inner.fill_next_frames(buffer)?;
                if matches!(next, (_, NextState::MetadataChanged)) {
                    self.handle_channel_count_change();
                }
                Ok(next)
            }
            ConverterType::MonoToStereo => {
                self.scratch.clear();
                self.scratch.resize(buffer.len() / 2, 0);
                let (count, next) = self.inner.fill_next_frames(&mut self.scratch)?;
                if matches!(next, NextState::MetadataChanged) {
                    self.handle_channel_count_change();
                }
                self.scratch[..count]
                    .iter()
                    .flat_map(|s| std::iter::repeat_n(*s, 2))
                    .zip(buffer.iter_mut())
                    .for_each(|(src, dst)| *dst = src);
                Ok((count * 2, next))
            }
            ConverterType::StereoToMono => {
                self.scratch.clear();
                self.scratch.resize(buffer.len() * 2, 0);
                let (count, next) = self.inner.fill_next_frames(&mut self.scratch)?;
                if matches!(next, NextState::MetadataChanged) {
                    self.handle_channel_count_change();
                }

                self.scratch[..count]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|[l, r]| i16::midpoint(*l, *r))
                    .zip(buffer.iter_mut())
                    .for_each(|(src, dst)| *dst = src);

                Ok((count / 2, next))
            }
        }
    }
}

impl<S: Sound> Wrapper for ChannelCountConverter<S> {
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
