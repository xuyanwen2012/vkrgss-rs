//! The Ruby class `Plane`.
//!
//! See [`element`] for what the scene classes share.

use std::ffi::CStr;

use vkxp_core::PlaneId;
use vkxp_ruby_sys::ffi::VALUE;
use vkxp_ruby_sys::{HostResult, Raised, Ruby};
use vkxp_vfs::Runtime;

use crate::color::{color_arg, color_of, color_set, tone_arg, tone_of, tone_set};
use crate::element::{self, field, kept, object, reader, writer};
use crate::rb;
use crate::wrap::{self, Owner};

/// The instance variable that holds the plane's `Bitmap` object, or `nil`.
const BITMAP: &CStr = c"bitmap";
/// The instance variables that hold the plane's own `Color` and `Tone` objects.
const COLOR: &CStr = c"color";
const TONE: &CStr = c"tone";

/// `initialize(viewport = nil)`.
fn initialize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    wrap::intern_owner_names(ruby)?;
    let viewport = element::viewport_arg(ruby, args)?;
    let id = element::create(ruby, recv, args.first(), |scene| {
        scene.create_plane(viewport)
    })?;
    let (color, tone) = element::operate(ruby, |guest| {
        let plane = guest.scene.plane(id)?;
        Ok((plane.color, plane.tone))
    })?;
    let color = color_of(ruby, color, Owner::PlaneColor(id))?;
    rb::ivar_set(ruby, recv, COLOR, color)?;
    let tone = tone_of(ruby, tone, Owner::PlaneTone(id))?;
    rb::ivar_set(ruby, recv, TONE, tone)?;
    Ok(recv)
}

kept!(bitmap, PlaneId, BITMAP);

/// `bitmap = bitmap_or_nil`: keeps the object.
fn set_bitmap(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let id = element::live::<PlaneId>(ruby, recv)?;
    let bitmap = element::bitmap_ref(ruby, args[0])?;
    element::operate(ruby, |guest| {
        guest.scene.plane_mut(id)?.bitmap = bitmap;
        Ok(())
    })?;
    rb::ivar_set(ruby, recv, BITMAP, args[0])?;
    Ok(args[0])
}

object!(color, set_color, PlaneId, COLOR, color_arg, color_set);
object!(tone, set_tone, PlaneId, TONE, tone_arg, tone_set);
field!(visible, set_visible, PlaneId, plane, plane_mut, visible: bool);

/// `visible` of RGSS2, which reads `true` whatever was set, as measured on the original
/// runtime. The writer hides the plane all the same.
fn visible_rgss2(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    element::live::<PlaneId>(ruby, recv)?;
    Ok(rb::boolean(true))
}

field!(ox, set_ox, PlaneId, plane, plane_mut, ox: i32);
field!(oy, set_oy, PlaneId, plane, plane_mut, oy: i32);
field!(zoom_x, set_zoom_x, PlaneId, plane, plane_mut, zoom_x: f64);
field!(zoom_y, set_zoom_y, PlaneId, plane, plane_mut, zoom_y: f64);
field!(blend_type, set_blend_type, PlaneId, plane, plane_mut, blend_type: i32);
element::blend_type_writer_rgss2!(set_blend_type_rgss2, PlaneId, plane_mut);
reader!(z, PlaneId, |guest, id| guest.scene.plane(id)?.z());
writer!(set_z, PlaneId, |guest, id, z: i32| guest
    .scene
    .set_plane_z(id, z)?);
reader!(opacity, PlaneId, |guest, id| i32::from(
    guest.scene.plane(id)?.opacity()
));
writer!(set_opacity, PlaneId, |guest, id, opacity: i32| guest
    .scene
    .plane_mut(id)?
    .set_opacity(opacity));

/// Defines `Plane` as `runtime` has it: in RGSS2 `viewport` takes the viewport to move to,
/// `visible` reads `true` always and `blend_type=` refuses a value other than 0, 1 and 2.
pub(crate) fn define(ruby: &Ruby, runtime: Runtime) -> Result<VALUE, Raised> {
    let class = element::define_class::<PlaneId>(ruby, c"Plane")?;
    element::define_plane_viewport(ruby, class, runtime)?;
    rb::method(ruby, class, c"initialize", -1, initialize)?;
    rb::method(ruby, class, c"bitmap", 0, bitmap)?;
    rb::method(ruby, class, c"bitmap=", 1, set_bitmap)?;
    if runtime == Runtime::Rgss1 {
        rb::method(ruby, class, c"visible", 0, visible)?;
    } else {
        rb::method(ruby, class, c"visible", 0, visible_rgss2)?;
    }
    rb::method(ruby, class, c"visible=", 1, set_visible)?;
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
    Ok(class)
}
