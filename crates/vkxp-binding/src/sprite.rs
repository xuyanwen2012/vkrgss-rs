//! The Ruby class `Sprite`.
//!
//! See [`element`] for what the scene classes share. A sprite keeps the `Bitmap` object it
//! was given, and assigning one sets the sprite's own `src_rect` object to the bitmap's
//! rectangle.

use std::ffi::CStr;

use vkxp_core::SpriteId;
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raised, Ruby};
use vkxp_vfs::Runtime;

use crate::color::{color_arg, color_of, color_set, tone_arg, tone_of, tone_set};
use crate::element::{self, field, kept_after_dispose, object, reader, writer};
use crate::rb;
use crate::rect::{rect_arg, rect_of, rect_set};
use crate::wrap::Owner;

/// The instance variable that holds the sprite's `Bitmap` object, or `nil`.
const BITMAP: &CStr = c"bitmap";
/// The instance variables that hold the sprite's own `Rect`, `Color` and `Tone` objects.
const SRC_RECT: &CStr = c"src_rect";
const COLOR: &CStr = c"color";
const TONE: &CStr = c"tone";

/// `initialize(viewport = nil)`.
fn initialize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let viewport = element::viewport_arg(ruby, args)?;
    let id = element::create(ruby, recv, args.first(), |scene| {
        scene.create_sprite(viewport)
    })?;
    let (src_rect, color, tone) = element::operate(ruby, |guest| {
        let sprite = guest.scene.sprite(id)?;
        Ok((sprite.src_rect, sprite.color, sprite.tone))
    })?;
    let src_rect = rect_of(ruby, src_rect, Owner::SpriteSrcRect(id))?;
    rb::ivar_set(ruby, recv, SRC_RECT, src_rect)?;
    let color = color_of(ruby, color, Owner::SpriteColor(id))?;
    rb::ivar_set(ruby, recv, COLOR, color)?;
    let tone = tone_of(ruby, tone, Owner::SpriteTone(id))?;
    rb::ivar_set(ruby, recv, TONE, tone)?;
    Ok(recv)
}

fn flash(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let id = element::live::<SpriteId>(ruby, recv)?;
    let (color, duration) = element::flash_args(ruby, args)?;
    element::operate(ruby, |guest| {
        guest.scene.sprite_mut(id)?.flash(color, duration);
        Ok(())
    })?;
    Ok(ffi::Qnil)
}

fn update(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let id = element::id::<SpriteId>(ruby, recv)?;
    element::operate(ruby, |guest| {
        guest.scene.sprite_mut(id)?.update();
        Ok(())
    })?;
    Ok(ffi::Qnil)
}

kept_after_dispose!(bitmap, SpriteId, BITMAP);

/// `bitmap = bitmap_or_nil`: keeps the object, and a live bitmap also becomes the whole
/// source rectangle.
fn set_bitmap(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let id = element::live::<SpriteId>(ruby, recv)?;
    let bitmap = element::bitmap_ref(ruby, args[0])?;
    let src_rect = element::operate(ruby, |guest| {
        let sprite = guest.scene.sprite_mut(id)?;
        sprite.set_bitmap(bitmap, &guest.bitmaps);
        Ok(sprite.src_rect)
    })?;
    rb::ivar_set(ruby, recv, BITMAP, args[0])?;
    // The scene has the source rectangle already; the sprite's own object follows it.
    let own = rb::ivar_get(ruby, recv, SRC_RECT)?;
    rect_set(ruby, own, src_rect)?;
    Ok(args[0])
}

object!(
    src_rect,
    set_src_rect,
    SpriteId,
    SRC_RECT,
    rect_arg,
    rect_set
);
object!(color, set_color, SpriteId, COLOR, color_arg, color_set);
object!(tone, set_tone, SpriteId, TONE, tone_arg, tone_set);
field!(visible, set_visible, SpriteId, sprite, sprite_mut, visible: bool);
field!(x, set_x, SpriteId, sprite, sprite_mut, x: i32);
field!(y, set_y, SpriteId, sprite, sprite_mut, y: i32);
field!(ox, set_ox, SpriteId, sprite, sprite_mut, ox: i32);
field!(oy, set_oy, SpriteId, sprite, sprite_mut, oy: i32);
field!(zoom_x, set_zoom_x, SpriteId, sprite, sprite_mut, zoom_x: f64);
field!(zoom_y, set_zoom_y, SpriteId, sprite, sprite_mut, zoom_y: f64);
field!(angle, set_angle, SpriteId, sprite, sprite_mut, angle: f64);
field!(mirror, set_mirror, SpriteId, sprite, sprite_mut, mirror: bool);
field!(bush_depth, set_bush_depth, SpriteId, sprite, sprite_mut, bush_depth: i32);
field!(blend_type, set_blend_type, SpriteId, sprite, sprite_mut, blend_type: i32);
element::blend_type_writer_rgss2!(set_blend_type_rgss2, SpriteId, sprite_mut);
reader!(z, SpriteId, |guest, id| guest.scene.sprite(id)?.z());
writer!(set_z, SpriteId, |guest, id, z: i32| guest
    .scene
    .set_sprite_z(id, z)?);
