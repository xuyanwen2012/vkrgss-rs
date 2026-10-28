//! The Ruby module `Audio`: each function finds and reads the sound file and sends the host
//! an audio command.
//!
//! In a game of RGSS3 the module has the functions and the limits of that runtime, and
//! `Audio.bgm_pos` and `Audio.bgs_pos` report the position of a model of what the music and
//! the background sound play ([`vkrgss_core::sound`]), stepped by the frames the guest shows.
//! The host is never asked: its audio runs on a clock of its own, or on none.
//!
//! Provenance: oracle measurement 2026-10-13, RGSS301 version 3.0.1.1 and RGSS300 version
//! 3.0.0.1, `ace-v1-api-REPORT.md` sections 6.1 to 6.4.

use vkrgss_core::sound::{Position, SoundFormat};
use vkrgss_proto::{AudioChannel, AudioCommand, GuestToHost};
use vkrgss_ruby_sys::ffi::{self, VALUE};
use vkrgss_ruby_sys::{HostResult, Raised, Ruby};
use vkrgss_vfs::{FileKind, Runtime};

use crate::error;
use crate::graphics;
use crate::guest::Guest;
use crate::rb;
use crate::runtime::Playing;

/// The description a looping channel remembers, or `None` for the channels that do not.
fn remembered(guest: &mut Guest, channel: AudioChannel) -> Option<&mut Option<Playing>> {
    match channel {
        AudioChannel::Bgm => Some(&mut guest.runtime.music),
        AudioChannel::Bgs => Some(&mut guest.runtime.background_sound),
        AudioChannel::Me | AudioChannel::Se => None,
    }
}

/// The model of what a looping channel of RGSS3 plays.
#[derive(Debug, Clone, Default)]
struct Track {
    /// The format of the file, if the probe knows it.
    format: Option<SoundFormat>,
    /// The position, for a file whose format is known.
    position: Option<Position>,
    /// Whether frames step the position: not after a stop, which freezes it.
    running: bool,
}

impl Track {
    fn step(&mut self, frame_rate: u32) {
        if self.running
            && let Some(position) = &mut self.position
        {
            position.step(frame_rate);
        }
    }

    /// The position in bytes of 16-bit PCM: 0 before the first play and for a sound whose
    /// format is not known.
    fn bytes(&self) -> u64 {
        self.position.as_ref().map_or(0, Position::bytes)
    }
}

/// What the audio of RGSS3 keeps of the music and the background sound for their positions,
/// and the frames for which a music effect still holds the music.
#[derive(Debug, Clone, Default)]
pub(crate) struct State {
    music: Track,
    background_sound: Track,
    held: u64,
}

impl State {
    fn track(&mut self, channel: AudioChannel) -> Option<&mut Track> {
        match channel {
            AudioChannel::Bgm => Some(&mut self.music),
            AudioChannel::Bgs => Some(&mut self.background_sound),
            AudioChannel::Me | AudioChannel::Se => None,
        }
    }

    /// Steps the positions by a frame shown at `frame_rate` frames per second. The music
    /// stands still while an effect holds it.
    pub(crate) fn frame_shown(&mut self, frame_rate: u32) {
        if self.held > 0 {
            self.held -= 1;
        } else {
            self.music.step(frame_rate);
        }
        self.background_sound.step(frame_rate);
    }
}

/// Sends the stop command of a channel. A stop freezes the position of a looping channel and
/// ends the hold of a music effect.
fn stop_channel(guest: &mut Guest, channel: AudioChannel) {
    if let Some(remembered) = remembered(guest, channel) {
        *remembered = None;
    }
    match guest.audio.track(channel) {
        Some(track) => track.running = false,
        None if channel == AudioChannel::Me => guest.audio.held = 0,
        None => {}
    }
    guest.send(GuestToHost::Audio(AudioCommand::Stop { channel }));
}

/// Stops the four channels.
pub(crate) fn stop_all(guest: &mut Guest) {
    for channel in [
        AudioChannel::Bgm,
        AudioChannel::Bgs,
        AudioChannel::Me,
        AudioChannel::Se,
    ] {
        stop_channel(guest, channel);
    }
}

/// `play(filename, volume = 100, pitch = 100)` on a channel. Music and background sound
/// that repeat the description of what their channel plays keep playing: nothing is sent.
fn play(ruby: &Ruby, channel: AudioChannel, args: &[VALUE]) -> HostResult {
    rb::check_count(ruby, args, 1, 3)?;
    let name = rb::text(ruby, args[0])?;
    let volume = match args.get(1) {
        Some(&volume) => rb::int(ruby, volume)?.clamp(0, 100) as u8,
        None => 100,
    };
    let pitch = match args.get(2) {
        Some(&pitch) => rb::int(ruby, pitch)?.clamp(50, 150) as u16,
        None => 100,
    };
    let description = Playing {
        name,
        volume,
        pitch,
    };
    Guest::with(ruby, |guest| {
        if remembered(guest, channel).is_some_and(|playing| playing.as_ref() == Some(&description))
        {
            return Ok(());
        }
        let data = guest.files.read(FileKind::Audio, &description.name)?;
        guest.send(GuestToHost::Audio(AudioCommand::Play {
            channel,
            data,
            volume,
            pitch,
            start: 0,
            music_return_ms: 0,
        }));
        if let Some(remembered) = remembered(guest, channel) {
            *remembered = Some(description);
        }
        Ok(())
    })
    .map_err(|error: error::Error| error::to_raise(ruby, error))?;
    Ok(ffi::Qnil)
}

