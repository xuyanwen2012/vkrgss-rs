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

/// How many arguments Ruby accepts for a host-backed method, and what `Method#arity` reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arity {
    /// Any number of arguments: the body counts them itself, for which
    /// [`check_arity`](Ruby::check_arity) exists. `arity` is -1.
    Any,
    /// Exactly this many arguments, from 0 to [`Arity::MAX_FIXED`]. Ruby checks the count
    /// before the body runs and raises its own `ArgumentError`
    /// (`wrong number of arguments (1 for 2)`); `arity` is the count.
    Fixed(u8),
}

impl Arity {
    /// The largest fixed argument count Ruby 1.8 accepts for a C function.
    pub const MAX_FIXED: u8 = 15;

    /// The `argc` the definition functions of the C API take.
    fn argc(self) -> c_int {
        match self {
            Arity::Any => -1,
            Arity::Fixed(count) => c_int::from(count),
        }
    }
}

/// Runs the host function of type `F` for one call by Ruby and returns its value or raises.
///
/// # Safety
///
/// Interpreter thread only, called by Ruby from a function registered for `F`; `args` must be
/// live values, valid for this call.
unsafe fn run<F>(recv: VALUE, args: &[VALUE]) -> VALUE
where
    F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
{
    let outcome = {
        const { assert!(size_of::<F>() == 0) };
        // SAFETY: `F` is zero-sized (asserted above and where it was registered) and `Copy`,
        // and a value of it was given to the registration, so one exists; a dangling,
        // aligned pointer is a valid reference to a zero-sized value.
        let function: &F = unsafe { NonNull::<F>::dangling().as_ref() };
        // SAFETY: Ruby only calls methods on the interpreter thread while it is alive.
        let ruby = unsafe { Ruby::new() };
        panic::catch_unwind(AssertUnwindSafe(|| function(&ruby, recv, args)))
    };
    // SAFETY: on the interpreter thread, called by Ruby.
    unsafe { finish(outcome) }
}

/// The function Ruby calls for a host function of type `F` that takes any number of
/// arguments. It is registered with `argc == -1`, so Ruby passes the arguments as a count and
/// a pointer.
unsafe extern "C" fn trampoline<F>(argc: c_int, argv: *const VALUE, recv: VALUE) -> VALUE
where
    F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
{
    let args: &[VALUE] = if argc > 0 && !argv.is_null() {
        // SAFETY: Ruby passes `argc` live values at `argv`, valid for this call.
        unsafe { std::slice::from_raw_parts(argv, argc as usize) }
    } else {
        &[]
    };
    // SAFETY: Ruby called this function, which was registered for `F`.
    unsafe { run::<F>(recv, args) }
}

/// `VALUE`, once per argument name: the parameter types of a fixed-arity C function.
macro_rules! value_type {
    ($arg:ident) => {
        VALUE
    };
}

/// The functions Ruby calls for a host function of type `F` registered with a fixed `argc`,
/// one per count: Ruby passes the receiver and then that many arguments. Each collects its
/// arguments into a slice, in order, and runs the same body as the any-arity trampoline.
macro_rules! fixed_trampolines {
    ($($count:literal => $name:ident($($arg:ident),*);)*) => {
        $(
            unsafe extern "C" fn $name<F>(recv: VALUE $(, $arg: VALUE)*) -> VALUE
            where
                F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
            {
                // Plain data in this frame, where the collector's stack scan sees it.
                let args: [VALUE; $count] = [$($arg),*];
                // SAFETY: Ruby called this function, which was registered for `F`, with live
                // values.
                unsafe { run::<F>(recv, &args) }
            }
        )*

        /// The fixed-arity trampoline of `F` for `count` arguments, as the pointer type the
        /// definition functions take.
        fn fixed_trampoline_of<F>(count: u8) -> ffi::AnyArgsFn
        where
            F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
        {
            match count {
                $(
                    // SAFETY: both are C function pointers of the same size and ABI. Ruby
                    // casts the pointer back to the signature of this many `VALUE`s after
                    // the receiver before calling it, because the method is registered
                    // with this count as its `argc`.
                    $count => unsafe {
                        std::mem::transmute::<
                            unsafe extern "C" fn(VALUE $(, value_type!($arg))*) -> VALUE,
                            ffi::AnyArgsFn,
                        >($name::<F>)
                    },
                )*
                _ => panic!(
                    "a host function takes 0 to {} fixed arguments, not {count}",
                    Arity::MAX_FIXED
                ),
            }
        }
    };
}

