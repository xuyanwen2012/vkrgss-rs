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

use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{Raise, Raised, Ruby};

use crate::guest::{self, Classes};
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

/// What a `Color`, `Tone` or `Rect` object is a live view of: a property of a scene element.
///
/// A sprite's `color`, for one, is an object whose changes reach the sprite. The scene
/// classes add the variants; until then no object has an owner.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Owner {}

/// The data of a `Color`, `Tone` or `Rect` object: the value, and the owner a change of the
/// value is passed on to.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Owned<T> {
    value: T,
    owner: Option<Owner>,
}

impl<T: Copy> Owned<T> {
    /// A value without an owner.
    pub(crate) fn new(value: T) -> Owned<T> {
        Owned { value, owner: None }
    }

    pub(crate) fn value(&self) -> T {
        self.value
    }

    /// Replaces the value and passes the change on to the owner.
    pub(crate) fn set(&mut self, value: T) {
        self.value = value;
        if let Some(owner) = self.owner {
            match owner {}
        }
    }
}
