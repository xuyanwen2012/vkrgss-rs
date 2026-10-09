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
//! 2. The configuration files are read and the command line is put over them (`config`);
//!    settings that do not go together end with the usage message and status 2. `Game.ini` is read, the run-time packages it names are looked up in the `--rtp` table,
//!    the built-in scripts file is found and a SoundFont named with `--soundfont` is checked
//!    to exist. The recording of `--replay` is read or the file of `--record` created, the
//!    random seed is settled, and the game's files are opened, with the encrypted archive
//!    `Game.rgssad` if there is one. A failure is a message and status 1, before any window
//!    or interpreter exists.
//! 3. The game directory becomes the working directory, because scripts write their saves
//!    with relative names. Paths given on the command line are made absolute first.
//! 4. SIGVTALRM is blocked while the main thread is still the only one (`signals`); the
//!    closure that runs the game unblocks it on the interpreter thread.
//! 5. The text backend over the font engine (`font`) and the binding's configuration are
//!    built and the interpreter is started.
//! 6. The host runs on the main thread. In a window it plays the game's audio on the default
//!    output device unless `--no-audio` is given; headless it opens none. The guest thread, which the windowed runner spawns
//!    and the headless mode spawns itself, sends the game to the interpreter thread and
//!    waits for it.
//! 7. The guest is joined, which drops the interpreter, and the status is 0 only if the
//!    game ended without a failure and the host without an error.
//!
//! A script can loop without ever calling `Graphics.update` and then never sees a quit
//! request. After a headless run has ended, the program waits [`GUEST_WAIT`] for the game
//! and then exits with a message and a non-zero status.

mod args;
mod config;
mod font;
mod signals;

use std::path::{Path, PathBuf, absolute};
use std::process::ExitCode;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use vkxp_binding::{Config, RunConfig, RunError, run_game};
use vkxp_host::{
    AudioOptions, HeadlessOptions, InputScript, InputSource, Recorder, Recording, Replayer,
    WindowedOptions, device_from_text, headless_link, run_headless, run_windowed,
};
use vkxp_proto::link::GuestEnd;
use vkxp_ruby_sys::Interpreter;
use vkxp_vfs::{GameFiles, GameIni, VfsError, resolve_rtps};

use crate::args::{Input, Options};
use crate::config::Settings;
use crate::font::FontBackend;

/// How long the program waits for the game's scripts to end after the host has.
const GUEST_WAIT: Duration = Duration::from_secs(5);

