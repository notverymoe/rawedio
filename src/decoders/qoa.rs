//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::io::Read;

use qoaudio::{DecodeError, QoaDecoder as RawDecoder, QoaItem};

use crate::legacy::NextSample;
use crate::{NextState, RawedioError, Sound};

/// Decoder for the [QOA](https://qoaformat.org/) format.
pub struct QoaDecoder<R>
where R: Read + Send
{
    raw_decoder: RawDecoder<R>,
    sample_rate: u32,
    channel_count: u16,
}

impl<R> QoaDecoder<R>
where R: Read + Send
{
    /// Attempts to decode the data as QOA audio.
    pub fn new(data: R) -> Result<QoaDecoder<R>, DecodeError> {
        let mut raw_decoder = RawDecoder::new(data)?;

        let QoaItem::FrameHeader(first_frame) = raw_decoder
            .next()
            .ok_or(DecodeError::InvalidFrameHeader)??
        else {
            return Err(DecodeError::InvalidFrameHeader);
        };
        let sample_rate = first_frame.sample_rate;
        let channel_count = first_frame.num_channels as u16;

        Ok(QoaDecoder {
            raw_decoder,
            sample_rate,
            channel_count,
        })
    }

    /// Return the wrapped Reader
    pub fn into_inner(self) -> R {
        self.raw_decoder.into_inner()
    }
}

impl<R> Sound for QoaDecoder<R>
where R: Read + Send
{
    fn channel_count(&self) -> u16 {
        self.channel_count
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
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

impl<R> QoaDecoder<R>
where R: Read + Send
{
    // TODO OPT `next_samples_for`

    fn next_sample(&mut self) -> Result<NextSample, RawedioError> {
        loop {
            let Some(next_sample) = self.raw_decoder.next() else {
                return Ok(NextSample::Finished);
            };
            let next_sample = next_sample?;

            match next_sample {
                QoaItem::Sample(s) => return Ok(NextSample::Sample(s)),
                QoaItem::FrameHeader(f) => {
                    if f.num_channels as u16 != self.channel_count
                        || f.sample_rate != self.sample_rate
                    {
                        self.channel_count = f.num_channels.into();
                        self.sample_rate = f.sample_rate;
                        return Ok(NextSample::MetadataChanged);
                    }
                    // No metadata change. Continue and read next sample
                    // continue;
                }
            }
        }
    }
}

impl From<DecodeError> for RawedioError {
    fn from(value: DecodeError) -> Self {
        match value {
            DecodeError::IoError(e) => e.into(),
            DecodeError::NotQoaFile
            | DecodeError::NoSamples
            | DecodeError::InvalidFrameHeader
            | DecodeError::IncompatibleFrame => RawedioError::FormatError(Box::new(value)),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::decoders::QoaDecoder;
    use crate::{NextState, Sound};

    const SINE_WAVE_FILE: &[u8] = include_bytes!("data/audiocheck.net_sin_1000Hz_0dBFS_0.1s.qoa");

    #[test]
    fn samples_of_test_file() {
        let mut buffer = [0];
        let mut decoder = QoaDecoder::new(std::io::Cursor::new(SINE_WAVE_FILE)).unwrap();

        assert_eq!(decoder.sample_rate(), 44100);
        assert_eq!(decoder.channel_count(), 1);

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [422]); // 1

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [4779]); // 2

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [8886]); // 3

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [13834]); // 4

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [17173]); // 5

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [21539]); // 6

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [24403]); // 7

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [27482]); // 8

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [29200]); // 9

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [31270]); // 10

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [31976]); // 11

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [32767]); // 12

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_eq!(buffer, [32183]); // 13

        for _i in 0..4398 {
            let (count, sample) = decoder.fill_next_frames(&mut buffer).unwrap();
            match sample {
                NextState::Playing => {}
                NextState::MetadataChanged => unreachable!(),
                NextState::Paused => unreachable!(),
                NextState::Finished => unreachable!(),
            }
            assert_eq!(count, 1);
        }
        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (0, NextState::Finished)
        );
    }
}
