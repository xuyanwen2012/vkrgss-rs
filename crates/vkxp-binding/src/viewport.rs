//! The Ruby class `Viewport`.
//!
//! A viewport has no `disposed?` and no viewport of its own; otherwise it is an element
//! like the others (see [`element`]).

use std::ffi::CStr;

use vkxp_core::ViewportId;
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raised, Ruby};

use crate::color::{color_arg, color_of, color_set, tone_arg, tone_of, tone_set};
use crate::element::{self, field, object, reader, writer};
use crate::rb;
use crate::rect::{rect_arg, rect_of, rect_set};
use crate::wrap::Owner;

/// The instance variables that hold the viewport's own `Rect`, `Color` and `Tone` objects.
const RECT: &CStr = c"rect";
const COLOR: &CStr = c"color";
const TONE: &CStr = c"tone";

/// `initialize(x, y, width, height)` or `initialize(rect)`.
fn initialize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let rect = match args {
        [rect] => rect_arg(ruby, *rect)?,
        [_, _, _, _] => element::rect_of_ints(ruby, args)?,
        _ => return Err(rb::count_error(ruby, args, &[1, 4])),
    };
    let id = element::create(ruby, recv, None, |scene| scene.create_viewport(rect))?;
    let (color, tone) = element::operate(ruby, |guest| {
        let viewport = guest.scene.viewport(id)?;
        Ok((viewport.color, viewport.tone))
    })?;
    let rect = rect_of(ruby, rect, Owner::ViewportRect(id))?;
    rb::ivar_set(ruby, recv, RECT, rect)?;
    let color = color_of(ruby, color, Owner::ViewportColor(id))?;
    rb::ivar_set(ruby, recv, COLOR, color)?;
    let tone = tone_of(ruby, tone, Owner::ViewportTone(id))?;
    rb::ivar_set(ruby, recv, TONE, tone)?;
    Ok(recv)
}

fn flash(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let id = element::live::<ViewportId>(ruby, recv)?;
    let (color, duration) = element::flash_args(ruby, args)?;
    element::operate(ruby, |guest| {
        guest.scene.viewport_mut(id)?.flash(color, duration);
        Ok(())
    })?;
    Ok(ffi::Qnil)
}

fn update(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let id = element::id::<ViewportId>(ruby, recv)?;
    element::operate(ruby, |guest| {
        guest.scene.viewport_mut(id)?.update();
        Ok(())
    })?;
    Ok(ffi::Qnil)
}

object!(rect, set_rect, ViewportId, RECT, rect_arg, rect_set);
object!(color, set_color, ViewportId, COLOR, color_arg, color_set);
object!(tone, set_tone, ViewportId, TONE, tone_arg, tone_set);
field!(visible, set_visible, ViewportId, viewport, viewport_mut, visible: bool);
// A tilemap of RGSS2 takes the origin of its viewport when that changes: the scene is told
// to look after every write.
reader!(ox, ViewportId, |guest, id| guest.scene.viewport(id)?.ox);
writer!(set_ox, ViewportId, |guest, id, ox: i32| {
    guest.scene.viewport_mut(id)?.ox = ox;
    guest.scene.notice_viewport_origins();
});
reader!(oy, ViewportId, |guest, id| guest.scene.viewport(id)?.oy);
writer!(set_oy, ViewportId, |guest, id, oy: i32| {
    guest.scene.viewport_mut(id)?.oy = oy;
    guest.scene.notice_viewport_origins();
});
reader!(z, ViewportId, |guest, id| guest.scene.viewport(id)?.z());
writer!(set_z, ViewportId, |guest, id, z: i32| guest
    .scene
    .set_viewport_z(id, z)?);

/// Defines `Viewport`.
pub(crate) fn define(ruby: &Ruby) -> Result<VALUE, Raised> {
    let class = element::define_class::<ViewportId>(ruby, c"Viewport")?;
    rb::method(ruby, class, c"initialize", -1, initialize)?;
    rb::method(ruby, class, c"flash", 2, flash)?;
    rb::method(ruby, class, c"update", 0, update)?;
    rb::method(ruby, class, c"rect", 0, rect)?;
    rb::method(ruby, class, c"rect=", 1, set_rect)?;
    rb::method(ruby, class, c"visible", 0, visible)?;
    rb::method(ruby, class, c"visible=", 1, set_visible)?;
    rb::method(ruby, class, c"z", 0, z)?;
    rb::method(ruby, class, c"z=", 1, set_z)?;
    rb::method(ruby, class, c"ox", 0, ox)?;
    rb::method(ruby, class, c"ox=", 1, set_ox)?;
    rb::method(ruby, class, c"oy", 0, oy)?;
    rb::method(ruby, class, c"oy=", 1, set_oy)?;
    rb::method(ruby, class, c"color", 0, color)?;
    rb::method(ruby, class, c"color=", 1, set_color)?;
    rb::method(ruby, class, c"tone", 0, tone)?;
    rb::method(ruby, class, c"tone=", 1, set_tone)?;
    Ok(class)
}
