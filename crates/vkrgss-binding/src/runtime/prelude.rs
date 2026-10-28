//! The preludes, the Ruby part of the preparation of the interpreter, and their evaluation.

use vkrgss_ruby_sys::{Raised, Ruby, RubyError};
use vkrgss_vfs::Runtime;

use crate::guest::Guest;
use crate::{audio, graphics, input, kernel, rb, win32api};

use super::startup::{rgss3_version, rgss3_version_source, test_mode_globals};

/// The Ruby part of the preparation of a game of RGSS1 or RGSS2: `Reset`, `save_data`,
/// `print`, `p` and the loader of the script archive.
const PRELUDE: &str = include_str!("../prelude.rb");

/// The Ruby part of the preparation of a game of RGSS3, in the place of [`PRELUDE`] and
/// after [`PRELUDE_SHARED`]: `RGSSReset`, `save_data`, `at_exit`, `exit!`, `msgbox`,
/// `msgbox_p` and the loader of the script archive, and [`PRELUDE_LOAD_RGSS3`] after it.
/// `RGSS_VERSION` is defined before it ([`rgss3_version`]), and `load_data` is a host
/// function of [`kernel`].
const PRELUDE_RGSS3: &str = include_str!("../prelude_rgss3.rb");

/// The rules of loading of RGSS3, after [`PRELUDE_RGSS3`], in whose lines from
/// [`PRELUDE_LOAD_RGSS3_LINE`] on they stood: `require` and `load` that compile as UTF-8,
/// `require_relative` and `autoload`. A script over Ruby 1.9 gets them too.
const PRELUDE_LOAD_RGSS3: &str = include_str!("../prelude_load_rgss3.rb");

/// The line [`PRELUDE_LOAD_RGSS3`] is evaluated from, so that its backtraces have the
/// positions they had in [`PRELUDE_RGSS3`].
const PRELUDE_LOAD_RGSS3_LINE: u32 = 48;

/// What the preparation evaluates for every runtime, after [`PRELUDE`] or before
/// [`PRELUDE_RGSS3`]: `require` and `load` limited to the game directory.
const PRELUDE_SHARED: &str = include_str!("../prelude_shared.rb");

/// `Win32API` of RGSS1 and RGSS2, evaluated before [`PRELUDE_SHARED`]. In RGSS3 the class is
/// one of the binding.
const PRELUDE_WIN32API: &str = include_str!("../prelude_win32api.rb");

/// `DL` of RGSS3, evaluated after [`PRELUDE_SHARED`].
const PRELUDE_DL: &str = include_str!("../prelude_dl.rb");

/// What is prepared over Ruby 1.8 only, before the prelude: `$KCODE` and the empty load path.
const PRELUDE_RUBY18: &str = include_str!("../prelude_ruby18.rb");

/// What the prelude has in RGSS1 only: `Hangup`.
const PRELUDE_RGSS1: &str = include_str!("../prelude_rgss1.rb");

/// The Ruby definitions that turn the functions that start or signal processes off.
const PROCESS_FUNCTIONS_OFF: &str = include_str!("../prelude_process_off.rb");

/// What turns off the functions that start processes that Ruby 1.9 has beside them.
const PROCESS_FUNCTIONS_OFF_RUBY19: &str = include_str!("../prelude_process_off_ruby19.rb");

/// The file name the prelude is reported under in backtraces.
pub(super) const PRELUDE_NAME: &str = "(prelude)";

/// Defines what a game meets outside the RGSS classes and that is implemented by host
/// functions. The Ruby part follows in [`prepare`].
pub(crate) fn define(ruby: &Ruby) -> Result<(), Raised> {
    graphics::define(ruby)?;
    input::define(ruby, Guest::with(ruby, |guest| guest.runtime.run.runtime))?;
    audio::define(ruby)?;
    win32api::define(ruby, Guest::with(ruby, |guest| guest.runtime.run.runtime))?;
    kernel::define(ruby)
}

/// Prepares the interpreter as the original runtime does before a game's scripts run:
/// evaluates the prelude of the runtime and the shared one, sets the globals of test mode
/// from the guest's [`RunConfig`], and turns the process functions off unless it enables
/// them.
///
/// [`RunConfig`]: super::RunConfig
pub(crate) fn prepare(ruby: &Ruby) -> Result<(), RubyError> {
    let (runtime, debug, battle_test, process_functions, library_version) =
        Guest::with(ruby, |guest| {
            let run = &guest.runtime.run;
            (
                run.runtime,
                run.debug,
                run.battle_test,
                run.process_functions,
                run.library_version,
            )
        });
    if !rb::RUBY_19 {
        ruby.eval_protected(PRELUDE_RUBY18, PRELUDE_NAME, 1)?;
    }
    if runtime == Runtime::Rgss3 {
        let version = rgss3_version_source(rgss3_version(library_version));
        ruby.eval_protected(&version, PRELUDE_NAME, 1)?;
        // The prelude of RGSS3 replaces `require` and `load` of the shared one.
        ruby.eval_protected(PRELUDE_SHARED, PRELUDE_NAME, 1)?;
        ruby.eval_protected(PRELUDE_RGSS3, PRELUDE_NAME, 1)?;
        ruby.eval_protected(PRELUDE_LOAD_RGSS3, PRELUDE_NAME, PRELUDE_LOAD_RGSS3_LINE)?;
        ruby.eval_protected(PRELUDE_DL, PRELUDE_NAME, 1)?;
    } else {
        ruby.eval_protected(PRELUDE, PRELUDE_NAME, 1)?;
        ruby.eval_protected(PRELUDE_WIN32API, PRELUDE_NAME, 1)?;
        ruby.eval_protected(PRELUDE_SHARED, PRELUDE_NAME, 1)?;
    }
    if runtime == Runtime::Rgss1 {
        ruby.eval_protected(PRELUDE_RGSS1, PRELUDE_NAME, 1)?;
    }
    if !process_functions {
        turn_process_functions_off(ruby)?;
    }
    // Last, so that debug mode does not report the exceptions the prelude itself may meet.
    let flags = test_mode_globals(runtime, debug, battle_test);
    ruby.eval_protected(&flags, PRELUDE_NAME, 1)?;
    Ok(())
}

/// Turns the functions that start or signal processes off, those of Ruby 1.9 included.
pub(crate) fn turn_process_functions_off(ruby: &Ruby) -> Result<(), RubyError> {
    ruby.eval_protected(PROCESS_FUNCTIONS_OFF, PRELUDE_NAME, 1)?;
    if rb::RUBY_19 {
        ruby.eval_protected(PROCESS_FUNCTIONS_OFF_RUBY19, PRELUDE_NAME, 1)?;
    }
    Ok(())
}

/// Prepares the interpreter of a script ([`crate::script`]) with the rules of loading of a
/// game of its interpreter: over Ruby 1.8 `$KCODE` and an empty load path and the
/// `require` and `load` of [`PRELUDE_SHARED`], over Ruby 1.9 those of RGSS3,
/// `require_relative` and `autoload` after it. The host functions they build on are the
/// caller's to define.
pub(crate) fn prepare_loading(ruby: &Ruby) -> Result<(), RubyError> {
    if !rb::RUBY_19 {
        ruby.eval_protected(PRELUDE_RUBY18, PRELUDE_NAME, 1)?;
    }
    ruby.eval_protected(PRELUDE_SHARED, PRELUDE_NAME, 1)?;
    if rb::RUBY_19 {
        ruby.eval_protected(PRELUDE_LOAD_RGSS3, PRELUDE_NAME, PRELUDE_LOAD_RGSS3_LINE)?;
    }
    Ok(())
}
