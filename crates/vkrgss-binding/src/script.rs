//! Running a Ruby file without a game: script mode.
//!
//! [`run_script`] prepares an interpreter that has run nothing yet with what a game of its
//! interpreter gets of the prelude outside the RGSS classes, which are not defined: over
//! Ruby 1.8 `$KCODE` and the empty load path, `require` and `load` limited to the
//! directory of the file, which stands in for the game directory, over Ruby 1.9 also
//! `require_relative` and `autoload` of RGSS3, and the functions that start processes
//! turned off unless the [`Script`] enables them. Then it evaluates the file at the top
//! level, with its arguments as `ARGV`, runs the blocks of `at_exit` and says what status
//! the process ends with, as `ruby` ends.
//!
//! `vkrgss_locate` and `vkrgss_relative`, the host functions those `require` and `load` build
//! on, find the files in the directory of the script, as they find those of a game in the
//! game directory: no guest is installed, and nothing else of `kernel` is defined.

use std::ffi::OsString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use vkrgss_ruby_sys::ffi::{self, VALUE};
use vkrgss_ruby_sys::{Arity, HostResult, Raised, Ruby, RubyError};
use vkrgss_vfs::{FileKind, GameFiles};

use crate::{kernel, rb, runtime};

/// A Ruby file to run, and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    /// The file, as an absolute path. Its directory is where `require` and `load` find
    /// files, and the path is `$0` and the file name of the top level.
    pub file: PathBuf,
    /// The arguments, which are `ARGV`.
    pub arguments: Vec<OsString>,
    /// Whether `system`, backticks, `exec`, `fork`, `syscall`, `IO.popen`, `Process.kill`,
    /// `open` with the name of a command and the like, and over Ruby 1.9 `spawn`, start or
    /// signal processes. They do nothing when this is `false`.
    pub process_functions: bool,
}

/// Why a script could not be run at all.
#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    /// The file could not be read.
    #[error("cannot read the script {}: {source}", path.display())]
    Read {
        /// The file of the [`Script`].
        path: PathBuf,
        source: io::Error,
    },
    /// The prelude or the host functions it builds on could not be defined.
    #[error("the interpreter could not be prepared: {0}")]
    Prepare(RubyError),
}

/// The files of the directory of the script, which `require` and `load` find files in. Not
/// a value of the interpreter thread: over Ruby 1.9 a Ruby thread is a thread of the system
/// and loads files too.
static FILES: OnceLock<GameFiles> = OnceLock::new();

/// The files of the directory of the script, for `access`, which does not call into Ruby.
fn with_files<R>(access: impl FnOnce(&GameFiles) -> R) -> R {
    access(FILES.get().expect("a script is running in this process"))
}

/// `vkrgss_locate` of a script: [`kernel::locate_with`] in the directory of the script.
fn locate(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    kernel::locate_with(ruby, args, |file| {
        with_files(|files| files.resolve(FileKind::Data, file))
    })
}

/// `vkrgss_relative` of a script: [`kernel::relative_with`] below the directory of the
/// script.
fn relative(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    kernel::relative_with(ruby, args, || {
        with_files(|files| files.game_dir().to_owned())
    })
}

