//! The vkrgss executable: `vkrgss <game directory>`.
//!
//! Side: executable.
//!
//! This is the one crate that depends on both sides and joins them. It reads the command
//! line and the game's configuration, starts the interpreter, and then runs the host on the
//! main thread, in a window or headless, and the game's scripts on the interpreter thread,
//! with the protocol link between them.
//!
//! Before anything else, an executable whose first argument is the hidden option of the
//! guest role is a guest process (`guest_process` of `vkrgss-guest-main`): it reads the guest record that
//! the option names and nothing else of its command line, and none of the steps below. The
//! program never starts itself so: its guest processes are the guest executables beside it.
//! Otherwise:
//!
//! 1. The arguments are parsed (`args`); a mistake there ends with the usage message and
//!    status 2. The subscriber that writes diagnostics to the error stream is installed
//!    (the diagnostics of `vkrgss-guest`), at the level `--verbose` and the environment variable `VKRGSS_LOG` ask for; a
//!    value of the variable that cannot be parsed ends the program the same way.
//! 2. The configuration files are read and the command line is put over them (`config`);
//!    settings that do not go together end with the usage message and status 2. `Game.ini`
//!    is read, the runtime of the game is taken from `--runtime` or from that file, the
//!    run-time packages it names are looked up in the `--rtp` table, the built-in scripts
//!    file of the runtime is found and a SoundFont named with `--soundfont` is checked to
//!    exist. The recording of `--replay` is read or the file of `--record` created, the
//!    random seed is settled, and the game's files are opened, with the encrypted archive
//!    of the runtime (`Game.rgssad`, `Game.rgss2a`, `Game.rgss3a`) if there is one. A
//!    failure is a message and status 1, before any window or interpreter exists.
//! 3. The game directory becomes the working directory, because scripts write their saves
//!    with relative names. Paths given on the command line are made absolute first.
//! 4. SIGVTALRM is blocked while the main thread is still the only one; the library of the
//!    guest role, `vkrgss-guest`, unblocks it on the interpreter thread before the game runs.
//! 5. The text backend over the font engine and the binding's configuration are built and
//!    the interpreter is started (the main of a guest, `vkrgss-guest-main`).
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
//!
//! With the guest in a process (`--guest process`, and in a window unless `--guest thread`
//! says otherwise) the game's scripts run in a child process instead of on a thread
//! (`process`). The program then does the steps above without the interpreter: it checks
//! what the game needs, writes what a guest needs of the settled run into the guest record
//! of `vkrgss-guest`, starts the guest executable of the game's runtime that lies beside it
//! in the guest role, with that record as its only argument, and gives the host a relay to
//! that process as its guest. The guest executable is `vkrgss-guest-rgss12` for RPG Maker XP
//! and VX and `vkrgss-guest-rgss3` for RPG Maker VX Ace; neither links anything of the host,
//! so the process that states copy with `fork` holds no window framework. A game of VX Ace,
//! whose scripts need Ruby 1.9.2, always has its guest in a process: the program refuses
//! `--guest thread` for it. The child parses no
//! command line and reads no configuration file: it installs the subscriber from the
//! record, looks the game up and does the steps 3 to 5 with the settings of the record,
//! opens no window, device or recording, attaches to the socket of its parent, plays the game over it and reports
//! how the game went, which is what the status of the program follows.
//!
//! A guest in a process can be saved and loaded: a state is a copy of that process, made
//! with `fork` and parked until it is loaded (the copier of `vkrgss-guest` in the guest process,
//! the relay of `process` in the program). F3 and F4 ask for it in a window, `--states` in a headless
//! run. With `--allow-process-functions` there are no states.
//!
//! A guest process confines itself before the first script runs (the sandbox of `vkrgss-guest`),
//! unless
//! `--sandbox off` or `--allow-process-functions` is given. A guest on a thread is not
//! confined, and neither is the program itself.

mod args;
mod config;
mod log;
mod process;

use std::path::{Path, PathBuf, absolute};
use std::process::ExitCode;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use vkrgss_host::{
    AudioOptions, HeadlessOptions, InputScript, InputSource, Recorder, Recording, Replayer,
    WindowedOptions, device_from_text, headless_link, run_headless, run_windowed,
};
use vkrgss_proto::link::GuestEnd;

