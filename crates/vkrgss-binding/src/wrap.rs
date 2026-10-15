//! Ruby objects that wrap Rust values.
//!
//! Each RGSS class is a Ruby data class with an allocator: every instance, also one made by
//! `allocate`, `clone` or `Marshal`, is a `T_DATA` object whose data pointer is a boxed Rust
//! struct of the class's [`Wrapped`] type. The box is freed when the object is collected.
//!
//! A [`Handle`] is the way to the struct of one object. It is valid while the object is
//! alive, which a host function has for its receiver and arguments for the length of the
//! call. The closures given to a handle do not call into Ruby.

use std::ffi::{CStr, c_void};
use std::ptr::NonNull;

use vkrgss_core::{
    Bitmaps, Color, PlaneId, Rect, Scene, SpriteId, Tone, ViewportId, WindowCall, WindowId,
};
use vkrgss_ruby_sys::ffi::{self, VALUE};
use vkrgss_ruby_sys::{Raise, Raised, Ruby};

use crate::guest::{self, Classes, Guest};
use crate::rb;

/// A Rust type that is the data of the instances of one Ruby class.
pub(crate) trait Wrapped: Sized + 'static {
    /// The name of the Ruby class, for messages.
    const CLASS_NAME: &'static str;

    /// The Ruby class among the installed ones.
    fn class(classes: &Classes) -> VALUE;

    /// The data of an object the allocator has made and `initialize` has not seen yet. Must
    /// not panic.
    fn blank() -> Self;

    /// Called with the data when the collector frees the object. Collection can start while
    /// a method holds the guest state, so this must not touch it; see
    /// [`defer_free`](guest::defer_free). Must not panic.
    fn collected(self) {}
}

unsafe extern "C" fn alloc<T: Wrapped>(class: VALUE) -> VALUE {
    let data = Box::into_raw(Box::new(T::blank()));
    // SAFETY: Ruby calls the allocator on the interpreter thread with the live class. Only
    // a raw pointer is alive in this frame, so a `NoMemoryError` jumps over no destructor
    // (and leaks the box). `free::<T>` takes back exactly this pointer.
    unsafe { ffi::rb_data_object_alloc(class, data.cast(), None, Some(free::<T>)) }
}

unsafe extern "C" fn free<T: Wrapped>(data: *mut c_void) {
    if data.is_null() {
        return;
    }
    // SAFETY: the collector passes the data pointer of an object made by `alloc::<T>`, once.
    let data = unsafe { Box::from_raw(data.cast::<T>()) };
    T::collected(*data);
}

/// Defines the top-level class `name` whose instances wrap a `T`.
pub(crate) fn define_class<T: Wrapped>(ruby: &Ruby, name: &CStr) -> Result<VALUE, Raised> {
    // SAFETY: `Object` is a live class.
    let class = unsafe { ruby.define_class(name, rb::object_class(ruby)) }?;
    rb::define_alloc(ruby, class, alloc::<T>)?;
    Ok(class)
}

/// Defines the class `name` inside the class or module `outer`, whose instances wrap a `T`.
pub(crate) fn define_class_under<T: Wrapped>(
    ruby: &Ruby,
    outer: VALUE,
    name: &CStr,
) -> Result<VALUE, Raised> {
    // SAFETY: `outer` is a live class or module by the condition of `rb`, and `Object` is a
    // live class.
    let class = unsafe { ruby.define_class_under(outer, name, rb::object_class(ruby)) }?;
    rb::define_alloc(ruby, class, alloc::<T>)?;
    Ok(class)
}

