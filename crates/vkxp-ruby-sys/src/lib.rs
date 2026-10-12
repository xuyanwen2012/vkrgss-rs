//! Build of the embedded Ruby and raw FFI.
//!
//! Side: guest.
//!
//! The build script compiles the vendored Ruby 1.8.7-p374 of `third_party/ruby-1.8.7/`, its
//! `Zlib` extension and, as its regular expression engine, the vendored Oniguruma of
//! `third_party/oniguruma-5.9.6/` into this crate, with a fixed file list and fixed flags.
//! This crate then offers three layers:
//!
//! - [`ffi`]: hand-written declarations of the C API, as raw as the C headers.
//! - [`Interpreter`]: the interpreter on a thread of its own, and the way onto that thread.
//! - [`Ruby`]: the operations that follow the rules below, available only on that thread:
//!   protected evaluation and calls, definitions backed by host functions, and rooting.
//!
//! ```no_run
//! use vkxp_ruby_sys::{Interpreter, Options, ffi};
//!
//! let interpreter = Interpreter::start(Options::default())?;
//! let answer = interpreter.run(|ruby| {
//!     ruby.eval_protected("6 * 7", "Section001", 1).map(ffi::fix2long)
//! })?;
//! assert_eq!(answer, 42);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # One thread
//!
//! Ruby 1.8 is not thread-safe, and its collector scans one machine stack. So there is one
//! interpreter per process, it lives on the thread that [`Interpreter::start`] creates, and
//! every call into it happens there: nothing of [`ffi`] may be called from any other thread.
//! [`Interpreter::run`] sends a closure to that thread and waits for the result; the closure
//! receives a [`Ruby`], which cannot leave the thread and is what the safe operations hang
//! on. In the running game the whole guest is one long closure. An interpreter cannot be
//! started a second time, not even after the first was dropped.
//!
//! The thread has a stack of at least [`MIN_STACK_SIZE`]. Ruby raises `SystemStackError`
//! when a script recurses to within [`STACK_HEADROOM`] of its end, and stays usable.
//!
//! # Entering and leaving Ruby
//!
//! Ruby raises with `longjmp`, which frees the frames between the raise and its handler
//! without running anything in them. Rust requires that no such frame has a value waiting to
//! be dropped, and a panic for its part must not unwind into C. Two rules follow.
//!
//! 1. **A call into Ruby that may raise goes through `rb_protect`.** The jump then ends
//!    inside C and the host sees a status. [`Ruby::eval_protected`], [`Ruby::eval`],
//!    [`Ruby::funcall`], [`Ruby::inspect`], [`Ruby::str_new`], [`Ruby::root`] and the
//!    `define_*` methods do this and return a `Result`: the error
//!    is a [`Raised`] (the exception object, as plain data) or a [`RubyError`] (class name,
//!    message and backtrace, copied into host memory). [`protect`] is what they are built
//!    on, and another crate uses it for C-API calls of its own: the plain data of the
//!    operation in a struct, and an `extern "C"` function that performs it and holds nothing
//!    with a destructor. Calling [`ffi`] directly is allowed
//!    where no Rust value with a destructor is alive in any frame the jump could cross, which
//!    is the caller's proof to make; most functions of the C API can raise, if only
//!    `NoMemoryError` or a `TypeError` for an argument of the wrong type.
//! 2. **A host function called by Ruby returns a [`HostResult`] and never raises or panics
//!    its way out.** The trampoline that Ruby actually calls runs the body inside
//!    `catch_unwind`. Only after the body has returned and every Rust value of the call is
//!    dropped does it raise with `rb_exc_raise`: the exception the body asked for through
//!    [`Raise`], or an `Exception` carrying the panic message. A `Raised` from a nested call
//!    into Ruby converts into a `Raise` with `?` and continues on its way.
//!
//! A host function is registered with one of the `define_*` methods of [`Ruby`]. The plain
//! ones ([`Ruby::define_method`] and its siblings) register it as taking any number of
//! arguments, which the body counts with [`Ruby::check_arity`]. The `_with_arity` ones
//! ([`Ruby::define_method_with_arity`], [`Ruby::define_private_method_with_arity`],
//! [`Ruby::define_singleton_method_with_arity`], [`Ruby::define_module_function_with_arity`],
//! [`Ruby::define_global_function_with_arity`]) take an [`Arity`]: with [`Arity::Fixed`] Ruby
//! checks the count before the body runs and `Method#arity` reports it. Rule 2 holds for both
//! in the same way.
//!
//! # Rooting
//!
//! The collector is conservative about the interpreter thread's stack and registers: a value
//! in a local on that thread is found and kept alive. It knows nothing of the host heap. A
//! value whose only reference is in a `Box`, a `Vec`, a field of a host object or a static
//! is collected, and the reference then points at a freed or reused object. Such a value must
//! be a [`Root`] (see [`Ruby::root`]), or be registered with
//! [`rb_gc_register_address`](ffi::rb_gc_register_address), or be reachable from a Ruby
//! object, for example through the `dmark` function of a `T_DATA` wrapper. [`Raised`] holds
//! an unrooted exception: keep it in a local until it is described or raised again.
//!
//! A [`VALUE`](ffi::VALUE) is a bare machine word and nothing checks that it is a live
//! object, which is why most operations that take one are `unsafe`.
//!
//! # Signals
//!
//! `ruby_init` replaces the handlers of [`SIGNALS_TAKEN_BY_RUBY`]. [`Interpreter::start`]
//! records the dispositions of the process before and puts them back afterwards, handler,
//! flags and mask, and once more when the interpreter is dropped; the host's signal handling
//! is the same after start as before. A script that calls `trap` still installs a handler of
//! its own.
//!
//! SIGVTALRM is left to Ruby. As soon as a script creates a second Ruby thread, Ruby arms a
//! process-wide virtual interval timer (`ITIMER_VIRTUAL`, every 10 ms of CPU time) and
//! switches its threads from the SIGVTALRM handler. The kernel may deliver that signal to
//! any thread of the process that does not block it, and a blocking system call there can
//! then fail with `EINTR`. Every thread other than the interpreter's should therefore block
//! SIGVTALRM, most simply by blocking it before any thread is spawned and unblocking it at
//! the start of the closure that runs the game. Dropping the interpreter disarms the timer
//! and restores the disposition of SIGVTALRM.
//!
//! # Start-up state
//!
//! After [`Interpreter::start`] the load path `$:` is empty, `Zlib` is defined as if `zlib`
//! had been required, and `$stdout` and `$stderr` are synchronous, so that output of a
//! script is not held back in a C stdio buffer when it does not go to a terminal. `$KCODE`,
//! the RGSS classes and the policy on functions that start processes belong to the binding,
//! not to this crate.

// The `Zlib` extension calls into the zlib that `libz-sys` builds; naming the crate is what
// makes rustc link it.
extern crate libz_sys;

mod call;
pub mod ffi;
mod host;
mod interp;
mod protect;
mod root;
mod signals;

pub use call::RubyError;
pub use host::{Arity, HostResult, Raise};
pub use interp::{
    GC_STRESS_VARIABLE, Interpreter, MIN_STACK_SIZE, Options, Ruby, STACK_HEADROOM, StartError,
    SymbolTable,
};
pub use protect::{Raised, protect};
pub use root::Root;
pub use signals::SIGNALS_TAKEN_BY_RUBY;
