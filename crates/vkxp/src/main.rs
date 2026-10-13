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
//!    status 2. The subscriber that writes diagnostics to the error stream is installed
//!    (`log`), at the level `--verbose` and the environment variable `VKXP_LOG` ask for; a
//!    value of the variable that cannot be parsed ends the program the same way.
//! 2. The configuration files are read and the command line is put over them (`config`);
//!    settings that do not go together end with the usage message and status 2. `Game.ini`
//!    is read, the runtime of the game is taken from `--runtime` or from that file, the
//!    run-time packages it names are looked up in the `--rtp` table, the built-in scripts
//!    file of the runtime is found and a SoundFont named with `--soundfont` is checked to
//!    exist. The recording of `--replay` is read or the file of `--record` created, the
//!    random seed is settled, and the game's files are opened, with the encrypted archive
//!    of the runtime (`Game.rgssad`, `Game.rgss2a`) if there is one. A failure is a message
//!    and status 1, before any window or interpreter exists; a game of RPG Maker VX Ace
//!    ends there too.
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
//!
//! With the guest in a process (`--guest process`, and in a window unless `--guest thread`
//! says otherwise) the game's scripts run in a child process instead of on a thread
//! (`process`). The program then does the steps above without the interpreter: it checks
//! what the game needs, starts itself again in the guest role, and gives the host a relay
//! to that process as its guest. The child does the steps 1 to 5, opens no window, device
//! or recording, attaches to the socket of its parent, plays the game over it and reports
//! how the game went, which is what the status of the program follows.
//!
//! A guest in a process can be saved and loaded: a state is a copy of that process, made
//! with `fork` and parked until it is loaded (`copier` in the guest process, the relay of
//! `process` in the program). F3 and F4 ask for it in a window, `--states` in a headless
//! run. With `--allow-process-functions` there are no states.
//!
//! A guest process confines itself before the first script runs (`sandbox`), unless
//! `--sandbox off` or `--allow-process-functions` is given. A guest on a thread is not
//! confined, and neither is the program itself.

mod args;
mod config;
mod copier;
mod font;
mod log;
mod process;

use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf, absolute};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use vkxp_binding::{Config, Copier, RunConfig, RunError, run_game};
use vkxp_guest::role::GuestRole;
use vkxp_guest::{sandbox, signals};
use vkxp_host::{
    AudioOptions, HeadlessOptions, InputScript, InputSource, Recorder, Recording, Replayer,
    WindowedOptions, device_from_text, headless_link, run_headless, run_windowed,
};
use vkxp_proto::GuestToHost;
use vkxp_proto::framing::write_message;
use vkxp_proto::link::GuestEnd;
use vkxp_ruby_sys::{Interpreter, SymbolTable};
use vkxp_vfs::{GameFiles, GameIni, Runtime, VfsError, resolve_rtps};

use crate::args::{GuestForm, Input, Options};
use crate::config::Settings;
use crate::copier::ForkCopier;
use crate::font::FontBackend;
use crate::process::GuestProcess;

/// How long the program waits for the game's scripts to end after the host has.
const GUEST_WAIT: Duration = Duration::from_secs(5);

/// The name of the tool that creates the built-in scripts file.
const EXTRACTION_TOOL: &str = "cargo xtask extract-rpg";

