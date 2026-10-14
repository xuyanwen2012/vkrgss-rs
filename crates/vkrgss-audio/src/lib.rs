//! Mixer, decoding and MIDI synthesis: what the host does with audio commands.
//!
//! Side: host.
//!
//! The crate has three layers:
//!
//! 1. [`decode`]: a [`Source`] yields interleaved stereo `f32` frames at a requested sample
//!    rate, from Ogg Vorbis, WAV or MP3 data through `symphonia`, from a Standard MIDI file
//!    through `rustysynth` and a SoundFont, or from WMA data through the `ffmpeg` program.
//! 2. [`Mixer`]: the four RGSS channels. It applies audio commands and renders into a buffer,
//!    without a thread or a device.
//! 3. [`Output`]: the default `cpal` device with a mixer behind its callback.

#![forbid(unsafe_code)]

pub mod decode;
mod midi;
mod mixer;
mod output;
mod wma;

pub use decode::{DecodeError, Source};
pub use midi::{SYSTEM_SOUNDFONTS, SoundFonts};
pub use mixer::{Mixer, MixerOptions};
pub use output::{Output, OutputOptions, Render, Start};