/// Runs `script` in an interpreter that has run nothing yet, on the interpreter thread, and
/// returns the status the process ends with, as `ruby` ends: the status of
/// `exit` (`SystemExit`), from the file or from a block of `at_exit`, 1 after an exception
/// that nothing rescued, whose report has been written to the error stream as `ruby`
/// writes it, and 0 otherwise. The blocks of `at_exit` run after the file, with the
/// exception that ended it as `$!`. `exit!` ends the process itself, and so does an
/// uncaught `SignalException`, such as `Interrupt`, by its signal. Called once per process.
pub fn run_script(ruby: &Ruby, script: &Script) -> Result<i32, ScriptError> {
    let source = std::fs::read(&script.file).map_err(|source| ScriptError::Read {
        path: script.file.clone(),
        source,
    })?;
    let dir = script.file.parent().unwrap_or(Path::new("/"));
    // One script runs per process, so the files are set once.
    let _ = FILES.set(GameFiles::new(dir, []));
    prepare(ruby, script.process_functions).map_err(ScriptError::Prepare)?;
    set_arguments(ruby, &script.arguments)
        .map_err(|raised| ScriptError::Prepare(describe(ruby, raised)))?;

    let name = String::from_utf8_lossy(script.file.as_os_str().as_bytes()).into_owned();
    let evaluated =
        rb::text_new(ruby, &source).and_then(|source| runtime::evaluate(ruby, source, &name));
    // The status after the file, and the exception that ended it, if one did.
    let (status, ended) = match evaluated {
        Ok(()) => (0, ffi::Qnil),
        Err(raised) => {
            let exception = if raised.state == ffi::TAG_RAISE {
                raised.exception
            } else {
                ffi::Qnil
            };
            match exit_status(ruby, exception) {
                Some(status) => (status, exception),
                // `ruby` writes no report for a `SignalException` itself, which ends it
                // by its signal; it does for one of a subclass, such as `Interrupt`.
                // SAFETY: `&Ruby` proves the thread, and the exception is in a local.
                None if is_exception(exception)
                    && unsafe { ffi::rb_obj_class(exception) == ffi::rb_eSignal } =>
                {
                    (1, exception)
                }
                None => {
                    let mut error = describe(ruby, raised);
                    // Over Ruby 1.9 the last position is the `eval` of the top level, which
                    // `ruby` has not.
                    let top = format!("{name}:in `eval'");
                    if error.backtrace.last() == Some(&top) {
                        error.backtrace.pop();
                    }
                    eprint!("{}", report(&error));
                    (1, exception)
                }
            }
        }
    };
    // SAFETY: `ended` is `nil` or the exception of `raised`, in a local since it was raised.
    unsafe { ruby.set_current_error(ended) };
    let after = ruby.run_end_procs();
    // SAFETY: `nil` is a value for the current error. What finalises the interpreter does
    // not see the errors of the script again.
    unsafe { ruby.set_current_error(ffi::Qnil) };
    let end = end_status(ruby, status, [after, ended]);
    // A script may have turned the synchronous writing of the streams off.
    let _ = ruby.eval("$stdout.flush\n$stderr.flush\n", PRELUDE_NAME, 1);
    match end {
        End::Status(status) => Ok(status),
        End::Signal(signal) => {
            // As `ruby_default_signal` of Ruby 1.9.2, for both interpreters: Ruby 1.8.7's
            // sends the signal with `kill`, which the filter of a confined guest denies.
            // SAFETY: the default action is a valid disposition for every signal, and
            // `raise` sends the signal to the calling thread; neither takes a pointer.
            unsafe {
                libc::signal(signal, libc::SIG_DFL);
                libc::raise(signal);
            }
            // A signal that does not end the process, as `ruby` goes on after it.
            Ok(status.max(1))
        }
    }
}

/// How a script ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum End {
    /// With this status.
    Status(i32),
    /// By this signal, after an uncaught `SignalException`.
    Signal(i32),
}

/// How a script whose file ended with `status` ends, by `errors`: the current error after
/// the blocks of `at_exit`, and the exception that ended the file or `nil`. As
/// `ruby_cleanup` of both interpreters: the first of them that is a `SystemExit` gives its
/// status and the first that is a `SignalException` its signal; another exception makes a
/// status of 0 a 1.
fn end_status(ruby: &Ruby, status: i32, errors: [VALUE; 2]) -> End {
    let mut status = status;
    for error in errors {
        if !is_exception(error) {
            continue;
        }
        if let Some(exit) = exit_status(ruby, error) {
            return End::Status(exit);
        }
        if let Some(signal) = signal_number(ruby, error) {
            return End::Signal(signal);
        }
        if status == 0 {
            status = 1;
        }
    }
    End::Status(status)
}

/// Whether `value` is an exception. Over Ruby 1.9 the current error can hold an internal
/// node, which is not an object.
fn is_exception(value: VALUE) -> bool {
    // SAFETY: a value that is no special constant has a type in its header; `&Ruby` of the
    // callers proves the thread, and the class globals are set once the interpreter runs.
    !ffi::special_const_p(value)
        && unsafe { ffi::builtin_type(value) } == ffi::T_OBJECT
        && rb::truthy(unsafe { ffi::rb_obj_is_kind_of(value, ffi::rb_eException) })
}

/// The status of `exception` if it is a `SystemExit`, and `None` for anything else.
fn exit_status(ruby: &Ruby, exception: VALUE) -> Option<i32> {
    if !is_exception(exception) {
        return None;
    }
    // SAFETY: `&Ruby` proves the thread; the class globals are set once the interpreter runs,
    // and the caller keeps the exception in a local.
    let exiting = unsafe { ffi::rb_obj_is_kind_of(exception, ffi::rb_eSystemExit) };
    if !rb::truthy(exiting) {
        return None;
    }
    // SAFETY: as above; `status` of a `SystemExit` returns an Integer.
    let status = unsafe { ruby.funcall(exception, c"status", &[]) }.ok()?;
    rb::int(ruby, status).ok()
}

