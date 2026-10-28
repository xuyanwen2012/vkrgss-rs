//! The scripts of a game: the built-in scripts, the preload scripts, the script archive, the
//! loop that evaluates them and the reset between two runs of it.

use vkrgss_core::InputState;
use vkrgss_ruby_sys::ffi::{self, VALUE};
use vkrgss_ruby_sys::{Raised, Ruby};
use vkrgss_vfs::Runtime;

use crate::guest::Guest;
use crate::{audio, rb};

use super::RunError;
use super::link::{reset_class_name, system_exit};
use super::prelude::PRELUDE_NAME;
use super::report::report_of_runtime;

/// The file name the built-in scripts are reported under.
const BUILTIN_NAME: &str = "(builtin)";

/// How an evaluation that did not return ended.
enum Ended {
    /// `SystemExit`: the game is over.
    Exit,
    /// `Reset`, or `RGSSReset` in RGSS3: the scripts start over.
    Reset,
    /// Anything else, as its report.
    Failed(String),
}

/// Evaluates a Ruby string at the top level under the file name `name`, as the original
/// runtime evaluates a script.
pub(crate) fn evaluate(ruby: &Ruby, source: VALUE, name: &str) -> Result<(), Raised> {
    let name = rb::bytes_new(ruby, name.as_bytes())?;
    // SAFETY: `&Ruby` proves the thread, on which the main object exists for the life of the
    // interpreter. `source` is live by the caller's word and `name` was just returned; both
    // are in locals.
    unsafe {
        let arguments = [source, ffi::Qnil, name, ffi::long2fix(1)];
        ruby.funcall(ruby.top_self(), c"eval", &arguments)?;
    }
    Ok(())
}

/// What a jump out of an evaluation means for the game.
fn ended(ruby: &Ruby, raised: Raised) -> Ended {
    let is_a = |class: VALUE| {
        // SAFETY: the exception has been in the local `raised` since it was raised, and the
        // class is kept alive by its constant; the check itself calls nothing.
        raised.state == ffi::TAG_RAISE
            && ffi::rtest(unsafe { ffi::rb_obj_is_kind_of(raised.exception, class) })
    };
    if is_a(system_exit(ruby)) {
        return Ended::Exit;
    }
    let runtime = Guest::with(ruby, |guest| guest.runtime.run.runtime);
    if rb::class_at(ruby, reset_class_name(runtime)).is_ok_and(is_a) {
        return Ended::Reset;
    }
    // SAFETY: as above, the exception is still alive in `raised`.
    let error = unsafe { ruby.describe(raised) };
    Ended::Failed(report_of_runtime(ruby, &error))
}

/// A preload script: the name it is evaluated under, which is its path as the caller gave
/// it, and its source.
type Preload = (String, Vec<u8>);

/// Seeds the random number generator with the seed of the guest's [`RunConfig`], if it has
/// one.
///
/// [`RunConfig`]: super::RunConfig
fn seed(ruby: &Ruby) -> Result<(), Raised> {
    if let Some(seed) = Guest::with(ruby, |guest| guest.runtime.run.seed) {
        ruby.eval(&format!("Kernel.srand({seed})"), PRELUDE_NAME, 1)?;
    }
    Ok(())
}

/// Reads the preload scripts, seeds the random number generator, evaluates the built-in
/// scripts file and reads the script archive. Returns the preload scripts and the number of
/// scripts in the archive.
pub(super) fn load(ruby: &Ruby) -> Result<(Vec<Preload>, usize), RunError> {
    let (builtin, archive, preload) = Guest::with(ruby, |guest| {
        let run = &guest.runtime.run;
        (
            run.builtin_scripts.clone(),
            run.ini.scripts.clone(),
            run.preload.clone(),
        )
    });
    let preload = preload
        .into_iter()
        .map(|path| match std::fs::read(&path) {
            Ok(source) => Ok((path.display().to_string(), source)),
            Err(source) => Err(RunError::Preload { path, source }),
        })
        .collect::<Result<Vec<Preload>, RunError>>()?;
    let source = std::fs::read(&builtin).map_err(|source| RunError::BuiltinScripts {
        path: builtin,
        source,
    })?;
    let prepare = |raised| {
        // SAFETY: `raised` was produced just now on this thread and sat in a local since.
        RunError::Prepare(unsafe { ruby.describe(raised) })
    };
    seed(ruby).map_err(prepare)?;
    // Bytes: the original of RGSS3 evaluates its built-in scripts from a string of bytes, so
    // their literals are ASCII-8BIT (a save shows it); over Ruby 1.8.7 bytes and text are one.
    let source = rb::bytes_new(ruby, &source).map_err(prepare)?;
    if let Err(raised) = evaluate(ruby, source, BUILTIN_NAME) {
        // SAFETY: as above.
        let error = unsafe { ruby.describe(raised) };
        return Err(RunError::Script(report_of_runtime(ruby, &error)));
    }

    let path = rb::text_new(ruby, archive.as_bytes()).map_err(prepare)?;
    // SAFETY: the main object lives as long as the interpreter, and `path` was just returned
    // and is in a local.
    let count = unsafe { ruby.funcall(ruby.top_self(), c"vkrgss_load_scripts", &[path]) };
    match count {
        Ok(count) if ffi::fixnum_p(count) => {
            Ok((preload, usize::try_from(ffi::fix2long(count)).unwrap_or(0)))
        }
        Ok(_) => Ok((preload, 0)),
        Err(raised) => {
            // SAFETY: as above.
            let error = unsafe { ruby.describe(raised) };
            Err(RunError::Archive {
                path: archive,
                reason: format!("{}: {}", error.class_name, error.message),
            })
        }
    }
}

