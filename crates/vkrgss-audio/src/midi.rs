//! MIDI: Standard MIDI files rendered by `rustysynth` with a SoundFont.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustysynth::{
    MidiFile, MidiFileLoopType, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings,
};

use crate::decode::{DecodeError, Source};

/// Where a SoundFont is looked for when none is named, in this order.
pub const SYSTEM_SOUNDFONTS: [&str; 3] = [
    "/usr/share/soundfonts/default.sf2",
    "/usr/share/soundfonts/FluidR3_GM.sf2",
    "/usr/share/sounds/sf2/FluidR3_GM.sf2",
];

/// How many frames the synthesiser renders at a time.
const BLOCK: usize = 1024;

/// Whether `data` is a Standard MIDI file.
pub(crate) fn is_midi(data: &[u8]) -> bool {
    data.starts_with(b"MThd")
}

enum State {
    /// Nothing has asked for the SoundFont yet.
    Unloaded,
    Loaded(Arc<SoundFont>),
    Failed(DecodeError),
}

/// The SoundFont for MIDI: the first file that exists among its locations, loaded when a
/// MIDI file is first opened and kept from then on.
pub struct SoundFonts {
    locations: Vec<PathBuf>,
    state: State,
}

impl SoundFonts {
    /// The SoundFont is the first of `locations` that exists.
    pub fn new(locations: Vec<PathBuf>) -> SoundFonts {
        SoundFonts {
            locations,
            state: State::Unloaded,
        }
    }

    /// The SoundFont is `named` or, without a name, the first of [`SYSTEM_SOUNDFONTS`] that
    /// exists.
    pub fn configured(named: Option<PathBuf>) -> SoundFonts {
        SoundFonts::new(match named {
            Some(path) => vec![path],
            None => SYSTEM_SOUNDFONTS.iter().map(PathBuf::from).collect(),
        })
    }

    pub(crate) fn get(&mut self) -> Result<Arc<SoundFont>, DecodeError> {
        if let State::Unloaded = self.state {
            self.state = match self.locations.iter().find(|path| path.is_file()) {
                Some(path) => match load(path) {
                    Ok(soundfont) => State::Loaded(Arc::new(soundfont)),
                    Err(reason) => State::Failed(DecodeError::SoundFont {
                        path: path.clone(),
                        reason,
                    }),
                },
                None => State::Failed(DecodeError::NoSoundFont {
                    searched: self.locations.clone(),
                }),
            };
        }
        match &self.state {
            State::Loaded(soundfont) => Ok(soundfont.clone()),
            State::Failed(error) => Err(error.clone()),
            State::Unloaded => unreachable!("the SoundFont state was settled above"),
        }
    }
}

fn load(path: &Path) -> Result<SoundFont, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    SoundFont::new(&mut BufReader::new(file)).map_err(|error| error.to_string())
}

/// The sample rates at which the synthesiser of `rustysynth` runs.
const SYNTHESISER_RATES: std::ops::RangeInclusive<u32> = 16_000..=192_000;

/// A MIDI file played by a synthesiser that runs at the requested sample rate, or at the
/// nearest rate it can run at, its frames then resampled to the requested rate by linear
/// interpolation. One that loops never ends: at the end of the track it continues at the
/// track's controller 111, the loop marker of RPG Maker, or at the beginning of a track
/// without one.
pub(crate) struct MidiSource {
    file: Arc<MidiFile>,
    sequencer: MidiFileSequencer,
    looping: bool,
    /// Frames of the synthesiser per frame given.
    step: f64,
    /// Frames of the synthesiser, interleaved, from the one before the next frame given on.
    frames: Vec<f32>,
    /// Where the next frame given is in `frames`, in frames.
    at: f64,
    /// How many frames of the synthesiser lay before `frames`, since the start of the track.
    before: u64,
    left: Vec<f32>,
    right: Vec<f32>,
}

impl MidiSource {
    pub(crate) fn new(
        data: &[u8],
        sample_rate: u32,
        soundfont: Arc<SoundFont>,
        looping: bool,
    ) -> Result<MidiSource, DecodeError> {
        let file = MidiFile::new_with_loop_type(&mut &data[..], MidiFileLoopType::RpgMaker)
            .map_err(|error| DecodeError::Unsupported(format!("MIDI: {error}")))?;
        let file = Arc::new(file);
        let rate = sample_rate.clamp(*SYNTHESISER_RATES.start(), *SYNTHESISER_RATES.end());
        let settings = SynthesizerSettings::new(rate as i32);
        let synthesizer = Synthesizer::new(&soundfont, &settings)
            .map_err(|error| DecodeError::Unsupported(format!("MIDI: {error}")))?;
        let mut sequencer = MidiFileSequencer::new(synthesizer);
        sequencer.play(&file, looping);
        Ok(MidiSource {
            file,
            sequencer,
            looping,
            step: f64::from(rate) / f64::from(sample_rate),
            frames: Vec::new(),
            at: 0.0,
            before: 0,
            left: vec![0.0; BLOCK],
            right: vec![0.0; BLOCK],
        })
    }

    /// Has the synthesiser render its next `frames` frames, at most [`BLOCK`], into `left`
    /// and `right`.
    fn synthesise(&mut self, frames: usize) {
        self.sequencer
            .render(&mut self.left[..frames], &mut self.right[..frames]);
    }

    /// Drops the frames before the one at `at` and appends the next block of the
    /// synthesiser. Returns `false` when the track has ended.
    fn refill(&mut self) -> bool {
        let consumed = (self.at as usize).min(self.frames.len() / 2);
        self.frames.drain(..consumed * 2);
        self.at -= consumed as f64;
        self.before += consumed as u64;
        if self.sequencer.end_of_sequence() {
            return false;
        }
        self.synthesise(BLOCK);
        let pairs = self.left.iter().zip(&self.right);
        self.frames
            .extend(pairs.flat_map(|(left, right)| [*left, *right]));
        true
    }
}

impl Source for MidiSource {
    fn read(&mut self, out: &mut [f32]) -> usize {
        let mut written = 0;
        for frame in out.as_chunks_mut::<2>().0 {
            // Interpolation needs the frame at the position and the one after it.
            while self.at as usize + 1 >= self.frames.len() / 2 && self.refill() {}
            let count = self.frames.len() / 2;
            let index = self.at as usize;
            if index >= count {
                break;
            }
            // The last frame of a track that ended has no successor and is taken as it is.
            let next = (index + 1).min(count - 1);
            let weight = (self.at - index as f64) as f32;
            for (side, sample) in frame.iter_mut().enumerate() {
                let from = self.frames[index * 2 + side];
                let to = self.frames[next * 2 + side];
                *sample = from + (to - from) * weight;
            }
            self.at += self.step;
            written += 1;
        }
        written
    }

    fn rewind(&mut self) {
        self.sequencer.play(&self.file, self.looping);
        self.frames.clear();
        self.at = 0.0;
        self.before = 0;
    }

    /// The frames of the synthesiser before the next frame given.
    fn position(&self) -> u64 {
        self.before + self.at as u64
    }

    /// Renders up to `position`, in frames of the synthesiser, as [`position`] counts.
    ///
    /// [`position`]: Source::position
    fn start_from(&mut self, position: u64) {
        let mut left = position;
        while left > 0 && !self.sequencer.end_of_sequence() {
            let frames = left.min(BLOCK as u64) as usize;
            self.synthesise(frames);
            left -= frames as u64;
        }
        self.before = position - left;
    }
}
