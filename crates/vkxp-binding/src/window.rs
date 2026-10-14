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

use vkxp_core::{Scene, SceneError, Tone, ViewportId, WindowCall, WindowId};
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raise, Raised, Ruby};
use vkxp_vfs::Runtime;

use crate::color::{tone_of, tone_set};
use crate::element::{self, field, kept, object, reader, writer};
use crate::rect::{rect_arg, rect_of, rect_set};
use crate::wrap::{self, Owned, Owner};
use crate::{bitmap, rb};

/// The instance variables that hold the window's `Bitmap` objects, or `nil`.
const WINDOWSKIN: &CStr = c"windowskin";
const CONTENTS: &CStr = c"contents";
/// The instance variable that holds the window's own `Rect` object.
const CURSOR_RECT: &CStr = c"cursor_rect";
/// The instance variable that holds the own `Tone` object of a window of RGSS3.
const TONE: &CStr = c"tone";

/// Creates the window of `recv` with `create` and its cursor rectangle.
fn create(
    ruby: &Ruby,
    recv: VALUE,
    args: &[VALUE],
    create: fn(&mut Scene, Option<ViewportId>) -> Result<WindowId, SceneError>,
) -> HostResult {
    let viewport = element::viewport_arg(ruby, args)?;
    let id = element::create(ruby, recv, args.first(), |scene| create(scene, viewport))?;
    keep_cursor_rect(ruby, recv, id)?;
    Ok(recv)
}

/// Makes the cursor rectangle object of the window `id` and keeps it in `recv`.
fn keep_cursor_rect(ruby: &Ruby, recv: VALUE, id: WindowId) -> Result<(), Raise> {
    let cursor_rect = element::operate(ruby, |guest| Ok(guest.scene.window(id)?.cursor_rect))?;
    let cursor_rect = rect_of(ruby, cursor_rect, Owner::WindowCursorRect(id))?;
    rb::ivar_set(ruby, recv, CURSOR_RECT, cursor_rect)?;
    Ok(())
}

/// `initialize(viewport = nil)`.
fn initialize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    create(ruby, recv, args, Scene::create_window)
}

/// `initialize(viewport = nil)` of RGSS2: the window is a window of RGSS2 and starts with a
/// contents bitmap of 1 by 1, an ordinary bitmap that outlives the window.
fn initialize_rgss2(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    wrap::intern_owner_names(ruby)?;
    create(ruby, recv, args, Scene::create_window_rgss2)?;
    let contents = bitmap::new_blank(ruby, 1, 1)?;
    set_contents(ruby, recv, &[contents])?;
    Ok(recv)
}

/// `initialize(x = 0, y = 0, width = 0, height = 0)` of RGSS3: no viewport, the original's
/// count message, the arguments converted before a window exists. The window is a window
/// of RGSS3 with the scene's defaults, given the four values as one `move`, and starts with
/// a contents bitmap of 1 by 1, as in RGSS2.
///
/// Provenance: oracle measurement 2026-10-13, RGSS300 version 3.0.0.1 and RGSS301 version
/// 3.0.1.1, `ace-v1-window-REPORT.md` section "The class".
fn initialize_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    rb::check_count_rgss3(ruby, args, 0, 4)?;
    let mut place = [0; 4];
    for (value, &arg) in place.iter_mut().zip(args) {
        *value = rb::int(ruby, arg)?;
    }
    let [x, y, width, height] = place;
    let id = element::create(ruby, recv, None, |scene| scene.create_window_rgss3(None))?;
    element::operate(ruby, |guest| {
        let window = guest.scene.window_mut(id)?;
        window.call(&mut guest.bitmaps, WindowCall::Move(x, y, width, height));
        Ok(())
    })?;
    keep_cursor_rect(ruby, recv, id)?;
    let tone = element::operate(ruby, |guest| Ok(guest.scene.window(id)?.tone()))?;
    let tone = tone_of(ruby, tone, Owner::WindowTone(id))?;
    rb::ivar_set(ruby, recv, TONE, tone)?;
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

