//! Decoding: [`Source`] values that yield interleaved stereo `f32` frames at a requested
//! sample rate.
//!
//! The format is recognised from the bytes: a Standard MIDI file by its `MThd` header,
//! everything else by `symphonia`'s probe (Ogg Vorbis, WAV, MP3).

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::midi::{self, SoundFonts};

/// Why a sound cannot be played.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The bytes are no supported format, or are too damaged to start decoding.
    #[error("the audio data is not in a supported format ({0})")]
    Unsupported(String),
    /// The bytes are a MIDI file and none of the SoundFont locations holds a file.
    #[error("no SoundFont was found for MIDI music (looked for {})", list(.searched))]
    NoSoundFont { searched: Vec<PathBuf> },
    /// The bytes are a MIDI file and the SoundFont file cannot be used.
    #[error("the SoundFont {} cannot be loaded: {reason}", .path.display())]
    SoundFont { path: PathBuf, reason: String },
}

fn list(paths: &[PathBuf]) -> String {
    let names: Vec<String> = paths
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    names.join(", ")
}

/// A sound being played: a stream of stereo frames at the sample rate it was opened for.
pub trait Source: Send {
    /// Writes the next frames to `out`, interleaved left and right, and returns how many
    /// frames were written. Fewer than `out.len() / 2` means the sound has ended.
    fn read(&mut self, out: &mut [f32]) -> usize;

    /// Starts again from the beginning.
    fn rewind(&mut self);
}

/// Opens `data` as a source of frames at `sample_rate`. `soundfonts` is consulted, and its
/// file loaded, only when the data is a MIDI file.
pub fn open(
    data: Arc<[u8]>,
    sample_rate: u32,
    soundfonts: &mut SoundFonts,
) -> Result<Box<dyn Source>, DecodeError> {
    if midi::is_midi(&data) {
        let source = midi::MidiSource::new(&data, sample_rate, soundfonts.get()?)?;
        Ok(Box::new(source))
    } else {
        Ok(Box::new(Sampled::new(data, sample_rate)?))
    }
}

/// The demuxer and decoder of one sampled file.
struct Stream {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track: u32,
}

impl Stream {
    fn open(data: Arc<[u8]>) -> Result<Stream, DecodeError> {
        let unsupported = |error: SymphoniaError| DecodeError::Unsupported(error.to_string());
        let stream = MediaSourceStream::new(Box::new(Cursor::new(data)), Default::default());
        let format_options = FormatOptions {
            // Leaves out the padding an MP3 encoder adds, so a loop has no gap.
            enable_gapless: true,
            ..FormatOptions::default()
        };
        let format = symphonia::default::get_probe()
            .format(
                &Hint::new(),
                stream,
                &format_options,
                &MetadataOptions::default(),
            )
            .map_err(unsupported)?
            .format;
        let track = format
            .tracks()
            .iter()
            .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| DecodeError::Unsupported("no audio track".to_owned()))?;
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(unsupported)?;
        Ok(Stream {
            track: track.id,
            format,
            decoder,
        })
    }

    /// Decodes the next block and appends it to `frames` as stereo. Returns the block's
    /// sample rate, or `None` at the end of the file. A damaged packet is skipped.
    fn next_block(
        &mut self,
        samples: &mut Option<SampleBuffer<f32>>,
        frames: &mut Vec<f32>,
    ) -> Option<u32> {
        loop {
            // Any error of the reader, the end of the file included, ends the sound.
            let packet = self.format.next_packet().ok()?;
            if packet.track_id() != self.track {
                continue;
            }
            let decoded = match self.decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(SymphoniaError::DecodeError(_) | SymphoniaError::IoError(_)) => continue,
                Err(_) => return None,
            };
            let spec = *decoded.spec();
            let channels = spec.channels.count();
            if decoded.frames() == 0 || channels == 0 {
                continue;
            }
            let buffer = match samples {
                Some(buffer) if buffer.capacity() >= decoded.capacity() * channels => buffer,
                _ => samples.insert(SampleBuffer::new(decoded.capacity() as u64, spec)),
            };
            buffer.copy_interleaved_ref(decoded);
            for frame in buffer.samples().chunks_exact(channels) {
                // Mono goes to both sides; of more than two channels the first two are kept.
                frames.push(frame[0]);
                frames.push(frame[channels.min(2) - 1]);
            }
            return Some(spec.rate);
        }
    }
}

/// A sampled file, decoded block by block and resampled by linear interpolation.
struct Sampled {
    data: Arc<[u8]>,
    sample_rate: u32,
    stream: Option<Stream>,
    samples: Option<SampleBuffer<f32>>,
    /// Decoded stereo frames not yet consumed.
    frames: Vec<f32>,
    /// The position of the next output frame in `frames`, in frames.
    position: f64,
    /// Input frames per output frame.
    step: f64,
}

impl Sampled {
    fn new(data: Arc<[u8]>, sample_rate: u32) -> Result<Sampled, DecodeError> {
        let stream = Stream::open(data.clone())?;
        let mut source = Sampled {
            data,
            sample_rate,
            stream: Some(stream),
            samples: None,
            frames: Vec::new(),
            position: 0.0,
            step: 1.0,
        };
        // The first block is decoded here, so that it is not decoded where the sound is mixed.
        source.refill();
        Ok(source)
    }

    /// Drops the frames before the position and appends the next block, if there is one.
    fn refill(&mut self) {
        let consumed = (self.position as usize).min(self.frames.len() / 2);
        self.frames.drain(..consumed * 2);
        self.position -= consumed as f64;
        let rate = self
            .stream
            .as_mut()
            .and_then(|stream| stream.next_block(&mut self.samples, &mut self.frames));
        match rate {
            Some(rate) => self.step = f64::from(rate) / f64::from(self.sample_rate),
            None => self.stream = None,
        }
    }
}

impl Source for Sampled {
    fn read(&mut self, out: &mut [f32]) -> usize {
        let mut written = 0;
        for frame in out.as_chunks_mut::<2>().0 {
            // Interpolation needs the frame at the position and the one after it.
            while self.stream.is_some() && self.position as usize + 1 >= self.frames.len() / 2 {
                self.refill();
            }
            let count = self.frames.len() / 2;
            let index = self.position as usize;
            if index >= count {
                break;
            }
            // The last frame of the file has no successor and is taken as it is.
            let next = (index + 1).min(count - 1);
            let weight = (self.position - index as f64) as f32;
            for (side, sample) in frame.iter_mut().enumerate() {
                let from = self.frames[index * 2 + side];
                let to = self.frames[next * 2 + side];
                *sample = from + (to - from) * weight;
            }
            self.position += self.step;
            written += 1;
        }
        written
    }

    fn rewind(&mut self) {
        // The file opened once, so it opens again.
        self.stream = Stream::open(self.data.clone()).ok();
        self.frames.clear();
        self.position = 0.0;
        self.refill();
    }
}