reader!(opacity, SpriteId, |guest, id| i32::from(
    guest.scene.sprite(id)?.opacity()
));
writer!(set_opacity, SpriteId, |guest, id, opacity: i32| guest
    .scene
    .sprite_mut(id)?
    .set_opacity(opacity));
// `width` and `height` of RGSS2: those of the source rectangle, whatever they are.
reader!(width, SpriteId, |guest, id| guest
    .scene
    .sprite(id)?
    .src_rect
    .width);
reader!(height, SpriteId, |guest, id| guest
    .scene
    .sprite(id)?
    .src_rect
    .height);

/// Defines `Sprite` as `runtime` has it: with `viewport=`, `width` and `height` in
/// RGSS2, where `blend_type=` refuses a value other than 0, 1 and 2.
pub(crate) fn define(ruby: &Ruby, runtime: Runtime) -> Result<VALUE, Raised> {
    let class = element::define_class::<SpriteId>(ruby, c"Sprite")?;
    element::define_in_viewport::<SpriteId>(ruby, class, runtime)?;
    rb::method(ruby, class, c"initialize", -1, initialize)?;
    rb::method(ruby, class, c"flash", 2, flash)?;
    rb::method(ruby, class, c"update", 0, update)?;
    rb::method(ruby, class, c"bitmap", 0, bitmap)?;
    rb::method(ruby, class, c"bitmap=", 1, set_bitmap)?;
    rb::method(ruby, class, c"src_rect", 0, src_rect)?;
    rb::method(ruby, class, c"src_rect=", 1, set_src_rect)?;
    rb::method(ruby, class, c"visible", 0, visible)?;
    rb::method(ruby, class, c"visible=", 1, set_visible)?;
    rb::method(ruby, class, c"x", 0, x)?;
    rb::method(ruby, class, c"x=", 1, set_x)?;
    rb::method(ruby, class, c"y", 0, y)?;
    rb::method(ruby, class, c"y=", 1, set_y)?;
    rb::method(ruby, class, c"z", 0, z)?;
    rb::method(ruby, class, c"z=", 1, set_z)?;
    rb::method(ruby, class, c"ox", 0, ox)?;
    rb::method(ruby, class, c"ox=", 1, set_ox)?;
    rb::method(ruby, class, c"oy", 0, oy)?;
    rb::method(ruby, class, c"oy=", 1, set_oy)?;
    rb::method(ruby, class, c"zoom_x", 0, zoom_x)?;
    rb::method(ruby, class, c"zoom_x=", 1, set_zoom_x)?;
    rb::method(ruby, class, c"zoom_y", 0, zoom_y)?;
    rb::method(ruby, class, c"zoom_y=", 1, set_zoom_y)?;
    rb::method(ruby, class, c"angle", 0, angle)?;
    rb::method(ruby, class, c"angle=", 1, set_angle)?;
    rb::method(ruby, class, c"mirror", 0, mirror)?;
    rb::method(ruby, class, c"mirror=", 1, set_mirror)?;
    rb::method(ruby, class, c"bush_depth", 0, bush_depth)?;
    rb::method(ruby, class, c"bush_depth=", 1, set_bush_depth)?;
    rb::method(ruby, class, c"opacity", 0, opacity)?;
    rb::method(ruby, class, c"opacity=", 1, set_opacity)?;
    rb::method(ruby, class, c"blend_type", 0, blend_type)?;
    if runtime == Runtime::Rgss1 {
        rb::method(ruby, class, c"blend_type=", 1, set_blend_type)?;
    } else {
        rb::method(ruby, class, c"blend_type=", 1, set_blend_type_rgss2)?;
    }
    rb::method(ruby, class, c"color", 0, color)?;
    rb::method(ruby, class, c"color=", 1, set_color)?;
    rb::method(ruby, class, c"tone", 0, tone)?;
    rb::method(ruby, class, c"tone=", 1, set_tone)?;
    if runtime != Runtime::Rgss1 {
        rb::method(ruby, class, c"width", 0, width)?;
        rb::method(ruby, class, c"height", 0, height)?;
    }
    Ok(class)
}
