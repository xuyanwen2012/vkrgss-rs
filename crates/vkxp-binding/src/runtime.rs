//! Running a game: the guest's side of the link, the preparation of the interpreter, and
//! the evaluation of the game's scripts from start to shutdown.
//!
//! The guest holds the [`GuestEnd`] of the link. Everything it tells the host goes through
//! [`Guest::send`], and everything the host tells it is consumed in one place,
//! [`Guest::pump`]. [`run_game`] is the one call that plays a game.

use std::io;
use std::path::PathBuf;

use vkxp_core::InputState;
use vkxp_proto::link::GuestEnd;
use vkxp_proto::{GuestToHost, HostToGuest};
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{Raise, Raised, Ruby, RubyError};
use vkxp_vfs::GameIni;

use crate::guest::{Config, Guest};
use crate::panel::Panel;
use crate::{audio, graphics, input, kernel, rb, win32api};

/// The Ruby part of the preparation: globals, `Reset` and `Hangup`, `save_data`, `Win32API`,
/// and `require` and `load` limited to the game directory.
const PRELUDE: &str = include_str!("prelude.rb");

/// The Ruby definitions that turn the functions that start processes off.
const PROCESS_FUNCTIONS_OFF: &str = include_str!("prelude_process_off.rb");

/// The file name the prelude is reported under in backtraces.
const PRELUDE_NAME: &str = "(prelude)";

/// What running a game needs beyond the guest's files, fonts and text.
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// The game's configuration file, read by the caller: the window title and the path of
    /// the script archive come from it.
    pub ini: GameIni,
    /// The file of the built-in `RPG` scripts, extracted from the original runtime's DLL.
    pub builtin_scripts: PathBuf,
    /// Debug mode: `$DEBUG` is `true`.
    pub debug: bool,
    /// Battle-test mode: `$BTEST` is `true`.
    pub battle_test: bool,
    /// Whether `system`, backticks, `exec`, `fork`, `syscall` and `IO.popen` start processes.
    /// They do nothing when this is `false`.
    pub process_functions: bool,
    /// The seed of the random number generator, set before the built-in scripts run and
    /// again on every reset. Without one the generator is left as the interpreter seeds it.
    pub seed: Option<u64>,
    /// Ruby files evaluated in this order after the built-in scripts and before the game's
    /// first script, and again after every reset. They are read from these paths as given,
    /// not from the game directory.
    pub preload: Vec<PathBuf>,
    /// Whether `p` is log-only: it writes to the error stream, shows no message panel and
    /// waits for nothing.
    pub log_p: bool,
}

impl Default for RunConfig {
    /// A game without a title whose scripts are in `Data/Scripts.rxdata`, with no built-in
    /// scripts file and every switch off.
    fn default() -> RunConfig {
        RunConfig {
            ini: GameIni {
                library: String::new(),
                scripts: "Data/Scripts.rxdata".to_owned(),
                title: String::new(),
                rtp: Default::default(),
            },
            builtin_scripts: PathBuf::new(),
            debug: false,
            battle_test: false,
            process_functions: false,
            seed: None,
            preload: Vec::new(),
            log_p: false,
        }
    }
}

/// What a call of [`Guest::pump`] waits for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Await {
    /// Nothing: take what has arrived and return.
    Nothing,
    /// The tick of the frame just sent.
    Tick,
}

/// What the host asked for and the next `Graphics.update` raises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pending {
    /// `Reset`: the scripts start over.
    Reset,
    /// `SystemExit`: the game ends.
    Quit,
}

/// What a looping channel plays, as the script described it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Playing {
    pub(crate) name: String,
    pub(crate) volume: u8,
    pub(crate) pitch: u16,
}

/// The part of the guest state that running a game adds.
pub(crate) struct State {
    pub(crate) run: RunConfig,
    /// The guest end of the link. A guest without one behaves as one whose host has gone.
    link: Option<GuestEnd>,
    pending: Option<Pending>,
    /// Whether the dismissal of the modal message being shown has arrived.
    dismissed: bool,
    /// What the music channel plays.
    pub(crate) music: Option<Playing>,
    /// What the background-sound channel plays.
    pub(crate) background_sound: Option<Playing>,
    /// `Graphics.visible`.
    pub(crate) visible: bool,
}