use crate::args::{GuestForm, Input, Options, Variable};
use crate::config::Settings;
use vkrgss_guest::diagnostics;
use vkrgss_guest::record::Diagnostics;
use vkrgss_guest::settings::GuestSettings;
use vkrgss_guest_main::{Outcome, game_succeeded, guest_inputs, guest_on_thread};
use vkrgss_vfs::Runtime;

use crate::process::{GuestProcess, Handed, socket_base};

/// How long the program waits for the game's scripts to end after the host has.
const GUEST_WAIT: Duration = Duration::from_secs(5);

fn main() -> ExitCode {
    // A guest process reads its record and nothing of what follows.
    if let Some(status) = vkrgss_guest_main::guest_process(env!("CARGO_PKG_VERSION")) {
        return status;
    }
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let usage = |message: String| {
        eprintln!("vkrgss: {message}");
        eprintln!("{}", args::USAGE);
        ExitCode::from(2)
    };
    let options = match args::parse(&arguments) {
        Ok(options) => options,
        Err(message) => return usage(message),
    };
    if options.help {
        println!("{}", args::USAGE);
        return ExitCode::SUCCESS;
    }
    if options.version {
        println!("{}", args::VERSION);
        return ExitCode::SUCCESS;
    }
    // Nothing before this emits an event: a mistake on the command line is written directly.
    let log = Variable::read(diagnostics::VARIABLE);
    match diagnostics::level(log.value.as_deref(), options.verbose) {
        Ok(level) => diagnostics::install(level),
        Err(message) => return usage(message.replacen(log.name, log.read_as(), 1)),
    }
    log.warn_if_former();
    // A guest process takes its level from the same two values.
    let diagnostics = Diagnostics {
        variable: log.value,
        verbose: options.verbose,
    };
    let variable = Variable::read(args::GUEST_VARIABLE);
    variable.warn_if_former();
    let from_environment = match args::guest_of_environment(&variable) {
        Ok(form) => form,
        Err(message) => return usage(message),
    };
    let variable = Variable::read(args::SANDBOX_VARIABLE);
    variable.warn_if_former();
    let sandbox = match args::sandbox_of_environment(&variable) {
        Ok(sandbox) => sandbox,
        Err(message) => return usage(message),
    };
    let options = Options {
        guest: options.guest.or(from_environment),
        sandbox: options.sandbox.or(sandbox),
        ..options
    };
    let (game_dir, mut settings) = match settings(options) {
        Ok(settled) => settled,
        Err(error) => {
            eprintln!("vkrgss: {error:#}");
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
            eprintln!("vkrgss: {message}");
            return ExitCode::FAILURE;
        }
    };
    let ran = run(game_dir, settings, script, diagnostics);
    match ran {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("vkrgss: {error:#}");
            ExitCode::FAILURE
        }
    }
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

/// What the guest reads of the settled `options`, for both forms of the guest.
fn guest_settings(options: &Options) -> GuestSettings {
    GuestSettings {
        encoding: options.encoding,
        runtime: options.runtime,
        rtps: options.rtps.clone(),
        builtin_scripts: options.builtin_scripts.clone(),
        font_dirs: options.font_dirs.clone(),
        font_own_metrics: options.font_own_metrics,
        preload: options.preload.clone(),
        debug: options.debug,
        process_functions: options.process_functions,
        log_p: options.log_p,
        frame_stats: options.frame_stats,
        sandbox: options.sandbox,
    }
}

/// `settings` with every path made absolute, as the guest record holds them: a record does
/// not depend on the working directory of whoever reads it.
fn absolute_settings(settings: GuestSettings) -> std::io::Result<GuestSettings> {
    let all = |paths: Vec<PathBuf>| paths.iter().map(absolute).collect::<std::io::Result<_>>();
    Ok(GuestSettings {
        rtps: settings
            .rtps
            .into_iter()
            .map(|(name, dir)| Ok((name, absolute(dir)?)))
            .collect::<std::io::Result<_>>()?,
        builtin_scripts: settings.builtin_scripts.map(absolute).transpose()?,
        font_dirs: all(settings.font_dirs)?,
        preload: all(settings.preload)?,
        ..settings
    })
}

