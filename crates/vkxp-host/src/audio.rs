//! Where the session sends the guest's audio commands.

use std::io::Write;
use std::path::PathBuf;

use vkxp_audio::{Output, OutputOptions, SoundFonts};
use vkxp_proto::AudioCommand;

/// The audio side as the session sees it.
pub trait AudioSink {
    /// Takes a command. It must not wait: the session calls it between frames.
    fn command(&mut self, command: AudioCommand);

    /// Remembers under `slot`, in place of what it held, what the music and the background
    /// sound play and where they are. It must not wait either.
    fn remember(&mut self, slot: u8);

    /// Arms the music and the background sound with what `slot` holds: the next play
    /// command of each starts where the slot remembers it, if it plays the remembered file.
    /// It must not wait either.
    fn recall(&mut self, slot: u8);
}

/// A sink that plays nothing and reports nothing, for headless runs.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullSink;

impl AudioSink for NullSink {
    fn command(&mut self, _command: AudioCommand) {}

    fn remember(&mut self, _slot: u8) {}

    fn recall(&mut self, _slot: u8) {}
}

impl AudioSink for Output {
    fn command(&mut self, command: AudioCommand) {
        Output::command(self, command);
    }

    fn remember(&mut self, slot: u8) {
        Output::remember(self, slot);
    }

    fn recall(&mut self, slot: u8) {
        Output::recall(self, slot);
    }
}

/// How the audio of a windowed run is set up.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioOptions {
    /// The SoundFont for MIDI. Without one the usual system locations are searched.
    pub soundfont: Option<PathBuf>,
}

impl AudioOptions {
    /// Opens the default output device. When that fails, the failure is reported once and
    /// the sink ignores what it is sent. Reports are written to `errors`.
    pub(crate) fn open(&self, errors: Box<dyn Write + Send>) -> Box<dyn AudioSink> {
        Box::new(Output::open(OutputOptions {
            soundfonts: SoundFonts::configured(self.soundfont.clone()),
            errors,
            ..OutputOptions::default()
        }))
    }
}