fn stop(ruby: &Ruby, channel: AudioChannel) -> HostResult {
    Guest::with(ruby, |guest| stop_channel(guest, channel));
    Ok(ffi::Qnil)
}

/// `fade(time)` on a channel, with the time in milliseconds.
fn fade(ruby: &Ruby, channel: AudioChannel, args: &[VALUE]) -> HostResult {
    let duration_ms = rb::int(ruby, args[0])?.max(0).unsigned_abs();
    Guest::with(ruby, |guest| {
        if let Some(remembered) = remembered(guest, channel) {
            *remembered = None;
        }
        guest.send(GuestToHost::Audio(AudioCommand::Fade {
            channel,
            duration_ms,
        }));
    });
    Ok(ffi::Qnil)
}

fn bgm_play(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    play(ruby, AudioChannel::Bgm, args)
}

fn bgm_stop(ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    stop(ruby, AudioChannel::Bgm)
}

fn bgm_fade(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    fade(ruby, AudioChannel::Bgm, args)
}

fn bgs_play(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    play(ruby, AudioChannel::Bgs, args)
}

fn bgs_stop(ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    stop(ruby, AudioChannel::Bgs)
}

fn bgs_fade(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    fade(ruby, AudioChannel::Bgs, args)
}

fn me_play(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    play(ruby, AudioChannel::Me, args)
}

fn me_stop(ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    stop(ruby, AudioChannel::Me)
}

fn me_fade(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    fade(ruby, AudioChannel::Me, args)
}

fn se_play(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    play(ruby, AudioChannel::Se, args)
}

fn se_stop(ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    stop(ruby, AudioChannel::Se)
}

// --- RGSS3 ------------------------------------------------------------------------------------

/// The highest rate the original asks of its audio back end, in Hz: a pitch that asks for
/// more plays at the normal rate. It is the documented limit of DirectSound.
const HIGHEST_RATE: i64 = 200_000;

/// The time in milliseconds over which the music returns to its volume after a music effect.
const MUSIC_RETURN_MS: u32 = 1000;

/// The kind of file the audio of a game of `runtime` is looked up as.
pub(crate) fn file_kind(runtime: Runtime) -> FileKind {
    match runtime {
        Runtime::Rgss1 | Runtime::Rgss2 => FileKind::Audio,
        Runtime::Rgss3 => FileKind::AudioRgss3,
    }
}

/// The volume of a play command of RGSS3: a volume below 0 is full volume.
fn volume_rgss3(volume: i32) -> u8 {
    match volume {
        1..=99 => volume as u8,
        0 => 0,
        _ => 100,
    }
}

/// The pitch of a play command of RGSS3: the normal rate for a pitch below 1 and for one
/// that asks for more than [`HIGHEST_RATE`] of a file whose rate is known.
fn pitch_rgss3(pitch: i32, format: Option<&SoundFormat>) -> u16 {
    let too_fast =
        format.is_some_and(|format| i64::from(format.rate) * i64::from(pitch) / 100 > HIGHEST_RATE);
    if pitch < 1 || too_fast {
        100
    } else {
        pitch.min(i32::from(u16::MAX)) as u16
    }
}