impl State {
    pub(crate) fn new(run: RunConfig) -> State {
        State {
            run,
            link: None,
            pending: None,
            dismissed: false,
            music: None,
            background_sound: None,
            visible: true,
        }
    }
}

impl Guest {
    /// Makes `link` the link to the host and forgets what the previous one left: a pending
    /// reset or quit, the input and what the looping channels play.
    pub(crate) fn connect(&mut self, link: GuestEnd) {
        self.runtime.link = Some(link);
        self.runtime.pending = None;
        self.runtime.music = None;
        self.runtime.background_sound = None;
        self.input = InputState::new();
    }

    /// Drops the link, which tells the host that the guest has gone.
    pub(crate) fn disconnect(&mut self) {
        self.runtime.link = None;
    }

    /// Sends a message to the host. When the host has gone the message is dropped and the
    /// guest is left to quit.
    pub(crate) fn send(&mut self, message: GuestToHost) {
        let sent = match &self.runtime.link {
            Some(link) => link.send(message).is_ok(),
            None => false,
        };
        if !sent {
            self.host_gone();
        }
    }

    /// Consumes the messages of the host, and returns when what `wait` names has arrived, or
    /// with [`Await::Nothing`] when no message is waiting.
    ///
    /// An input snapshot is kept for the next `Input.update`, and a dismissal is noted for
    /// [`Guest::show_message`]. A reset or quit request is
    /// recorded for [`raise_pending`]; a host that has gone counts as a quit request and ends
    /// the wait.
    pub(crate) fn pump(&mut self, wait: Await) {
        loop {
            let Some(link) = &self.runtime.link else {
                return self.host_gone();
            };
            let message = if wait == Await::Nothing {
                match link.try_recv() {
                    Ok(Some(message)) => message,
                    Ok(None) => return,
                    Err(_) => return self.host_gone(),
                }
            } else {
                match link.recv() {
                    Ok(message) => message,
                    Err(_) => return self.host_gone(),
                }
            };
            match message {
                HostToGuest::Input(snapshot) => self.input.receive(snapshot),
                HostToGuest::Tick if wait == Await::Tick => return,
                HostToGuest::Tick => {}
                HostToGuest::ModalDismissed => self.runtime.dismissed = true,
                HostToGuest::Reset => {
                    self.runtime.pending.get_or_insert(Pending::Reset);
                }
                HostToGuest::Quit => self.runtime.pending = Some(Pending::Quit),
            }
        }
    }

    /// Sends the host the modal message `text` and shows it in the message panel until the
    /// dismissal has arrived: a frame with the panel is sent and its tick awaited, at least
    /// once and then again for as long as the host has not dismissed the message. These
    /// frames do not advance `Graphics.frame_count`. A reset or quit request ends the wait
    /// and is left for [`raise_pending`].
    pub(crate) fn show_message(&mut self, text: String) {
        let panel = Panel::show(self, &text);
        self.runtime.dismissed = false;
        self.send(GuestToHost::Modal { text });
        loop {
            let frame_count = self.scene.frame_count();
            let mut packet = self.scene.extract(&mut self.bitmaps);
            self.scene.set_frame_count(frame_count);
            if let Some(panel) = &panel {
                panel.bring_to_front(&mut packet.ops);
            }
            self.send(GuestToHost::Frame(packet));
            self.pump(Await::Tick);
            let frame_rate = self.scene.frame_rate().get();
            self.win32api.frame_shown(frame_rate.into());
            if self.runtime.dismissed || self.runtime.pending.is_some() {
                break;
            }
        }
        if let Some(panel) = panel {
            panel.dispose(self);
        }
    }

    fn host_gone(&mut self) {
        self.runtime.link = None;
        self.runtime.pending = Some(Pending::Quit);
    }
}

