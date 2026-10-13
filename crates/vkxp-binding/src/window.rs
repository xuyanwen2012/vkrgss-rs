//! The Ruby class `Window`.
//!
//! See [`element`] for what the scene classes share. A window keeps the `Bitmap` objects
//! given as its window skin and its contents, and its cursor rectangle is an object of its
//! own.
//!
//! In a game of RGSS2 the window is the window of RGSS2 of the scene, which is drawn by
//! other rules and latches part of what it shows at certain calls. The binding does not
//! imitate that: `update` and every assignment the scene has a call for are passed to
//! [`vkxp_core::Window::call`], in both runtimes.

use std::ffi::CStr;

use vkxp_core::{Scene, SceneError, ViewportId, WindowCall, WindowId};
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raised, Ruby};
use vkxp_vfs::Runtime;

use crate::element::{self, field, kept, object, reader, writer};
use crate::rect::{rect_arg, rect_of, rect_set};
use crate::wrap::Owner;
use crate::{bitmap, rb};

/// The instance variables that hold the window's `Bitmap` objects, or `nil`.
const WINDOWSKIN: &CStr = c"windowskin";
const CONTENTS: &CStr = c"contents";
/// The instance variable that holds the window's own `Rect` object.
const CURSOR_RECT: &CStr = c"cursor_rect";

/// Creates the window of `recv` with `create` and its cursor rectangle.
fn create(
    ruby: &Ruby,
    recv: VALUE,
    args: &[VALUE],
    create: fn(&mut Scene, Option<ViewportId>) -> Result<WindowId, SceneError>,
) -> HostResult {
    let viewport = element::viewport_arg(ruby, args)?;
    let id = element::create(ruby, recv, args.first(), |scene| create(scene, viewport))?;
    let cursor_rect = element::operate(ruby, |guest| Ok(guest.scene.window(id)?.cursor_rect))?;
    let cursor_rect = rect_of(ruby, cursor_rect, Owner::WindowCursorRect(id))?;
    rb::ivar_set(ruby, recv, CURSOR_RECT, cursor_rect)?;
    Ok(recv)
}

/// `initialize(viewport = nil)`.
fn initialize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    create(ruby, recv, args, Scene::create_window)
}

/// `initialize(viewport = nil)` of RGSS2: the window is a window of RGSS2 and starts with a
/// contents bitmap of 1 by 1, an ordinary bitmap that outlives the window.
fn initialize_rgss2(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    create(ruby, recv, args, Scene::create_window_rgss2)?;
    let contents = bitmap::new_blank(ruby, 1, 1)?;
    set_contents(ruby, recv, &[contents])?;
    Ok(recv)
}

/// Defines the writer `$name` of a property that the window takes as the call `$call`: in
/// RGSS2 an assignment can latch what the window shows, which the scene sees to.
macro_rules! called {
    ($name:ident, $call:ident: $type:ty) => {
        writer!($name, WindowId, |guest, id, value: $type| guest
            .scene
            .window_mut(id)?
            .call(&mut guest.bitmaps, WindowCall::$call(value)));
    };
}

fn update(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let id = element::id::<WindowId>(ruby, recv)?;
    element::operate(ruby, |guest| {
        let window = guest.scene.window_mut(id)?;
        window.call(&mut guest.bitmaps, WindowCall::Update);
        Ok(())
    })?;
    Ok(ffi::Qnil)
}

kept!(windowskin, WindowId, WINDOWSKIN);

/// `windowskin = bitmap_or_nil`: keeps the object.
fn set_windowskin(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let id = element::live::<WindowId>(ruby, recv)?;
    let bitmap = element::bitmap_ref(ruby, args[0])?;
    element::operate(ruby, |guest| {
        let window = guest.scene.window_mut(id)?;
        window.call(&mut guest.bitmaps, WindowCall::Windowskin(bitmap));
        Ok(())
    })?;
    rb::ivar_set(ruby, recv, WINDOWSKIN, args[0])?;
    Ok(args[0])
}

kept!(contents, WindowId, CONTENTS);

/// `contents = bitmap_or_nil`: keeps the object.
fn set_contents(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let id = element::live::<WindowId>(ruby, recv)?;
    let bitmap = element::bitmap_ref(ruby, args[0])?;
    element::operate(ruby, |guest| {
        let window = guest.scene.window_mut(id)?;
        window.call(&mut guest.bitmaps, WindowCall::Contents(bitmap));
        Ok(())
    })?;
    rb::ivar_set(ruby, recv, CONTENTS, args[0])?;
    Ok(args[0])
}

