//! Host functions that Ruby can call, and the definitions that hang them on classes.
//!
//! Ruby raises with `longjmp`, and a panic must not unwind into C. So a host function never
//! raises or panics its way out: its body returns a [`HostResult`], and the trampoline that
//! Ruby actually calls raises only after the body has returned and every Rust value of the
//! call is dropped.

use std::any::Any;
use std::ffi::{CStr, c_char, c_int, c_long};
use std::panic::{self, AssertUnwindSafe};
use std::ptr::NonNull;

use crate::ffi::{self, VALUE};
use crate::interp::Ruby;
use crate::protect::{self, Raised};

/// What a host function hands back to Ruby: a value, or the request to raise.
pub type HostResult = Result<VALUE, Raise>;

/// A request to leave a host function by raising in Ruby.
#[derive(Debug)]
pub enum Raise {
    /// Raise a new exception of this class with this message.
    New {
        /// An exception class, such as [`rb_eArgError`](ffi::rb_eArgError).
        class: VALUE,
        /// The exception's message.
        message: String,
    },
    /// Raise this exception object.
    Exception(VALUE),
    /// Continue a jump that a protected call into Ruby stopped: re-raise its exception, or
    /// resume its `throw` or `break`. `?` on a `Result<_, Raised>` produces this.
    Resume(Raised),
}

impl Raise {
    /// A new exception of `class` with `message`.
    pub fn new(class: VALUE, message: impl Into<String>) -> Self {
        Self::New {
            class,
            message: message.into(),
        }
    }
}

impl From<Raised> for Raise {
    fn from(raised: Raised) -> Self {
        Self::Resume(raised)
    }
}

/// The plain data of creating an exception.
struct NewException {
    class: VALUE,
    message: *const c_char,
    len: c_long,
}

unsafe extern "C" fn new_exception_body(data: VALUE) -> VALUE {
    // SAFETY: `finish` passes a pointer to a `NewException` whose message buffer outlives
    // the call.
    unsafe {
        let new = &*(data as *const NewException);
        ffi::rb_exc_new(new.class, new.message, new.len)
    }
}

/// The text of a panic payload, as far as it is the usual `&str` or `String`.
fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_owned()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "a panic with a payload that is not text".to_owned()
    }
}

/// What is left to do once every Rust value of a host call is gone: plain data only.
#[derive(Clone, Copy)]
enum Exit {
    Raise(VALUE),
    Jump(c_int),
}

/// Turns the outcome of a host function's body into a return value or a Ruby raise.
///
/// # Safety
///
/// Interpreter thread only, called by Ruby (so that there is a handler to jump to).
unsafe fn finish(outcome: std::thread::Result<HostResult>) -> VALUE {
    let raise = match outcome {
        Ok(Ok(value)) => return value,
        Ok(Err(raise)) => raise,
        Err(payload) => {
            let message = format!("panic in a host function: {}", panic_message(&*payload));
            // A payload whose destructor panics must not start a second unwind here.
            if let Err(second) = panic::catch_unwind(AssertUnwindSafe(move || drop(payload))) {
                std::mem::forget(second);
            }
            // Directly `Exception`, so that a script's bare `rescue` does not swallow it.
            // SAFETY: the class globals are set once the interpreter runs.
            Raise::new(unsafe { ffi::rb_eException }, message)
        }
    };

    let from_raised = |raised: Raised| {
        if raised.state == ffi::TAG_RAISE || raised.state == ffi::TAG_FATAL {
            Exit::Raise(raised.exception)
        } else {
            Exit::Jump(raised.state)
        }
    };
    let exit = match raise {
        Raise::Exception(exception) => Exit::Raise(exception),
        Raise::Resume(raised) => from_raised(raised),
        Raise::New { class, message } => {
            let mut new = NewException {
                class,
                message: message.as_ptr().cast(),
                len: c_long::try_from(message.len()).expect("a message below 2^63 bytes"),
            };
            // SAFETY: `new_exception_body` fits the contract of `protect`. Creating the
            // exception is protected because `message` is still alive here: a failure comes
            // back as a value instead of jumping over it.
            let created = unsafe { protect::protect(&mut new, new_exception_body) };
            drop(message);
            match created {
                Ok(exception) => Exit::Raise(exception),
                Err(raised) => from_raised(raised),
            }
        }
    };

    // Only `exit`, which is plain data, is alive now: the jump skips no destructor.
    // SAFETY: Ruby called us, so a handler exists; `rb_exc_raise` itself rejects a value
    // that is not an exception with a `TypeError`.
    unsafe {
        match exit {
            Exit::Raise(exception) => ffi::rb_exc_raise(exception),
            Exit::Jump(state) => ffi::rb_jump_tag(state),
        }
    }
}

