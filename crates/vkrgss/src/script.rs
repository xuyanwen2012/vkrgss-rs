//! Script mode: `vkrgss --script FILE [ARG...]` runs a Ruby file with the interpreter of a
//! runtime and no game.
//!
//! The program settles the runtime (`--runtime`, `rgss3` without it), the sandbox
//! (`--sandbox`, then `VKRGSS_SANDBOX`), the functions that start processes and the level of
//! the diagnostics from its command line and its environment; it reads no configuration
//! file. A script of RGSS1 or RGSS2 runs in this process, over the Ruby 1.8.7 the program
//! links (`run_script` of `vkrgss-guest-main`). Ruby 1.9.2, which a script of RGSS3 needs,
//! cannot be linked beside it, so for such a script the program replaces itself, with
//! `exec`, by the guest executable `vkrgss-guest-rgss3` beside it, with the hidden option of
//! script mode of `vkrgss-guest`: the process, its standard streams and its status stay
//! those of the program.
//!
//! Either way the interpreter thread confines itself before the script runs, as a guest
//! process does (the sandbox and the filter of `vkrgss-guest`), with the directory of the
//! script in the place of the game directory, unless `--sandbox off` or
//! `--allow-process-functions` is given.

use std::os::unix::process::CommandExt;
use std::path::absolute;
use std::process::{Command, ExitCode};

use anyhow::Result;
use vkrgss_guest::diagnostics;
use vkrgss_guest::script::ScriptRole;
use vkrgss_vfs::Runtime;

use crate::args::{Options, ScriptFile};
use crate::process;

/// The runtime of a script for which none was given.
const DEFAULT_RUNTIME: Runtime = Runtime::Rgss3;

/// Runs `script` as `options` say. `log` is the value of the environment variable of the
/// diagnostics as the program read it, which the executable of RGSS3 is handed under its
/// name. The status is that of the script, and 1 with a message when it could not start.
pub(crate) fn run(script: ScriptFile, options: &Options, log: Option<&str>) -> ExitCode {
    match start(script, options, log) {
        Ok(status) => status,
        Err(error) => {
            eprintln!("vkrgss: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn start(script: ScriptFile, options: &Options, log: Option<&str>) -> Result<ExitCode> {
    let role = ScriptRole {
        runtime: options.runtime.unwrap_or(DEFAULT_RUNTIME),
        sandbox: options.sandbox,
        process_functions: options.process_functions,
        verbose: options.verbose,
        file: absolute(&script.file)?,
        arguments: script.arguments.into_iter().map(Into::into).collect(),
    };
    tracing::info!("runtime of the script: {}", role.runtime.name());
    if vkrgss_guest_main::interpreter_runs(role.runtime) {
        return vkrgss_guest_main::run_script(&role);
    }
    let program = process::guest_executable(role.runtime)?;
    let mut command = Command::new(&program);
    command.args(role.arguments());
    match log {
        Some(value) => command.env(diagnostics::VARIABLE, value),
        None => command.env_remove(diagnostics::VARIABLE),
    };
    // `exec` returns only when it failed.
    let error = command.exec();
    Err(anyhow::Error::new(error).context(format!("cannot start {}", program.display())))
}
