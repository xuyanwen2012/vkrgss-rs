//! The calls into the interpreter that the binding makes, each in a form that cannot jump.
//!
//! Ruby raises with `longjmp`, and most functions of the C API can raise, if only
//! `NoMemoryError`. Following the rule of `vkxp-ruby-sys`, every such call here runs under
//! `rb_protect`: the operation is plain data in a [`Call`] on the caller's stack, and one
//! `extern "C"` function that holds nothing with a destructor performs it. What comes back
//! is a `Result`, which a host function passes on with `?`.
//!
//! The functions of this module are safe to call inside the crate under one condition that
//! every caller keeps: each `VALUE` passed in is live. In a host function that is the
//! receiver, an argument, or a value one of these functions returned during the same call
//! and that has been in a local since, where the collector's stack scan finds it.

use std::ffi::{CStr, c_char, c_int, c_long};

use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{Arity, HostResult, Raise, Raised, Ruby};

/// The allocator of a class, as `rb_define_alloc_func` takes it.
pub(crate) type AllocFn = unsafe extern "C" fn(VALUE) -> VALUE;

/// One operation of the C API that can raise.
#[derive(Clone, Copy)]
enum Op {
    /// `NUM2LONG`, into [`Call::long`].
    Long(VALUE),
    /// `NUM2DBL`, into [`Call::double`].
    Double(VALUE),
    /// A new `Float`.
    Float(f64),
    /// A new instance from the class's allocator, without `initialize`.
    Alloc(VALUE),
    IvarSet {
        object: VALUE,
        name: *const c_char,
        value: VALUE,
    },
    IvarGet {
        object: VALUE,
        name: *const c_char,
    },
    /// `to_s`, with Ruby's fallback when that does not return a string.
    AsString(VALUE),
    /// The class or module at a path such as `Errno::ENOENT`.
    ClassAt(*const c_char),
    AllocFunc {
        class: VALUE,
        function: AllocFn,
    },
    /// A new array of new strings.
    Strings {
        items: *const (*const c_char, c_long),
        len: usize,
    },
}

/// The plain data of one protected operation: the input and the outputs that are not values.
struct Call {
    op: Op,
    long: c_long,
    double: f64,
}

unsafe extern "C" fn call_body(data: VALUE) -> VALUE {
    // SAFETY: `perform` passes a pointer to a `Call` that outlives this function, whose
    // values are live by the condition of this module and whose pointers are valid for the
    // call. Everything here is the C API on plain data: a raise jumps over no destructor.
    unsafe {
        let call = &mut *(data as *mut Call);
        match call.op {
            Op::Long(value) => {
                call.long = ffi::num2long(value);
                ffi::Qnil
            }
            Op::Double(value) => {
                call.double = ffi::rb_num2dbl(value);
                ffi::Qnil
            }
            Op::Float(number) => ffi::rb_float_new(number),
            Op::Alloc(class) => ffi::rb_obj_alloc(class),
            Op::IvarSet {
                object,
                name,
                value,
            } => ffi::rb_ivar_set(object, ffi::rb_intern(name), value),
            Op::IvarGet { object, name } => ffi::rb_attr_get(object, ffi::rb_intern(name)),
            Op::AsString(value) => ffi::rb_obj_as_string(value),
            Op::ClassAt(path) => ffi::rb_path2class(path),
            Op::AllocFunc { class, function } => {
                ffi::rb_define_alloc_func(class, function);
                ffi::Qnil
            }
            Op::Strings { items, len } => {
                let array = ffi::rb_ary_new();
                let mut index = 0;
                while index < len {
                    let (bytes, len) = *items.add(index);
                    ffi::rb_ary_push(array, ffi::rb_str_new(bytes, len));
                    index += 1;
                }
                array
            }
        }
    }
}

/// Performs `op` under `rb_protect` and returns its value and outputs.
fn perform(_ruby: &Ruby, op: Op) -> Result<(VALUE, Call), Raised> {
    let mut call = Call {
        op,
        long: 0,
        double: 0.0,
    };
    let mut state: c_int = 0;
    // SAFETY: `&Ruby` proves the interpreter thread. `call` outlives the call and
    // `call_body` treats its argument as a pointer to it; `rb_protect` returns normally
    // whatever the body does.
    let value = unsafe {
        ffi::rb_protect(
            call_body,
            std::ptr::from_mut(&mut call) as VALUE,
            &mut state,
        )
    };
    if state == 0 {
        return Ok((value, call));
    }
    // SAFETY: on the interpreter thread `ruby_errinfo` is only touched by the interpreter,
    // which is not running now. Taking the exception out leaves `$!` clean.
    let exception = unsafe {
        let pending = ffi::ruby_errinfo;
        ffi::ruby_errinfo = ffi::Qnil;
        pending
    };
    let is_raise = state == ffi::TAG_RAISE || state == ffi::TAG_FATAL;
    Err(Raised {
        state,
        exception: if is_raise { exception } else { ffi::Qnil },
    })
}

// --- Exception classes ------------------------------------------------------------------------

