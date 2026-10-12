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

/// A MIDI file played by a synthesiser that runs at the requested sample rate. One that
/// loops never ends: at the end of the track it continues at the track's controller 111,
/// the loop marker of RPG Maker, or at the beginning of a track without one.
pub(crate) struct MidiSource {
    file: Arc<MidiFile>,
    sequencer: MidiFileSequencer,
    looping: bool,
    left: Vec<f32>,
    right: Vec<f32>,
}

impl MidiSource {
    pub(crate) fn new(
        mut data: &[u8],
        sample_rate: u32,
        soundfont: Arc<SoundFont>,
        looping: bool,
    ) -> Result<MidiSource, DecodeError> {
        let file = MidiFile::new_with_loop_type(&mut data, MidiFileLoopType::RpgMaker)
            .map_err(|error| DecodeError::Unsupported(format!("MIDI: {error}")))?;
        let file = Arc::new(file);
        let settings = SynthesizerSettings::new(sample_rate as i32);
        let synthesizer = Synthesizer::new(&soundfont, &settings)
            .map_err(|error| DecodeError::Unsupported(format!("MIDI: {error}")))?;
        let mut sequencer = MidiFileSequencer::new(synthesizer);
        sequencer.play(&file, looping);
        Ok(MidiSource {
            file,
            sequencer,
            looping,
            left: vec![0.0; BLOCK],
            right: vec![0.0; BLOCK],
        })
    }
}

impl Source for MidiSource {
    fn read(&mut self, out: &mut [f32]) -> usize {
        let mut written = 0;
        for block in out.chunks_mut(BLOCK * 2) {
            let frames = block.len() / 2;
            if frames == 0 || self.sequencer.end_of_sequence() {
                break;
            }
            self.sequencer
                .render(&mut self.left[..frames], &mut self.right[..frames]);
            for (index, frame) in block.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                frame[0] = self.left[index];
                frame[1] = self.right[index];
            }
            written += frames;
        }
        written
    }

    fn rewind(&mut self) {
        self.sequencer.play(&self.file, self.looping);
    }
}
