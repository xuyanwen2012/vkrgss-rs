//! Mixer, decoding and MIDI synthesis: what the host does with audio commands.
//!
//! Side: host.
//!
//! The crate has three layers:
//!
//! 1. [`decode`]: a [`Source`] yields interleaved stereo `f32` frames at a requested sample
//!    rate, from Ogg Vorbis, WAV or MP3 data through `symphonia` or from a Standard MIDI file
//!    through `rustysynth` and a SoundFont.
//! 2. [`Mixer`]: the four RGSS channels. It applies audio commands and renders into a buffer,
//!    without a thread or a device.
//! 3. [`Output`]: the default `cpal` device with a mixer behind its callback.

#![forbid(unsafe_code)]

pub mod decode;
mod midi;

pub use decode::{DecodeError, Source};
pub use midi::{SYSTEM_SOUNDFONTS, SoundFonts};
