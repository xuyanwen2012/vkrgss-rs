//! Build of the embedded Ruby 1.9.2 and raw FFI.
//!
//! Side: guest.
//!
//! The build script compiles the vendored Ruby 1.9.2-p0 of `third_party/ruby-1.9.2/`, the
//! release that the original runtime of RPG Maker VX Ace embeds, with the encodings that
//! runtime has and its `Zlib` extension, with a fixed file list and fixed flags. This crate
//! then offers the three layers of `vkrgss-ruby-sys`, under the same names:
//!
//! - `ffi`: hand-written declarations of the C API, as raw as the C headers.
//! - `Interpreter`: the interpreter on a thread of its own, and the way onto that thread.
//! - `Ruby`: the operations that follow the rules below, available only on that thread:
//!   protected evaluation and calls, definitions backed by host functions, and rooting.
//!
//! The crate is for the targets of `targets.rs`, `x86_64-unknown-linux-gnu`,
//! `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin` and `aarch64-linux-android`. For any
//! other target the build script compiles nothing and the crate has no item. The two
//! interpreters define the same symbols: no program links this crate and `vkrgss-ruby-sys`
//! together, which a dependency rule of the workspace checks.
//!
//! # One thread
//!
//! There is one interpreter per process, it lives on the thread that `Interpreter::start`
//! creates, and every call into it happens there: nothing of `ffi` may be called from any
//! other thread. `Interpreter::run` sends a closure to that thread and waits for the result;
//! the closure receives a `Ruby`, which cannot leave the thread and is what the safe
//! operations hang on. An interpreter cannot be started a second time, not even after the
//! first was dropped.
//!
//! The thread has a stack of at least `MIN_STACK_SIZE`. The limit of Ruby's machine stack
//! follows the size of that stack, less `STACK_HEADROOM`, and not the resource limit of the
//! process. A script that recurses without end raises `SystemStackError`, and the interpreter
//! stays usable.
//!
//! Ruby threads of 1.9 are threads of the operating system under one lock. While the main
//! thread is the only Ruby thread, the interpreter has created no thread of its own; its
//! timer thread starts when a script creates its first thread. From then on thread switches
//! follow a timer and a run is no longer repeatable.
//!
//! # Entering and leaving Ruby
//!
//! Ruby raises with `longjmp`, which frees the frames between the raise and its handler
//! without running anything in them. Rust requires that no such frame has a value waiting to
//! be dropped, and a panic for its part must not unwind into C. Two rules follow.
//!
//! 1. **A call into Ruby that may raise goes through `rb_protect`.** The jump then ends
//!    inside C and the host sees a status. `Ruby::eval_protected`, `Ruby::eval`,
//!    `Ruby::funcall`, `Ruby::inspect`, `Ruby::str_new`, `Ruby::str_new_text`, `Ruby::root`
//!    and the `define_*` methods do this and return a `Result`: the error is a `Raised` (the
//!    exception object, as plain data) or a `RubyError` (class name, message and backtrace,
//!    copied into host memory). `protect` is what they are built on, and another crate uses
//!    it for C-API calls of its own. Calling `ffi` directly is allowed where no Rust value
//!    with a destructor is alive in any frame the jump could cross, which is the caller's
//!    proof to make.
//! 2. **A host function called by Ruby returns a `HostResult` and never raises or panics its
//!    way out.** The trampoline that Ruby actually calls runs the body inside `catch_unwind`.
//!    Only after the body has returned and every Rust value of the call is dropped does it
//!    raise: the exception the body asked for through `Raise`, or an `Exception` carrying the
//!    panic message. A `Raised` from a nested call into Ruby converts into a `Raise` with `?`
//!    and continues on its way.
//!
//! A host function may run on the stack of a Fiber and be suspended there, when the block it
//! yields to calls `Fiber.yield`. Its locals are kept alive like those of any other frame.
//!
//! # Rooting
//!
//! The collector is conservative about the interpreter thread's stack and registers, and
//! about the saved stacks of suspended Fibers: a value in a local there is found and kept
//! alive. It knows nothing of the host heap. A value whose only reference is in a `Box`, a
//! `Vec`, a field of a host object or a static is collected. Such a value must be a `Root`
//! (see `Ruby::root`), or be registered with `rb_gc_register_address`, or be reachable from a
//! Ruby object. `Raised` holds an unrooted exception: keep it in a local until it is
//! described or raised again.
//!
//! # Strings
//!
//! A string of Ruby 1.9 has an encoding. `Ruby::str_new` makes a string of bytes, labelled
//! ASCII-8BIT; `Ruby::str_new_text` makes text, labelled UTF-8 as the literals of a script
//! are, whether the bytes are valid UTF-8 or not, and `Ruby::str_new_utf8` is the same for a
//! `&str`. `Ruby::str_bytes` returns the bytes of a string whatever its encoding. Source given
//! to `Ruby::eval` is read as UTF-8.
//!
//! # Signals
//!
//! `ruby_init` replaces the dispositions of `SIGNALS_TAKEN_BY_RUBY` and blocks every signal
//! on the interpreter thread. `Interpreter::start` records the dispositions and the mask
//! before and puts them back afterwards, handler, flags and mask, and once more when the
//! interpreter is dropped; the host's signal handling is the same after start as before. A
//! script that calls `trap` still installs a handler of its own.
//!
//! When a script creates the first Ruby thread, the interpreter starts its timer thread,
//! blocks the signals on the interpreter thread again, and takes SIGVTALRM, which is not
//! among `SIGNALS_TAKEN_BY_RUBY`: it wakes a thread that blocks by sending it that signal,
//! and installs an empty handler for it. Dropping the interpreter puts the disposition of
//! SIGVTALRM back with the others.
//!
//! # Start-up state
//!
//! After `Interpreter::start` the state is the one measured on the original runtime: `$:` is
//! `["."]`, `$LOADED_FEATURES` is `["enumerator.so"]`, the default external encoding is
//! UTF-8 and there is no default internal one, the filesystem encoding, which `__FILE__`
//! and `Dir.pwd` are labelled with, is ASCII-8BIT, `Zlib` is defined without `zlib.so` being a
//! loaded feature, and the release's prelude has not run (`Mutex#synchronize` and
//! `Thread.exclusive` do not exist). `Encoding.list` holds ASCII-8BIT, UTF-8, US-ASCII and
//! UTF-16LE; Shift_JIS, Windows-31J and ISO-8859-1 are registered at the first look-up of
//! their name, and every other name is unknown. No file is ever looked for to find an
//! encoding. `$stdout` and `$stderr` are synchronous. The seed of `String#hash` and the
//! other hashes is `HASH_SEED` in every run. The RGSS classes and the policy on functions
//! that start processes belong to the binding, not to this crate.
//!
//! With the environment variable `VKRGSS_GC_STRESS` set to a number of 1 or more, the
//! collector runs at every allocation from the end of start-up on (`GC.stress = true`).
//!
//! # The shared glue
//!
//! As in `vkrgss-ruby-sys`, the part that bridges Rust and Ruby is that of
//! `vkrgss-ruby-glue`: `Ruby` and `Root` are the types of that crate for `CApi`, this
//! interpreter's implementation of its trait, `protect` is its protected call for this
//! interpreter, and `Raised`, `RubyError`, `HostResult`, `Raise` and `Arity` are its types.
//!
//! # Differences from `vkrgss-ruby-sys`
//!
//! The public items have the names and the meaning they have in `vkrgss-ruby-sys`, except:
//!
//! | `vkrgss-ruby-sys` | here |
//! |---|---|
//! | `ffi::ruby_errinfo`, a variable | absent; `ffi::rb_errinfo()` and `ffi::rb_set_errinfo()`, which `protect` and `Ruby::set_current_error` use |
//! | `ffi::ruby_top_self`, a variable | absent; `ffi::rb_vm_top_self()` and `Ruby::top_self()` |
//! | `ffi::ruby_set_stack_size` | absent; `Options::stack_size` is applied through the variable `ffi::ruby_vkxp_stack_size` of a patch |
//! | `ffi::ruby_vkxp_gc_stress`, `ffi::ruby_vkxp_gc_count` | absent; `GC.stress` and `GC.count` of the release. `VKRGSS_GC_STRESS` has no "every n-th" form |
//! | `ffi::ruby_vkxp_select_symbols`, `ffi::ruby_vkxp_mark_game_scripts`, `SymbolTable`, `Options::symbols` | absent; symbols are numbered as the release numbers them, and a dump does not depend on the numbers. `Options::for_rgss` gives the defaults for RGSS1 and RGSS2 and an empty `Options::script_name` for RGSS3 |
//! | `Ruby::mark_game_scripts` | exists and does nothing |
//! | `restart_thread_timer` arms the timer of the green threads in a copy of the process | forgets the timer thread, which a copy made by `fork` does not have, and makes its lock and condition variable afresh; the interpreter starts one when a script next creates a thread. It also forgets the threads of the copied process that waited for the interpreter's lock. Nothing changes when no script ever made a thread |
//! | `threads_lost_in_a_copy` is always `false`: the threads are memory of the interpreter | `true` exactly while a Ruby thread other than the main one is alive: a Ruby thread is a thread of the system, and a copy made by `fork` does not have it |
//! | `RUBY_API` is `RubyApi::Ruby18` | `RubyApi::Ruby19` |
//! | `ffi::RString`, `ffi::RArray`, `ffi::RAux`, `ffi::rstring()`, `ffi::rarray()` | absent; `ffi::rstring_ptr()`, `ffi::rstring_len()`, `ffi::rarray_ptr()` and `ffi::rarray_len()`, which follow `RSTRING_PTR` and its siblings for the embedded and the heap form. The layouts are in `ffi::layout` |
//! | `ffi::RObject` with `iv_tbl` | the layout of 1.9.2: slots inside the object, or a pointer to them |
//! | the type tags `ffi::T_*` and the flag bits `ffi::FL_*` | the values of the 1.9.2 headers; `T_COMPLEX`, `T_RATIONAL`, `T_NODE`, `T_ZOMBIE` and `FL_UNTRUSTED` are new |
//! | `ffi::ruby_finalize` ends the interpreter | still declared; `ffi::ruby_cleanup`, which also terminates the Ruby threads, is what ends it |
//! | `Ruby::str_new`, `Ruby::str_new_text` and `Ruby::str_new_utf8`, the latter for a `&str`, give the same string | `Ruby::str_new` labels the string ASCII-8BIT and `Ruby::str_new_text` and `Ruby::str_new_utf8` UTF-8, valid or not |
//! | `Ruby::eval`, `Ruby::eval_protected` | the same; the source is handed over as a UTF-8 string, a backtrace position outside a method reads ``file:line:in `<main>'``, and the backtrace ends with the frame of the `eval` call itself (``vkrgss:in `eval'``, with the script name, or ``ruby:in `eval'`` without one) |
//! | `$0` is the script name | the same; an empty `Options::script_name` gives the interpreter no name, and `$0` is `false` |
//! | `Raised` of a `throw` or `break` | the same shape; the interpreter keeps what the jump carries as its current error until the jump is continued or described |
//! | `SIGNALS_TAKEN_BY_RUBY`, ten signals | the eleven whose dispositions `Init_signal` of 1.9.2 changes: SIGCHLD is new. SIGVTALRM is no timer signal any more and is taken only once a script has created a thread |
//! | after start the load path is empty, and `zlib.so` is a loaded feature | the start-up state above |
//!
//! New and without a counterpart: `HASH_SEED`, `ffi::rb_enc_str_new`, `ffi::rb_encdb_alias`,
//! `ffi::rb_utf8_encindex`, `ffi::rb_ascii8bit_encindex`, `ffi::rb_enc_get_index`,
//! `ffi::rb_enc_associate_index` (which the binding reaches under the `cfg` `vkrgss_ruby_19`
//! that `vkrgss-binding-rgss3` sets),
//! `ffi::rb_const_remove`,
//! `ffi::rb_utf8_encoding`, `ffi::rb_enc_from_encoding`, `ffi::rb_enc_set_default_external`,
//! the `Init_` functions of the linked encodings and transcoders, and the variables of the
//! patches (`ffi::ruby_vkxp_lazy_timer_thread`, `ffi::ruby_vkxp_hashseed_fixed`,
//! `ffi::ruby_vkxp_hashseed`, `ffi::ruby_vkxp_stack_size`).