/// `SystemExit`.
fn system_exit(_ruby: &Ruby) -> VALUE {
    // SAFETY: `&Ruby` proves the thread; the class globals are set once the interpreter runs.
    unsafe { ffi::rb_eSystemExit }
}

/// Raises what the host asked for while the guest pumped: `Reset` for a reset request,
/// `SystemExit` for a quit request. Called after [`Guest::pump`] has returned and the guest
/// is released.
pub(crate) fn raise_pending(ruby: &Ruby) -> Result<(), Raise> {
    match Guest::with(|guest| guest.runtime.pending.take()) {
        None => Ok(()),
        Some(Pending::Quit) => Err(Raise::new(system_exit(ruby), "exit")),
        Some(Pending::Reset) => Err(Raise::new(rb::class_at(ruby, c"Reset")?, "reset")),
    }
}

/// Defines what a game meets outside the RGSS classes and that is implemented by host
/// functions. The Ruby part follows in [`prepare`].
pub(crate) fn define(ruby: &Ruby) -> Result<(), Raised> {
    graphics::define(ruby)?;
    input::define(ruby)?;
    audio::define(ruby)?;
    win32api::define(ruby)?;
    kernel::define(ruby)
}

/// Prepares the interpreter as the original runtime does before a game's scripts run:
/// evaluates the prelude, sets `$DEBUG` and `$BTEST` from the guest's [`RunConfig`], and
/// turns the process functions off unless it enables them.
pub(crate) fn prepare(ruby: &Ruby) -> Result<(), RubyError> {
    let (debug, battle_test, process_functions) = Guest::with(|guest| {
        let run = &guest.runtime.run;
        (run.debug, run.battle_test, run.process_functions)
    });
    ruby.eval_protected(PRELUDE, PRELUDE_NAME, 1)?;
    if !process_functions {
        ruby.eval_protected(PROCESS_FUNCTIONS_OFF, PRELUDE_NAME, 1)?;
    }
    // Last, so that debug mode does not report the exceptions the prelude itself may meet.
    let flags = format!("$DEBUG = {debug}\n$BTEST = {battle_test}\n");
    ruby.eval_protected(&flags, PRELUDE_NAME, 1)?;
    Ok(())
}

/// The file name the built-in scripts are reported under.
const BUILTIN_NAME: &str = "(builtin)";

/// Why a game did not run to its end.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// The classes or the prelude of the binding could not be defined.
    #[error("the interpreter could not be prepared: {0}")]
    Prepare(RubyError),
    /// The file of the built-in `RPG` scripts could not be read.
    #[error(
        "cannot read the built-in scripts file {}: {source}; it holds the RPG scripts of the \
         original runtime, which vkxp's script extraction tool takes out of the RGSS DLL",
        path.display()
    )]
    BuiltinScripts {
        /// The file the caller named.
        path: PathBuf,
        source: io::Error,
    },
    /// A preload script could not be read.
    #[error("cannot read the preload script {}: {source}", path.display())]
    Preload {
        /// The file the caller named.
        path: PathBuf,
        source: io::Error,
    },
    /// The script archive could not be found, read or taken apart.
    #[error("cannot load the script archive {path}: {reason}")]
    Archive {
        /// The path as the game's configuration file gives it.
        path: String,
        /// The exception that reading it raised.
        reason: String,
    },
    /// A script raised an exception that no script rescued. This is the report, which was
    /// also written to the error stream and shown by the host.
    #[error("{0}")]
    Script(String),
}

/// How an evaluation that did not return ended.
enum Ended {
    /// `SystemExit`: the game is over.
    Exit,
    /// `Reset`: the scripts start over.
    Reset,
    /// Anything else, as its report.
    Failed(String),
}

