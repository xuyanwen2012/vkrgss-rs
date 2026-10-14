//! Calling into Ruby: protected evaluation and method calls, and what an exception was.

use std::ffi::{CStr, c_char, c_long};
use std::fmt;

use crate::ffi::{self, VALUE};
use crate::interp::Ruby;
use crate::protect::{self, Raised};

/// A Ruby exception described in host terms, with no reference into the interpreter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RubyError {
    /// The name of the exception's class, such as `ArgumentError`. Empty when the jump out
    /// of Ruby was not an exception.
    pub class_name: String,
    /// The exception's message.
    pub message: String,
    /// The backtrace, innermost position first, as ``file:line:in `method'``; a position
    /// outside any method is ``file:line:in `<main>'``.
    pub backtrace: Vec<String>,
}

impl fmt::Display for RubyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.backtrace.first() {
            Some(position) => write!(f, "{position}: {} ({})", self.message, self.class_name),
            None => write!(f, "{} ({})", self.message, self.class_name),
        }
    }
}

impl std::error::Error for RubyError {}

/// The plain data of one evaluation.
struct Eval {
    source: *const c_char,
    source_len: c_long,
    file: *const c_char,
    file_len: c_long,
    line: c_long,
}

unsafe extern "C" fn eval_body(data: VALUE) -> VALUE {
    // SAFETY: `Ruby::eval` passes a pointer to an `Eval` whose buffers outlive the call. The
    // argument array is a local of this frame, where the collector's stack scan sees it.
    // The source is labelled UTF-8, which makes it the encoding of the literals in it.
    unsafe {
        let eval = &*(data as *const Eval);
        let utf8 = ffi::rb_utf8_encoding();
        let args = [
            ffi::rb_enc_str_new(eval.source, eval.source_len, utf8),
            ffi::Qnil,
            ffi::rb_enc_str_new(eval.file, eval.file_len, utf8),
            ffi::long2fix(eval.line),
        ];
        ffi::rb_funcall2(
            ffi::rb_vm_top_self(),
            ffi::rb_intern(c"eval".as_ptr()),
            4,
            args.as_ptr(),
        )
    }
}

/// The plain data of describing one exception: the input and three Ruby values as output.
struct Describe {
    exception: VALUE,
    class_name: VALUE,
    message: VALUE,
    backtrace: VALUE,
}

unsafe extern "C" fn describe_body(data: VALUE) -> VALUE {
    // SAFETY: `Ruby::describe` passes a pointer to a `Describe` on its stack, where the
    // collector sees the values stored into it. `message` and `backtrace` are Ruby methods
    // that a script may have redefined, which is why this runs protected.
    unsafe {
        let describe = &mut *(data as *mut Describe);
        let exception = describe.exception;
        describe.class_name = ffi::rb_class_path(ffi::rb_obj_class(exception));
        let message = ffi::rb_funcall(exception, ffi::rb_intern(c"message".as_ptr()), 0);
        describe.message = ffi::rb_obj_as_string(message);
        let backtrace = ffi::rb_funcall(exception, ffi::rb_intern(c"backtrace".as_ptr()), 0);
        let backtrace = ffi::rb_check_array_type(backtrace);
        let lines = ffi::rb_ary_new();
        describe.backtrace = lines;
        if !ffi::nil_p(backtrace) {
            let mut index: c_long = 0;
            while index < ffi::rarray_len(backtrace) {
                let line = ffi::rb_obj_as_string(ffi::rb_ary_entry(backtrace, index));
                ffi::rb_ary_push(lines, line);
                index += 1;
            }
        }
    }
    ffi::Qnil
}

/// The plain data of creating a string.
struct NewString {
    bytes: *const c_char,
    len: c_long,
    /// Whether the string is text, labelled UTF-8, or bytes, labelled ASCII-8BIT.
    text: bool,
}

unsafe extern "C" fn new_string_body(data: VALUE) -> VALUE {
    // SAFETY: `Ruby::str_new` and `Ruby::str_new_text` pass a pointer to a `NewString` whose
    // buffer outlives the call. The allocation raises `NoMemoryError` when it fails, hence
    // the protection.
    unsafe {
        let new = &*(data as *const NewString);
        if new.text {
            ffi::rb_enc_str_new(new.bytes, new.len, ffi::rb_utf8_encoding())
        } else {
            ffi::rb_str_new(new.bytes, new.len)
        }
    }
}

unsafe extern "C" fn set_errinfo_body(data: VALUE) -> VALUE {
    // SAFETY: `Ruby::set_current_error` passes a pointer to the value. `rb_set_errinfo`
    // raises `TypeError` for a value that is neither `nil` nor an exception, hence the
    // protection.
    unsafe { ffi::rb_set_errinfo(*(data as *const VALUE)) };
    ffi::Qnil
}