/// Where it is written how the built-in scripts file of RGSS2 is made.
const RGSS2_BUILTIN_GUIDE: &str =
    "docs/running.md, section \"The built-in scripts of RPG Maker VX\"";

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
    if options.help {
        println!("{}", args::USAGE);
        return ExitCode::SUCCESS;
    }
    if options.version {
        println!("{}", args::VERSION);
        return ExitCode::SUCCESS;
    }
    // Nothing before this emits an event: a mistake on the command line is written directly.
    let variable = std::env::var(log::VARIABLE).ok();
    match log::level(variable.as_deref(), options.verbose) {
        Ok(level) => log::install(level),
        Err(message) => return usage(message),
    }
    let variable = std::env::var(args::GUEST_VARIABLE).ok();
    let from_environment = match args::guest_of_environment(variable.as_deref()) {
        Ok(form) => form,
        Err(message) => return usage(message),
    };
    let variable = std::env::var(args::SANDBOX_VARIABLE).ok();
    let sandbox = match args::sandbox_of_environment(variable.as_deref()) {
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
            eprintln!("vkxp: {error:#}");
            return ExitCode::FAILURE;
        }
    };
    // Whether the settings go together is asked of the final ones, wherever each came from.
    if let Err(message) = args::check(&settings.options) {
        return usage(message);
    }
    let ran = match settings.options.guest_role.take() {
        Some(role) => run_guest(game_dir, settings.options, &role),
        None => {
            // A script in a file is a value like one given in place.
            let script = match input_script(settings.options.input.take()) {
                Ok(script) => script,
                Err(InputError::Malformed(message)) => return usage(message),
                Err(InputError::Unreadable(message)) => {
                    eprintln!("vkxp: {message}");
                    return ExitCode::FAILURE;
                }
            };
            run(game_dir, settings, script)
        }
    };
    match ran {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("vkxp: {error:#}");
            ExitCode::FAILURE
        }
    }
}

/// The name of the built-in scripts file of `runtime` at the default location.
fn builtin_scripts_name(runtime: Runtime) -> &'static str {
    match runtime {
        Runtime::Rgss1 => "rpg_builtin.rb",
        Runtime::Rgss2 => "rpg_builtin_rgss2.rb",
        Runtime::Rgss3 => "rpg_builtin_rgss3.rb",
    }
}

/// The start-up symbol table of `runtime`, which numbers the symbols as its original does.
/// A game of RGSS3 ends before an interpreter is started.
fn symbol_table(runtime: Runtime) -> SymbolTable {
    match runtime {
        Runtime::Rgss1 => SymbolTable::Rgss1,
        Runtime::Rgss2 | Runtime::Rgss3 => SymbolTable::Rgss2,
    }
}

/// Starts the interpreter of a game of `runtime`.
fn start_interpreter(runtime: Runtime) -> Result<Interpreter> {
    let options = vkxp_ruby_sys::Options {
        symbols: symbol_table(runtime),
        ..vkxp_ruby_sys::Options::default()
    };
    Ok(Interpreter::start(options)?)
}

/// The built-in scripts file of `runtime`: the one named on the command line, or the default
/// under the user's data directory. Fails when it does not exist.
fn builtin_scripts(named: Option<PathBuf>, runtime: Runtime) -> Result<PathBuf> {
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
            data.join("vkxp").join(builtin_scripts_name(runtime))
        }
    };
    if !path.is_file() && runtime != Runtime::Rgss1 {
        bail!(
            "the built-in scripts file {path} does not exist. It holds the RPG scripts of the \
             original runtime of {product}, which are not part of vkxp: \
             {RGSS2_BUILTIN_GUIDE}, says how it is made from your own copy of that runtime; \
             or name another file with --builtin-scripts",
            path = path.display(),
            product = runtime.product()
        );
    }
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

/// The encrypted archive of the game in `game_dir`, which the game's files are opened with:
/// a file with the archive name of `runtime` under any letter case.
fn archive(game_dir: &Path, runtime: Runtime) -> Option<PathBuf> {
    let archive_name = runtime.archive_name()?;
    std::fs::read_dir(game_dir)
        .ok()?
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.eq_ignore_ascii_case(archive_name))
        })
        .map(|entry| entry.path())
        .find(|path| path.is_file())
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

/// What the guest of a run is set up with, as far as it is looked up before any game file
/// is opened.
struct GuestInputs {
    ini: GameIni,
    runtime: Runtime,
    rtp_dirs: Vec<PathBuf>,
    builtin_scripts: PathBuf,
    font_dirs: Vec<PathBuf>,
    preload: Vec<PathBuf>,
}