/// `play(filename, volume = 100, pitch = 100, pos = 0)` of RGSS3, `pos` on the looping
/// channels only. Music and background sound of the name and the pitch of what their channel
/// plays go on where they are, whatever `pos` says; a volume of their own is sent as a volume
/// command.
fn play_rgss3(ruby: &Ruby, channel: AudioChannel, args: &[VALUE]) -> HostResult {
    let looping = matches!(channel, AudioChannel::Bgm | AudioChannel::Bgs);
    rb::check_count_rgss3(ruby, args, 1, if looping { 4 } else { 3 })?;
    let name = rb::text(ruby, args[0])?;
    let volume = match args.get(1) {
        Some(&volume) => volume_rgss3(rb::int(ruby, volume)?),
        None => 100,
    };
    let pitch = match args.get(2) {
        Some(&pitch) => rb::int(ruby, pitch)?,
        None => 100,
    };
    let pos = match args.get(3) {
        Some(&pos) if !ffi::nil_p(pos) => u64::try_from(rb::int(ruby, pos)?).unwrap_or(0),
        _ => 0,
    };
    Guest::with(ruby, |guest| {
        let format = guest.audio.track(channel).and_then(|track| track.format);
        if let Some(Some(playing)) = remembered(guest, channel)
            && playing.name == name
            && playing.pitch == pitch_rgss3(pitch, format.as_ref())
        {
            if playing.volume != volume {
                playing.volume = volume;
                guest.send(GuestToHost::Audio(AudioCommand::Volume { channel, volume }));
            }
            return Ok(());
        }
        let data = guest.files.read(FileKind::AudioRgss3, &name)?;
        let format = SoundFormat::probe(&data);
        let pitch = pitch_rgss3(pitch, format.as_ref());
        let start = format.map_or(0, |format| pos / format.frame_bytes());
        guest.send(GuestToHost::Audio(AudioCommand::Play {
            channel,
            data,
            volume,
            pitch,
            start,
            music_return_ms: if channel == AudioChannel::Me {
                MUSIC_RETURN_MS
            } else {
                0
            },
        }));
        if let Some(remembered) = remembered(guest, channel) {
            *remembered = Some(Playing {
                name,
                volume,
                pitch,
            });
        }
        if let Some(track) = guest.audio.track(channel) {
            *track = Track {
                format,
                position: format.map(|format| Position::start(&format, pos, pitch)),
                running: true,
            };
        } else if channel == AudioChannel::Me {
            let frame_rate = guest.scene.frame_rate().get().into();
            guest.audio.held = format.map_or(0, |format| format.frames_shown(pitch, frame_rate));
        }
        Ok(())
    })
    .map_err(|error: error::Error| error::to_raise(ruby, error))?;
    Ok(ffi::Qnil)
}

/// The position of the model of a looping channel, an Integer.
fn position(ruby: &Ruby, channel: AudioChannel) -> HostResult {
    let bytes = Guest::with(ruby, |guest| {
        guest.audio.track(channel).map_or(0, |track| track.bytes())
    });
    // A count of bytes does not reach 2^63.
    let bytes = i64::try_from(bytes).unwrap_or(i64::MAX);
    graphics::integer_new(ruby, bytes)
}

fn bgm_play_rgss3(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    play_rgss3(ruby, AudioChannel::Bgm, args)
}

fn bgs_play_rgss3(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    play_rgss3(ruby, AudioChannel::Bgs, args)
}

fn me_play_rgss3(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    play_rgss3(ruby, AudioChannel::Me, args)
}

fn se_play_rgss3(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    play_rgss3(ruby, AudioChannel::Se, args)
}

fn bgm_pos(ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    position(ruby, AudioChannel::Bgm)
}

fn bgs_pos(ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    position(ruby, AudioChannel::Bgs)
}

/// `Audio.setup_midi`: the original prepares its MIDI synthesiser ahead of the first MIDI
/// file. The mixer of vkrgss needs no such step.
fn setup_midi(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(ffi::Qnil)
}

/// Defines `Audio`, with the functions of the runtime of the game.
pub(crate) fn define(ruby: &Ruby) -> Result<(), Raised> {
    let module = ruby.define_module(c"Audio")?;
    let rgss3 = Guest::with(ruby, |guest| guest.runtime.run.runtime) == Runtime::Rgss3;
    if rgss3 {
        rb::class_method(ruby, module, c"bgm_play", -1, bgm_play_rgss3)?;
    } else {
        rb::class_method(ruby, module, c"bgm_play", -1, bgm_play)?;
    }
    rb::class_method(ruby, module, c"bgm_stop", 0, bgm_stop)?;
    rb::class_method(ruby, module, c"bgm_fade", 1, bgm_fade)?;
    if rgss3 {
        rb::class_method(ruby, module, c"bgs_play", -1, bgs_play_rgss3)?;
    } else {
        rb::class_method(ruby, module, c"bgs_play", -1, bgs_play)?;
    }
    rb::class_method(ruby, module, c"bgs_stop", 0, bgs_stop)?;
    rb::class_method(ruby, module, c"bgs_fade", 1, bgs_fade)?;
    if rgss3 {
        rb::class_method(ruby, module, c"me_play", -1, me_play_rgss3)?;
    } else {
        rb::class_method(ruby, module, c"me_play", -1, me_play)?;
    }
    rb::class_method(ruby, module, c"me_stop", 0, me_stop)?;
    rb::class_method(ruby, module, c"me_fade", 1, me_fade)?;
    if rgss3 {
        rb::class_method(ruby, module, c"se_play", -1, se_play_rgss3)?;
    } else {
        rb::class_method(ruby, module, c"se_play", -1, se_play)?;
    }
    rb::class_method(ruby, module, c"se_stop", 0, se_stop)?;
    if rgss3 {
        rb::class_method(ruby, module, c"bgm_pos", 0, bgm_pos)?;
        rb::class_method(ruby, module, c"bgs_pos", 0, bgs_pos)?;
        rb::class_method(ruby, module, c"setup_midi", 0, setup_midi)?;
        rb::class_method(ruby, module, c"__reset__", 0, reset)?;
    }
    Ok(())
}

/// `Audio.__reset__` of RGSS3: stops the four channels.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301, `ace-v1-api-REPORT.md` section 1.5.
fn reset(ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Guest::with(ruby, stop_all);
    Ok(ffi::Qnil)
}
