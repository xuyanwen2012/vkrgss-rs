//! Build of the embedded Ruby and raw FFI.
//!
//! Side: guest.

// The `Zlib` extension calls into the zlib that `libz-sys` builds; naming the crate is what
// makes rustc link it.
extern crate libz_sys;

pub mod ffi;
mod interp;
mod protect;
mod signals;

pub use interp::{Interpreter, MIN_STACK_SIZE, Options, Ruby, STACK_HEADROOM, StartError};
pub use protect::Raised;
pub use signals::SIGNALS_TAKEN_BY_RUBY;
