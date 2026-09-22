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
    sample_rate: usize,
    channel_count: usize,
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
        let sample_rate = first_frame.sample_rate as usize;
        let channel_count = first_frame.num_channels as usize;

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
    fn channel_count(&self) -> usize {
        self.channel_count
    }

    fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    fn fill_next_frames(&mut self, buffer: &mut [f32]) -> Result<(usize, NextState), RawedioError> {
        for (i, dst) in buffer.iter_mut().enumerate() {
            match self.next_sample() {
                Ok(NextSample::Sample(s)) => *dst = s,
                Ok(NextSample::MetadataChanged) => return Ok((i, NextState::MetadataChanged)),
                Ok(NextSample::WouldBlock) => return Ok((i, NextState::WouldBlock)),
                Ok(NextSample::Paused) => return Ok((i, NextState::Paused)),
                Ok(NextSample::Finished) => return Ok((i, NextState::Finished)),
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
                QoaItem::Sample(s) => return Ok(NextSample::Sample(s as f32 / 32_768.0)),
                QoaItem::FrameHeader(f) => {
                    if f.num_channels as usize != self.channel_count
                        || f.sample_rate as usize != self.sample_rate
                    {
                        self.channel_count = f.num_channels as usize;
                        self.sample_rate = f.sample_rate as usize;
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
    use crate::{assert_float_all_ulp_eq, NextState, Sound};

    const SINE_WAVE_FILE: &[u8] = include_bytes!("data/audiocheck.net_sin_1000Hz_0dBFS_0.1s.qoa");

    #[test]
    fn samples_of_test_file() {
        let mut buffer = [0.0];
        let mut decoder = QoaDecoder::new(std::io::Cursor::new(SINE_WAVE_FILE)).unwrap();

        assert_eq!(decoder.sample_rate(), 44100);
        assert_eq!(decoder.channel_count(), 1);

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [0.012_878_418]); // 1

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [0.145_843_5]); // 2

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [0.271_179_2]); // 3

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [0.422_180_18]); // 4

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [0.524_078_37]); // 5

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [0.657_318_1]); // 6

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [0.744_720_46]); // 7

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [0.838_684_1]); // 8

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [0.891_113_3]); // 9

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [0.954_284_67]); // 10

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [0.975_830_1]); // 11

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [0.999_969_5]); // 12

        assert_eq!(
            decoder.fill_next_frames(&mut buffer).unwrap(),
            (1, NextState::Playing)
        );
        assert_float_all_ulp_eq!(buffer, [0.982_147_2]); // 13

        for _i in 0..4398 {
            let (count, sample) = decoder.fill_next_frames(&mut buffer).unwrap();
            match sample {
                NextState::Playing => {}
                NextState::MetadataChanged => unreachable!(),
                NextState::WouldBlock => unreachable!(),
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
