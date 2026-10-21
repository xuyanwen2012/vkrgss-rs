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
    /// The backtrace, innermost position first, as `file:line` or ``file:line:in `method'``.
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

unsafe extern "C" fn end_procs_body(_data: VALUE) -> VALUE {
    // SAFETY: `Ruby::run_end_procs` calls this on the interpreter thread. Each block runs
    // under a tag of the interpreter, so nothing raises out of the call.
    unsafe {
        ffi::rb_exec_end_proc();
        ffi::ruby_errinfo
    }
}

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
    unsafe {
        let eval = &*(data as *const Eval);
        let args = [
            ffi::rb_str_new(eval.source, eval.source_len),
            ffi::Qnil,
            ffi::rb_str_new(eval.file, eval.file_len),
            ffi::long2fix(eval.line),
        ];
        ffi::rb_funcall2(
            ffi::ruby_top_self,
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
            while index < (*ffi::rarray(backtrace)).len {
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
}

unsafe extern "C" fn new_string_body(data: VALUE) -> VALUE {
    // SAFETY: `Ruby::str_new` passes a pointer to a `NewString` whose buffer outlives the
    // call. The allocation raises `NoMemoryError` when it fails, hence the protection.
    unsafe {
        let new = &*(data as *const NewString);
        ffi::rb_str_new(new.bytes, new.len)
    }
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
    // SAFETY: the type check makes the cast valid, and a string's `ptr` is valid for `len`
    // bytes when it is not null. Nothing here can raise or collect.
    unsafe {
        if ffi::rb_type(value) != ffi::T_STRING {
            return String::new();
        }
        let string = &*ffi::rstring(value);
        if string.ptr.is_null() || string.len <= 0 {
            return String::new();
        }
        let bytes = std::slice::from_raw_parts(string.ptr.cast::<u8>(), string.len as usize);
        String::from_utf8_lossy(bytes).into_owned()
    }
}

impl Ruby {
    /// Evaluates Ruby source at the top level, reported as `file` starting at `line`, and
    /// returns its value or the jump that left it.
    ///
    /// This calls Ruby's own `eval(source, nil, file, line)`. Unlike `rb_eval_string` that
    /// leaves exception messages alone and gives backtraces the positions asked for.
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
                let lines = &*ffi::rarray(describe.backtrace);
                for index in 0..usize::try_from(lines.len).unwrap_or(0) {
                    backtrace.push(text_of(*lines.ptr.add(index)));
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

    /// Runs the blocks of `at_exit` registered so far, the last first, as the interpreter
    /// runs them when it ends, and returns the current error, `$!`, after them: what it was
    /// before, or the exception of the last block that raised. The interpreter reports a
    /// block that raises on the error stream, as `ruby` does, and runs the next. The blocks
    /// are not run again when the interpreter ends. The value is the caller's to keep in a
    /// local.
    pub fn run_end_procs(&self) -> VALUE {
        // SAFETY: `&self` proves the thread, and `end_procs_body` fits the contract of
        // `protect`; it reads the current error before `protect` puts back the one it found.
        unsafe { protect::protect(&mut (), end_procs_body) }.unwrap_or(ffi::Qnil)
    }

    /// Makes `exception` the current error, `$!`, whose backtrace is `$@`. `nil` clears it.
    ///
    /// # Safety
    ///
    /// `exception` must be `nil` or a live exception object.
    pub unsafe fn set_current_error(&self, exception: VALUE) {
        // SAFETY: `&self` proves the thread, on which `ruby_errinfo` is only touched by the
        // interpreter, which is not running now. The caller guarantees the value, and the
        // variable is one the collector marks.
        unsafe { ffi::ruby_errinfo = exception };
    }

    /// The `main` object, `self` at the top level.
    pub fn top_self(&self) -> VALUE {
        // SAFETY: `&self` proves the thread, on which only the interpreter writes the
        // variable, once, at start-up.
        unsafe { ffi::ruby_top_self }
    }

    /// A new Ruby string with these bytes. The error is the `NoMemoryError` of a failed
    /// allocation.
    pub fn str_new(&self, bytes: &[u8]) -> Result<VALUE, Raised> {
        let mut new = NewString {
            bytes: bytes.as_ptr().cast(),
            len: c_long::try_from(bytes.len()).expect("fewer than 2^63 bytes"),
        };
        // SAFETY: `&self` proves the thread; `new_string_body` fits the contract of
        // `protect`, and the buffer is valid for `len` bytes and is copied.
        unsafe { protect::protect(&mut new, new_string_body) }
    }

    /// A new Ruby string with this text. A string of Ruby 1.8 has no encoding: it is the
    /// string [`str_new`](Self::str_new) makes, and the bytes need not be UTF-8. The error is
    /// the `NoMemoryError` of a failed allocation.
    pub fn str_new_text(&self, bytes: &[u8]) -> Result<VALUE, Raised> {
        self.str_new(bytes)
    }

    /// The bytes of a Ruby string, or `None` when the value is not a string.
    ///
    /// # Safety
    ///
    /// `value` must be a special constant or a live object.
    pub unsafe fn str_bytes(&self, value: VALUE) -> Option<Vec<u8>> {
        // SAFETY: the type check makes the cast valid, and a string's `ptr` is valid for
        // `len` bytes when it is not null.
        unsafe {
            if ffi::rb_type(value) != ffi::T_STRING {
                return None;
            }
            let string = &*ffi::rstring(value);
            if string.ptr.is_null() || string.len <= 0 {
                return Some(Vec::new());
            }
            Some(std::slice::from_raw_parts(string.ptr.cast::<u8>(), string.len as usize).to_vec())
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
