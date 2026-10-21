//! Running a Ruby file without a game: script mode.
//!
//! [`run_script`] prepares an interpreter that has run nothing yet with what a game of its
//! interpreter gets of the prelude outside the RGSS classes, which are not defined: over
//! Ruby 1.8 `$KCODE` and the empty load path, `require` and `load` limited to the
//! directory of the file, which stands in for the game directory, over Ruby 1.9 also
//! `require_relative` and `autoload` of RGSS3, and the functions that start processes
//! turned off unless the [`Script`] enables them. Then it evaluates the file at the top
//! level, with its arguments as `ARGV`, and says what status the process ends with.
//!
//! `vkrgss_locate` and `vkrgss_relative`, the host functions those `require` and `load` build
//! on, find the files in the directory of the script, as they find those of a game in the
//! game directory: no guest is installed, and nothing else of `kernel` is defined.

use std::cell::RefCell;
use std::ffi::OsString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

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
    /// Whether `system`, backticks, `exec`, `fork`, `syscall`, `IO.popen` and over Ruby 1.9
    /// `spawn` and the like start processes. They do nothing when this is `false`.
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

thread_local! {
    /// The files of the directory of the script, which `require` and `load` find files in.
    static FILES: RefCell<Option<GameFiles>> = const { RefCell::new(None) };
}

/// The files of the directory of the script, for `access`, which does not call into Ruby.
fn with_files<R>(access: impl FnOnce(&GameFiles) -> R) -> R {
    FILES.with(|cell| {
        let files = cell.borrow();
        access(files.as_ref().expect("a script is running on this thread"))
    })
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
/// returns the status the process ends with, as `ruby` ends: 0 when the file has been
/// evaluated to its end, the status of `exit` (`SystemExit`), and 1 after an exception that
/// nothing rescued, whose report has been written to the error stream as `ruby` writes it.
/// `exit!` ends the process itself. Called once per process.
pub fn run_script(ruby: &Ruby, script: &Script) -> Result<i32, ScriptError> {
    let source = std::fs::read(&script.file).map_err(|source| ScriptError::Read {
        path: script.file.clone(),
        source,
    })?;
    let dir = script.file.parent().unwrap_or(Path::new("/"));
    FILES.with(|cell| *cell.borrow_mut() = Some(GameFiles::new(dir, [])));
    prepare(ruby, script.process_functions).map_err(ScriptError::Prepare)?;
    set_arguments(ruby, &script.arguments)
        .map_err(|raised| ScriptError::Prepare(describe(ruby, raised)))?;

    let name = String::from_utf8_lossy(script.file.as_os_str().as_bytes()).into_owned();
    let evaluated =
        rb::text_new(ruby, &source).and_then(|source| runtime::evaluate(ruby, source, &name));
    let status = match evaluated {
        Ok(()) => 0,
        Err(raised) => match exit_status(ruby, raised) {
            Some(status) => status,
            None => {
                let mut error = describe(ruby, raised);
                // Over Ruby 1.9 the last position is the `eval` of the top level, which
                // `ruby` has not.
                let top = format!("{name}:in `eval'");
                if error.backtrace.last() == Some(&top) {
                    error.backtrace.pop();
                }
                eprint!("{}", report(&error));
                1
            }
        },
    };
    // A script may have turned the synchronous writing of the streams off.
    let _ = ruby.eval("$stdout.flush\n$stderr.flush\n", PRELUDE_NAME, 1);
    Ok(status)
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

/// The status of the `SystemExit` that left the script, and `None` for anything else.
fn exit_status(ruby: &Ruby, raised: Raised) -> Option<i32> {
    if raised.state != ffi::TAG_RAISE {
        return None;
    }
    // SAFETY: `&Ruby` proves the thread; the class globals are set once the interpreter runs,
    // and the exception of `raised` sat in a local since it was raised.
    let exiting = unsafe { ffi::rb_obj_is_kind_of(raised.exception, ffi::rb_eSystemExit) };
    if !rb::truthy(exiting) {
        return None;
    }
    // SAFETY: as above; `status` of a `SystemExit` returns an Integer.
    let status = unsafe { ruby.funcall(raised.exception, c"status", &[]) }.ok()?;
    rb::int(ruby, status).ok()
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