/// What a reset does between two runs of the scripts: audio stops, every scene element and
/// bitmap is disposed, the input state starts afresh and the random number generator is
/// seeded again. The interpreter and what the scripts defined stay, as in the original
/// runtime.
///
/// In RGSS3 `Graphics.__reset__` is called through Ruby, which also sets the brightness and
/// the frame rate back and ends a freeze, and the generator is not seeded again: it was
/// seeded once, before the built-in scripts.
fn reset(ruby: &Ruby) -> Result<(), Raised> {
    let runtime = Guest::with(ruby, |guest| {
        audio::stop_all(guest);
        guest.runtime.run.runtime
    });
    let graphics = rb::class_at(ruby, c"Graphics")?;
    let method = match runtime {
        Runtime::Rgss1 | Runtime::Rgss2 => c"_reset",
        Runtime::Rgss3 => c"__reset__",
    };
    // SAFETY: the module is kept alive by its constant.
    unsafe { ruby.funcall(graphics, method, &[]) }?;
    Guest::with(ruby, |guest| {
        guest.input = InputState::with_rules(guest.input.rules());
        guest.runtime.pending = None;
    });
    match runtime {
        Runtime::Rgss1 | Runtime::Rgss2 => seed(ruby),
        Runtime::Rgss3 => Ok(()),
    }
}

/// Evaluates the source of the script at `index` of `$RGSS_SCRIPTS` as it is now, under the
/// name its runtime gives it: `Section` and the index as three digits, or in RGSS3 `{` and
/// four digits and `}`. In RGSS3 the source, which is stored as bytes, is compiled as UTF-8:
/// what is evaluated is a new string of text with its bytes, and the stored one is left as
/// it is.
fn evaluate_script(ruby: &Ruby, index: usize, runtime: Runtime) -> Result<(), Raised> {
    let source = ruby.eval(&format!("$RGSS_SCRIPTS[{index}][3]"), PRELUDE_NAME, 1)?;
    match runtime {
        Runtime::Rgss1 | Runtime::Rgss2 => evaluate(ruby, source, &format!("Section{index:03}")),
        Runtime::Rgss3 => {
            let text = match rb::string_bytes(ruby, source) {
                Some(bytes) => rb::text_new(ruby, &bytes)?,
                // `eval` raises what it raises for anything else.
                None => source,
            };
            evaluate(ruby, text, &format!("{{{index:04}}}"))
        }
    }
}

/// Evaluates the preload scripts and then the `count` scripts of `$RGSS_SCRIPTS` in order
/// ([`evaluate_script`]), again from the first preload script after a reset, until the last
/// returns or one exits. The runs after a reset start with the `Reset` that ended the one
/// before as the current error, as in the original runtime: `alias old new unless $@`
/// relies on it.
pub(super) fn run_scripts(ruby: &Ruby, preload: &[Preload], count: usize) -> Result<(), RunError> {
    let runtime = Guest::with(ruby, |guest| guest.runtime.run.runtime);
    let mut index = 0;
    while index < preload.len() + count {
        let evaluated = match preload.get(index) {
            Some((name, source)) => {
                rb::text_new(ruby, source).and_then(|source| evaluate(ruby, source, name))
            }
            None => evaluate_script(ruby, index - preload.len(), runtime),
        };
        index += 1;
        let Err(raised) = evaluated else {
            continue;
        };
        match ended(ruby, raised) {
            Ended::Exit => break,
            Ended::Reset => match reset(ruby) {
                Ok(()) => {
                    // SAFETY: the exception has been in the local `raised` since it was
                    // raised.
                    unsafe { ruby.set_current_error(raised.exception) };
                    index = 0;
                }
                Err(raised) => {
                    if let Ended::Failed(report) = ended(ruby, raised) {
                        return Err(RunError::Script(report));
                    }
                    break;
                }
            },
            Ended::Failed(report) => return Err(RunError::Script(report)),
        }
    }
    Ok(())
}