/// `move(x, y, width, height)` of RGSS3: the four assignments as one call; the receiver.
fn move_to(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let id = element::live::<WindowId>(ruby, recv)?;
    let place = element::rect_of_ints(ruby, args)?;
    element::operate(ruby, |guest| {
        let window = guest.scene.window_mut(id)?;
        let call = WindowCall::Move(place.x, place.y, place.width, place.height);
        window.call(&mut guest.bitmaps, call);
        Ok(())
    })?;
    Ok(recv)
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
kept!(tone, WindowId, TONE);

/// `tone = tone` of RGSS3: copies the values of a `Tone` into the window's own object.
/// Anything else raises `TypeError` in the words of the original runtime, `can't convert
/// NilClass into Tone`.
///
/// Provenance: oracle measurement 2026-10-13, RGSS300 version 3.0.0.1 and RGSS301 version
/// 3.0.1.1, `ace-v1-window-REPORT.md` section "The class".
fn set_tone(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    element::live::<WindowId>(ruby, recv)?;
    let tone = wrap::find::<Owned<Tone>>(ruby, args[0]).ok_or_else(|| {
        rb::wrong_type(ruby, args[0], |class| {
            format!("can't convert {class} into Tone")
        })
    })?;
    let tone = tone.get().value();
    let own = rb::ivar_get(ruby, recv, TONE)?;
    tone_set(ruby, own, tone)?;
    Ok(args[0])
}

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
reader!(open, WindowId, |guest, id| guest
    .scene
    .window(id)?
    .openness()
    == 255);
reader!(close, WindowId, |guest, id| guest
    .scene
    .window(id)?
    .openness()
    == 0);
reader!(padding, WindowId, |guest, id| guest
    .scene
    .window(id)?
    .padding());
called!(set_padding, Padding: i32);
reader!(padding_bottom, WindowId, |guest, id| guest
    .scene
    .window(id)?
    .padding_bottom());
called!(set_padding_bottom, PaddingBottom: i32);
reader!(arrows_visible, WindowId, |guest, id| guest
    .scene
    .window(id)?
    .arrows_visible());
called!(set_arrows_visible, ArrowsVisible: bool);

/// Defines `Window` as `runtime` has it: in RGSS2 the window of RGSS2, with `viewport=`,
/// `openness`, a contents bitmap from the start and no `stretch`.
pub(crate) fn define(ruby: &Ruby, runtime: Runtime) -> Result<VALUE, Raised> {
    let class = element::define_class::<WindowId>(ruby, c"Window")?;
    element::define_in_viewport::<WindowId>(ruby, class, runtime)?;
    match runtime {
        Runtime::Rgss1 => {
            rb::method(ruby, class, c"initialize", -1, initialize)?;
            rb::method(ruby, class, c"stretch", 0, stretch)?;
            rb::method(ruby, class, c"stretch=", 1, set_stretch)?;
        }
        Runtime::Rgss2 => {
            rb::method(ruby, class, c"initialize", -1, initialize_rgss2)?;
            rb::method(ruby, class, c"openness", 0, openness)?;
            rb::method(ruby, class, c"openness=", 1, set_openness)?;
        }
        Runtime::Rgss3 => {
            rb::method(ruby, class, c"initialize", -1, initialize_rgss3)?;
            rb::method(ruby, class, c"openness", 0, openness)?;
            rb::method(ruby, class, c"openness=", 1, set_openness)?;
            rb::method(ruby, class, c"move", 4, move_to)?;
            rb::method(ruby, class, c"open?", 0, open)?;
            rb::method(ruby, class, c"close?", 0, close)?;
            rb::method(ruby, class, c"padding", 0, padding)?;
            rb::method(ruby, class, c"padding=", 1, set_padding)?;
            rb::method(ruby, class, c"padding_bottom", 0, padding_bottom)?;
            rb::method(ruby, class, c"padding_bottom=", 1, set_padding_bottom)?;
            rb::method(ruby, class, c"arrows_visible", 0, arrows_visible)?;
            rb::method(ruby, class, c"arrows_visible=", 1, set_arrows_visible)?;
            rb::method(ruby, class, c"tone", 0, tone)?;
            rb::method(ruby, class, c"tone=", 1, set_tone)?;
        }
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
