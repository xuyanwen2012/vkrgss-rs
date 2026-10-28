//! Running a game: the guest's side of the link, the preparation of the interpreter, and
//! the evaluation of the game's scripts from start to shutdown.
//!
//! The guest holds the [`GuestEnd`] of the link. Everything it tells the host goes through
//! [`Guest::send`], and everything the host tells it is consumed in one place,
//! [`Guest::pump`]. [`run_game`] is the one call that plays a game.
//!
//! # States
//!
//! A guest whose [`RunConfig`] has a [`Copier`] can be saved. A request to save a state is
//! noted when it arrives and served when the guest next enters a wait for a tick whose frame
//! was sent while the display was not frozen and no modal message was shown: there the
//! copier makes a copy of the guest, and the guest goes on as if nothing had happened. A
//! copy that is resumed comes back from the same call with a new link, on which it sends
//! what a host that holds nothing needs: the title, what the music and background sound
//! channels play, and the frame of that wait with every bitmap and table whole. Then it
//! waits for the tick like the guest it was copied from.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use vkrgss_core::BitmapError;
use vkrgss_proto::GuestToHost;
use vkrgss_proto::link::GuestEnd;
use vkrgss_ruby_sys::{Ruby, RubyError};
use vkrgss_vfs::{GameIni, LibraryVersion, Runtime};

use crate::guest::{Config, Guest};
use crate::stats;
use crate::{audio, rb};

mod link;
mod prelude;
mod report;
mod scripts;
mod startup;

pub(crate) use link::{Await, Playing, raise_pending};
pub use link::{Copied, Copier, CopyFailed};
pub(crate) use prelude::{define, prepare, prepare_loading, turn_process_functions_off};
pub(crate) use scripts::evaluate;
pub use startup::frame_size;
pub(crate) use startup::{new_scene, rules};

use link::Pending;
use scripts::{load, run_scripts};

/// What running a game needs beyond the guest's files, fonts and text.
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// The game's configuration file, read by the caller: the window title and the path of
    /// the script archive come from it.
    pub ini: GameIni,
    /// The runtime the game was made for, settled by the caller.
    pub runtime: Runtime,
    /// The file of the built-in `RPG` scripts, extracted from the original runtime's DLL.
    pub builtin_scripts: PathBuf,
    /// Debug mode: `$DEBUG` is `true` in RGSS1, `$TEST` in the later runtimes.
    pub debug: bool,
    /// Battle-test mode: `$BTEST` is `true`, and in the runtimes after RGSS1 `$TEST` too.
    pub battle_test: bool,
    /// Whether `system`, backticks, `exec`, `fork`, `syscall`, `IO.popen`, `Process.kill`,
    /// and `open` and the class methods of `IO` that read a file with the name of a command,
    /// start or signal processes. They do nothing when this is `false`.
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
    /// Whether the guest measures where its frames spend their time and writes the summary
    /// to the error stream, every 600 frames and when it ends: rows for the scripts, the
    /// bitmap operations, the text and the extraction of the frame. Without it no clock is
    /// read.
    pub frame_stats: bool,
    /// The way the guest has itself copied when it is asked to save a state. A guest
    /// without one answers such a request with the failure message.
    pub copier: Option<Arc<dyn Copier>>,
    /// The version of the DLL the game's `Library` names, read by the caller; `None` for no
    /// DLL and for a file without a version record. In a game of RGSS3 it decides
    /// `RGSS_VERSION`: `"3.0.0"` for a version whose first three numbers are 3.0.0
    /// ([`LibraryVersion::is_3_0_0`]) and `"3.0.1"` otherwise, without one included. The
    /// other runtimes do not read it.
    pub library_version: Option<LibraryVersion>,
}

impl Default for RunConfig {
    /// A game of RGSS1 without a title whose scripts are in `Data/Scripts.rxdata`, with no
    /// built-in scripts file and every switch off.
    fn default() -> RunConfig {
        RunConfig {
            ini: GameIni {
                library: String::new(),
                scripts: "Data/Scripts.rxdata".to_owned(),
                title: String::new(),
                rtp: String::new(),
                rtps: Default::default(),
            },
            runtime: Runtime::Rgss1,
            builtin_scripts: PathBuf::new(),
            debug: false,
            battle_test: false,
            process_functions: false,
            seed: None,
            preload: Vec::new(),
            log_p: false,
            frame_stats: false,
            copier: None,
            library_version: None,
        }
    }
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
    /// The slots of the state requests that wait for a frame at which the guest may be
    /// copied.
    saves: Vec<u8>,
    /// Whether the display holds a picture: from `Graphics.freeze` or the start of a
    /// transition to the end of the transition.
    pub(crate) held: bool,
    /// Whether a modal message is shown.
    modal: bool,
    /// The error of the bitmap arena that failed the extraction of a frame, which fails the
    /// session.
    failure: Option<BitmapError>,
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
            saves: Vec::new(),
            held: false,
            modal: false,
            failure: None,
        }
    }
}