/// The name of the tool that creates the built-in scripts file.
const EXTRACTION_TOOL: &str = "cargo xtask extract-rpg";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let usage = |message: String| {
        eprintln!("vkxp: {message}");
        eprintln!("{}", args::USAGE);
        ExitCode::from(2)
    };
    let options = match args::parse(&arguments) {
        Ok(options) => options,
        Err(message) => return usage(message),
    };
    let (game_dir, mut settings) = match settings(options) {
        Ok(settled) => settled,
        Err(error) => {
            eprintln!("vkxp: {error:#}");
            return ExitCode::FAILURE;
        }
    };
    // Whether the settings go together is asked of the final ones, wherever each came from.
    if let Err(message) = args::check(&settings.options) {
        return usage(message);
    }
    // A script in a file is a value like one given in place.
    let script = match input_script(settings.options.input.take()) {
        Ok(script) => script,
        Err(InputError::Malformed(message)) => return usage(message),
        Err(InputError::Unreadable(message)) => {
            eprintln!("vkxp: {message}");
            return ExitCode::FAILURE;
        }
    };
    match run(game_dir, settings, script) {
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

/// The audio of a windowed run: `None`, which opens no audio device, when it is disabled. A
/// SoundFont that is named has to exist, whether or not it will be used.
fn audio_options(disabled: bool, soundfont: Option<&Path>) -> Result<Option<AudioOptions>> {
    let soundfont = soundfont.map(absolute).transpose()?;
    if let Some(path) = soundfont.as_deref().filter(|path| !path.is_file()) {
        bail!(
            "the SoundFont {} does not exist; name another file with --soundfont, or leave \
             the option out to use the system's",
            path.display()
        );
    }
    Ok((!disabled).then_some(AudioOptions { soundfont }))
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

/// Why the input script of a headless run could not be taken from its file.
#[derive(Debug, PartialEq, Eq)]
enum InputError {
    /// The file could not be read.
    Unreadable(String),
    /// The file does not hold a script: a malformed value of `--input`.
    Malformed(String),
}

/// The input script of a headless run.
fn input_script(input: Option<Input>) -> Result<InputScript, InputError> {
    match input {
        None => Ok(InputScript::new()),
        Some(Input::Script(script)) => Ok(script),
        Some(Input::File(path)) => {
            let text = std::fs::read_to_string(&path).map_err(|error| {
                InputError::Unreadable(format!(
                    "cannot read the input script {}: {error}",
                    path.display()
                ))
            })?;
            InputScript::parse(&text).map_err(|error| {
                InputError::Malformed(format!("--input: in {}: {error}", path.display()))
            })
        }
    }
}

/// The seed of a run and the recorder or replayer of its input. The seed is the one given;
/// without one it is that of the replayed recording, 0 in a headless run and otherwise taken
/// from the clock. A recording is read, and a file to record into created, here: before any
/// window or interpreter exists.
fn seed_and_input(options: &Options) -> Result<(u64, Option<Box<dyn InputSource>>)> {
    let recording = match &options.replay {
        Some(path) => Some(Recording::read(&absolute(path)?)?),
        None => None,
    };
    let seed = match (options.seed, &recording) {
        (Some(seed), _) => seed,
        (None, Some(recording)) => recording.seed,
        (None, None) if options.headless => 0,
        (None, None) => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            // The low 64 bits of the nanoseconds; a clock before 1970 gives 0.
            .map_or(0, |since| since.as_nanos() as u64),
    };
    let input: Option<Box<dyn InputSource>> = match (recording, &options.record) {
        (Some(recording), _) => Some(Box::new(Replayer::new(recording))),
        (None, Some(path)) => Some(Box::new(Recorder::create(&absolute(path)?, seed)?)),
        (None, None) => None,
    };
    Ok((seed, input))
}

/// The game directory and the settings of the run: the command line over the configuration
/// files.
fn settings(options: Options) -> Result<(PathBuf, Settings)> {
    let game_dir = absolute(options.game_dir.as_deref().unwrap_or(Path::new(".")))?;
    let settings = config::settle(config::load(&options, &game_dir)?, options)?;
    Ok((game_dir, settings))
}

/// Runs the game in `game_dir` as the settings describe, headless with the input `script`.
/// `Ok(false)` is a run that failed and has said why.
fn run(game_dir: PathBuf, settings: Settings, script: InputScript) -> Result<bool> {
    let Settings {
        options,
        keys,
        pads,
    } = settings;
    let ini = GameIni::read(&game_dir, options.encoding)?;
    let rtp_table = options
        .rtps
        .iter()
        .map(|(name, dir)| Ok((name.clone(), absolute(dir)?)))
        .collect::<Result<_>>()?;
    let rtp_dirs = resolve_rtps(&ini.rtp, &rtp_table).map_err(|error| match error {
        VfsError::UnknownRtp { name } => anyhow!(
            "the game needs the run-time package `{name}`; pass its directory with --rtp \
             {name}=DIR or name it under [rtp] in a configuration file"
        ),
        other => other.into(),
    })?;
    let builtin_scripts = builtin_scripts(options.builtin_scripts.clone())?;
    let audio = audio_options(options.no_audio, options.soundfont.as_deref())?;
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

    let preload = options
        .preload
        .iter()
        .map(absolute)
        .collect::<std::io::Result<Vec<_>>>()?;
    let (seed, input) = seed_and_input(&options)?;
    let files = GameFiles::open(game_dir.clone(), rtp_dirs)?;

    std::env::set_current_dir(&game_dir)
        .with_context(|| format!("cannot enter the game directory {}", game_dir.display()))?;
    signals::block_virtual_timer();

    let title = ini.title.clone();
    let config = Config {
        files,
        font_names: default_font_names(&ini.library),
        text: Box::new(FontBackend::for_game(
            &game_dir,
            &font_dirs,
            options.font_own_metrics,
        )),
        run: RunConfig {
            ini,
            builtin_scripts,
            debug: options.debug,
            process_functions: options.process_functions,
            log_p: options.log_p,
            seed: Some(seed),
            preload,
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
            input,
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
            filter: options.filter.unwrap_or_default(),
            fullscreen: options.fullscreen,
            keys,
            pads,
            audio,
            input,
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
    fn an_input_script_file_is_read_and_parsed() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("keys.txt");
        std::fs::write(&file, "2:RIGHT\n6:\n").unwrap();
        assert_eq!(
            input_script(Some(Input::File(file.clone()))),
            Ok(InputScript::parse("2:RIGHT, 6:").unwrap())
        );

        std::fs::write(&file, "2:SIDEWAYS").unwrap();
        let error = input_script(Some(Input::File(file))).unwrap_err();
        assert!(
            matches!(&error, InputError::Malformed(message) if message.contains("SIDEWAYS")),
            "{error:?}"
        );

        let absent = Input::File(dir.path().join("absent.txt"));
        let error = input_script(Some(absent)).unwrap_err();
        assert!(matches!(error, InputError::Unreadable(_)), "{error:?}");
    }

    #[test]
    fn a_missing_built_in_scripts_file_names_the_tool() {
        let dir = tempfile::tempdir().unwrap();
        let error = builtin_scripts(Some(dir.path().join("absent.rb"))).unwrap_err();
        let message = error.to_string();
        assert!(message.contains(EXTRACTION_TOOL), "{message}");
        assert!(message.contains("absent.rb"), "{message}");
    }

    /// Scenario "Disabled": without audio options the windowed runner opens no device.
    #[test]
    fn no_audio_leaves_the_windowed_run_without_audio_options() {
        assert_eq!(audio_options(true, None).unwrap(), None);
        assert_eq!(
            audio_options(false, None).unwrap(),
            Some(AudioOptions { soundfont: None })
        );
    }

    #[test]
    fn a_named_soundfont_is_passed_on_with_its_absolute_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("gm.sf2");
        std::fs::write(&file, "").unwrap();
        assert_eq!(
            audio_options(false, Some(&file)).unwrap(),
            Some(AudioOptions {
                soundfont: Some(file)
            })
        );
    }

    /// Scenario "SoundFont missing".
    #[test]
    fn a_missing_soundfont_is_an_error_naming_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("absent.sf2");
        for disabled in [false, true] {
            let error = audio_options(disabled, Some(&absent)).unwrap_err();
            let message = error.to_string();
            assert!(message.contains("absent.sf2"), "{message}");
            assert!(message.contains("--soundfont"), "{message}");
        }
    }

    #[test]
    fn an_existing_built_in_scripts_file_is_taken() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("rpg.rb");
        std::fs::write(&file, "module RPG;end\n").unwrap();
        assert_eq!(builtin_scripts(Some(file.clone())).unwrap(), file);
    }
}
