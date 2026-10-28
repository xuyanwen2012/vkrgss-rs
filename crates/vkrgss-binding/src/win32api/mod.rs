//! The emulated functions behind `Win32API`, and the class `Win32API` of RGSS3.
//!
//! In RGSS1 and RGSS2 the prelude's `Win32API#call` passes the library name, the function
//! name and the arguments to the host function `vkrgss_win32api`. That looks the pair up in
//! [`TABLE`] and runs the entry on the guest state, or returns `nil` for a function outside
//! the table, which the prelude turns into 0 and one report.
//!
//! In RGSS3 `Win32API` is a class defined here, as the original runtime of RGSS3 has it
//! (oracle measurement 2026-10-13, RGSS301 3.0.1.1, API report 5.8): it finds the library,
//! reads the import and the export, counts and converts the parameters with the errors of
//! the original, and runs the entry of [`TABLE`] or reports the function once.
//!
//! An integer argument reaches an entry as an `i64` and `nil` as 0, the null pointer. A Ruby
//! string reaches it as a copy of its bytes that the entry may overwrite but not resize; a
//! copy that was changed is put back into the very string object, so a script finds the
//! result in the buffer it passed.
//!
//! [`TABLE`]: table::TABLE

use std::collections::HashSet;
use std::ffi::{CStr, c_long};

use vkrgss_core::InputState;
use vkrgss_ruby_sys::ffi::{self, VALUE};
use vkrgss_ruby_sys::{Arity, HostResult, Raised, Ruby};
use vkrgss_vfs::Runtime;

use crate::guest::Guest;
use crate::rb;

mod input;
mod profile;
mod rgss3;
mod system;
mod table;
mod text;

#[cfg(test)]
mod tests;

use rgss3::define_rgss3;
use table::{Arg, find, run_shim};

/// What the emulated functions keep between calls.
#[derive(Debug, Default)]
pub(crate) struct State {
    /// The tick count: the milliseconds of the frames shown so far.
    milliseconds: u64,
    /// What the frames shown so far leave over of a millisecond, in units of one over the
    /// frame rate.
    remainder: u64,
    /// The display count of `ShowCursor`.
    cursor: i64,
    /// The toggle bit of each virtual key, which `GetKeyState` and `GetKeyboardState` report.
    toggles: KeySet,
    /// The keys `SetKeyboardState` said are down, until the next `Input.update`.
    set_down: Option<KeySet>,
    /// The library and function pairs outside the table that a `Win32API` of RGSS3 has
    /// reported.
    reported: HashSet<String>,
}

/// One bit for each of the 256 virtual-key codes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct KeySet([u64; 4]);

impl KeySet {
    fn has(&self, key: u8) -> bool {
        self.0[usize::from(key >> 6)] & (1 << (key & 63)) != 0
    }

    fn set(&mut self, key: u8, on: bool) {
        let bit = 1 << (key & 63);
        let word = &mut self.0[usize::from(key >> 6)];
        *word = if on { *word | bit } else { *word & !bit };
    }
}

impl State {
    /// Advances the tick count by one frame of `frame_rate` frames per second. The count
    /// follows the frames, not the clock, so a run is the same every time; and it follows
    /// the frames shown, not `Graphics.frame_count`, which a script may set back.
    pub(crate) fn frame_shown(&mut self, frame_rate: u64) {
        self.remainder += 1000;
        self.milliseconds += self.remainder / frame_rate.max(1);
        self.remainder %= frame_rate.max(1);
    }

    /// `Input.update` has latched a snapshot into `input`: the toggle of every key that is
    /// down in it and was not in the one before flips, and what `SetKeyboardState` said is
    /// down is over.
    pub(crate) fn input_updated(&mut self, input: &InputState) {
        for key in 0..=255u8 {
            if input.is_key_new(key) {
                self.toggles.set(key, !self.toggles.has(key));
            }
        }
        self.set_down = None;
    }

    /// Whether the pointer is to be seen: while the display count is 0 or more.
    pub(crate) fn pointer_visible(&self) -> bool {
        self.cursor >= 0
    }
}

/// `vkrgss_win32api(library, function, *arguments)`: the value of the emulated function, or
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
    let mut strings = Vec::with_capacity(values.len());
    for &value in values {
        if ffi::nil_p(value) {
            converted.push(Arg::Int(0));
            strings.push(None);
        } else if let Some(bytes) = rb::string_bytes(ruby, value) {
            converted.push(Arg::Buffer(bytes.into_boxed_slice()));
            strings.push(Some(value));
        } else {
            converted.push(Arg::Int(rb::long(ruby, value)?));
            strings.push(None);
        }
    }
    let result = run_shim(ruby, shim, &strings, converted, false)?;
    Ok(ffi::long2fix(result as c_long))
}

/// The names of the instance variables in which the `Win32API` of the original runtime of VX
/// keeps the library, the function and the argument types, in the order it interns them.
const NAMES: [&CStr; 6] = [
    c"__hdll__",
    c"__dll__",
    c"__dllname__",
    c"__proc__",
    c"__import__",
    c"__export__",
];

/// `vkrgss_win32api_names`: called by `Win32API#initialize`. In a game of RGSS2 it interns the
/// six names the original interns at the first `Win32API.new` that loads its library, where
/// they take the next numbers of the symbol sequence. vkrgss loads no library and keeps what
/// it needs in variables of its own, so it has no use for any of the six; they are interned
/// only so that the names a game makes afterwards have the numbers of the original.
///
/// Provenance: oracle measurement 2026-10-12, RGSS202E version 2.0.2.1, `vx-v3-save-REPORT.md`
/// section 2.
fn names(ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    if Guest::with(ruby, |guest| guest.runtime.run.runtime) == Runtime::Rgss2 {
        for name in NAMES {
            rb::intern(ruby, name)?;
        }
    }
    Ok(ffi::Qnil)
}

/// Defines `Win32API` for a game of `runtime`: in RGSS1 and RGSS2 the host functions the
/// class of the prelude builds on, in RGSS3 the class itself.
pub(crate) fn define(ruby: &Ruby, runtime: Runtime) -> Result<(), Raised> {
    if runtime == Runtime::Rgss3 {
        return define_rgss3(ruby);
    }
    ruby.define_global_function_with_arity(c"vkrgss_win32api_names", Arity::Fixed(0), names)?;
    ruby.define_global_function_with_arity(c"vkrgss_win32api", Arity::Any, call)
}