object!(
    cursor_rect,
    set_cursor_rect,
    WindowId,
    CURSOR_RECT,
    rect_arg,
    rect_set
);
field!(stretch, set_stretch, WindowId, window, window_mut, stretch: bool);
reader!(active, WindowId, |guest, id| guest.scene.window(id)?.active);
called!(set_active, Active: bool);
field!(visible, set_visible, WindowId, window, window_mut, visible: bool);
reader!(pause, WindowId, |guest, id| guest.scene.window(id)?.pause);
called!(set_pause, Pause: bool);
reader!(x, WindowId, |guest, id| guest.scene.window(id)?.x);
called!(set_x, X: i32);
reader!(y, WindowId, |guest, id| guest.scene.window(id)?.y);
called!(set_y, Y: i32);
reader!(width, WindowId, |guest, id| guest.scene.window(id)?.width);
called!(set_width, Width: i32);
reader!(height, WindowId, |guest, id| guest.scene.window(id)?.height);
called!(set_height, Height: i32);
reader!(ox, WindowId, |guest, id| guest.scene.window(id)?.ox);
called!(set_ox, Ox: i32);
reader!(oy, WindowId, |guest, id| guest.scene.window(id)?.oy);
called!(set_oy, Oy: i32);
reader!(z, WindowId, |guest, id| guest.scene.window(id)?.z());
writer!(set_z, WindowId, |guest, id, z: i32| guest
    .scene
    .set_window_z(id, z)?);
reader!(opacity, WindowId, |guest, id| i32::from(
    guest.scene.window(id)?.opacity()
));
writer!(set_opacity, WindowId, |guest, id, opacity: i32| guest
    .scene
    .window_mut(id)?
    .set_opacity(opacity));
reader!(back_opacity, WindowId, |guest, id| i32::from(
    guest.scene.window(id)?.back_opacity()
));
writer!(set_back_opacity, WindowId, |guest, id, opacity: i32| guest
    .scene
    .window_mut(id)?
    .set_back_opacity(opacity));
reader!(contents_opacity, WindowId, |guest, id| i32::from(
    guest.scene.window(id)?.contents_opacity()
));
called!(set_contents_opacity, ContentsOpacity: i32);
reader!(openness, WindowId, |guest, id| i32::from(
    guest.scene.window(id)?.openness()
));
called!(set_openness, Openness: i32);

/// Defines `Window` as `runtime` has it: in RGSS2 the window of RGSS2, with `viewport=`,
/// `openness`, a contents bitmap from the start and no `stretch`.
pub(crate) fn define(ruby: &Ruby, runtime: Runtime) -> Result<VALUE, Raised> {
    let class = element::define_class::<WindowId>(ruby, c"Window")?;
    element::define_in_viewport::<WindowId>(ruby, class, runtime)?;
    if runtime == Runtime::Rgss1 {
        rb::method(ruby, class, c"initialize", -1, initialize)?;
        rb::method(ruby, class, c"stretch", 0, stretch)?;
        rb::method(ruby, class, c"stretch=", 1, set_stretch)?;
    } else {
        rb::method(ruby, class, c"initialize", -1, initialize_rgss2)?;
        rb::method(ruby, class, c"openness", 0, openness)?;
        rb::method(ruby, class, c"openness=", 1, set_openness)?;
    }
    rb::method(ruby, class, c"update", 0, update)?;
    rb::method(ruby, class, c"windowskin", 0, windowskin)?;
    rb::method(ruby, class, c"windowskin=", 1, set_windowskin)?;
    rb::method(ruby, class, c"contents", 0, contents)?;
    rb::method(ruby, class, c"contents=", 1, set_contents)?;
    rb::method(ruby, class, c"cursor_rect", 0, cursor_rect)?;
    rb::method(ruby, class, c"cursor_rect=", 1, set_cursor_rect)?;
    rb::method(ruby, class, c"active", 0, active)?;
    rb::method(ruby, class, c"active=", 1, set_active)?;
    rb::method(ruby, class, c"visible", 0, visible)?;
    rb::method(ruby, class, c"visible=", 1, set_visible)?;
    rb::method(ruby, class, c"pause", 0, pause)?;
    rb::method(ruby, class, c"pause=", 1, set_pause)?;
    rb::method(ruby, class, c"x", 0, x)?;
    rb::method(ruby, class, c"x=", 1, set_x)?;
    rb::method(ruby, class, c"y", 0, y)?;
    rb::method(ruby, class, c"y=", 1, set_y)?;
    rb::method(ruby, class, c"width", 0, width)?;
    rb::method(ruby, class, c"width=", 1, set_width)?;
    rb::method(ruby, class, c"height", 0, height)?;
    rb::method(ruby, class, c"height=", 1, set_height)?;
    rb::method(ruby, class, c"z", 0, z)?;
    rb::method(ruby, class, c"z=", 1, set_z)?;
    rb::method(ruby, class, c"ox", 0, ox)?;
    rb::method(ruby, class, c"ox=", 1, set_ox)?;
    rb::method(ruby, class, c"oy", 0, oy)?;
    rb::method(ruby, class, c"oy=", 1, set_oy)?;
    rb::method(ruby, class, c"opacity", 0, opacity)?;
    rb::method(ruby, class, c"opacity=", 1, set_opacity)?;
    rb::method(ruby, class, c"back_opacity", 0, back_opacity)?;
    rb::method(ruby, class, c"back_opacity=", 1, set_back_opacity)?;
    rb::method(ruby, class, c"contents_opacity", 0, contents_opacity)?;
    rb::method(ruby, class, c"contents_opacity=", 1, set_contents_opacity)?;
    Ok(class)
}