fixed_trampolines! {
    0 => fixed_0();
    1 => fixed_1(a);
    2 => fixed_2(a, b);
    3 => fixed_3(a, b, c);
    4 => fixed_4(a, b, c, d);
    5 => fixed_5(a, b, c, d, e);
    6 => fixed_6(a, b, c, d, e, f);
    7 => fixed_7(a, b, c, d, e, f, g);
    8 => fixed_8(a, b, c, d, e, f, g, h);
    9 => fixed_9(a, b, c, d, e, f, g, h, i);
    10 => fixed_10(a, b, c, d, e, f, g, h, i, j);
    11 => fixed_11(a, b, c, d, e, f, g, h, i, j, k);
    12 => fixed_12(a, b, c, d, e, f, g, h, i, j, k, l);
    13 => fixed_13(a, b, c, d, e, f, g, h, i, j, k, l, m);
    14 => fixed_14(a, b, c, d, e, f, g, h, i, j, k, l, m, n);
    15 => fixed_15(a, b, c, d, e, f, g, h, i, j, k, l, m, n, o);
}

/// The trampoline of `F` for `arity`, as the pointer type the definition functions take.
fn trampoline_of<F>(_function: F, arity: Arity) -> ffi::AnyArgsFn
where
    F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
{
    const {
        assert!(
            size_of::<F>() == 0,
            "a host function must be a `fn` item or a closure that captures nothing"
        )
    };
    match arity {
        Arity::Any => ffi::method_argv(trampoline::<F>),
        Arity::Fixed(count) => fixed_trampoline_of::<F>(count),
    }
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
    /// The `argc` the function is registered with; it matches the function's signature.
    argc: c_int,
}

unsafe extern "C" fn define_body(data: VALUE) -> VALUE {
    // SAFETY: `define` passes a pointer to a `Define` with a NUL-terminated name that
    // outlives the call, and for every target that takes one a function whose signature is
    // the one Ruby calls for `argc`. The definition functions raise for a frozen class or a
    // superclass mismatch, hence the protection.
    unsafe {
        let define = &*(data as *const Define);
        let name = define.name;
        let argc = define.argc;
        match (define.target, define.function) {
            (Target::Class { superclass }, _) => ffi::rb_define_class(name, superclass),
            (Target::Module, _) => ffi::rb_define_module(name),
            (Target::ClassUnder { outer, superclass }, _) => {
                ffi::rb_define_class_under(outer, name, superclass)
            }
            (Target::ModuleUnder { outer }, _) => ffi::rb_define_module_under(outer, name),
            (Target::Method { class }, Some(function)) => {
                ffi::rb_define_method(class, name, function, argc);
                ffi::Qnil
            }
            (Target::PrivateMethod { class }, Some(function)) => {
                ffi::rb_define_private_method(class, name, function, argc);
                ffi::Qnil
            }
            (Target::SingletonMethod { object }, Some(function)) => {
                ffi::rb_define_singleton_method(object, name, function, argc);
                ffi::Qnil
            }
            (Target::ModuleFunction { module }, Some(function)) => {
                ffi::rb_define_module_function(module, name, function, argc);
                ffi::Qnil
            }
            (Target::GlobalFunction, Some(function)) => {
                ffi::rb_define_global_function(name, function, argc);
                ffi::Qnil
            }
            (_, None) => ffi::Qnil,
        }
    }
}

/// # Safety
///
/// Interpreter thread only; the values inside `target` must be live.
unsafe fn define(target: Target, name: &CStr) -> Result<VALUE, Raised> {
    let mut define = Define {
        target,
        name: name.as_ptr(),
        function: None,
        argc: -1,
    };
    // SAFETY: `define_body` fits the contract of `protect`.
    unsafe { protect::protect(&mut define, define_body) }
}

/// Defines a host function at `target`, registered for `arity`.
///
/// # Safety
///
/// Interpreter thread only; the values inside `target` must be live.
unsafe fn define_function<F>(
    target: Target,
    name: &CStr,
    arity: Arity,
    function: F,
) -> Result<(), Raised>
where
    F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
{
    let mut define = Define {
        target,
        name: name.as_ptr(),
        function: Some(trampoline_of(function, arity)),
        argc: arity.argc(),
    };
    // SAFETY: `define_body` fits the contract of `protect`, and the trampoline was chosen
    // for the same arity as `argc`.
    unsafe { protect::protect(&mut define, define_body) }.map(|_| ())
}

