//! The emulated functions behind `Win32API`.
//!
//! The prelude's `Win32API#call` passes the library name, the function name and the arguments
//! to the host function `vkxp_win32api`. That looks the pair up in [`TABLE`] and runs the
//! entry on the guest state, or returns `nil` for a function outside the table, which the
//! prelude turns into 0 and one report.
//!
//! An integer argument reaches an entry as an `i64` and `nil` as 0, the null pointer. A Ruby
//! string reaches it as a copy of its bytes that the entry may overwrite but not resize; a
//! copy that was changed is put back into the very string object, so a script finds the
//! result in the buffer it passed.

use std::ffi::c_long;

use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{Arity, HostResult, Raised, Ruby};

use crate::guest::Guest;
use crate::rb;

/// One argument of a call.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Arg {
    /// An integer, or 0 for `nil`.
    Int(i64),
    /// The bytes of a Ruby string, of fixed length.
    Buffer(Box<[u8]>),
}

/// An emulated function: it reads its arguments, writes into buffers and returns the value
/// of the call.
type Shim = fn(&mut Guest, &mut [Arg]) -> i64;

/// The emulated functions: library name in lower case without `.dll`, function name, entry.
const TABLE: &[(&str, &str, Shim)] = &[("kernel32", "RtlMoveMemory", move_memory)];

/// The entry for a function of a library. The library name matches without regard to letter
/// case and with or without `.dll`; like the original `Win32API`, a function name also finds
/// the entry of its ANSI variant, whose name ends in `A`.
fn find(library: &str, function: &str) -> Option<Shim> {
    let library = library.trim().to_ascii_lowercase();
    let library = library.strip_suffix(".dll").unwrap_or(&library);
    let entry = |name: &str| {
        TABLE
            .iter()
            .find(|(in_library, entry, _)| *in_library == library && *entry == name)
            .map(|&(_, _, shim)| shim)
    };
    entry(function).or_else(|| entry(&format!("{function}A")))
}

/// The integer argument at `index`; 0 for a buffer and for an argument that was not given.
fn int(args: &[Arg], index: usize) -> i64 {
    match args.get(index) {
        Some(&Arg::Int(value)) => value,
        _ => 0,
    }
}

/// The buffer argument at `index`, or `None` for an integer such as the null pointer.
fn buffer(args: &[Arg], index: usize) -> Option<&[u8]> {
    match args.get(index) {
        Some(Arg::Buffer(bytes)) => Some(bytes),
        _ => None,
    }
}

/// The buffer argument at `index`, to write into.
fn buffer_mut(args: &mut [Arg], index: usize) -> Option<&mut [u8]> {
    match args.get_mut(index) {
        Some(Arg::Buffer(bytes)) => Some(bytes),
        _ => None,
    }
}

/// `RtlMoveMemory(destination, source, length)` between two strings. An address given as an
/// integer names memory the guest does not have, and nothing is copied.
fn move_memory(_guest: &mut Guest, args: &mut [Arg]) -> i64 {
    let Some(source) = buffer(args, 1).map(<[u8]>::to_vec) else {
        return 0;
    };
    let length = usize::try_from(int(args, 2)).unwrap_or(0);
    if let Some(destination) = buffer_mut(args, 0) {
        let length = length.min(source.len()).min(destination.len());
        destination[..length].copy_from_slice(&source[..length]);
    }
    0
}

/// `vkxp_win32api(library, function, *arguments)`: the value of the emulated function, or
/// `nil` when the function is not in the table.
fn call(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    rb::check_count(ruby, args, 2, usize::MAX)?;
    let library = rb::text(ruby, args[0])?;
    let function = rb::text(ruby, args[1])?;
    let Some(shim) = find(&library, &function) else {
        return Ok(ffi::Qnil);
    };
    let values = &args[2..];
    let mut converted = Vec::with_capacity(values.len());
    for &value in values {
        converted.push(if ffi::nil_p(value) {
            Arg::Int(0)
        } else if let Some(bytes) = rb::string_bytes(ruby, value) {
            Arg::Buffer(bytes.into_boxed_slice())
        } else {
            Arg::Int(rb::long(ruby, value)?)
        });
    }
    let before = converted.clone();
    let result = Guest::with(|guest| shim(guest, &mut converted));
    for ((&value, after), before) in values.iter().zip(&converted).zip(&before) {
        if let Arg::Buffer(bytes) = after
            && after != before
        {
            let contents = rb::string_new(ruby, bytes)?;
            // SAFETY: `value` is an argument of this call and `contents` was just returned
            // and is in a local.
            unsafe { ruby.funcall(value, c"replace", &[contents]) }?;
        }
    }
    Ok(ffi::long2fix(result as c_long))
}

/// Defines the host function the prelude's `Win32API` builds on.
pub(crate) fn define(ruby: &Ruby) -> Result<(), Raised> {
    ruby.define_global_function_with_arity(c"vkxp_win32api", Arity::Any, call)
}