unsafe extern "C" fn inspect_body(data: VALUE) -> VALUE {
    // SAFETY: `Ruby::inspect` passes a pointer to the live value to inspect. `inspect` may
    // return any object, whose `to_s` runs in the conversion and may raise as well, so both
    // steps are inside the protection.
    unsafe {
        let value = *(data as *const VALUE);
        let text = ffi::rb_funcall(value, ffi::rb_intern(c"inspect".as_ptr()), 0);
        ffi::rb_obj_as_string(text)
    }
}

/// The bytes of a Ruby string as host text, with invalid UTF-8 replaced. Anything that is
/// not a string gives an empty text.
///
/// # Safety
///
/// Interpreter thread only; `value` must be a special constant or a live object.
unsafe fn text_of(value: VALUE) -> String {
    // SAFETY: the type check makes the accessors valid, and a string's bytes are valid for
    // its length when the pointer is not null. Nothing here can raise or collect.
    unsafe {
        if ffi::rb_type(value) != ffi::T_STRING {
            return String::new();
        }
        let (ptr, len) = (ffi::rstring_ptr(value), ffi::rstring_len(value));
        if ptr.is_null() || len <= 0 {
            return String::new();
        }
        let bytes = std::slice::from_raw_parts(ptr.cast::<u8>(), len as usize);
        String::from_utf8_lossy(bytes).into_owned()
    }
}

impl Ruby {
    /// Evaluates Ruby source at the top level, reported as `file` starting at `line`, and
    /// returns its value or the jump that left it.
    ///
    /// This calls Ruby's own `eval(source, nil, file, line)`. Unlike `rb_eval_string` that
    /// leaves exception messages alone and gives backtraces the positions asked for. The
    /// source is handed over as a UTF-8 string: `__ENCODING__` and the encoding of its
    /// string literals are UTF-8.
    pub fn eval(&self, source: &str, file: &str, line: u32) -> Result<VALUE, Raised> {
        let mut eval = Eval {
            source: source.as_ptr().cast(),
            source_len: c_long::try_from(source.len()).expect("a source below 2^63 bytes"),
            file: file.as_ptr().cast(),
            file_len: c_long::try_from(file.len()).expect("a file name below 2^63 bytes"),
            line: c_long::from(line),
        };
        // SAFETY: holding `&self` means this is the interpreter thread, and `eval_body` fits
        // the contract of `protect`.
        unsafe { protect::protect(&mut eval, eval_body) }
    }

    /// Evaluates Ruby source as [`eval`](Self::eval) does and returns its value, or the
    /// raised exception's class name, message and backtrace.
    ///
    /// Neither an exception nor a syntax error unwinds through host frames, and the
    /// interpreter stays usable after either.
    pub fn eval_protected(&self, source: &str, file: &str, line: u32) -> Result<VALUE, RubyError> {
        self.eval(source, file, line).map_err(|raised| {
            // SAFETY: `raised` was produced just now on this thread and sat in locals since.
            unsafe { self.describe(raised) }
        })
    }

    /// Calls a method by name, ignoring its visibility, and returns its value or the jump
    /// that left it.
    ///
    /// # Safety
    ///
    /// `recv` and every element of `args` must be special constants or live objects.
    pub unsafe fn funcall(
        &self,
        recv: VALUE,
        method: &CStr,
        args: &[VALUE],
    ) -> Result<VALUE, Raised> {
        // SAFETY: `&self` proves the thread; the caller guarantees the values.
        unsafe { protect::funcall(recv, method, args) }
    }

    /// Copies the class name, message and backtrace of a raised exception into host memory.
    ///
    /// # Safety
    ///
    /// `raised` must come from this interpreter, and its exception must still be alive: kept
    /// in a local since it was raised, or rooted.
    pub unsafe fn describe(&self, raised: Raised) -> RubyError {
        if raised.state != ffi::TAG_RAISE && raised.state != ffi::TAG_FATAL {
            // The jump is not continued: what it carried is no longer the current error.
            // SAFETY: `&self` proves the thread, and `nil` is always accepted.
            unsafe { ffi::rb_set_errinfo(ffi::Qnil) };
            return RubyError {
                class_name: String::new(),
                message: format!(
                    "Ruby code left through a non-local exit (tag {}) instead of returning",
                    raised.state
                ),
                backtrace: Vec::new(),
            };
        }
        let mut describe = Describe {
            exception: raised.exception,
            class_name: ffi::Qnil,
            message: ffi::Qnil,
            backtrace: ffi::Qnil,
        };
        // SAFETY: `describe_body` fits the contract of `protect`; the caller guarantees the
        // exception. Whatever the body stored before a failure is still a valid value, and
        // `text_of` copes with the `nil` of a field it did not reach.
        unsafe {
            let complete = protect::protect(&mut describe, describe_body).is_ok();
            let mut backtrace = Vec::new();
            if ffi::rb_type(describe.backtrace) == ffi::T_ARRAY {
                let lines = ffi::rarray_ptr(describe.backtrace);
                let len = ffi::rarray_len(describe.backtrace);
                for index in 0..usize::try_from(len).unwrap_or(0) {
                    backtrace.push(text_of(*lines.add(index)));
                }
            }
            let mut message = text_of(describe.message);
            if !complete && message.is_empty() {
                message = "(the exception could not be described)".to_owned();
            }
            RubyError {
                class_name: text_of(describe.class_name),
                message,
                backtrace,
            }
        }
    }

