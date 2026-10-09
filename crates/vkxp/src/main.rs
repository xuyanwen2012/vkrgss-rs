//! The vkxp executable: `vkxp <game directory>`.
//!
//! Side: executable.
//!
//! This is the one crate that depends on both sides and joins them. It reads the command
//! line and the game's configuration, starts the interpreter, and then runs the host on the
//! main thread, in a window or headless, and the game's scripts on the interpreter thread,
//! with the protocol link between them:
//!
//! 1. The arguments are parsed (`args`); a mistake there ends with the usage message and
//!    status 2.
//! 2. `Game.ini` is read, the run-time packages it names are looked up in the `--rtp` table
//!    and the built-in scripts file is found. A failure is a message and status 1, before
//!    any window or interpreter exists.
//! 3. The game directory becomes the working directory, because scripts write their saves
//!    with relative names. Paths given on the command line are made absolute first.
//! 4. SIGVTALRM is blocked while the main thread is still the only one (`signals`); the
//!    closure that runs the game unblocks it on the interpreter thread.
//! 5. The text backend over the font engine (`font`) and the binding's configuration are
//!    built and the interpreter is started.
//! 6. The host runs on the main thread. The guest thread, which the windowed runner spawns
//!    and the headless mode spawns itself, sends the game to the interpreter thread and
//!    waits for it.
//! 7. The guest is joined, which drops the interpreter, and the status is 0 only if the
//!    game ended without a failure and the host without an error.
//!
//! A script can loop without ever calling `Graphics.update` and then never sees a quit
//! request. After a headless run has ended, the program waits [`GUEST_WAIT`] for the game
//! and then exits with a message and a non-zero status.

mod args;
mod font;
mod signals;

use std::path::{Path, PathBuf, absolute};
use std::process::ExitCode;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use vkxp_binding::{Config, RunConfig, RunError, run_game};
use vkxp_host::{
    HeadlessOptions, InputScript, WindowedOptions, device_from_text, headless_link, run_headless,
    run_windowed,
};
use vkxp_proto::link::GuestEnd;
use vkxp_ruby_sys::Interpreter;
use vkxp_vfs::{GameFiles, GameIni, VfsError, resolve_rtps};

use crate::args::{Input, Options};
use crate::font::FontBackend;

/// How long the program waits for the game's scripts to end after the host has.
const GUEST_WAIT: Duration = Duration::from_secs(5);

/// The name of the tool that creates the built-in scripts file.
const EXTRACTION_TOOL: &str = "cargo xtask extract-rpg";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let options = match args::parse(&arguments) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("vkxp: {message}");
            eprintln!("{}", args::USAGE);
            return ExitCode::from(2);
        }
    };
    match run(options) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("vkxp: {error:#}");
            ExitCode::FAILURE
        }
    }
}

/// The built-in scripts file: the one named on the command line, or the default under the
/// user's data directory. Fails when it does not exist.
fn builtin_scripts(named: Option<PathBuf>) -> Result<PathBuf> {
    let path = match named {
        Some(path) => absolute(&path)?,
        None => {
            let variable = |name| std::env::var_os(name).filter(|value| !value.is_empty());
            let data = match (variable("XDG_DATA_HOME"), variable("HOME")) {
                (Some(data), _) => PathBuf::from(data),
                (None, Some(home)) => Path::new(&home).join(".local").join("share"),
                (None, None) => bail!(
                    "cannot tell where the built-in scripts file is: neither XDG_DATA_HOME nor \
                     HOME is set. Name it with --builtin-scripts; `{EXTRACTION_TOOL} <DLL> \
                     <OUTPUT>` creates it from your own RGSS DLL"
                ),
            };
            data.join("vkxp").join("rpg_builtin.rb")
        }
    };
    if !path.is_file() {
        bail!(
            "the built-in scripts file {path} does not exist. It holds the RPG scripts of the \
             original runtime, which are not part of vkxp: create it from your own RGSS DLL \
             with `{EXTRACTION_TOOL} <DLL> {path}`, or name another file with \
             --builtin-scripts",
            path = path.display()
        );
    }
    Ok(path)
}

/// The names `Font.default_name` starts with. The original runtime's Japanese edition, which
/// a trailing `J` in the name of the game's library stands for, starts with another font.
fn default_font_names(library: &str) -> Vec<String> {
    let stem = library.rsplit_once('.').map_or(library, |(stem, _)| stem);
    let name = if stem.ends_with(['J', 'j']) {
        "ＭＳ Ｐゴシック"
    } else {
        "Arial"
    };
    vec![name.to_owned()]
}

/// The input script of a headless run.
fn input_script(input: Option<Input>) -> Result<InputScript> {
    match input {
        None => Ok(InputScript::new()),
        Some(Input::Script(script)) => Ok(script),
        Some(Input::File(path)) => {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("cannot read the input script {}", path.display()))?;
            InputScript::parse(&text)
                .with_context(|| format!("in the input script {}", path.display()))
        }
    }
}