/// Evaluates a Ruby string at the top level under the file name `name`, as the original
/// runtime evaluates a script.
fn evaluate(ruby: &Ruby, source: VALUE, name: &str) -> Result<(), Raised> {
    let name = ruby.str_new(name.as_bytes())?;
    // SAFETY: `&Ruby` proves the thread, on which the main object exists for the life of the
    // interpreter. `source` is live by the caller's word and `name` was just returned; both
    // are in locals.
    unsafe {
        let arguments = [source, ffi::Qnil, name, ffi::long2fix(1)];
        ruby.funcall(ffi::ruby_top_self, c"eval", &arguments)?;
    }
    Ok(())
}

/// The file and line at the start of a backtrace entry such as ``Section001:3:in `update'``.
fn position(entry: &str) -> Option<(&str, u32)> {
    let (file, rest) = entry.split_once(':')?;
    let digits = rest.split(|c: char| !c.is_ascii_digit()).next()?;
    Some((file, digits.parse().ok()?))
}

/// The index of a script in the archive from the file name it is evaluated under.
fn section_index(file: &str) -> Option<usize> {
    let digits = file.strip_prefix("Section")?;
    (digits.len() >= 3 && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| digits.parse().ok())?
}

/// The name the archive stores for the script at `index`.
fn script_name(ruby: &Ruby, index: usize) -> Option<String> {
    let name = ruby
        .eval(&format!("$RGSS_SCRIPTS[{index}][1]"), PRELUDE_NAME, 1)
        .ok()?;
    rb::string_bytes(ruby, name).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// The report of an uncaught exception, in the form of the original runtime: the script by
/// the name the archive stores for it, found through the first position of the backtrace
/// (or, for a script that does not parse, of the message) that lies in a script.
fn report(ruby: &Ruby, error: &RubyError) -> String {
    let positions = || {
        error
            .backtrace
            .iter()
            .map(String::as_str)
            .chain(error.message.lines())
            .filter_map(position)
    };
    let in_a_script = positions().find_map(|(file, line)| {
        let name = script_name(ruby, section_index(file)?)?;
        Some((name, line))
    });
    let (name, line) = in_a_script
        .or_else(|| {
            positions()
                .next()
                .map(|(file, line)| (file.to_owned(), line))
        })
        .unwrap_or_default();
    format!(
        "Script '{name}' line {line}: {} occurred.\n\n{}",
        error.class_name, error.message
    )
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
    if rb::class_at(ruby, c"Reset").is_ok_and(is_a) {
        return Ended::Reset;
    }
    // SAFETY: as above, the exception is still alive in `raised`.
    let error = unsafe { ruby.describe(raised) };
    Ended::Failed(report(ruby, &error))
}

/// A preload script: the name it is evaluated under, which is its path as the caller gave
/// it, and its source.
type Preload = (String, Vec<u8>);

/// Seeds the random number generator with the seed of the guest's [`RunConfig`], if it has
/// one.
fn seed(ruby: &Ruby) -> Result<(), Raised> {
    if let Some(seed) = Guest::with(|guest| guest.runtime.run.seed) {
        ruby.eval(&format!("Kernel.srand({seed})"), PRELUDE_NAME, 1)?;
    }
    Ok(())
}

/// Reads the preload scripts, seeds the random number generator, evaluates the built-in
/// scripts file and reads the script archive. Returns the preload scripts and the number of
/// scripts in the archive.
fn load(ruby: &Ruby) -> Result<(Vec<Preload>, usize), RunError> {
    let (builtin, archive, preload) = Guest::with(|guest| {
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
    let source = ruby.str_new(&source).map_err(prepare)?;
    if let Err(raised) = evaluate(ruby, source, BUILTIN_NAME) {
        // SAFETY: as above.
        let error = unsafe { ruby.describe(raised) };
        return Err(RunError::Script(report(ruby, &error)));
    }

    let path = ruby.str_new(archive.as_bytes()).map_err(prepare)?;
    // SAFETY: the main object lives as long as the interpreter, and `path` was just returned
    // and is in a local.
    let count = unsafe { ruby.funcall(ffi::ruby_top_self, c"vkxp_load_scripts", &[path]) };
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
fn reset(ruby: &Ruby) -> Result<(), Raised> {
    Guest::with(audio::stop_all);
    let graphics = rb::class_at(ruby, c"Graphics")?;
    // SAFETY: the module is kept alive by its constant.
    unsafe { ruby.funcall(graphics, c"_reset", &[]) }?;
    Guest::with(|guest| {
        guest.input = InputState::new();
        guest.runtime.pending = None;
    });
    seed(ruby)
}

/// Evaluates the preload scripts and then the `count` scripts of `$RGSS_SCRIPTS` in order,
/// each of the latter under the name `Section` and its index, again from the first preload
/// script after a reset, until the last returns or one exits.
fn run_scripts(ruby: &Ruby, preload: &[Preload], count: usize) -> Result<(), RunError> {
    let mut index = 0;
    while index < preload.len() + count {
        let evaluated = match preload.get(index) {
            Some((name, source)) => ruby
                .str_new(source)
                .and_then(|source| evaluate(ruby, source, name)),
            None => {
                let index = index - preload.len();
                ruby.eval(&format!("$RGSS_SCRIPTS[{index}][3]"), PRELUDE_NAME, 1)
                    .and_then(|source| evaluate(ruby, source, &format!("Section{index:03}")))
            }
        };
        index += 1;
        let Err(raised) = evaluated else {
            continue;
        };
        match ended(ruby, raised) {
            Ended::Exit => break,
            Ended::Reset => match reset(ruby) {
                Ok(()) => index = 0,
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

/// Plays the game of the installed guest's [`RunConfig`] over `link`: everything of
/// [`run_game`] after the installation of the guest.
pub(crate) fn play(ruby: &Ruby, link: GuestEnd) -> Result<(), RunError> {
    Guest::with(|guest| {
        guest.connect(link);
        let title = guest.runtime.run.ini.title.clone();
        guest.send(GuestToHost::SetTitle { title });
    });
    let outcome = prepare(ruby)
        .map_err(RunError::Prepare)
        .and_then(|()| load(ruby))
        .and_then(|(preload, count)| run_scripts(ruby, &preload, count));
    if let Err(RunError::Script(report)) = &outcome {
        eprintln!("{report}");
        Guest::with(|guest| guest.show_message(report.clone()));
    }
    Guest::with(|guest| {
        audio::stop_all(guest);
        guest.send(GuestToHost::Shutdown);
        guest.disconnect();
    });
    outcome
}

/// Plays a game from start to shutdown. Called once, on the interpreter thread, in an
/// interpreter that has run nothing yet.
///
/// 1. Installs the guest of `config` ([`Guest::install`]) and sends the host the window
///    title of the game's configuration file.
/// 2. Prepares the interpreter with the prelude of this crate: `$KCODE`, an empty load path,
///    `$DEBUG` and `$BTEST`, `Reset` and `Hangup`, `save_data`, `print` and `p`, `Win32API`,
///    the process functions and `require` and `load`.
/// 3. Reads the preload scripts, seeds the random number generator when the configuration
///    has a seed, and evaluates the built-in scripts file under the name `(builtin)`.
/// 4. Reads the script archive into `$RGSS_SCRIPTS`, each entry with its inflated source
///    as fourth element.
/// 5. Evaluates the preload scripts under their paths, then the scripts in order as
///    `Section000`, `Section001` and so on. `Reset` leaving a script stops the audio,
///    disposes every scene element and bitmap, clears the input state, seeds the generator
///    again and starts over from the first preload script; `SystemExit` ends the game. Any
///    other exception ends it with [`RunError::Script`], after its report was written to the
///    error stream, shown in the message panel and dismissed by the host.
/// 6. Stops the audio, sends the host the shutdown message and drops the link, also when a
///    step before failed.
///
/// # Panics
///
/// Panics when a guest is already installed on this thread.
pub fn run_game(ruby: &Ruby, config: Config, link: GuestEnd) -> Result<(), RunError> {
    Guest::install(ruby, config).map_err(|raised| {
        // SAFETY: `raised` was produced just now on this thread and sat in a local since.
        RunError::Prepare(unsafe { ruby.describe(raised) })
    })?;
    play(ruby, link)
}