    /// Makes `exception` the current error, `$!`, whose backtrace is `$@`. `nil` clears it.
    ///
    /// # Safety
    ///
    /// `exception` must be `nil` or a live exception object.
    pub unsafe fn set_current_error(&self, exception: VALUE) {
        let mut exception = exception;
        // SAFETY: `&self` proves the thread, and `set_errinfo_body` fits the contract of
        // `protect`. The caller guarantees the value; with it the call cannot fail, and the
        // current error is a slot the collector marks.
        let _ = unsafe { protect::protect(&mut exception, set_errinfo_body) };
    }

    /// The `main` object, `self` at the top level.
    pub fn top_self(&self) -> VALUE {
        // SAFETY: `&self` proves the thread; the function reads a field of the interpreter.
        unsafe { ffi::rb_vm_top_self() }
    }

    /// A new Ruby string with these bytes, labelled ASCII-8BIT: for binary data, such as
    /// what a `_dump` method returns. The error is the `NoMemoryError` of a failed
    /// allocation.
    pub fn str_new(&self, bytes: &[u8]) -> Result<VALUE, Raised> {
        let mut new = NewString {
            bytes: bytes.as_ptr().cast(),
            len: c_long::try_from(bytes.len()).expect("fewer than 2^63 bytes"),
            text: false,
        };
        // SAFETY: `&self` proves the thread; `new_string_body` fits the contract of
        // `protect`, and the buffer is valid for `len` bytes and is copied.
        unsafe { protect::protect(&mut new, new_string_body) }
    }

    /// A new Ruby string with these bytes as text, labelled UTF-8 whether they are valid
    /// UTF-8 or not, as a string literal of a script is. The error is the `NoMemoryError` of
    /// a failed allocation.
    pub fn str_new_text(&self, bytes: &[u8]) -> Result<VALUE, Raised> {
        let mut new = NewString {
            bytes: bytes.as_ptr().cast(),
            len: c_long::try_from(bytes.len()).expect("fewer than 2^63 bytes"),
            text: true,
        };
        // SAFETY: as for `str_new`.
        unsafe { protect::protect(&mut new, new_string_body) }
    }

    /// [`str_new_text`](Self::str_new_text) of host text.
    pub fn str_new_utf8(&self, text: &str) -> Result<VALUE, Raised> {
        self.str_new_text(text.as_bytes())
    }

    /// The bytes of a Ruby string, whatever its encoding, or `None` when the value is not a
    /// string.
    ///
    /// # Safety
    ///
    /// `value` must be a special constant or a live object.
    pub unsafe fn str_bytes(&self, value: VALUE) -> Option<Vec<u8>> {
        // SAFETY: the type check makes the accessors valid, and a string's bytes are valid
        // for its length when the pointer is not null.
        unsafe {
            if ffi::rb_type(value) != ffi::T_STRING {
                return None;
            }
            let (ptr, len) = (ffi::rstring_ptr(value), ffi::rstring_len(value));
            if ptr.is_null() || len <= 0 {
                return Some(Vec::new());
            }
            Some(std::slice::from_raw_parts(ptr.cast::<u8>(), len as usize).to_vec())
        }
    }

    /// `value.inspect` as host text.
    ///
    /// Both the `inspect` method and the conversion of what it returns to a string run
    /// protected: an exception from either comes back as the error.
    ///
    /// # Safety
    ///
    /// `value` must be a special constant or a live object.
    pub unsafe fn inspect(&self, value: VALUE) -> Result<String, Raised> {
        let mut value = value;
        // SAFETY: `&self` proves the thread; the caller guarantees the value, and
        // `inspect_body` fits the contract of `protect`. The resulting string stays in a
        // local until its bytes are copied, which neither raises nor collects.
        unsafe {
            let text = protect::protect(&mut value, inspect_body)?;
            Ok(text_of(text))
        }
    }
}