/// The signal of `exception` if it is a `SignalException`, and `None` for anything else.
fn signal_number(ruby: &Ruby, exception: VALUE) -> Option<i32> {
    // SAFETY: as for `exit_status`.
    let signalled = unsafe { ffi::rb_obj_is_kind_of(exception, ffi::rb_eSignal) };
    if !rb::truthy(signalled) {
        return None;
    }
    // The variable without `@` that both interpreters keep the number in; the method `signo`
    // of Ruby 1.8 reads another one.
    let signal = rb::ivar_get(ruby, exception, c"signo").ok()?;
    rb::int(ruby, signal).ok()
}

/// The file name the prelude is reported under in backtraces, as in a game.
const PRELUDE_NAME: &str = "(prelude)";

/// Defines the host functions of [`locate`] and [`relative`] and evaluates the parts of the
/// prelude of a game that a script gets.
fn prepare(ruby: &Ruby, process_functions: bool) -> Result<(), RubyError> {
    let describe = |raised| describe(ruby, raised);
    ruby.define_global_function_with_arity(c"vkrgss_locate", Arity::Fixed(2), locate)
        .map_err(describe)?;
    ruby.define_global_function_with_arity(c"vkrgss_relative", Arity::Fixed(2), relative)
        .map_err(describe)?;
    runtime::prepare_loading(ruby)?;
    if !process_functions {
        runtime::turn_process_functions_off(ruby)?;
    }
    Ok(())
}

/// Puts `arguments` into `ARGV`, which the interpreter defined empty, as text.
fn set_arguments(ruby: &Ruby, arguments: &[OsString]) -> Result<(), Raised> {
    let argv = ruby.eval("ARGV", PRELUDE_NAME, 1)?;
    for argument in arguments {
        let text = rb::text_new(ruby, argument.as_bytes())?;
        // SAFETY: `ARGV` is kept alive by its constant, and `text` was just returned and is
        // in a local.
        unsafe { ruby.funcall(argv, c"push", &[text]) }?;
    }
    Ok(())
}

/// The class name, message and backtrace of a raised exception.
fn describe(ruby: &Ruby, raised: Raised) -> RubyError {
    // SAFETY: `raised` was produced just now on this thread and sat in a local since.
    unsafe { ruby.describe(raised) }
}

/// The report of an exception that nothing rescued, as `ruby` writes it to the error
/// stream: the first position of the backtrace, the first line of the message and the
/// class in parentheses, the other lines of the message, and a line `from` for each
/// further position. A `RuntimeError` without a message is an `unhandled exception`, and
/// another exception without one is named by its class alone.
fn report(error: &RubyError) -> String {
    let mut text = String::new();
    if let Some(position) = error.backtrace.first() {
        text.push_str(position);
        text.push_str(": ");
    }
    match error.message.split_once('\n') {
        _ if error.message.is_empty() && error.class_name == "RuntimeError" => {
            text.push_str("unhandled exception\n");
        }
        _ if error.message.is_empty() => {
            text.push_str(&error.class_name);
            text.push('\n');
        }
        None => text.push_str(&format!("{} ({})\n", error.message, error.class_name)),
        Some((first, rest)) => {
            text.push_str(&format!("{first} ({})\n{rest}", error.class_name));
            if !rest.ends_with('\n') {
                text.push('\n');
            }
        }
    }
    for position in error.backtrace.iter().skip(1) {
        text.push_str(&format!("\tfrom {position}\n"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(class_name: &str, message: &str, backtrace: &[&str]) -> RubyError {
        RubyError {
            class_name: class_name.to_owned(),
            message: message.to_owned(),
            backtrace: backtrace.iter().map(|line| (*line).to_owned()).collect(),
        }
    }

    #[test]
    fn a_report_is_written_as_ruby_writes_it() {
        let raised = error(
            "RuntimeError",
            "boom",
            &["s.rb:2:in `f'", "s.rb:4:in `<main>'"],
        );
        assert_eq!(
            report(&raised),
            "s.rb:2:in `f': boom (RuntimeError)\n\tfrom s.rb:4:in `<main>'\n"
        );
        let lines = error("ArgumentError", "one\ntwo", &["s.rb:1"]);
        assert_eq!(report(&lines), "s.rb:1: one (ArgumentError)\ntwo\n");
        let unhandled = error("RuntimeError", "", &["s.rb:1"]);
        assert_eq!(report(&unhandled), "s.rb:1: unhandled exception\n");
        let bare = error("IndexError", "", &[]);
        assert_eq!(report(&bare), "IndexError\n");
    }
}
