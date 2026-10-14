//! The Ruby class `Viewport`.
//!
//! A viewport has no `disposed?` and no viewport of its own; otherwise it is an element
//! like the others (see [`element`]).
//!
//! A game of RGSS3 gets another `initialize`, as measured on the original runtime of RGSS3:
//! without arguments it makes a viewport of the screen as it is then, and a wrong count says
//! `(expected for given)`. Disposing a viewport of RGSS3 leaves its elements alive (see
//! [`element`]).

use std::ffi::CStr;

use vkxp_core::{Rect, Scene, SceneError, ViewportId};
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raised, Ruby};
use vkxp_vfs::Runtime;

use crate::color::{color_of, color_set, tone_of, tone_set};
use crate::element::{self, color_arg, field, object, reader, rect_arg, tone_arg, writer};
use crate::rb;
use crate::rect::{rect_of, rect_set};
use crate::wrap::{self, Owned, Owner};

/// The instance variables that hold the viewport's own `Rect`, `Color` and `Tone` objects.
const RECT: &CStr = c"rect";
const COLOR: &CStr = c"color";
const TONE: &CStr = c"tone";

/// `initialize(x, y, width, height)` or `initialize(rect)`.
fn initialize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    wrap::intern_owner_names(ruby)?;
    let rect = match args {
        [rect] => rect_arg(ruby, *rect)?,
        [_, _, _, _] => element::rect_of_ints(ruby, args)?,
        _ => return Err(rb::count_error(ruby, args, &[1, 4])),
    };
    create(ruby, recv, |scene| scene.create_viewport(rect))
}

/// `initialize` of RGSS3: no argument for the screen as it is now, one `Rect` or four
/// values. Any other count, or one argument that is no `Rect`, raises `ArgumentError`
/// worded as the original runtime of RGSS3 words it: `(1 for given)` when the first
/// argument is a `Rect`, `(4 for given)` otherwise.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 version 3.0.1.1 and RGSS300 version
/// 3.0.0.1, `ace-v1-sprite-REPORT.md` section 3 and its exploration log, part 2.
fn initialize_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    wrap::intern_owner_names(ruby)?;
    let first_is_rect = args
        .first()
        .is_some_and(|&first| wrap::find::<Owned<Rect>>(ruby, first).is_some());
    match args {
        [] => create(ruby, recv, Scene::create_screen_viewport),
        [rect] if first_is_rect => {
            let rect = rect_arg(ruby, *rect)?;
            create(ruby, recv, |scene| scene.create_viewport(rect))
        }
        [_, _, _, _] if !first_is_rect => {
            let rect = element::rect_of_ints(ruby, args)?;
            create(ruby, recv, |scene| scene.create_viewport(rect))
        }
        _ => {
            let expected = if first_is_rect { 1 } else { 4 };
            rb::check_count_rgss3(ruby, args, expected, expected)?;
            unreachable!("a count other than the expected one was given")
        }
    }
}

/// Creates the viewport of the object `recv` with `create` and gives the object its own
/// `Rect`, `Color` and `Tone`.
fn create(
    ruby: &Ruby,
    recv: VALUE,
    create: impl FnOnce(&mut Scene) -> Result<ViewportId, SceneError>,
) -> HostResult {
    let id = element::create(ruby, recv, None, create)?;
    let (rect, color, tone) = element::operate(ruby, |guest| {
        let viewport = guest.scene.viewport(id)?;
        Ok((viewport.rect, viewport.color, viewport.tone))
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

/// Defines `Viewport` as `runtime` has it.
pub(crate) fn define(ruby: &Ruby, runtime: Runtime) -> Result<VALUE, Raised> {
    let class = element::define_class::<ViewportId>(ruby, c"Viewport")?;
    if runtime == Runtime::Rgss3 {
        rb::method(ruby, class, c"initialize", -1, initialize_rgss3)?;
    } else {
        rb::method(ruby, class, c"initialize", -1, initialize)?;
    }
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
