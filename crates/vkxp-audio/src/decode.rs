//! Decoding: [`Source`] values that yield interleaved stereo `f32` frames at a requested
//! sample rate.
//!
//! The format is recognised from the bytes: a Standard MIDI file by its `MThd` header,
//! everything else by `symphonia`'s probe (Ogg Vorbis, WAV, MP3).

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;

use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::midi::{self, SoundFonts};

/// Why a sound cannot be played.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The bytes are no supported format, or are so damaged that nothing of them decodes.
    #[error("the audio data is not in a supported format ({0})")]
    Unsupported(String),
    /// Part of the data does not decode. The sound is played without that part.
    #[error("the audio data is damaged ({0})")]
    Damaged(String),
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

/// Where a source reports damage it meets while it plays: [`DecodeError::Damaged`], at most
/// once for each source. It is called where the source is read, so it must not wait.
pub type Report = Arc<dyn Fn(&DecodeError) + Send + Sync>;

/// Opens `data` as a source of frames at `sample_rate`. `soundfonts` is consulted, and its
/// file loaded, only when the data is a MIDI file. Data of which nothing can be played is an
/// error; data of which a part cannot is played without it and told to `report`.
pub fn open(
    data: Arc<[u8]>,
    sample_rate: u32,
    soundfonts: &mut SoundFonts,
    report: &Report,
) -> Result<Box<dyn Source>, DecodeError> {
    if midi::is_midi(&data) {
        let source = midi::MidiSource::new(&data, sample_rate, soundfonts.get()?)?;
        Ok(Box::new(source))
    } else {
        Ok(Box::new(Sampled::new(data, sample_rate, report.clone())?))
    }
}

/// The demuxer and decoder of one sampled file.
struct Stream {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track: u32,
}

impl Stream {
    fn open(data: Arc<[u8]>) -> Result<Stream, DecodeError> {
        let unsupported = |error: SymphoniaError| DecodeError::Unsupported(error.to_string());
        let stream = MediaSourceStream::new(Box::new(Cursor::new(data)), Default::default());
        let format = symphonia::default::get_probe()
            .probe(
                &Hint::new(),
                stream,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(unsupported)?;
        let no_track = || DecodeError::Unsupported("no audio track".to_owned());
        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(no_track)?;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .ok_or_else(no_track)?;
        // Leaves out the padding an MP3 encoder adds, so a loop has no gap.
        let decoder_options = AudioDecoderOptions::default().gapless(true);
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &decoder_options)
            .map_err(unsupported)?;
        Ok(Stream {
            track: track.id,
            format,
            decoder,
        })
    }

    /// Decodes the next block and appends it to `frames` as stereo. Returns the block's
    /// sample rate, or `None` when the sound has ended. A packet that does not decode is
    /// skipped, and what the library said about the first of them is left in `damage`; so is
    /// the reason when the sound ends for anything but the end of its data.
    fn next_block(
        &mut self,
        samples: &mut Vec<f32>,
        frames: &mut Vec<f32>,
        damage: &mut Option<String>,
    ) -> Option<u32> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => return None,
                // This is how the library says that the data of a file that was cut short
                // has ended.
                Err(SymphoniaError::IoError(error))
                    if error.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return None;
                }
                Err(error) => {
                    damage.get_or_insert_with(|| error.to_string());
                    return None;
                }
            };
            if packet.track_id != self.track {
                continue;
            }
            let decoded = match self.decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(error @ (SymphoniaError::DecodeError(_) | SymphoniaError::IoError(_))) => {
                    damage.get_or_insert_with(|| error.to_string());
                    continue;
                }
                Err(error) => {
                    damage.get_or_insert_with(|| error.to_string());
                    return None;
                }
            };
            let rate = decoded.spec().rate();
            let channels = decoded.spec().channels().count();
            if decoded.frames() == 0 || channels == 0 {
                continue;
            }
            decoded.copy_to_vec_interleaved(samples);
            for frame in samples.chunks_exact(channels) {
                // Mono goes to both sides; of more than two channels the first two are kept.
                frames.push(frame[0]);
                frames.push(frame[channels.min(2) - 1]);
            }
            return Some(rate);
        }
    }
}

/// A sampled file, decoded block by block and resampled by linear interpolation.
struct Sampled {
    data: Arc<[u8]>,
    sample_rate: u32,
    stream: Option<Stream>,
    /// The last decoded block, interleaved as the file has it.
    samples: Vec<f32>,
    /// Decoded stereo frames not yet consumed.
    frames: Vec<f32>,
    /// The position of the next output frame in `frames`, in frames.
    position: f64,
    /// Input frames per output frame.
    step: f64,
    /// Where damage is reported, until it has been.
    report: Option<Report>,
}

impl Sampled {
    fn new(data: Arc<[u8]>, sample_rate: u32, report: Report) -> Result<Sampled, DecodeError> {
        let stream = Stream::open(data.clone())?;
        let mut source = Sampled {
            data,
            sample_rate,
            stream: Some(stream),
            samples: Vec::new(),
            frames: Vec::new(),
            position: 0.0,
            step: 1.0,
            report: Some(report),
        };
        // The first block is decoded here, so that it is not decoded where the sound is mixed.
        // Packets that do not decode are skipped on the way to it.
        if let Some(damage) = source.refill() {
            if source.frames.is_empty() {
                return Err(DecodeError::Unsupported(damage));
            }
            source.damaged(damage);
        }
        Ok(source)
    }

    /// Reports damage, the first time for this source.
    fn damaged(&mut self, damage: String) {
        if let Some(report) = self.report.take() {
            report(&DecodeError::Damaged(damage));
        }
    }

    /// Drops the frames before the position and appends the next block, if there is one.
    /// Returns what the library said if data did not decode on the way.
    fn refill(&mut self) -> Option<String> {
        let consumed = (self.position as usize).min(self.frames.len() / 2);
        self.frames.drain(..consumed * 2);
        self.position -= consumed as f64;
        let mut damage = None;
        let rate = self
            .stream
            .as_mut()
            .and_then(|stream| stream.next_block(&mut self.samples, &mut self.frames, &mut damage));
        match rate {
            Some(rate) => self.step = f64::from(rate) / f64::from(self.sample_rate),
            None => self.stream = None,
        }
        damage
    }
}

impl Source for Sampled {
    fn read(&mut self, out: &mut [f32]) -> usize {
        let mut written = 0;
        for frame in out.as_chunks_mut::<2>().0 {
            // Interpolation needs the frame at the position and the one after it.
            while self.stream.is_some() && self.position as usize + 1 >= self.frames.len() / 2 {
                if let Some(damage) = self.refill() {
                    self.damaged(damage);
                }
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
        if let Some(damage) = self.refill() {
            self.damaged(damage);
        }
    }
}
