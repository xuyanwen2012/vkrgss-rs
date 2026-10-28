//! The protected call of this interpreter.

use crate::capi::CApi;
use crate::ffi::{ProtectFn, VALUE};
use crate::Raised;

/// [`vkrgss_ruby_glue::protect`] over this interpreter: runs `body(data)` under `rb_protect`,
/// so that a jump out of `body` ends inside C and comes back as a [`Raised`] instead of crossing
/// the caller's frames. After a raise `$!` is as it was before the call.
///
/// `data` is the plain data of the operation, inputs and the outputs that are not values;
/// `body` receives its address as the argument and returns the value of the operation.
///
/// # Safety
///
/// As for [`vkrgss_ruby_glue::protect`]: on a thread that holds the interpreter, with a `body` that casts
/// its argument back to `*mut T`, reads only live values and pointers from there, does not
/// unwind and holds no value with a destructor while it calls anything that can raise.
pub unsafe fn protect<T>(data: &mut T, body: ProtectFn) -> Result<VALUE, Raised> {
    // SAFETY: the caller keeps the contract of the shared function.
    unsafe { vkrgss_ruby_glue::protect::<CApi, T>(data, body) }
}
