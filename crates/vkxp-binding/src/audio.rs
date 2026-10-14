//! The Ruby module `Audio`: each function finds and reads the sound file and sends the host
//! an audio command.

use vkxp_proto::{AudioChannel, AudioCommand, GuestToHost};
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raised, Ruby};
use vkxp_vfs::{FileKind, Runtime};

use crate::error;
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

/// Sends the stop command of a channel.
fn stop_channel(guest: &mut Guest, channel: AudioChannel) {
    if let Some(remembered) = remembered(guest, channel) {
        *remembered = None;
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
    Guest::with(|guest| {
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

fn stop(channel: AudioChannel) -> HostResult {
    Guest::with(|guest| stop_channel(guest, channel));
    Ok(ffi::Qnil)
}

/// `fade(time)` on a channel, with the time in milliseconds.
fn fade(ruby: &Ruby, channel: AudioChannel, args: &[VALUE]) -> HostResult {
    let duration_ms = rb::int(ruby, args[0])?.max(0).unsigned_abs();
    Guest::with(|guest| {
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

fn bgm_stop(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    stop(AudioChannel::Bgm)
}

fn bgm_fade(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    fade(ruby, AudioChannel::Bgm, args)
}

fn bgs_play(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    play(ruby, AudioChannel::Bgs, args)
}

fn bgs_stop(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    stop(AudioChannel::Bgs)
}

fn bgs_fade(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    fade(ruby, AudioChannel::Bgs, args)
}

fn me_play(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    play(ruby, AudioChannel::Me, args)
}

fn me_stop(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    stop(AudioChannel::Me)
}

fn me_fade(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    fade(ruby, AudioChannel::Me, args)
}

fn se_play(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    play(ruby, AudioChannel::Se, args)
}

fn se_stop(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    stop(AudioChannel::Se)
}

/// Defines `Audio`.
pub(crate) fn define(ruby: &Ruby) -> Result<(), Raised> {
    let module = ruby.define_module(c"Audio")?;
    rb::class_method(ruby, module, c"bgm_play", -1, bgm_play)?;
    rb::class_method(ruby, module, c"bgm_stop", 0, bgm_stop)?;
    rb::class_method(ruby, module, c"bgm_fade", 1, bgm_fade)?;
    rb::class_method(ruby, module, c"bgs_play", -1, bgs_play)?;
    rb::class_method(ruby, module, c"bgs_stop", 0, bgs_stop)?;
    rb::class_method(ruby, module, c"bgs_fade", 1, bgs_fade)?;
    rb::class_method(ruby, module, c"me_play", -1, me_play)?;
    rb::class_method(ruby, module, c"me_stop", 0, me_stop)?;
    rb::class_method(ruby, module, c"me_fade", 1, me_fade)?;
    rb::class_method(ruby, module, c"se_play", -1, se_play)?;
    rb::class_method(ruby, module, c"se_stop", 0, se_stop)?;
    if Guest::with(|guest| guest.runtime.run.runtime) == Runtime::Rgss3 {
        rb::class_method(ruby, module, c"__reset__", 0, reset)?;
    }
    Ok(())
}

/// `Audio.__reset__` of RGSS3: stops the four channels.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301, `ace-v1-api-REPORT.md` section 1.5.
fn reset(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Guest::with(stop_all);
    Ok(ffi::Qnil)
}