/// The form of the guest of a run of a game of `runtime`: the one that was chosen, and
/// without a choice a process in a window, where the state keys need it, and in every run of
/// RGSS3, and a thread in a headless run of RGSS1 or RGSS2. A game of RGSS3, whose
/// interpreter the program does not link, refuses a thread.
fn guest_form(chosen: Option<GuestForm>, headless: bool, runtime: Runtime) -> Result<GuestForm> {
    match (chosen, runtime) {
        (Some(GuestForm::Thread), Runtime::Rgss3) => bail!(
            "a game of {} runs with the guest in a process only: give --guest process, or no \
             choice of the guest with --guest, VKRGSS_GUEST or the key `guest`",
            runtime.product()
        ),
        (Some(form), _) => Ok(form),
        (None, Runtime::Rgss3) => Ok(GuestForm::Process),
        (None, Runtime::Rgss1 | Runtime::Rgss2) => Ok(if headless {
            GuestForm::Thread
        } else {
            GuestForm::Process
        }),
    }
}

/// The guest of a run, as the host is given it and as the program waits for it.
enum Guest {
    /// The interpreter thread of this process, and what the guest thread reports.
    Thread(Outcome),
    /// A child process and its copies, and what the relay to them reports.
    Process(Receiver<bool>),
}

impl Guest {
    /// Waits for the game's scripts to end, once the host has, and says whether the game
    /// ended without a failure.
    fn ended(self) -> Result<bool> {
        match self {
            Guest::Thread(outcome) => game_ended(&outcome),
            Guest::Process(outcome) => outcome
                .recv()
                .map_err(|_| anyhow!("the guest thread panicked")),
        }
    }
}

/// Runs the game in `game_dir` as the settings describe, headless with the input `script`.
/// `diagnostics` are the inputs of the level of this run, which a guest process is handed.
/// `Ok(false)` is a run that failed and has said why.
fn run(
    game_dir: PathBuf,
    settings: Settings,
    script: InputScript,
    diagnostics: Diagnostics,
) -> Result<bool> {
    let guest_settings = guest_settings(&settings.options);
    let inputs = guest_inputs(&game_dir, &guest_settings)?;
    let form = guest_form(
        settings.options.guest,
        settings.options.headless,
        inputs.runtime,
    )?;
    // The keyboard maps to the buttons as the original runtime of the game does.
    let keys = settings.keys_of(inputs.runtime).clone();
    let Settings { options, pads, .. } = settings;
    let audio = audio_options(options.no_audio, options.soundfont.as_deref())?;
    let dumps = options
        .dumps
        .iter()
        .map(|(frame, path)| Ok((*frame, absolute(path)?)))
        .collect::<Result<Vec<_>>>()?;
    let (seed, input) = seed_and_input(&options)?;
    let title = inputs.ini.title.clone();

    let frame_size = vkrgss_binding::frame_size(inputs.runtime);

    // What the guest thread does, and what the program waits for once the host has ended.
    let (guest, waited): (Box<dyn FnOnce(GuestEnd) + Send>, Guest) = match form {
        GuestForm::Thread => {
            // The restriction would bind the thread and not the files both sides share.
            // Said whatever the setting of the sandbox is.
            tracing::info!("the guest is a thread and is not confined");
            let (guest, outcome) = guest_on_thread(&game_dir, &guest_settings, inputs, seed)?;
            (Box::new(guest), Guest::Thread(outcome))
        }
        GuestForm::Process => {
            // The child sets the guest up itself, from the record of what this process has
            // settled; this process has no interpreter. It is handed the built-in scripts
            // file found here, so that the file is looked up, and a file at its former place
            // warned about, once.
            let settings = absolute_settings(GuestSettings {
                builtin_scripts: Some(inputs.builtin_scripts().to_owned()),
                ..guest_settings
            })?;
            let handed = Handed {
                seed,
                game_dir: &game_dir,
                settings: &settings,
                diagnostics,
            };
            let process = GuestProcess::start(inputs.runtime, &socket_base(), handed)?;
            let (done, outcome) = mpsc::channel();
            let guest = move |link: GuestEnd| {
                let _ = done.send(process.relay(link));
            };
            (Box::new(guest), Guest::Process(outcome))
        }
    };

    let device = options.device.as_deref().map(device_from_text);
    if options.headless {
        let host_options = HeadlessOptions {
            device: device.unwrap_or(HeadlessOptions::default().device),
            frames: options.frames,
            script,
            input,
            states: options.states.clone(),
            dumps,
            frame_stats: options.frame_stats,
            errors: Box::new(log::HostReporter),
        };
        let (guest_end, link) = headless_link();
        let guest_thread = thread::Builder::new()
            .name("vkrgss-guest".to_owned())
            .spawn(move || guest(guest_end))
            .context("cannot start the guest thread")?;
        let host_ok = match run_headless(host_options, link) {
            Ok(_) => true,
            Err(error) => {
                eprintln!("vkrgss: {error}");
                false
            }
        };
        let game_ok = waited.ended()?;
        guest_thread
            .join()
            .map_err(|_| anyhow!("the guest thread panicked"))?;
        Ok(host_ok && game_ok)
    } else {
        let host_options = WindowedOptions {
            title,
            logical_size: frame_size,
            filter: options.filter.unwrap_or_default(),
            fullscreen: options.fullscreen,
            present_stats: options.present_stats,
            frame_stats: options.frame_stats,
            keys,
            pads,
            audio,
            input,
            states: form == GuestForm::Process,
            errors: Box::new(log::HostReporter),
            audio_errors: log::EventLines::audio(),
            ..WindowedOptions::new(device.unwrap_or_default())
        };
        // The windowed runner returns once the guest thread has ended.
        let end = run_windowed(host_options, guest)?;
        let game_ok = waited.ended()?;
        if !end.is_success() {
            eprintln!("vkrgss: the session ended without the game shutting down ({end:?})");
        }
        Ok(end.is_success() && game_ok)
    }
}

