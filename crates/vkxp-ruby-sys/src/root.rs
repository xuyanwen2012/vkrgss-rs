//! Keeping Ruby values alive from host memory.

use std::marker::PhantomData;

use crate::ffi::{self, VALUE};
use crate::interp::Ruby;
use crate::protect::{self, Raised};

/// A Ruby value that the collector treats as reachable for as long as the `Root` exists.
///
/// The collector is conservative about the interpreter thread's stack: a value in a local
/// there needs nothing. It does not see the host heap, so a value kept in a `Box`, a `Vec` or
/// a field of a host object must be a `Root`, or be reachable from a Ruby object.
///
/// The value sits in a heap slot whose address is registered with the collector, so the
/// `Root` itself can move freely. It is neither `Send` nor `Sync`: it is created, used and
/// dropped on the interpreter thread, while the interpreter is alive.
pub struct Root {
    slot: Box<VALUE>,
    not_send: PhantomData<*mut ()>,
}

impl Root {
    /// The rooted value.
    pub fn get(&self) -> VALUE {
        *self.slot
    }

    /// Replaces the rooted value. The previous one is no longer kept alive by this root.
    ///
    /// # Safety
    ///
    /// `value` must be a special constant or a live object.
    pub unsafe fn set(&mut self, value: VALUE) {
        *self.slot = value;
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        // SAFETY: a `Root` cannot leave the interpreter thread, and this is the address that
        // `Ruby::root` registered.
        unsafe { ffi::rb_gc_unregister_address(&mut *self.slot) };
    }
}

unsafe extern "C" fn register_body(data: VALUE) -> VALUE {
    // SAFETY: `Ruby::root` passes the address of a heap slot that holds a live value and
    // stays valid until it is unregistered. Registering allocates a list node, which raises
    // `NoMemoryError` when it fails, hence the protection.
    unsafe { ffi::rb_gc_register_address(data as *mut VALUE) };
    ffi::Qnil
}

impl Ruby {
    /// Registers a value as a root of the collector.
    ///
    /// The error is the `NoMemoryError` of a failed registration; nothing is registered
    /// then.
    ///
    /// # Safety
    ///
    /// `value` must be a special constant or a live object, and the returned `Root` must be
    /// dropped before the interpreter is.
    pub unsafe fn root(&self, value: VALUE) -> Result<Root, Raised> {
        let mut slot = Box::new(value);
        // SAFETY: `&self` proves the thread. `protect` hands `register_body` the address of
        // the slot itself, which is stable for the life of the box; `Drop` unregisters it
        // before the box is freed. On failure the slot was not registered and the box is
        // simply dropped, with the value still held by the caller's local.
        unsafe { protect::protect(&mut *slot, register_body) }?;
        Ok(Root {
            slot,
            not_send: PhantomData,
        })
    }
}