/// The function Ruby calls for a host function of type `F`. It is registered with
/// `argc == -1`, so Ruby passes the arguments as a count and a pointer.
unsafe extern "C" fn trampoline<F>(argc: c_int, argv: *const VALUE, recv: VALUE) -> VALUE
where
    F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
{
    let outcome = {
        const { assert!(size_of::<F>() == 0) };
        // SAFETY: `F` is zero-sized (asserted above and where it was registered) and `Copy`,
        // and a value of it was given to the registration, so one exists; a dangling,
        // aligned pointer is a valid reference to a zero-sized value.
        let function: &F = unsafe { NonNull::<F>::dangling().as_ref() };
        let args: &[VALUE] = if argc > 0 && !argv.is_null() {
            // SAFETY: Ruby passes `argc` live values at `argv`, valid for this call.
            unsafe { std::slice::from_raw_parts(argv, argc as usize) }
        } else {
            &[]
        };
        // SAFETY: Ruby only calls methods on the interpreter thread while it is alive.
        let ruby = unsafe { Ruby::new() };
        panic::catch_unwind(AssertUnwindSafe(|| function(&ruby, recv, args)))
    };
    // SAFETY: on the interpreter thread, called by Ruby.
    unsafe { finish(outcome) }
}

/// The trampoline of `F` as the pointer type the definition functions take.
fn trampoline_of<F>(_function: F) -> ffi::AnyArgsFn
where
    F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
{
    const {
        assert!(
            size_of::<F>() == 0,
            "a host function must be a `fn` item or a closure that captures nothing"
        )
    };
    ffi::method_argv(trampoline::<F>)
}

/// Where a definition goes.
#[derive(Clone, Copy)]
enum Target {
    Class { superclass: VALUE },
    Module,
    ClassUnder { outer: VALUE, superclass: VALUE },
    ModuleUnder { outer: VALUE },
    Method { class: VALUE },
    PrivateMethod { class: VALUE },
    SingletonMethod { object: VALUE },
    ModuleFunction { module: VALUE },
    GlobalFunction,
}

/// The plain data of one definition.
struct Define {
    target: Target,
    name: *const c_char,
    function: Option<ffi::AnyArgsFn>,
}

unsafe extern "C" fn define_body(data: VALUE) -> VALUE {
    // SAFETY: `define` passes a pointer to a `Define` with a NUL-terminated name that
    // outlives the call, and a function for every target that takes one. The definition
    // functions raise for a frozen class or a superclass mismatch, hence the protection.
    unsafe {
        let define = &*(data as *const Define);
        let name = define.name;
        match (define.target, define.function) {
            (Target::Class { superclass }, _) => ffi::rb_define_class(name, superclass),
            (Target::Module, _) => ffi::rb_define_module(name),
            (Target::ClassUnder { outer, superclass }, _) => {
                ffi::rb_define_class_under(outer, name, superclass)
            }
            (Target::ModuleUnder { outer }, _) => ffi::rb_define_module_under(outer, name),
            (Target::Method { class }, Some(function)) => {
                ffi::rb_define_method(class, name, function, -1);
                ffi::Qnil
            }
            (Target::PrivateMethod { class }, Some(function)) => {
                ffi::rb_define_private_method(class, name, function, -1);
                ffi::Qnil
            }
            (Target::SingletonMethod { object }, Some(function)) => {
                ffi::rb_define_singleton_method(object, name, function, -1);
                ffi::Qnil
            }
            (Target::ModuleFunction { module }, Some(function)) => {
                ffi::rb_define_module_function(module, name, function, -1);
                ffi::Qnil
            }
            (Target::GlobalFunction, Some(function)) => {
                ffi::rb_define_global_function(name, function, -1);
                ffi::Qnil
            }
            (_, None) => ffi::Qnil,
        }
    }
}

/// # Safety
///
/// Interpreter thread only; the values inside `target` must be live.
unsafe fn define(
    target: Target,
    name: &CStr,
    function: Option<ffi::AnyArgsFn>,
) -> Result<VALUE, Raised> {
    let mut define = Define {
        target,
        name: name.as_ptr(),
        function,
    };
    // SAFETY: `define_body` fits the contract of `protect`.
    unsafe { protect::protect(&mut define, define_body) }
}

/// Definitions of classes, modules and host-backed methods.
///
/// A host function is a `fn` item or a closure that captures nothing, of the shape
/// `Fn(&Ruby, VALUE, &[VALUE]) -> HostResult`: the receiver and the arguments in, a value or
/// a [`Raise`] out. A capturing closure is rejected at compile time. Methods accept any
/// number of arguments as far as Ruby is concerned; the body checks the count, for which
/// [`check_arity`](Ruby::check_arity) exists. A panic in the body reaches Ruby as an
/// `Exception` whose message carries the panic message.
///
/// The body runs on the interpreter thread in the middle of Ruby's own frames. The values it
/// receives stay alive for the call; a value it creates stays alive while a local holds it.
impl Ruby {
    /// Defines a top-level class, or returns the existing one of that name.
    ///
    /// # Safety
    ///
    /// `superclass` must be a live class.
    pub unsafe fn define_class(&self, name: &CStr, superclass: VALUE) -> Result<VALUE, Raised> {
        // SAFETY: `&self` proves the thread; the caller guarantees the value.
        unsafe { define(Target::Class { superclass }, name, None) }
    }

