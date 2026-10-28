//! The protected call of this interpreter.

use crate::Raised;
use crate::capi::CApi;
use crate::ffi::{self, ProtectFn, VALUE};

/// [`vkrgss_ruby_glue::protect`] over this interpreter: runs `body(data)` under `rb_protect`,
/// so that a jump out of `body` ends inside C and comes back as a [`Raised`], with `$!` as it
/// was before the call, instead of crossing the caller's frames.
///
/// `data` is the plain data of the operation, inputs and the outputs that are not values;
/// `body` receives its address as the argument and returns the value of the operation.
///
/// # Safety
///
/// As for [`vkrgss_ruby_glue::protect`]: on the interpreter thread, with a `body` that casts
/// its argument back to `*mut T`, reads only live values and pointers from there, does not
/// unwind and holds no value with a destructor while it calls anything that can raise.
pub unsafe fn protect<T>(data: &mut T, body: ProtectFn) -> Result<VALUE, Raised> {
    // SAFETY: the caller keeps the contract of the shared function.
    unsafe { vkrgss_ruby_glue::protect::<CApi, T>(data, body) }
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