/// The runtime of the game whose configuration file is `ini`: the one the option names, and
/// without the option what the file says. A game of RGSS3 is refused here.
fn runtime_of(ini: &GameIni, named: Option<Runtime>) -> Result<Runtime> {
    let runtime = match named {
        Some(runtime) => runtime,
        None => Runtime::detect(ini).map_err(|error| {
            anyhow!("{error}; name the runtime with --runtime rgss1, rgss2 or rgss3")
        })?,
    };
    if runtime == Runtime::Rgss3 {
        bail!(
            "this is a game of {} ({}), which is planned and not supported yet",
            runtime.product(),
            runtime.name()
        );
    }
    Ok(runtime)
}

/// Reads `Game.ini`, settles the runtime of the game, looks the run-time packages it names
/// up in the table of the options and finds the built-in scripts file.
fn guest_inputs(game_dir: &Path, options: &Options) -> Result<GuestInputs> {
    let ini = GameIni::read(game_dir, options.encoding)?;
    let runtime = runtime_of(&ini, options.runtime)?;
    tracing::info!("runtime: {} ({})", runtime.name(), runtime.product());
    let rtp_table = options
        .rtps
        .iter()
        .map(|(name, dir)| Ok((name.clone(), absolute(dir)?)))
        .collect::<Result<_>>()?;
    let rtp_dirs =
        resolve_rtps(ini.rtp_names(runtime), &rtp_table).map_err(|error| match error {
            VfsError::UnknownRtp { name } => anyhow!(
                "the game needs the run-time package `{name}`; pass its directory with --rtp \
             {name}=DIR or name it under [rtp] in a configuration file"
            ),
            other => other.into(),
        })?;
    for dir in &rtp_dirs {
        tracing::info!("run-time package directory: {}", dir.display());
    }
    let builtin_scripts = builtin_scripts(options.builtin_scripts.clone(), runtime)?;
    tracing::info!("built-in scripts file: {}", builtin_scripts.display());
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
    Ok(GuestInputs {
        ini,
        runtime,
        rtp_dirs,
        builtin_scripts,
        font_dirs,
        preload,
    })
}

/// Opens the game's files, enters the game directory, blocks SIGVTALRM in the calling
/// thread, which has to be the only one still, and builds the binding's configuration.
fn guest_config(
    game_dir: &Path,
    options: &Options,
    inputs: GuestInputs,
    seed: u64,
) -> Result<Config> {
    let GuestInputs {
        ini,
        runtime,
        rtp_dirs,
        builtin_scripts,
        font_dirs,
        preload,
    } = inputs;
    let files = GameFiles::open(game_dir.to_owned(), rtp_dirs, runtime)?;
    if let Some(archive) = archive(game_dir, runtime) {
        tracing::info!("encrypted archive: {}", archive.display());
    }

    std::env::set_current_dir(game_dir)
        .with_context(|| format!("cannot enter the game directory {}", game_dir.display()))?;
    signals::block_virtual_timer();

    Ok(Config {
        files,
        font_names: default_font_names(&ini.library),
        text: Box::new(FontBackend::for_game(
            game_dir,
            &font_dirs,
            options.font_own_metrics,
        )),
        run: RunConfig {
            ini,
            runtime,
            builtin_scripts,
            debug: options.debug,
            process_functions: options.process_functions,
            log_p: options.log_p,
            frame_stats: options.frame_stats,
            seed: Some(seed),
            preload,
            ..RunConfig::default()
        },
    })
}

/// The form of the guest of a run: the one that was chosen, and without a choice a process
/// in a window, where the state keys need it, and a thread in a headless run.
fn guest_form(chosen: Option<GuestForm>, headless: bool) -> GuestForm {
    chosen.unwrap_or(if headless {
        GuestForm::Thread
    } else {
        GuestForm::Process
    })
}

/// Whether a guest process confines itself: unless the setting says no, and never with the
/// functions that start processes.
fn confined(sandbox: Option<bool>, process_functions: bool) -> bool {
    sandbox.unwrap_or(true) && !process_functions
}