    /// Defines a top-level module, or returns the existing one of that name.
    pub fn define_module(&self, name: &CStr) -> Result<VALUE, Raised> {
        // SAFETY: `&self` proves the thread; no value is involved.
        unsafe { define(Target::Module, name, None) }
    }

    /// Defines a class inside a class or module.
    ///
    /// # Safety
    ///
    /// `outer` must be a live class or module and `superclass` a live class.
    pub unsafe fn define_class_under(
        &self,
        outer: VALUE,
        name: &CStr,
        superclass: VALUE,
    ) -> Result<VALUE, Raised> {
        // SAFETY: `&self` proves the thread; the caller guarantees the values.
        unsafe { define(Target::ClassUnder { outer, superclass }, name, None) }
    }

    /// Defines a module inside a class or module.
    ///
    /// # Safety
    ///
    /// `outer` must be a live class or module.
    pub unsafe fn define_module_under(&self, outer: VALUE, name: &CStr) -> Result<VALUE, Raised> {
        // SAFETY: `&self` proves the thread; the caller guarantees the value.
        unsafe { define(Target::ModuleUnder { outer }, name, None) }
    }

    /// Defines a public instance method of `class`, implemented by a host function.
    ///
    /// # Safety
    ///
    /// `class` must be a live class or module.
    pub unsafe fn define_method<F>(
        &self,
        class: VALUE,
        name: &CStr,
        function: F,
    ) -> Result<(), Raised>
    where
        F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
    {
        let function = Some(trampoline_of(function));
        // SAFETY: `&self` proves the thread; the caller guarantees the value.
        unsafe { define(Target::Method { class }, name, function) }.map(|_| ())
    }

    /// Defines a private instance method of `class`, implemented by a host function.
    ///
    /// # Safety
    ///
    /// `class` must be a live class or module.
    pub unsafe fn define_private_method<F>(
        &self,
        class: VALUE,
        name: &CStr,
        function: F,
    ) -> Result<(), Raised>
    where
        F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
    {
        let function = Some(trampoline_of(function));
        // SAFETY: `&self` proves the thread; the caller guarantees the value.
        unsafe { define(Target::PrivateMethod { class }, name, function) }.map(|_| ())
    }

    /// Defines a method on one object only, such as a class method when `object` is a class.
    ///
    /// # Safety
    ///
    /// `object` must be a live heap object.
    pub unsafe fn define_singleton_method<F>(
        &self,
        object: VALUE,
        name: &CStr,
        function: F,
    ) -> Result<(), Raised>
    where
        F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
    {
        let function = Some(trampoline_of(function));
        // SAFETY: `&self` proves the thread; the caller guarantees the value.
        unsafe { define(Target::SingletonMethod { object }, name, function) }.map(|_| ())
    }

    /// Defines a module function: callable as `Module.name` and, where the module is
    /// included, as a private method.
    ///
    /// # Safety
    ///
    /// `module` must be a live module.
    pub unsafe fn define_module_function<F>(
        &self,
        module: VALUE,
        name: &CStr,
        function: F,
    ) -> Result<(), Raised>
    where
        F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
    {
        let function = Some(trampoline_of(function));
        // SAFETY: `&self` proves the thread; the caller guarantees the value.
        unsafe { define(Target::ModuleFunction { module }, name, function) }.map(|_| ())
    }

    /// Defines a global function: a private method of `Kernel`, callable from anywhere.
    pub fn define_global_function<F>(&self, name: &CStr, function: F) -> Result<(), Raised>
    where
        F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
    {
        let function = Some(trampoline_of(function));
        // SAFETY: `&self` proves the thread; no value is involved.
        unsafe { define(Target::GlobalFunction, name, function) }.map(|_| ())
    }

    /// Checks the argument count of a host function the way Ruby words it:
    /// `wrong number of arguments (given for expected)`, an `ArgumentError`.
    pub fn check_arity(&self, args: &[VALUE], expected: usize) -> Result<(), Raise> {
        if args.len() == expected {
            return Ok(());
        }
        // SAFETY: `&self` proves the thread, and the class globals are set by then.
        let class = unsafe { ffi::rb_eArgError };
        Err(Raise::new(
            class,
            format!("wrong number of arguments ({} for {expected})", args.len()),
        ))
    }
}