/// Runs the game the options describe. `Ok(false)` is a run that failed and has said why.
fn run(options: Options) -> Result<bool> {
    let game_dir = absolute(options.game_dir.as_deref().unwrap_or(Path::new(".")))?;
    let ini = GameIni::read(&game_dir, options.encoding)?;
    let rtp_table = options
        .rtps
        .iter()
        .map(|(name, dir)| Ok((name.clone(), absolute(dir)?)))
        .collect::<Result<_>>()?;
    let rtp_dirs = resolve_rtps(&ini.rtp, &rtp_table).map_err(|error| match error {
        VfsError::UnknownRtp { name } => anyhow!(
            "the game needs the run-time package `{name}`; pass its directory with --rtp \
             {name}=DIR"
        ),
        other => other.into(),
    })?;
    let builtin_scripts = builtin_scripts(options.builtin_scripts)?;
    let script = input_script(options.input)?;
    let dumps = options
        .dumps
        .iter()
        .map(|(frame, path)| Ok((*frame, absolute(path)?)))
        .collect::<Result<Vec<_>>>()?;
    let font_dirs = options
        .font_dirs
        .iter()
        .map(absolute)
        .collect::<std::io::Result<Vec<_>>>()?;

    std::env::set_current_dir(&game_dir)
        .with_context(|| format!("cannot enter the game directory {}", game_dir.display()))?;
    signals::block_virtual_timer();

    let title = ini.title.clone();
    let config = Config {
        files: GameFiles::new(game_dir.clone(), rtp_dirs),
        font_names: default_font_names(&ini.library),
        text: Box::new(FontBackend::for_game(&game_dir, &font_dirs)),
        run: RunConfig {
            ini,
            builtin_scripts,
            debug: options.debug,
            process_functions: options.process_functions,
            ..RunConfig::default()
        },
    };
    let interpreter = Interpreter::start(vkxp_ruby_sys::Options::default())?;

    // What the guest thread does: it sends the game to the interpreter thread, waits for it,
    // reports how it went and lets go of the interpreter.
    let (done, outcome) = mpsc::channel();
    let guest = move |link: GuestEnd| {
        let result = interpreter.run(move |ruby| {
            signals::unblock_virtual_timer();
            run_game(ruby, config, link)
        });
        // Nobody listens once the program has given up waiting.
        let _ = done.send(result);
        drop(interpreter);
    };

    let device = options.device.as_deref().map(device_from_text);
    if options.headless {
        let defaults = HeadlessOptions::default();
        let host_options = HeadlessOptions {
            device: device.unwrap_or(defaults.device),
            frames: options.frames,
            script,
            dumps,
            ..defaults
        };
        let (guest_end, link) = headless_link();
        let guest_thread = thread::Builder::new()
            .name("vkxp-guest".to_owned())
            .spawn(move || guest(guest_end))
            .context("cannot start the guest thread")?;
        let host_ok = match run_headless(host_options, link) {
            Ok(_) => true,
            Err(error) => {
                eprintln!("vkxp: {error}");
                false
            }
        };
        let game_ok = game_ended(&outcome)?;
        guest_thread
            .join()
            .map_err(|_| anyhow!("the guest thread panicked"))?;
        Ok(host_ok && game_ok)
    } else {
        let host_options = WindowedOptions {
            title,
            ..WindowedOptions::new(device.unwrap_or_default())
        };
        // The windowed runner returns once the guest thread has ended.
        let end = run_windowed(host_options, guest)?;
        let game_ok = game_ended(&outcome)?;
        if !end.is_success() {
            eprintln!("vkxp: the session ended without the game shutting down ({end:?})");
        }
        Ok(end.is_success() && game_ok)
    }
}

/// Waits for the game's scripts to end, once the host has, and says whether the game ended
/// without a failure. A game that does not end within [`GUEST_WAIT`] ends the process.
fn game_ended(outcome: &Receiver<Result<(), RunError>>) -> Result<bool> {
    match outcome.recv_timeout(GUEST_WAIT) {
        Ok(Ok(())) => Ok(true),
        // The report of a script error is on the error stream already.
        Ok(Err(RunError::Script(_))) => Ok(false),
        Ok(Err(error)) => {
            eprintln!("vkxp: {error}");
            Ok(false)
        }
        Err(RecvTimeoutError::Timeout) => {
            eprintln!(
                "vkxp: the game's scripts did not end within {} seconds of being asked to \
                 quit; giving up",
                GUEST_WAIT.as_secs()
            );
            // The interpreter thread is still inside the script and cannot be joined.
            std::process::exit(1);
        }
        Err(RecvTimeoutError::Disconnected) => bail!("the guest thread panicked"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_font_follows_the_edition_of_the_library() {
        assert_eq!(default_font_names("RGSS104E.dll"), ["Arial"]);
        assert_eq!(default_font_names("RGSS102J.dll"), ["ＭＳ Ｐゴシック"]);
        assert_eq!(default_font_names("rgss103j"), ["ＭＳ Ｐゴシック"]);
        assert_eq!(default_font_names(""), ["Arial"]);
    }

    #[test]
    fn a_missing_built_in_scripts_file_names_the_tool() {
        let dir = tempfile::tempdir().unwrap();
        let error = builtin_scripts(Some(dir.path().join("absent.rb"))).unwrap_err();
        let message = error.to_string();
        assert!(message.contains(EXTRACTION_TOOL), "{message}");
        assert!(message.contains("absent.rb"), "{message}");
    }

    #[test]
    fn an_existing_built_in_scripts_file_is_taken() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("rpg.rb");
        std::fs::write(&file, "module RPG;end\n").unwrap();
        assert_eq!(builtin_scripts(Some(file.clone())).unwrap(), file);
    }
}