/// `ArgumentError`.
pub(crate) fn arg_error(_ruby: &Ruby) -> VALUE {
    // SAFETY: `&Ruby` proves the thread; the class globals are set once the interpreter runs.
    unsafe { ffi::rb_eArgError }
}

/// `TypeError`.
pub(crate) fn type_error(_ruby: &Ruby) -> VALUE {
    // SAFETY: as in `arg_error`.
    unsafe { ffi::rb_eTypeError }
}

/// `RangeError`.
pub(crate) fn range_error(_ruby: &Ruby) -> VALUE {
    // SAFETY: as in `arg_error`.
    unsafe { ffi::rb_eRangeError }
}

/// `IOError`.
pub(crate) fn io_error(_ruby: &Ruby) -> VALUE {
    // SAFETY: as in `arg_error`.
    unsafe { ffi::rb_eIOError }
}

/// `StandardError`.
pub(crate) fn standard_error(_ruby: &Ruby) -> VALUE {
    // SAFETY: as in `arg_error`.
    unsafe { ffi::rb_eStandardError }
}

/// `Object`.
pub(crate) fn object_class(_ruby: &Ruby) -> VALUE {
    // SAFETY: as in `arg_error`.
    unsafe { ffi::rb_cObject }
}

/// The class or module at `path`, such as `Errno::ENOENT`.
pub(crate) fn class_at(ruby: &Ruby, path: &CStr) -> Result<VALUE, Raised> {
    Ok(perform(ruby, Op::ClassAt(path.as_ptr()))?.0)
}

/// The name of the value's class, for messages.
pub(crate) fn class_name(_ruby: &Ruby, value: VALUE) -> String {
    // SAFETY: the value is live by the condition of this module; the function returns a
    // NUL-terminated name owned by the interpreter and does not raise.
    unsafe {
        CStr::from_ptr(ffi::rb_obj_classname(value))
            .to_string_lossy()
            .into_owned()
    }
}

// --- Arguments --------------------------------------------------------------------------------

/// Checks the argument count of a method with optional arguments, with Ruby's usual message:
/// `wrong number of arguments (given for expected)`, an `ArgumentError`.
pub(crate) fn check_count(
    ruby: &Ruby,
    args: &[VALUE],
    min: usize,
    max: usize,
) -> Result<(), Raise> {
    let expected = if args.len() < min {
        min
    } else if args.len() > max {
        max
    } else {
        return Ok(());
    };
    Err(Raise::new(
        arg_error(ruby),
        format!("wrong number of arguments ({} for {expected})", args.len()),
    ))
}

/// An integer argument of 64 bits: a `Fixnum`, a `Bignum` in range, or a `Float`, which is
/// truncated. Anything else raises `TypeError`, a number out of range `RangeError`.
pub(crate) fn long(ruby: &Ruby, value: VALUE) -> Result<i64, Raise> {
    if ffi::fixnum_p(value) {
        return Ok(ffi::fix2long(value));
    }
    Ok(perform(ruby, Op::Long(value))?.1.long)
}

/// An integer argument, as Ruby's own `NUM2INT` converts it: see [`long`], with the range of
/// 32 bits.
pub(crate) fn int(ruby: &Ruby, value: VALUE) -> Result<i32, Raise> {
    let number = long(ruby, value)?;
    i32::try_from(number).map_err(|_| {
        let size = if number < 0 { "small" } else { "big" };
        Raise::new(
            range_error(ruby),
            format!("integer {number} too {size} to convert to `int'"),
        )
    })
}

/// A floating-point argument: any `Numeric`. Anything else raises `TypeError`.
pub(crate) fn float(ruby: &Ruby, value: VALUE) -> Result<f64, Raise> {
    if ffi::fixnum_p(value) {
        return Ok(ffi::fix2long(value) as f64);
    }
    Ok(perform(ruby, Op::Double(value))?.1.double)
}

/// A truth value, as Ruby tests one.
pub(crate) fn truthy(value: VALUE) -> bool {
    ffi::rtest(value)
}

/// The bytes of a `String` argument, or `None` for anything else.
pub(crate) fn string_bytes(ruby: &Ruby, value: VALUE) -> Option<Vec<u8>> {
    // SAFETY: the value is live by the condition of this module.
    unsafe { ruby.str_bytes(value) }
}

/// The bytes of a `String` argument. Anything else raises `TypeError`.
pub(crate) fn bytes(ruby: &Ruby, value: VALUE) -> Result<Vec<u8>, Raise> {
    string_bytes(ruby, value).ok_or_else(|| {
        Raise::new(
            type_error(ruby),
            format!("can't convert {} into String", class_name(ruby, value)),
        )
    })
}

/// A `String` argument as text. Bytes that are not valid UTF-8 are replaced, not rejected.
/// Anything but a `String` raises `TypeError`.
pub(crate) fn text(ruby: &Ruby, value: VALUE) -> Result<String, Raise> {
    Ok(String::from_utf8_lossy(&bytes(ruby, value)?).into_owned())
}