/// Why a game did not run to its end.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// The classes or the prelude of the binding could not be defined.
    #[error("the interpreter could not be prepared: {0}")]
    Prepare(RubyError),
    /// The file of the built-in `RPG` scripts could not be read.
    #[error(
        "cannot read the built-in scripts file {}: {source}; it holds the RPG scripts of the \
         original runtime, which vkrgss's script extraction tool takes out of the RGSS DLL",
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
    /// The game is of a runtime that this binding's interpreter does not run: a game of
    /// RGSS3 over Ruby 1.8.7. Nothing of the game was evaluated.
    #[error(
        "a game of {} ({}) runs over Ruby 1.9.2, and this binding is built over Ruby 1.8.7",
        runtime.product(),
        runtime.name()
    )]
    Interpreter {
        /// The runtime of the game.
        runtime: Runtime,
    },
    /// The bitmap arena failed while a frame was extracted. No script can cause this; the
    /// game was ended as on a quit request.
    #[error("a frame could not be extracted: {0}")]
    Extraction(BitmapError),
}

/// The modal message of a game of RGSS3 whose script archive cannot be loaded, measured on
/// the original runtime (`ace-v1-api-REPORT.md` section 1.1). The error that names the
/// archive is the caller's to write.
const FAILED_TO_LOAD_SCRIPT: &str = "Failed to load script.";

/// Plays the game of the installed guest's [`RunConfig`] over `link`: everything of
/// [`run_game`] after the installation of the guest.
pub(crate) fn play(ruby: &Ruby, link: GuestEnd) -> Result<(), RunError> {
    Guest::with(ruby, |guest| {
        guest.connect(link);
        stats::start(guest.runtime.run.frame_stats);
        let title = guest.runtime.run.ini.title.clone();
        guest.send(GuestToHost::SetTitle { title });
    });
    let outcome = prepare(ruby)
        .map_err(RunError::Prepare)
        .and_then(|()| load(ruby))
        .and_then(|(preload, count)| {
            // Symbols the scripts create from here on are numbered as in the original.
            ruby.mark_game_scripts();
            run_scripts(ruby, &preload, count)
        });
    let outcome = match Guest::with(ruby, |guest| guest.runtime.failure.take()) {
        Some(error) => Err(RunError::Extraction(error)),
        None => outcome,
    };
    let shown = match &outcome {
        Err(RunError::Script(report)) => Some(report.clone()),
        Err(RunError::Archive { .. })
            if Guest::with(ruby, |guest| guest.runtime.run.runtime) == Runtime::Rgss3 =>
        {
            Some(FAILED_TO_LOAD_SCRIPT.to_owned())
        }
        _ => None,
    };
    if let Some(text) = shown {
        eprintln!("{text}");
        Guest::with(ruby, |guest| guest.show_message(text));
    }
    Guest::with(ruby, |guest| {
        audio::stop_all(guest);
        guest.send(GuestToHost::Outcome {
            ok: outcome.is_ok(),
        });
        guest.send(GuestToHost::Shutdown);
        guest.disconnect();
    });
    stats::finish();
    outcome
}

/// Plays a game from start to shutdown. Called once, on the interpreter thread, in an
/// interpreter that has run nothing yet.
///
/// 1. Installs the guest of `config` ([`Guest::install`]) and sends the host the window
///    title of the game's configuration file.
/// 1. Refuses a game of RGSS3 over Ruby 1.8.7 with [`RunError::Interpreter`], before
///    anything of the game is evaluated. Installs the guest of `config`
///    ([`Guest::install`]) and sends the host the window title of the game's configuration
///    file.
/// 2. Prepares the interpreter with the prelude of this crate: over Ruby 1.8 `$KCODE` and an
///    empty load path, `$DEBUG` and `$BTEST`, `Reset` and in RGSS1 `Hangup`, `save_data`, `print` and `p`, `Win32API`,
///    the process functions and `require` and `load`. A game of RGSS3 gets
///    `RGSS_VERSION`, by the [`RunConfig::library_version`], `RGSSReset`, `save_data`,
///    `load_data`, `msgbox` and `msgbox_p` in the place of `Reset`, `print` and `p`, which
///    are Ruby's own there, `at_exit` that runs nothing, `exit!` that ends the process,
///    `require` and `load` that compile as UTF-8, `require_relative` and `autoload`.
/// 3. Reads the preload scripts, seeds the random number generator when the configuration
///    has a seed, and evaluates the built-in scripts file under the name `(builtin)`.
/// 4. Reads the script archive into `$RGSS_SCRIPTS`, each entry with its inflated source
///    as fourth element. In RGSS3 an archive that cannot be loaded is written to the error
///    stream and shown as `Failed to load script.` before step 6.
/// 5. Marks the start of the game's scripts for the interpreter's symbol numbering, once per
///    process. Evaluates the preload scripts under their paths, then the scripts in order as
///    `Section000`, `Section001` and so on, or `{0000}`, `{0001}` in RGSS3. `Reset` leaving a script stops the audio,
///    disposes every scene element and bitmap, clears the input state, seeds the generator
///    again and starts over from the first preload script; `SystemExit` ends the game. Any
///    other exception ends it with [`RunError::Script`], after its report was written to the
///    error stream, shown in the message panel and dismissed by the host.
/// 6. Stops the audio, sends the host the outcome of the game and the shutdown message and
///    drops the link, also when a step before failed.
///
/// # Panics
///
/// Panics when a guest is already installed.
pub fn run_game(ruby: &Ruby, config: Config, link: GuestEnd) -> Result<(), RunError> {
    let runtime = config.run.runtime;
    if runtime == Runtime::Rgss3 && !rb::RUBY_19 {
        return Err(RunError::Interpreter { runtime });
    }
    Guest::install(ruby, config).map_err(|raised| {
        // SAFETY: `raised` was produced just now on this thread and sat in a local since.
        RunError::Prepare(unsafe { ruby.describe(raised) })
    })?;
    play(ruby, link)
}
