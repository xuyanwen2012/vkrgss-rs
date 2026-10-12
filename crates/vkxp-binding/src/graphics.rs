//! The Ruby module `Graphics`.
//!
//! A frame is extracted from the guest's scene, sent to the host, and its tick awaited in
//! [`Guest::pump`]. What the host asked for meanwhile is raised once the guest is released.

use std::ffi::c_long;

use vkxp_compose_ref::Compositor;
use vkxp_proto::{BitmapDelta, BitmapId, GuestToHost, TransitionRule};
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raise, Raised, Ruby};
use vkxp_vfs::{FileKind, Runtime};

use crate::bitmap;
use crate::error::{self, Error};
use crate::guest::{self, Guest};
use crate::rb;
use crate::runtime::{self, Await};
use crate::stats::{Part, Span};

/// `Graphics._reset`, in Ruby because it goes through the objects of the scripts: every
/// sprite, viewport, window, plane, tilemap and bitmap is disposed by the `dispose` of its
/// RGSS class, whatever a subclass made of that method. A class that is not defined yet has
/// no objects.
const RESET: &str = "\
def Graphics._reset
  [:Sprite, :Viewport, :Window, :Plane, :Tilemap, :Bitmap].each do |name|
    next unless Object.const_defined?(name)
    rgss_class = Object.const_get(name)
    dispose = rgss_class.instance_method(:dispose)
    ObjectSpace.each_object(rgss_class) { |object| dispose.bind(object).call }
  end
  nil
end
";

/// Extracts one frame, sends it with `front` before its own bitmap deltas, and waits for its
/// tick. The tick count of the emulated `Win32API` functions advances by the frame.
fn frame(mut front: Vec<BitmapDelta>) {
    Guest::with(|guest| {
        let extraction = Span::enter(Part::Extraction);
        let mut packet = guest.scene.extract(&mut guest.bitmaps);
        if !front.is_empty() {
            front.append(&mut packet.bitmaps);
            packet.bitmaps = front;
        }
        guest.send(GuestToHost::Frame(packet));
        drop(extraction);
        guest.pump(Await::Tick);
        let frame_rate = guest.scene.frame_rate().get();
        guest.win32api.frame_shown(frame_rate.into());
    });
}

fn update(ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    frame(Vec::new());
    runtime::raise_pending(ruby)?;
    Ok(ffi::Qnil)
}

/// Shows `frames` frames as `Graphics.update` shows one, without calling the Ruby method of
/// that name. `before` runs ahead of each frame with its number, counted from 1. A request of
/// the host ends the frames and is raised.
fn frames(ruby: &Ruby, frames: i32, mut before: impl FnMut(i64)) -> Result<(), Raise> {
    for number in 1..=i64::from(frames) {
        before(number);
        frame(Vec::new());
        runtime::raise_pending(ruby)?;
    }
    Ok(())
}

/// `Graphics.wait(duration)` of RGSS2: that many frames, none for a duration below 1.
fn wait(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let duration = rb::int(ruby, args[0])?;
    frames(ruby, duration, |_| {})?;
    Ok(ffi::Qnil)
}

/// A fade of RGSS2 from the brightness as it is to `end`: frame k of `duration` shows the
/// scene at `b0 + (end - b0) * k / duration` in integer division, and the brightness is `end`
/// afterwards, also without a frame. A duration below 0 is taken as 0, which was not
/// measured.
///
/// Provenance: oracle measurement 2026-10-12, RGSS202E versions 2.0.0.1 and 2.0.2.1,
/// `vx-v0-REPORT.md` section 6.
fn fade(ruby: &Ruby, duration: VALUE, end: i64) -> HostResult {
    let duration = rb::int(ruby, duration)?.max(0);
    let start = i64::from(Guest::with(|guest| guest.scene.brightness()));
    // The quotient is rounded towards zero from either side, as `b0 - b0 * k / n` is.
    frames(ruby, duration, |number| {
        let level = start + (end - start) * number / i64::from(duration);
        Guest::with(|guest| guest.scene.set_brightness(level));
    })?;
    Guest::with(|guest| guest.scene.set_brightness(end));
    Ok(ffi::Qnil)
}

/// `Graphics.fadeout(duration)` of RGSS2.
fn fadeout(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    fade(ruby, args[0], 0)
}

/// `Graphics.fadein(duration)` of RGSS2.
fn fadein(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    fade(ruby, args[0], 255)
}

/// `Graphics.brightness` of RGSS2: the brightness of the scene.
fn brightness(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    let brightness = Guest::with(|guest| guest.scene.brightness());
    Ok(rb::fix(i32::from(brightness)))
}

