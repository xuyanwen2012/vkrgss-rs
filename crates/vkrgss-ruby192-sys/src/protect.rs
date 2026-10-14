//! Running a piece of C-API calls so that a Ruby `longjmp` ends inside C.
//!
//! The pattern is always the same: the plain data of one operation sits in a struct on the
//! caller's stack, and an `extern "C"` function that does nothing but C-API calls on that data
//! runs under `rb_protect`. The only frames a raise can jump over are that function's, and it
//! holds no Rust value with a destructor.

use std::ffi::{CStr, c_int};

use crate::ffi::{self, VALUE};

/// A jump out of Ruby code that [`rb_protect`](ffi::rb_protect) stopped: almost always a
/// raised exception, otherwise a `throw`, `break` or similar that found no target inside the
/// protected call.
///
/// It is plain data. `exception` is not rooted: keep the `Raised` in a local on the
/// interpreter thread, where the collector's stack scan finds it, and do not store it on the
/// heap across a collection.
///
/// For a jump that is not a raise the interpreter keeps what the jump carries (the tag and
/// the value of a `throw`) as its current error until the jump is continued, which is what
/// [`Raise::Resume`](crate::Raise::Resume) does, or until
/// [`Ruby::describe`](crate::Ruby::describe) or
/// [`Ruby::set_current_error`](crate::Ruby::set_current_error) clears it. Continue such a
/// jump before anything else runs in the interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raised {
    /// The status of `rb_protect`; [`TAG_RAISE`](ffi::TAG_RAISE) for an exception.
    pub state: c_int,
    /// The exception object, or `nil` when the jump was not a raise.
    pub exception: VALUE,
}

/// Runs `body(data)` under `rb_protect`: a jump out of `body` ends inside C and comes back as
/// a [`Raised`], instead of crossing the caller's frames. After a raise `$!` is as it was
/// before the call.
///
/// `data` is the plain data of the operation, inputs and the outputs that are not values;
/// `body` receives its address as the argument and returns the value of the operation.
///
/// # Safety
///
/// - The caller is on the interpreter thread.
/// - `body` casts its argument back to `*mut T` and uses it as nothing else. Every `VALUE`
///   and pointer it reads from there is live for the call.
/// - `body` does not unwind: it does not panic and calls nothing that can.
/// - `body` holds no value with a destructor while it calls anything that can raise, since
///   the jump frees its frame without running anything in it.
pub unsafe fn protect<T>(
    data: &mut T,
    body: unsafe extern "C" fn(VALUE) -> VALUE,
) -> Result<VALUE, Raised> {
    let mut state: c_int = 0;
    // SAFETY: on the interpreter thread; reading the current error raises nothing.
    let before = unsafe { ffi::rb_errinfo() };
    // SAFETY: `data` outlives the call, and `body` treats the argument as `*mut T` by the
    // contract of this function. `rb_protect` returns normally whatever `body` does.
    let value = unsafe { ffi::rb_protect(body, std::ptr::from_mut(data) as VALUE, &mut state) };
    if state == 0 {
        return Ok(value);
    }
    if state != ffi::TAG_RAISE && state != ffi::TAG_FATAL {
        // The current error holds what the jump carries, and continuing the jump needs it.
        return Err(Raised {
            state,
            exception: ffi::Qnil,
        });
    }
    // SAFETY: as above. Taking the exception out leaves `$!` as the call found it, which is
    // what a `rescue` that ends does; `before` has been in a local since it was read and was
    // the current error, so setting it again cannot raise.
    let exception = unsafe {
        let pending = ffi::rb_errinfo();
        ffi::rb_set_errinfo(before);
        pending
    };
    Err(Raised { state, exception })
}

struct Funcall {
    recv: VALUE,
    method: *const std::ffi::c_char,
    argc: c_int,
    argv: *const VALUE,
}

unsafe extern "C" fn funcall_body(data: VALUE) -> VALUE {
    // SAFETY: `funcall` passes a pointer to a `Funcall` that outlives this call, with a
    // NUL-terminated name and `argc` values at `argv`.
    unsafe {
        let call = &*(data as *const Funcall);
        ffi::rb_funcall2(call.recv, ffi::rb_intern(call.method), call.argc, call.argv)
    }
}

/// Calls a method by name, ignoring its visibility, and stops any jump out of it.
///
/// # Safety
///
/// Interpreter thread only; `recv` and `args` must be live Ruby values.
pub(crate) unsafe fn funcall(recv: VALUE, method: &CStr, args: &[VALUE]) -> Result<VALUE, Raised> {
    let mut call = Funcall {
        recv,
        method: method.as_ptr(),
        argc: c_int::try_from(args.len()).expect("fewer than 2^31 arguments"),
        argv: args.as_ptr(),
    };
    // SAFETY: `funcall_body` fits the contract of `protect`, and the caller guarantees the
    // thread and the values.
    unsafe { protect(&mut call, funcall_body) }
}
