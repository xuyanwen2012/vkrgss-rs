//! Running a piece of C-API calls so that a Ruby `longjmp` ends inside C.
//!
//! The pattern is always the same: the plain data of one operation sits in a struct on the
//! caller's stack, and an `extern "C"` function that does nothing but C-API calls on that data
//! runs under `rb_protect`. The only frames a raise can jump over are that function's, and it
//! holds no Rust value with a destructor.

use std::ffi::c_int;

use crate::ffi::{self, VALUE};

/// A jump out of Ruby code that [`rb_protect`](ffi::rb_protect) stopped: almost always a
/// raised exception, otherwise a `throw`, `break` or similar that found no target inside the
/// protected call.
///
/// It is plain data. `exception` is not rooted: keep the `Raised` in a local on the
/// interpreter thread, where the collector's stack scan finds it, and do not store it on the
/// heap across a collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raised {
    /// The status of `rb_protect`; [`TAG_RAISE`](ffi::TAG_RAISE) for an exception.
    pub state: c_int,
    /// The exception object, or `nil` when the jump was not a raise.
    pub exception: VALUE,
}

/// Runs `body(data)` under `rb_protect`.
///
/// # Safety
///
/// Interpreter thread only. `body` must cast its argument back to `*mut T`, must not unwind,
/// and must hold no value with a destructor while it calls anything that can raise.
pub(crate) unsafe fn protect<T>(
    data: &mut T,
    body: unsafe extern "C" fn(VALUE) -> VALUE,
) -> Result<VALUE, Raised> {
    let mut state: c_int = 0;
    // SAFETY: `data` outlives the call, and `body` treats the argument as `*mut T` by the
    // contract of this function. `rb_protect` returns normally whatever `body` does.
    let value = unsafe { ffi::rb_protect(body, std::ptr::from_mut(data) as VALUE, &mut state) };
    if state == 0 {
        return Ok(value);
    }
    // SAFETY: on the interpreter thread `ruby_errinfo` is only touched by the interpreter,
    // which is not running now. Taking the exception out leaves `$!` clean for what follows.
    let exception = unsafe {
        let pending = ffi::ruby_errinfo;
        ffi::ruby_errinfo = ffi::Qnil;
        pending
    };
    let is_raise = state == ffi::TAG_RAISE || state == ffi::TAG_FATAL;
    Err(Raised {
        state,
        exception: if is_raise { exception } else { ffi::Qnil },
    })
}

unsafe extern "C" fn call_void_body(data: VALUE) -> VALUE {
    // SAFETY: `call_void` passes a pointer to a function pointer of this type.
    unsafe {
        let function = *(data as *const unsafe extern "C" fn());
        function();
    }
    ffi::Qnil
}

/// Calls a C function that takes and returns nothing, and stops any jump out of it.
///
/// # Safety
///
/// Interpreter thread only; `function` must be safe to call there.
pub(crate) unsafe fn call_void(function: unsafe extern "C" fn()) -> Result<(), Raised> {
    let mut function = function;
    // SAFETY: `call_void_body` fits the contract of `protect`.
    unsafe { protect(&mut function, call_void_body) }.map(|_| ())
}