/// Any value as text: a `String` as it is, anything else through its `to_s`. Bytes that are
/// not valid UTF-8 are replaced.
pub(crate) fn to_text(ruby: &Ruby, value: VALUE) -> Result<String, Raise> {
    let string = perform(ruby, Op::AsString(value))?.0;
    Ok(String::from_utf8_lossy(&string_bytes(ruby, string).unwrap_or_default()).into_owned())
}

/// The elements of an `Array`, or `None` for anything else. The copies are for immediate
/// use: keep them in this frame and do not hold them across a call that can collect unless
/// the array itself stays in a local.
pub(crate) fn array_elements(_ruby: &Ruby, value: VALUE) -> Option<Vec<VALUE>> {
    // SAFETY: the value is live by the condition of this module; the type check makes the
    // cast valid, and an array's `ptr` is valid for `len` elements when it is not null.
    unsafe {
        if ffi::rb_type(value) != ffi::T_ARRAY {
            return None;
        }
        let array = &*ffi::rarray(value);
        if array.ptr.is_null() || array.len <= 0 {
            return Some(Vec::new());
        }
        Some(std::slice::from_raw_parts(array.ptr, array.len as usize).to_vec())
    }
}

// --- Results ----------------------------------------------------------------------------------

/// The `Fixnum` of a 32-bit integer.
pub(crate) fn fix(integer: i32) -> VALUE {
    ffi::long2fix(c_long::from(integer))
}

/// `true` or `false`.
pub(crate) fn boolean(value: bool) -> VALUE {
    if value { ffi::Qtrue } else { ffi::Qfalse }
}

/// A new `Float`.
pub(crate) fn float_new(ruby: &Ruby, number: f64) -> HostResult {
    Ok(perform(ruby, Op::Float(number))?.0)
}

/// A new `String` of these bytes.
pub(crate) fn string_new(ruby: &Ruby, bytes: &[u8]) -> HostResult {
    Ok(ruby.str_new(bytes)?)
}

/// A new `Array` of new `String`s.
pub(crate) fn strings_new(ruby: &Ruby, strings: &[String]) -> HostResult {
    let items: Vec<(*const c_char, c_long)> = strings
        .iter()
        .map(|string| (string.as_ptr().cast(), string.len() as c_long))
        .collect();
    let op = Op::Strings {
        items: items.as_ptr(),
        len: items.len(),
    };
    Ok(perform(ruby, op)?.0)
}

// --- Objects ----------------------------------------------------------------------------------

/// A new instance of `class` from its allocator, without `initialize`.
pub(crate) fn allocate(ruby: &Ruby, class: VALUE) -> Result<VALUE, Raised> {
    Ok(perform(ruby, Op::Alloc(class))?.0)
}

/// Sets the allocator of `class`.
pub(crate) fn define_alloc(ruby: &Ruby, class: VALUE, function: AllocFn) -> Result<(), Raised> {
    perform(ruby, Op::AllocFunc { class, function }).map(|_| ())
}

/// Sets an instance variable. A name without `@` is invisible to scripts, which is how the
/// binding keeps the Ruby objects that belong to an object reachable from it.
pub(crate) fn ivar_set(
    ruby: &Ruby,
    object: VALUE,
    name: &CStr,
    value: VALUE,
) -> Result<(), Raised> {
    let op = Op::IvarSet {
        object,
        name: name.as_ptr(),
        value,
    };
    perform(ruby, op).map(|_| ())
}

/// An instance variable, `nil` when it is not set.
pub(crate) fn ivar_get(ruby: &Ruby, object: VALUE, name: &CStr) -> Result<VALUE, Raised> {
    let op = Op::IvarGet {
        object,
        name: name.as_ptr(),
    };
    Ok(perform(ruby, op)?.0)
}

// --- Definitions ------------------------------------------------------------------------------

/// The arity of a method as the method list of the original runtime writes it: -1 for any
/// number of arguments, otherwise the fixed count.
fn arity_of(listed: i8) -> Arity {
    match u8::try_from(listed) {
        Ok(count) => Arity::Fixed(count),
        Err(_) => Arity::Any,
    }
}

/// Defines an instance method of `class` with the arity of the method list.
pub(crate) fn method<F>(
    ruby: &Ruby,
    class: VALUE,
    name: &CStr,
    arity: i8,
    function: F,
) -> Result<(), Raised>
where
    F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
{
    // SAFETY: the class is live by the condition of this module.
    unsafe { ruby.define_method_with_arity(class, name, arity_of(arity), function) }
}

/// Defines a class-level method of `class` with the arity of the method list.
pub(crate) fn class_method<F>(
    ruby: &Ruby,
    class: VALUE,
    name: &CStr,
    arity: i8,
    function: F,
) -> Result<(), Raised>
where
    F: Fn(&Ruby, VALUE, &[VALUE]) -> HostResult + Copy + 'static,
{
    // SAFETY: the class is live by the condition of this module.
    unsafe { ruby.define_singleton_method_with_arity(class, name, arity_of(arity), function) }
}