/// Waits for the game's scripts to end, once the host has, and says whether the game ended
/// without a failure. A game that does not end within [`GUEST_WAIT`] ends the process.
fn game_ended(outcome: &Outcome) -> Result<bool> {
    match outcome.recv_timeout(GUEST_WAIT) {
        Ok(result) => Ok(game_succeeded(result)),
        Err(RecvTimeoutError::Timeout) => {
            eprintln!(
                "vkrgss: the game's scripts did not end within {} seconds of being asked to \
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

    /// Requirement "Guest option": the default by kind of run, and scenario "Headless
    /// default".
    #[test]
    fn without_a_choice_the_guest_is_a_process_in_a_window_and_a_thread_headless() {
        for runtime in [Runtime::Rgss1, Runtime::Rgss2] {
            let form = |chosen, headless| guest_form(chosen, headless, runtime).unwrap();
            assert_eq!(form(None, false), GuestForm::Process);
            assert_eq!(form(None, true), GuestForm::Thread);
            for headless in [false, true] {
                for chosen in [GuestForm::Thread, GuestForm::Process] {
                    assert_eq!(form(Some(chosen), headless), chosen);
                }
            }
        }
    }

    /// Requirement "Guest option": a game of RGSS3 has its guest in a process, and a thread
    /// that was chosen ends the run.
    #[test]
    fn a_game_of_vx_ace_has_its_guest_in_a_process() {
        for headless in [false, true] {
            let form = |chosen| guest_form(chosen, headless, Runtime::Rgss3);
            assert_eq!(form(None).unwrap(), GuestForm::Process);
            assert_eq!(form(Some(GuestForm::Process)).unwrap(), GuestForm::Process);
            let message = form(Some(GuestForm::Thread)).unwrap_err().to_string();
            assert!(message.contains("RPG Maker VX Ace"), "{message}");
            assert!(message.contains("--guest process"), "{message}");
        }
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

    /// Requirement "Built-in scripts file": the default file of each runtime.
    #[test]
    fn the_default_built_in_scripts_file_follows_the_runtime() {
        let names = vkrgss_vfs::Runtime::ALL.map(vkrgss_guest_main::builtin_scripts_name);
        let expected = [
            "rpg_builtin.rb",
            "rpg_builtin_rgss2.rb",
            "rpg_builtin_rgss3.rb",
        ];
        assert_eq!(names, expected);
        // The usage text names all three.
        for name in expected {
            let named = format!("$XDG_DATA_HOME/vkrgss/{name}");
            assert!(args::USAGE.contains(&named), "{name}");
        }
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
}
