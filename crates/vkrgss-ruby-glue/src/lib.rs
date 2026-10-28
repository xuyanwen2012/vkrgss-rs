//! The bridge between Rust and an embedded Ruby that both interpreter crates share.
//!
//! Side: guest.
//!
//! `vkrgss-ruby-sys` embeds Ruby 1.8.7 and `vkrgss-ruby192-sys` Ruby 1.9.2. The Rust code
//! that follows the rules of entering and leaving Ruby is the same for both and lives here
//! once: host functions and their trampolines, the definitions that hang them on classes,
//! rooting, the protected call and the calls built on it. It is generic over
//! [`Interpreter`], the part of the C API it needs, which each interpreter crate implements on
//! a unit type with its own declarations. That crate then offers the items of this one under
//! its own paths: [`Ruby`] with its unit type, [`Root`] likewise, and [`protect`] for its
//! interpreter; the rules themselves are documented there.
//!
//! This crate has no C code and exports no unmangled symbol, so that both interpreter
//! crates may depend on it although no program links the two together.

mod call;
mod host;
mod interpreter;
mod protect;
mod root;
mod ruby;

pub use call::RubyError;
pub use host::{Arity, HostResult, Raise};
pub use interpreter::Interpreter;
pub use protect::{Raised, protect};
pub use root::Root;
pub use ruby::Ruby;

/// A Ruby value: an immediate (fixnum, symbol, `nil`, `true`, `false`) or a pointer to a heap
/// object. Both interpreters define it as a machine word.
pub type VALUE = usize;

/// An interned name.
pub type ID = usize;

/// `VALUE (*)(ANYARGS)`: a method implementation whose real signature depends on the `argc`
/// it is registered with.
pub type AnyArgsFn = unsafe extern "C" fn() -> VALUE;

/// The signature Ruby calls for a method registered with `argc == -1`.
pub type ArgvFn =
    unsafe extern "C" fn(argc: std::ffi::c_int, argv: *const VALUE, recv: VALUE) -> VALUE;

/// The callback of `rb_protect`.
pub type ProtectFn = unsafe extern "C" fn(arg: VALUE) -> VALUE;