/// `Graphics.brightness=` of RGSS2, limited to 0 to 255.
fn set_brightness(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let brightness = rb::long(ruby, args[0])?;
    Guest::with(|guest| guest.scene.set_brightness(brightness));
    Ok(args[0])
}

fn freeze(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Guest::with(|guest| {
        guest.runtime.held = true;
        guest.send(GuestToHost::Freeze);
    });
    Ok(ffi::Qnil)
}

/// Loads the picture of a transition as `Bitmap.new(path)` loads one.
fn load_mask(ruby: &Ruby, path: &str) -> Result<BitmapId, Raise> {
    Guest::with(|guest| -> Result<BitmapId, Error> {
        let bytes = guest.files.read(FileKind::Image, path)?;
        let image = vkxp_image::decode(&bytes)?;
        Ok(guest.bitmaps.create_from_rgba(
            image.width,
            image.height,
            image.rgba,
            &guest.font_defaults,
        )?)
    })
    .map_err(|error| error::to_raise(ruby, error))
}

/// `transition(duration, filename = nil, vague = 20)`, by the runtime of the run.
///
/// RGSS1: the duration defaults to 8, and the transition is shown whether or not the display
/// was frozen.
///
/// RGSS2: the duration defaults to 10. With nothing frozen the function returns at once: no
/// message, no frame, the brightness as it is, and the picture is not loaded (the last was
/// not measured). After a freeze the brightness becomes 255, also for a duration of 0, and
/// the host steps the transition by the rule of RGSS2.
///
/// Provenance: oracle measurement 2026-10-12, RGSS202E versions 2.0.0.1 and 2.0.2.1,
/// `vx-v0-REPORT.md` section 6.
fn transition(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    rb::check_count(ruby, args, 0, 3)?;
    let rgss1 = Guest::with(|guest| guest.runtime.run.runtime) == Runtime::Rgss1;
    let (default_duration, rule) = if rgss1 {
        (8, TransitionRule::Rgss1)
    } else {
        if !Guest::with(|guest| guest.runtime.held) {
            return Ok(ffi::Qnil);
        }
        (10, TransitionRule::Rgss2)
    };
    let duration = match args.first() {
        Some(&duration) => rb::int(ruby, duration)?.max(0).unsigned_abs(),
        None => default_duration,
    };
    let filename = match args.get(1) {
        Some(&filename) if !ffi::nil_p(filename) => Some(rb::text(ruby, filename)?),
        _ => None,
    };
    let vague = match args.get(2) {
        Some(&vague) => rb::int(ruby, vague)?.clamp(0, i32::from(u16::MAX)) as u16,
        None => 20,
    };
    let loaded = match filename {
        Some(path) => Some(load_mask(ruby, &path)?),
        None => None,
    };
    // No op refers to the mask, so its deltas are asked from the arena here and travel in
    // front of the first frame's own. Without a frame the mask is never announced.
    let (mask, mut mask_deltas) = Guest::with(|guest| {
        let (mask, mask_deltas) = match loaded {
            Some(id) if duration > 0 => (Some(id), guest.bitmaps.deltas(&[id]).unwrap_or_default()),
            Some(id) => {
                guest.bitmaps.dispose(id);
                (None, Vec::new())
            }
            None => (None, Vec::new()),
        };
        // The display holds a picture until the last frame of the transition.
        guest.runtime.held = true;
        if !rgss1 {
            guest.scene.set_brightness(255);
        }
        guest.send(GuestToHost::Transition {
            duration,
            mask,
            vague,
            rule,
        });
        (mask, mask_deltas)
    });
    let mut interrupted = Ok(());
    for _ in 0..duration {
        frame(std::mem::take(&mut mask_deltas));
        interrupted = runtime::raise_pending(ruby);
        if interrupted.is_err() {
            break;
        }
    }
    Guest::with(|guest| {
        guest.runtime.held = false;
        if let Some(id) = mask {
            // The destroy delta travels with a later frame.
            guest.bitmaps.dispose(id);
        }
    });
    interrupted?;
    Ok(ffi::Qnil)
}

fn frame_reset(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Guest::with(|guest| guest.send(GuestToHost::FrameReset));
    Ok(ffi::Qnil)
}

fn frame_rate(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    let rate = Guest::with(|guest| guest.scene.frame_rate().get());
    Ok(rb::fix(i32::from(rate)))
}

fn set_frame_rate(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let rate = rb::long(ruby, args[0])?;
    Guest::with(|guest| guest.scene.set_frame_rate(rate));
    Ok(args[0])
}

