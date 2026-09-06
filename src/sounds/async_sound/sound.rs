
use symphonia::core::audio::AmbisonicBFormat::Z;

use crate::{NextSample, NextSampleBuffer, RawedioError, Sound, sounds::async_sound::controller::AsyncSoundId};

pub const ASYNC_SAMPLE_CHUNK_CAP: usize = 2048; // ~4KiB

pub struct AsyncSampleChunk {
    pub response: NextSampleBuffer,
    pub channel_count: u16,
    pub sample_rate: u32,
    pub max_samples: usize,
    pub data: [i16; ASYNC_SAMPLE_CHUNK_CAP],
}

impl AsyncSampleChunk {
    pub fn written_samples(&self) -> usize {
        match self.response {
            NextSampleBuffer::Continue => self.max_samples,
            NextSampleBuffer::MetadataChanged(count)
            | NextSampleBuffer::Paused(count)
            | NextSampleBuffer::Finished(count) => count
        }
    }
}

pub struct AsyncSound {
    id: AsyncSoundId,

    rx: rtrb::Consumer<AsyncSampleChunk>,
    channel_count: u16,
    sample_rate: u32,

    chunks: Vec<AsyncSampleChunk>,
    chunk_offset: usize,
}

impl AsyncSound {

    pub(super) fn new(
        id: AsyncSoundId,
        rx: rtrb::Consumer<AsyncSampleChunk>,
        channel_count: u16,
        sample_rate: u32,
    ) -> Self {
        Self{
            id,

            rx,
            channel_count,
            sample_rate,
            
            chunks: Vec::with_capacity(5),
            chunk_offset: 0,
        }
    }

}

impl AsyncSound {

    fn try_pull(&mut self) {
        let to_pull = self.rx.slots().min(5_usize.saturating_sub(self.chunks.len()));
        if to_pull > 0 {
            if let Ok(chunks) = self.rx.read_chunk(to_pull){
                for chunk in chunks {
                    self.chunks.push(chunk);
                }
            }
        }
    }

    fn next_chunk(&mut self) -> Result<&AsyncSampleChunk, bool> {
        match self.chunks.first() {
            Some(chunk) => Ok(chunk),
            None        => Err(self.rx.is_abandoned())
        }
    }
 
    fn try_sample_chunk(&mut self, dst: &mut [i16]) -> Result<usize, NextSampleBuffer> {

        let src = match self.chunks.first() {
            Some(src) => src,
            None => return Err(if self.rx.is_abandoned() {
                NextSampleBuffer::Finished(0) 
            } else { 
                NextSampleBuffer::Paused(0)
            })
        };

        let from = self.chunk_offset;

        let remaining_dst = dst.len();
        let remaining_src = src.written_samples() - from;
        let sample_count = usize::min(remaining_dst, remaining_src);

        let to = from + sample_count;
        dst.copy_from_slice(&src.data[from..to]);

        // If we're not at the end of the chunk,
        // then the status is continue.
        if to != src.data.len() {
            return Ok(sample_count);
        }
        
        // Adjust response count based on where we read from
        let response = match src.response {
            NextSampleBuffer::Continue => {
                Ok(sample_count)
            },
            NextSampleBuffer::MetadataChanged(count) => {
                Err(NextSampleBuffer::MetadataChanged(count - from))
            },
            NextSampleBuffer::Paused(count) => {
                Err(NextSampleBuffer::Paused(count - from))
            },
            NextSampleBuffer::Finished(count) => {
                Err(NextSampleBuffer::Finished(count - from))
            },
        };

        // Remove chunk and continue
        self.chunks.pop();
        self.chunk_offset = 0;
        response
    }

    fn try_sample(&mut self) -> Option<NextSample> {
        let samples_in_chunk = self.chunk_offset;
        let chunk = match self.next_chunk() {
            Ok(chunk) => chunk,
            Err(abandoned) => return Some(
                if abandoned {
                    NextSample::Finished 
                } else { 
                    NextSample::Paused 
                }
            )
        };

        if samples_in_chunk >= chunk.written_samples() {
            let response = chunk.response;
            self.chunks.pop();
            self.chunk_offset = 0;
            match response {
                NextSampleBuffer::Continue           => return None,
                NextSampleBuffer::MetadataChanged(_) => return Some(NextSample::MetadataChanged),
                NextSampleBuffer::Paused(_)          => return Some(NextSample::Paused),
                NextSampleBuffer::Finished(_)        => return Some(NextSample::Finished),
            }
        }

        let sample = NextSample::Sample(chunk.data[samples_in_chunk]);
        self.chunk_offset += 1;
        return Some(sample);
    }

}

impl Sound for AsyncSound {

    fn on_start_of_batch(&mut self) {
        self.try_pull();
    }

    fn channel_count(&self) -> u16 {
        self.channel_count
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn next_samples_for(&mut self, buffer: &mut [i16]) -> Result<NextSampleBuffer, RawedioError> {
        let mut read = 0;
        while read < buffer.len() {
            match self.try_sample_chunk(&mut buffer[read..]) {
                Ok(count) => read += count,
                Err(NextSampleBuffer::Continue) => unreachable!(),
                Err(NextSampleBuffer::MetadataChanged(count)) => {
                    read += count;
                    return Ok(NextSampleBuffer::MetadataChanged(read));
                },
                Err(NextSampleBuffer::Paused(count)) => {
                    read += count;
                    return Ok(NextSampleBuffer::Paused(read));
                },
                Err(NextSampleBuffer::Finished(count)) => {
                    read += count;
                    return Ok(NextSampleBuffer::Finished(read));
                },
            }
        }
        Ok(NextSampleBuffer::Continue)
    }

    fn next_sample(&mut self) -> Result<NextSample, RawedioError> {
        loop {
            if let Some(response) = self.try_sample() {
                return Ok(response);
            }
        }
    }

}