/// Definitions of classes, modules and host-backed methods.
///
/// A host function is a `fn` item or a closure that captures nothing, of the shape
/// `Fn(&Ruby, VALUE, &[VALUE]) -> HostResult`: the receiver and the arguments in, a value or
/// a [`Raise`] out. A capturing closure is rejected at compile time. A panic in the body
/// reaches Ruby as an `Exception` whose message carries the panic message.
///
/// Each `define_*` function for a method comes in two forms. The plain one registers the
/// method as accepting any number of arguments as far as Ruby is concerned: `arity` is -1 and
/// the body checks the count, for which [`check_arity`](Ruby::check_arity) exists. The
/// `_with_arity` one takes an [`Arity`]: with [`Arity::Fixed`] Ruby itself rejects a wrong
/// argument count before the body runs, `arity` reports the count, and the body receives
/// exactly that many arguments, in order. Panics and raises behave the same in both.
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
        unsafe { define(Target::Class { superclass }, name) }
    }

    /// Defines a top-level module, or returns the existing one of that name.
    pub fn define_module(&self, name: &CStr) -> Result<VALUE, Raised> {
        // SAFETY: `&self` proves the thread; no value is involved.
        unsafe { define(Target::Module, name) }
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
        unsafe { define(Target::ClassUnder { outer, superclass }, name) }
    }

    /// Defines a module inside a class or module.
    ///
    /// # Safety
    ///
    /// `outer` must be a live class or module.
    pub unsafe fn define_module_under(&self, outer: VALUE, name: &CStr) -> Result<VALUE, Raised> {
        // SAFETY: `&self` proves the thread; the caller guarantees the value.
        unsafe { define(Target::ModuleUnder { outer }, name) }
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
        // SAFETY: the caller guarantees the same as the function called requires.
        unsafe { self.define_method_with_arity(class, name, Arity::Any, function) }
    }

    /// As [`define_method`](Self::define_method), with the method registered for `arity`.
    ///
    /// # Panics
    ///
    /// Panics when `arity` is a fixed count above [`Arity::MAX_FIXED`].
    ///
    /// # Safety
    ///
    /// `class` must be a live class or module.
    pub unsafe fn define_method_with_arity<F>(
        &self,
        class: VALUE,
        name: &CStr,
        arity: Arity,
        function: F,
    ) -> Result<(), Raised>
    where
        F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
    {
        // SAFETY: `&self` proves the thread; the caller guarantees the value.
        unsafe { define_function(Target::Method { class }, name, arity, function) }
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
        // SAFETY: the caller guarantees the same as the function called requires.
        unsafe { self.define_private_method_with_arity(class, name, Arity::Any, function) }
    }

    /// As [`define_private_method`](Self::define_private_method), with the method registered
    /// for `arity`.
    ///
    /// # Panics
    ///
    /// Panics when `arity` is a fixed count above [`Arity::MAX_FIXED`].
    ///
    /// # Safety
    ///
    /// `class` must be a live class or module.
    pub unsafe fn define_private_method_with_arity<F>(
        &self,
        class: VALUE,
        name: &CStr,
        arity: Arity,
        function: F,
    ) -> Result<(), Raised>
    where
        F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
    {
        // SAFETY: `&self` proves the thread; the caller guarantees the value.
        unsafe { define_function(Target::PrivateMethod { class }, name, arity, function) }
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
        // SAFETY: the caller guarantees the same as the function called requires.
        unsafe { self.define_singleton_method_with_arity(object, name, Arity::Any, function) }
    }

    /// As [`define_singleton_method`](Self::define_singleton_method), with the method
    /// registered for `arity`.
    ///
    /// # Panics
    ///
    /// Panics when `arity` is a fixed count above [`Arity::MAX_FIXED`].
    ///
    /// # Safety
    ///
    /// `object` must be a live heap object.
    pub unsafe fn define_singleton_method_with_arity<F>(
        &self,
        object: VALUE,
        name: &CStr,
        arity: Arity,
        function: F,
    ) -> Result<(), Raised>
    where
        F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
    {
        // SAFETY: `&self` proves the thread; the caller guarantees the value.
        unsafe { define_function(Target::SingletonMethod { object }, name, arity, function) }
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
        // SAFETY: the caller guarantees the same as the function called requires.
        unsafe { self.define_module_function_with_arity(module, name, Arity::Any, function) }
    }

    /// As [`define_module_function`](Self::define_module_function), with the function
    /// registered for `arity`.
    ///
    /// # Panics
    ///
    /// Panics when `arity` is a fixed count above [`Arity::MAX_FIXED`].
    ///
    /// # Safety
    ///
    /// `module` must be a live module.
    pub unsafe fn define_module_function_with_arity<F>(
        &self,
        module: VALUE,
        name: &CStr,
        arity: Arity,
        function: F,
    ) -> Result<(), Raised>
    where
        F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
    {
        // SAFETY: `&self` proves the thread; the caller guarantees the value.
        unsafe { define_function(Target::ModuleFunction { module }, name, arity, function) }
    }

    /// Defines a global function: a private method of `Kernel`, callable from anywhere.
    pub fn define_global_function<F>(&self, name: &CStr, function: F) -> Result<(), Raised>
    where
        F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
    {
        self.define_global_function_with_arity(name, Arity::Any, function)
    }

    /// As [`define_global_function`](Self::define_global_function), with the function
    /// registered for `arity`.
    ///
    /// # Panics
    ///
    /// Panics when `arity` is a fixed count above [`Arity::MAX_FIXED`].
    pub fn define_global_function_with_arity<F>(
        &self,
        name: &CStr,
        arity: Arity,
        function: F,
    ) -> Result<(), Raised>
    where
        F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
    {
        // SAFETY: `&self` proves the thread; no value is involved.
        unsafe { define_function(Target::GlobalFunction, name, arity, function) }
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