/// Declares the items for the targets the interpreter is built for, those for which the build
/// script does not set the `cfg` `vkrgss_interpreter_absent`.
macro_rules! on_target {
    ($($item:item)*) => {
        $(
            #[cfg(not(vkrgss_interpreter_absent))]
            $item
        )*
    };
}

on_target! {
    // The `Zlib` extension calls into the zlib that `libz-sys` builds; naming the crate is
    // what makes rustc link it.
    extern crate libz_sys;

    mod capi;
    pub mod ffi;
    mod interp;
    mod protect;
    mod signals;

    pub use capi::CApi;
    pub use interp::{
        GC_STRESS_VARIABLE, HASH_SEED, Interpreter, MIN_STACK_SIZE, Options, RUBY_API, Ruby,
        RubyApi, STACK_HEADROOM, StartError,
    };
    pub use protect::protect;
    pub use vkrgss_ruby_glue::{Arity, HostResult, Raise, Raised, RubyError};

    /// A Ruby value that the collector treats as reachable for as long as the `Root` exists:
    /// `vkrgss_ruby_glue::Root` over this interpreter, made by `Ruby::root`.
    pub type Root = vkrgss_ruby_glue::Root<CApi>;
    pub use signals::{SIGNALS_TAKEN_BY_RUBY, restart_thread_timer, threads_lost_in_a_copy};
}