/// The way to the Rust data of one live Ruby object.
pub(crate) struct Handle<T> {
    data: NonNull<T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> Handle<T> {
    /// A copy of the data.
    pub(crate) fn get(self) -> T
    where
        T: Clone,
    {
        // SAFETY: the object is alive (see the module documentation), its data pointer is
        // the box `alloc` made, and no other reference to the data exists: every access
        // goes through a handle and ends before the next begins.
        unsafe { self.data.as_ref().clone() }
    }

    /// Runs `access` on the data. `access` must not call into Ruby or use another handle.
    pub(crate) fn with<R>(self, access: impl FnOnce(&mut T) -> R) -> R {
        // SAFETY: as in `get`; `access` does not reach the data another way.
        access(unsafe { &mut *self.data.as_ptr() })
    }
}

/// The handle of `value` if it is an instance of the class of `T`, or of a subclass.
pub(crate) fn find<T: Wrapped>(_ruby: &Ruby, value: VALUE) -> Option<Handle<T>> {
    let class = T::class(&guest::classes());
    // SAFETY: the value is live by the condition of `rb`, and the class is installed. An
    // instance of the class comes from its allocator (Ruby rejects any other allocation), so
    // a data object of that class has a `T` behind its data pointer.
    unsafe {
        if ffi::special_const_p(value)
            || ffi::builtin_type(value) != ffi::T_DATA
            || !ffi::rtest(ffi::rb_obj_is_kind_of(value, class))
        {
            return None;
        }
        NonNull::new(ffi::data_ptr(value).cast::<T>()).map(|data| Handle { data })
    }
}

/// The handle of an argument that must be an instance of the class of `T`. Anything else
/// raises `TypeError` naming the expected class.
pub(crate) fn expect<T: Wrapped>(ruby: &Ruby, value: VALUE) -> Result<Handle<T>, Raise> {
    find(ruby, value).ok_or_else(|| {
        rb::wrong_type(ruby, value, |class| {
            format!("wrong argument type {class} (expected {})", T::CLASS_NAME)
        })
    })
}

/// A new instance of the class of `T` with this data, without `initialize`.
pub(crate) fn new_object<T: Wrapped>(ruby: &Ruby, data: T) -> Result<VALUE, Raise> {
    let object = rb::allocate(ruby, T::class(&guest::classes()))?;
    expect::<T>(ruby, object)?.with(|slot| *slot = data);
    Ok(object)
}

/// What a `Color`, `Tone` or `Rect` object is a live view of: a property of a scene element,
/// named by the element's handle and the property.
///
/// A sprite's `color`, for one, is an object whose changes reach the sprite.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Owner {
    SpriteColor(SpriteId),
    SpriteTone(SpriteId),
    SpriteSrcRect(SpriteId),
    ViewportColor(ViewportId),
    ViewportTone(ViewportId),
    ViewportRect(ViewportId),
    PlaneColor(PlaneId),
    PlaneTone(PlaneId),
    WindowCursorRect(WindowId),
    /// The tone of a window of RGSS3.
    WindowTone(WindowId),
}

/// A value that can be a property of a scene element.
pub(crate) trait Property: Copy {
    /// Makes this the value of the property `owner` names. A disposed element, and a
    /// property of another type, are left alone. `bitmaps` is for the property whose
    /// assignment draws: the cursor rectangle of a window of RGSS2 or RGSS3.
    fn store(self, owner: Owner, scene: &mut Scene, bitmaps: &mut Bitmaps);
}

impl Property for Color {
    fn store(self, owner: Owner, scene: &mut Scene, _bitmaps: &mut Bitmaps) {
        match owner {
            Owner::SpriteColor(id) => {
                let _ = scene.sprite_mut(id).map(|sprite| sprite.color = self);
            }
            Owner::ViewportColor(id) => {
                let _ = scene.viewport_mut(id).map(|viewport| viewport.color = self);
            }
            Owner::PlaneColor(id) => {
                let _ = scene.plane_mut(id).map(|plane| plane.color = self);
            }
            _ => {}
        }
    }
}

impl Property for Tone {
    fn store(self, owner: Owner, scene: &mut Scene, _bitmaps: &mut Bitmaps) {
        match owner {
            Owner::SpriteTone(id) => {
                let _ = scene.sprite_mut(id).map(|sprite| sprite.tone = self);
            }
            Owner::ViewportTone(id) => {
                let _ = scene.viewport_mut(id).map(|viewport| viewport.tone = self);
            }
            Owner::PlaneTone(id) => {
                let _ = scene.plane_mut(id).map(|plane| plane.tone = self);
            }
            Owner::WindowTone(id) => {
                let _ = scene.window_mut(id).map(|window| window.set_tone(self));
            }
            _ => {}
        }
    }
}