/// The guest of a run, as the host is given it and as the program waits for it.
enum Guest {
    /// The interpreter thread of this process, and what the guest thread reports.
    Thread(Receiver<Result<(), RunError>>),
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

/// The guest role: plays the game in `game_dir` over the socket of the parent process.
/// `Ok(false)` is a game that failed and has said why.
fn run_guest(game_dir: PathBuf, options: Options, role: &GuestRole) -> Result<bool> {
    signals::die_with_parent();
    let inputs = guest_inputs(&game_dir, &options)?;
    let runtime = inputs.runtime;
    let mut config = guest_config(&game_dir, &options, inputs, role.seed)?;
    let interpreter = start_interpreter(runtime)?;
    let mut stream = UnixStream::connect(&role.socket).with_context(|| {
        format!(
            "cannot connect to the socket {} of the parent process",
            role.socket.display()
        )
    })?;
    let attach = GuestToHost::Attach {
        pid: std::process::id(),
        parked: None,
    };
    write_message(&mut stream, &attach).context("cannot attach to the parent process")?;
    // The functions that start processes need the children of this process to be waited
    // for, and states need them not to be: with those functions there are no states.
    if !options.process_functions {
        signals::ignore_children();
        let copier: Arc<dyn Copier> = Arc::new(ForkCopier::new(role.socket.clone(), &stream));
        config.run.copier = Some(copier);
    }
    let confined = confined(options.sandbox, options.process_functions);
    if options.process_functions {
        // A program the game starts would inherit the restriction. Said whatever the
        // setting of the sandbox is.
        tracing::info!("the guest process is not confined: --allow-process-functions is given");
    }
    // The interpreter thread reads and writes the socket itself.
    let link = GuestEnd::over_stream(stream);
    let ok = interpreter.run(move |ruby| {
        signals::unblock_virtual_timer();
        if confined {
            sandbox::confine(&game_dir);
        }
        let ok = game_succeeded(run_game(ruby, config, link));
        // A copy of the guest process has no other thread to go back to.
        copier::exit_if_copy(ok);
        ok
    });
    drop(interpreter);
    Ok(ok)
}

/// Runs the game in `game_dir` as the settings describe, headless with the input `script`.
/// `Ok(false)` is a run that failed and has said why.
fn run(game_dir: PathBuf, settings: Settings, script: InputScript) -> Result<bool> {
    let inputs = guest_inputs(&game_dir, &settings.options)?;
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

    let frame_size = vkxp_binding::frame_size(inputs.runtime);

    let form = guest_form(options.guest, options.headless);
    // What the guest thread does, and what the program waits for once the host has ended.
    let (guest, waited): (Box<dyn FnOnce(GuestEnd) + Send>, Guest) = match form {
        GuestForm::Thread => {
            // The restriction would bind the thread and not the files both sides share.
            // Said whatever the setting of the sandbox is.
            tracing::info!("the guest is a thread and is not confined");
            let runtime = inputs.runtime;
            let config = guest_config(&game_dir, &options, inputs, seed)?;
            let interpreter = start_interpreter(runtime)?;
            // It sends the game to the interpreter thread, waits for it, reports how
            // it went and lets go of the interpreter.
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
            (Box::new(guest), Guest::Thread(outcome))
        }
        GuestForm::Process => {
            // The child does what `guest_config` does; this process has no interpreter.
            let process = GuestProcess::start(seed)?;
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
            eprintln!("vkxp: the session ended without the game shutting down ({end:?})");
        }
        Ok(end.is_success() && game_ok)
    }
}

/// Waits for the game's scripts to end, once the host has, and says whether the game ended
/// without a failure. A game that does not end within [`GUEST_WAIT`] ends the process.
fn game_ended(outcome: &Receiver<Result<(), RunError>>) -> Result<bool> {
    match outcome.recv_timeout(GUEST_WAIT) {
        Ok(result) => Ok(game_succeeded(result)),
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

/// Whether the game ended without a failure. A failure that is not on the error stream yet
/// is written there.
fn game_succeeded(result: Result<(), RunError>) -> bool {
    match result {
        Ok(()) => true,
        // The report of a script error is on the error stream already.
        Err(RunError::Script(_)) => false,
        Err(error) => {
            eprintln!("vkxp: {error}");
            false
        }
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

    /// Requirement "Guest option": the default by kind of run, and scenario "Headless
    /// default".
    #[test]
    fn without_a_choice_the_guest_is_a_process_in_a_window_and_a_thread_headless() {
        assert_eq!(guest_form(None, false), GuestForm::Process);
        assert_eq!(guest_form(None, true), GuestForm::Thread);
        for headless in [false, true] {
            for form in [GuestForm::Thread, GuestForm::Process] {
                assert_eq!(guest_form(Some(form), headless), form);
            }
        }
    }

    /// Requirement "Sandbox option": the default, and the functions that start processes.
    #[test]
    fn a_guest_process_is_confined_unless_told_otherwise() {
        assert!(confined(None, false));
        assert!(confined(Some(true), false));
        assert!(!confined(Some(false), false));
        for sandbox in [None, Some(true), Some(false)] {
            assert!(!confined(sandbox, true));
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

    #[test]
    fn a_missing_built_in_scripts_file_names_the_tool() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("absent.rb");
        let error = builtin_scripts(Some(absent.clone()), Runtime::Rgss1).unwrap_err();
        let message = error.to_string();
        assert!(message.contains(EXTRACTION_TOOL), "{message}");
        assert!(message.contains("absent.rb"), "{message}");
        // The file of RGSS2 is not made from the DLL file.
        let error = builtin_scripts(Some(absent), Runtime::Rgss2).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("docs/running.md"), "{message}");
        assert!(message.contains("absent.rb"), "{message}");
    }

    fn ini(library: &str, scripts: &str) -> GameIni {
        let text = format!("[Game]\nLibrary={library}\nScripts={scripts}\n");
        GameIni::parse(text.as_bytes(), None).unwrap()
    }

    /// Requirement "Runtime of the run".
    #[test]
    fn the_runtime_is_the_one_named_or_the_one_the_file_tells() {
        let xp = ini("RGSS104E.dll", "Data\\Scripts.rxdata");
        assert_eq!(runtime_of(&xp, None).unwrap(), Runtime::Rgss1);
        assert_eq!(
            runtime_of(&xp, Some(Runtime::Rgss2)).unwrap(),
            Runtime::Rgss2
        );

        let conflict = ini("RGSS104E.dll", "Data\\Scripts.rvdata");
        let message = runtime_of(&conflict, None).unwrap_err().to_string();
        assert!(message.contains("--runtime"), "{message}");
        assert!(message.contains("RGSS104E.dll"), "{message}");
        assert!(message.contains("Data\\Scripts.rvdata"), "{message}");
        let named = runtime_of(&conflict, Some(Runtime::Rgss2));
        assert_eq!(named.unwrap(), Runtime::Rgss2);
    }

    /// Requirement "VX Ace is refused".
    #[test]
    fn a_game_of_vx_ace_is_refused_however_its_runtime_is_known() {
        let ace = ini("System\\RGSS301.dll", "Data\\Scripts.rvdata2");
        let xp = ini("RGSS104E.dll", "Data\\Scripts.rxdata");
        for (ini, named) in [(&ace, None), (&xp, Some(Runtime::Rgss3))] {
            let message = runtime_of(ini, named).unwrap_err().to_string();
            assert!(message.contains("RPG Maker VX Ace"), "{message}");
            assert!(message.contains("not supported"), "{message}");
            assert!(message.contains("planned"), "{message}");
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

    #[test]
    fn an_existing_built_in_scripts_file_is_taken() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("rpg.rb");
        std::fs::write(&file, "module RPG;end\n").unwrap();
        for runtime in [Runtime::Rgss1, Runtime::Rgss2] {
            assert_eq!(builtin_scripts(Some(file.clone()), runtime).unwrap(), file);
        }
    }
}
