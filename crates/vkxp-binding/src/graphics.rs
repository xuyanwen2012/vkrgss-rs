//! The Ruby module `Graphics`.
//!
//! A frame is extracted from the guest's scene, sent to the host, and its tick awaited in
//! [`Guest::pump`]. What the host asked for meanwhile is raised once the guest is released.

use std::ffi::c_long;

use vkxp_proto::{BitmapDelta, BitmapId, GuestToHost};
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raise, Raised, Ruby};
use vkxp_vfs::FileKind;

use crate::error::{self, Error};
use crate::guest::Guest;
use crate::rb;
use crate::runtime::{self, Await};

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
        let mut packet = guest.scene.extract(&mut guest.bitmaps);
        if !front.is_empty() {
            front.append(&mut packet.bitmaps);
            packet.bitmaps = front;
        }
        guest.send(GuestToHost::Frame(packet));
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

fn freeze(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Guest::with(|guest| guest.send(GuestToHost::Freeze));
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

/// `transition(duration = 8, filename = nil, vague = 40)`.
fn transition(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    rb::check_count(ruby, args, 0, 3)?;
    let duration = match args.first() {
        Some(&duration) => rb::int(ruby, duration)?.max(0).unsigned_abs(),
        None => 8,
    };
    let filename = match args.get(1) {
        Some(&filename) if !ffi::nil_p(filename) => Some(rb::text(ruby, filename)?),
        _ => None,
    };
    let vague = match args.get(2) {
        Some(&vague) => rb::int(ruby, vague)?.clamp(0, i32::from(u16::MAX)) as u16,
        None => 40,
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
        guest.send(GuestToHost::Transition {
            duration,
            mask,
            vague,
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
    if let Some(id) = mask {
        // The destroy delta travels with a later frame.
        Guest::with(|guest| guest.bitmaps.dispose(id));
    }
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

/// `Graphics.visible`: a stored flag without an effect.
fn visible(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::boolean(Guest::with(|guest| guest.runtime.visible)))
}

fn set_visible(_ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let visible = rb::truthy(args[0]);
    Guest::with(|guest| guest.runtime.visible = visible);
    Ok(args[0])
}

/// Defines `Graphics`.
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
    ruby.eval(RESET, "Graphics", 1)?;
    Ok(())
}