impl Property for Rect {
    fn store(self, owner: Owner, scene: &mut Scene, bitmaps: &mut Bitmaps) {
        match owner {
            Owner::SpriteSrcRect(id) => {
                let _ = scene.sprite_mut(id).map(|sprite| sprite.src_rect = self);
            }
            Owner::ViewportRect(id) => {
                let _ = scene.viewport_mut(id).map(|viewport| viewport.rect = self);
            }
            Owner::WindowCursorRect(id) => {
                let _ = scene
                    .window_mut(id)
                    .map(|window| window.call(bitmaps, WindowCall::CursorRect(self)));
            }
            _ => {}
        }
    }
}

/// Interns `owner` and then `observer` when a game of RGSS2 has not done so yet. The original
/// runtime of VX interns the two at the first change of a `Color`, `Tone` or `Rect` (`set`, a
/// writer, `Rect#empty`) and at the first `Font.new`, `Bitmap.new`, `Sprite.new`,
/// `Viewport.new`, `Window.new`, `Plane.new` or `Graphics.snap_to_bitmap`, and they take the
/// next numbers of the symbol sequence; those functions call this first. vkrgss keeps an owner
/// in [`Owned`] and has no use for the names.
///
/// Provenance: oracle measurement 2026-10-12, RGSS202E version 2.0.2.1, `vx-v3-save-REPORT.md`
/// section 2. Not inside [`Guest::with`].
pub(crate) fn intern_owner_names(ruby: &Ruby) -> Result<(), Raised> {
    if Guest::with(|guest| std::mem::take(&mut guest.owner_names_pending)) {
        rb::intern(ruby, c"owner")?;
        rb::intern(ruby, c"observer")?;
    }
    Ok(())
}

/// The data of a `Color`, `Tone` or `Rect` object: the value, and the owner a change of the
/// value is passed on to.
///
/// The object keeps its own copy of the value, so it goes on working as a plain value when
/// its element is disposed.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Owned<T> {
    value: T,
    owner: Option<Owner>,
}

impl<T: Property> Owned<T> {
    /// A value without an owner.
    pub(crate) fn new(value: T) -> Owned<T> {
        Owned { value, owner: None }
    }

    /// The value of the property `owner` names, which follows every change of this value.
    pub(crate) fn of(value: T, owner: Owner) -> Owned<T> {
        Owned {
            value,
            owner: Some(owner),
        }
    }

    pub(crate) fn value(&self) -> T {
        self.value
    }

    /// Replaces the value and passes the change on to the owner. With an owner this enters
    /// the guest state, so it must not be called from inside [`Guest::with`].
    pub(crate) fn set(&mut self, value: T) {
        self.value = value;
        if let Some(owner) = self.owner {
            Guest::with(|guest| value.store(owner, &mut guest.scene, &mut guest.bitmaps));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{self, Harness};

    fn harness() -> &'static Harness {
        testing::start_as(vkrgss_vfs::Runtime::Rgss1)
    }

    #[test]
    fn an_owned_value_follows_its_element_and_outlives_it() {
        let red = Color::new(255.0, 0.0, 0.0, 255.0);
        let blue = Color::new(0.0, 0.0, 255.0, 255.0);
        let (in_scene, after_disposal) = harness().run(move |_ruby| {
            let sprite = Guest::with(|guest| guest.scene.create_sprite(None)).unwrap();
            let mut color = Owned::of(Color::TRANSPARENT, Owner::SpriteColor(sprite));
            color.set(red);
            let in_scene = Guest::with(|guest| guest.scene.sprite(sprite).map(|s| s.color));
            Guest::with(|guest| guest.scene.dispose_sprite(sprite));
            // The handle names nothing now: the object is a plain value.
            color.set(blue);
            (in_scene, color.value())
        });
        assert_eq!(in_scene, Ok(red));
        assert_eq!(after_disposal, blue);
    }

    #[test]
    fn a_property_of_another_type_is_left_alone() {
        let rect = harness().run(|_ruby| {
            Guest::with(|guest| {
                let sprite = guest.scene.create_sprite(None).unwrap();
                let color = Owner::SpriteColor(sprite);
                Rect::new(1, 2, 3, 4).store(color, &mut guest.scene, &mut guest.bitmaps);
                let rect = guest.scene.sprite(sprite).unwrap().src_rect;
                guest.scene.dispose_sprite(sprite);
                rect
            })
        });
        assert_eq!(rect, Rect::default());
    }
}