fn frame_count(ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    let count = Guest::with(|guest| guest.scene.frame_count());
    match c_long::try_from(count) {
        Ok(count) if ffi::fixable(count) => Ok(ffi::long2fix(count)),
        // A `Bignum`, which the interpreter makes from its literal.
        _ => Ok(ruby.eval(&count.to_string(), "Graphics", 1)?),
    }
}

fn set_frame_count(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let count = rb::long(ruby, args[0])?;
    Guest::with(|guest| guest.scene.set_frame_count(count));
    Ok(args[0])
}

/// `Graphics.width` of RGSS2: the width of the frame.
fn width(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    let size = Guest::with(|guest| guest.scene.logical_size());
    Ok(ffi::long2fix(size.width.into()))
}

/// `Graphics.height` of RGSS2: the height of the frame.
fn height(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    let size = Guest::with(|guest| guest.scene.logical_size());
    Ok(ffi::long2fix(size.height.into()))
}

/// `Graphics.snap_to_bitmap` of RGSS2: a new bitmap of the size of the frame with the scene
/// as its objects are now, shown or not, frozen or not, darkened by the brightness, with an
/// alpha of 255 everywhere. No frame is shown and the frame count stays.
///
/// The scene is drawn here in the guest by the reference compositor, in one of its own that
/// lives for this call, from a packet that takes nothing from what the host is sent next.
///
/// Provenance: oracle measurement 2026-10-12, RGSS202E versions 2.0.0.1 and 2.0.2.1,
/// `vx-v0-REPORT.md` section 6.
fn snap_to_bitmap(ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    let created = Guest::with(|guest| -> Option<Result<BitmapId, Error>> {
        let packet = guest.scene.snapshot(&mut guest.bitmaps);
        let mut frame = Compositor::new().render(&packet).ok()?;
        for pixel in frame.rgba.as_chunks_mut::<4>().0 {
            pixel[3] = 255;
        }
        let (width, height) = (frame.size.width, frame.size.height);
        let bitmaps = &mut guest.bitmaps;
        let id = bitmaps.create_from_rgba(width, height, frame.rgba, &guest.font_defaults);
        Some(id.map_err(Error::from))
    });
    match created {
        Some(Ok(id)) => bitmap::bitmap_new(ruby, id),
        Some(Err(error)) => Err(error::to_raise(ruby, error)),
        // The reference accepts every packet of a scene; a packet it refuses gives no bitmap.
        None => Err(Raise::new(
            guest::classes().errors.rgss_error,
            "failed to create bitmap",
        )),
    }
}

/// `Graphics.visible`: a stored flag without an effect.
fn visible(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::boolean(Guest::with(|guest| guest.runtime.visible)))
}

fn set_visible(_ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let visible = rb::truthy(args[0]);
    Guest::with(|guest| guest.runtime.visible = visible);
    Ok(args[0])
}

/// Defines `Graphics` as the runtime of the run has it.
pub(crate) fn define(ruby: &Ruby) -> Result<(), Raised> {
    let module = ruby.define_module(c"Graphics")?;
    rb::class_method(ruby, module, c"update", 0, update)?;
    rb::class_method(ruby, module, c"freeze", 0, freeze)?;
    rb::class_method(ruby, module, c"transition", -1, transition)?;
    rb::class_method(ruby, module, c"frame_reset", 0, frame_reset)?;
    rb::class_method(ruby, module, c"frame_rate", 0, frame_rate)?;
    rb::class_method(ruby, module, c"frame_rate=", 1, set_frame_rate)?;
    rb::class_method(ruby, module, c"frame_count", 0, frame_count)?;
    rb::class_method(ruby, module, c"frame_count=", 1, set_frame_count)?;
    rb::class_method(ruby, module, c"visible", 0, visible)?;
    rb::class_method(ruby, module, c"visible=", 1, set_visible)?;
    if Guest::with(|guest| guest.runtime.run.runtime) != Runtime::Rgss1 {
        rb::class_method(ruby, module, c"width", 0, width)?;
        rb::class_method(ruby, module, c"height", 0, height)?;
        rb::class_method(ruby, module, c"brightness", 0, brightness)?;
        rb::class_method(ruby, module, c"brightness=", 1, set_brightness)?;
        rb::class_method(ruby, module, c"wait", 1, wait)?;
        rb::class_method(ruby, module, c"fadeout", 1, fadeout)?;
        rb::class_method(ruby, module, c"fadein", 1, fadein)?;
        rb::class_method(ruby, module, c"snap_to_bitmap", 0, snap_to_bitmap)?;
    }
    ruby.eval(RESET, "Graphics", 1)?;
    Ok(())
}
