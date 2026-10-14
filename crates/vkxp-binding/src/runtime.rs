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

use std::ffi::CStr;
use std::fmt::Debug;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use vkxp_core::{InputState, Rules, Scene};
use vkxp_proto::link::GuestEnd;
use vkxp_proto::{AudioChannel, AudioCommand, GuestToHost, HostToGuest, Size};
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{Raise, Raised, Ruby, RubyError};
use vkxp_vfs::{GameIni, LibraryVersion, Runtime};

use crate::guest::{Config, Guest};
use crate::panel::Panel;
use crate::stats::{self, Part, Span};
use crate::{audio, graphics, input, kernel, rb, win32api};

/// The Ruby part of the preparation of a game of RGSS1 or RGSS2: `Reset`, `save_data`,
/// `print`, `p` and the loader of the script archive.
const PRELUDE: &str = include_str!("prelude.rb");

/// The Ruby part of the preparation of a game of RGSS3, in the place of [`PRELUDE`] and
/// after [`PRELUDE_SHARED`]: `RGSSReset`, `save_data`, `at_exit`, `exit!`, `msgbox`,
/// `msgbox_p`, `require` and `load` that compile as UTF-8, `require_relative`, `autoload`
/// and the loader of the script archive. `RGSS_VERSION` is defined before it
/// ([`rgss3_version`]), and `load_data` is a host function of [`kernel`].
const PRELUDE_RGSS3: &str = include_str!("prelude_rgss3.rb");

/// What the preparation evaluates for every runtime, after [`PRELUDE`] or before
/// [`PRELUDE_RGSS3`]: `require` and `load` limited to the game directory.
const PRELUDE_SHARED: &str = include_str!("prelude_shared.rb");

/// `Win32API` of RGSS1 and RGSS2, evaluated before [`PRELUDE_SHARED`]. In RGSS3 the class is
/// one of the binding.
const PRELUDE_WIN32API: &str = include_str!("prelude_win32api.rb");

/// `DL` of RGSS3, evaluated after [`PRELUDE_SHARED`].
const PRELUDE_DL: &str = include_str!("prelude_dl.rb");

/// What is prepared over Ruby 1.8 only, before the prelude: `$KCODE` and the empty load path.
const PRELUDE_RUBY18: &str = include_str!("prelude_ruby18.rb");

/// What the prelude has in RGSS1 only: `Hangup`.
const PRELUDE_RGSS1: &str = include_str!("prelude_rgss1.rb");

/// The Ruby definitions that turn the functions that start processes off.
const PROCESS_FUNCTIONS_OFF: &str = include_str!("prelude_process_off.rb");

/// What turns off the functions that start processes that Ruby 1.9 has beside them.
const PROCESS_FUNCTIONS_OFF_RUBY19: &str = include_str!("prelude_process_off_ruby19.rb");

/// The file name the prelude is reported under in backtraces.
const PRELUDE_NAME: &str = "(prelude)";

/// How a guest came back from having itself copied.
#[derive(Debug)]
pub enum Copied {
    /// This is the guest that asked. A copy of it is parked.
    WentOn,
    /// This is a copy that was parked and has been told to resume, with its link to the
    /// host, which holds nothing of this guest.
    Resumed(GuestEnd),
}

/// No copy could be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the guest could not be copied")]
pub struct CopyFailed;

/// The way a guest has itself copied for a state. The guest calls it on the interpreter
/// thread, in a wait for a tick.
pub trait Copier: Debug + Send + Sync {
    /// Parks a copy of the guest as the state of `slot`. Returns [`Copied::WentOn`] in the
    /// guest that asked and, in a copy that was resumed, the new link.
    fn park(&self, slot: u8) -> Result<Copied, CopyFailed>;
}

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
    /// `Reset`, or `RGSSReset` in RGSS3: the scripts start over.
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
    /// The slots of the state requests that wait for a frame at which the guest may be
    /// copied.
    saves: Vec<u8>,
    /// Whether the display holds a picture: from `Graphics.freeze` or the start of a
    /// transition to the end of the transition.
    pub(crate) held: bool,
    /// Whether a modal message is shown.
    modal: bool,
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
        }
    }
}

impl Guest {
    /// Makes `link` the link to the host and forgets what the previous one left: a pending
    /// reset or quit, state requests, a held picture, the input and what the looping
    /// channels play.
    pub(crate) fn connect(&mut self, link: GuestEnd) {
        self.runtime.link = Some(link);
        self.runtime.pending = None;
        self.runtime.saves.clear();
        self.runtime.held = false;
        self.runtime.modal = false;
        self.runtime.music = None;
        self.runtime.background_sound = None;
        self.audio = audio::State::default();
        self.input = InputState::with_rules(self.input.rules());
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
    /// the wait. A request to save a state is noted, or answered with the failure message
    /// by a guest that cannot be copied.
    ///
    /// A wait for a tick is where a frame of the guest's frame time statistics ends and the
    /// next begins, and where the guest is copied for the state requests that wait; see
    /// the module documentation.
    pub(crate) fn pump(&mut self, wait: Await) {
        if wait == Await::Tick {
            stats::frame_ends();
            self.receive(wait);
            stats::frame_begins();
        } else {
            self.receive(wait);
        }
    }

    fn receive(&mut self, wait: Await) {
        if wait == Await::Tick {
            self.save_states();
        }
        loop {
            let Some(link) = &self.runtime.link else {
                return self.host_gone();
            };
            let message = if wait == Await::Nothing {
                match link.try_recv() {
                    Ok(Some(message)) => message,
                    Ok(None) => return,
                    Err(_) => return self.host_silent(),
                }
            } else {
                match link.recv() {
                    Ok(message) => message,
                    Err(_) => return self.host_silent(),
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
                HostToGuest::SaveState { slot } if self.runtime.run.copier.is_some() => {
                    self.runtime.saves.push(slot);
                }
                // A guest that cannot be copied has no states either.
                HostToGuest::SaveState { slot } => {
                    self.send(GuestToHost::StateFailed { slot, load: false });
                }
                HostToGuest::LoadState { slot } => {
                    self.send(GuestToHost::StateFailed { slot, load: true });
                }
                // Only a parked copy is told to resume, and it is not here while it waits.
                HostToGuest::Resume => {}
            }
        }
    }

    /// Has the guest copied for every state request that waits, if the frame just sent is
    /// one at which it may be: the display holds no picture and no modal message is shown.
    /// Called on entering the wait for the tick of that frame.
    fn save_states(&mut self) {
        if self.runtime.held || self.runtime.modal || self.runtime.saves.is_empty() {
            return;
        }
        let Some(copier) = self.runtime.run.copier.clone() else {
            return;
        };
        for slot in std::mem::take(&mut self.runtime.saves) {
            match copier.park(slot) {
                Ok(Copied::WentOn) => {}
                // The requests that still waited are those of the guest that goes on.
                Ok(Copied::Resumed(link)) => return self.resume(link),
                Err(CopyFailed) => self.send(GuestToHost::StateFailed { slot, load: false }),
            }
        }
    }

    /// What a copy does when it is resumed, in the wait for a tick it was made in: `link`
    /// becomes its link, and it sends the host, which holds nothing of this guest, the
    /// title, whether the pointer is to be seen, the play commands of the music and the background sound it remembers, and the
    /// frame of the wait again with every live bitmap and table whole.
    fn resume(&mut self, link: GuestEnd) {
        self.runtime.link = Some(link);
        self.runtime.pending = None;
        self.runtime.dismissed = false;
        self.bitmaps.resend_all();
        self.scene.resend_tables();
        let title = self.runtime.run.ini.title.clone();
        self.send(GuestToHost::SetTitle { title });
        let visible = self.win32api.pointer_visible();
        self.send(GuestToHost::PointerVisible { visible });
        let looping = [
            (AudioChannel::Bgm, self.runtime.music.clone()),
            (AudioChannel::Bgs, self.runtime.background_sound.clone()),
        ];
        for (channel, playing) in looping {
            let Some(playing) = playing else {
                continue;
            };
            // A file that has gone since plays nothing, as if the channel had stopped.
            let kind = audio::file_kind(self.runtime.run.runtime);
            if let Ok(data) = self.files.read(kind, &playing.name) {
                self.send(GuestToHost::Audio(AudioCommand::Play {
                    channel,
                    data,
                    volume: playing.volume,
                    pitch: playing.pitch,
                    start: 0,
                    music_return_ms: 0,
                }));
            }
        }
        // The frame of this wait was counted when it was first sent.
        let frame_count = self.scene.frame_count();
        let packet = self.scene.extract(&mut self.bitmaps);
        self.scene.set_frame_count(frame_count);
        self.send(GuestToHost::Frame(packet));
    }

    /// Sends the host the modal message `text` and shows it in the message panel until the
    /// dismissal has arrived: a frame with the panel is sent and its tick awaited, at least
    /// once and then again for as long as the host has not dismissed the message. These
    /// frames do not advance `Graphics.frame_count`. A reset or quit request ends the wait
    /// and is left for [`raise_pending`].
    pub(crate) fn show_message(&mut self, text: String) {
        let panel = Panel::show(self, &text);
        self.runtime.dismissed = false;
        self.runtime.modal = true;
        self.send(GuestToHost::Modal { text });
        loop {
            let frame_count = self.scene.frame_count();
            let extraction = Span::enter(Part::Extraction);
            let mut packet = self.scene.extract(&mut self.bitmaps);
            self.scene.set_frame_count(frame_count);
            if let Some(panel) = &panel {
                panel.bring_to_front(&mut packet.ops);
            }
            self.send(GuestToHost::Frame(packet));
            drop(extraction);
            self.pump(Await::Tick);
            let frame_rate = self.scene.frame_rate().get();
            self.win32api.frame_shown(frame_rate.into());
            if self.runtime.dismissed || self.runtime.pending.is_some() {
                break;
            }
        }
        self.runtime.modal = false;
        if let Some(panel) = panel {
            panel.dispose(self);
        }
    }

    /// The host sends nothing more, which counts as a quit request. The link stays for
    /// what the guest still has to say: whoever reads it may still be there.
    fn host_silent(&mut self) {
        self.runtime.pending = Some(Pending::Quit);
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

/// The class of the exception that resets a game of `runtime`: `Reset`, or `RGSSReset` in
/// RGSS3.
fn reset_class_name(runtime: Runtime) -> &'static CStr {
    match runtime {
        Runtime::Rgss1 | Runtime::Rgss2 => c"Reset",
        Runtime::Rgss3 => c"RGSSReset",
    }
}

/// Raises what the host asked for while the guest pumped: `Reset` with the message `reset`
/// for a reset request, or in RGSS3 `RGSSReset` with an empty message, and `SystemExit` for
/// a quit request. Called after [`Guest::pump`] has returned and the guest is released.
pub(crate) fn raise_pending(ruby: &Ruby) -> Result<(), Raise> {
    let (pending, runtime) =
        Guest::with(|guest| (guest.runtime.pending.take(), guest.runtime.run.runtime));
    match pending {
        None => Ok(()),
        Some(Pending::Quit) => Err(Raise::new(system_exit(ruby), "exit")),
        Some(Pending::Reset) => {
            let class = rb::class_at(ruby, reset_class_name(runtime))?;
            let message = if runtime == Runtime::Rgss3 {
                ""
            } else {
                "reset"
            };
            Err(Raise::new(class, message))
        }
    }
}

/// Defines what a game meets outside the RGSS classes and that is implemented by host
/// functions. The Ruby part follows in [`prepare`].
pub(crate) fn define(ruby: &Ruby) -> Result<(), Raised> {
    graphics::define(ruby)?;
    input::define(ruby, Guest::with(|guest| guest.runtime.run.runtime))?;
    audio::define(ruby)?;
    win32api::define(ruby, Guest::with(|guest| guest.runtime.run.runtime))?;
    kernel::define(ruby)
}

/// Prepares the interpreter as the original runtime does before a game's scripts run:
/// evaluates the prelude of the runtime and the shared one, sets the globals of test mode
/// from the guest's [`RunConfig`], and turns the process functions off unless it enables
/// them.
pub(crate) fn prepare(ruby: &Ruby) -> Result<(), RubyError> {
    let (runtime, debug, battle_test, process_functions, library_version) = Guest::with(|guest| {
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
        ruby.eval_protected(PROCESS_FUNCTIONS_OFF, PRELUDE_NAME, 1)?;
        if rb::RUBY_19 {
            ruby.eval_protected(PROCESS_FUNCTIONS_OFF_RUBY19, PRELUDE_NAME, 1)?;
        }
    }
    // Last, so that debug mode does not report the exceptions the prelude itself may meet.
    let flags = test_mode_globals(runtime, debug, battle_test);
    ruby.eval_protected(&flags, PRELUDE_NAME, 1)?;
    Ok(())
}

/// `RGSS_VERSION` of a game of RGSS3 whose DLL has `version`: `"3.0.0"` for a version
/// whose first three numbers are 3.0.0 ([`LibraryVersion::is_3_0_0`]), and `"3.0.1"`, the
/// version vkxp follows, for every other version, a DLL without a version record and no
/// DLL. This is the table of the glyph rule of `docs/decisions.md`.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1 and RGSS300 3.0.0.1: each
/// states its own version.
fn rgss3_version(version: Option<LibraryVersion>) -> &'static str {
    match version {
        Some(version) if version.is_3_0_0() => "3.0.0",
        _ => "3.0.1",
    }
}

/// The source that defines `RGSS_VERSION` as `version`, in ASCII-8BIT. A process prepares
/// once; the harness of the tests prepares again for every game it plays on its one
/// interpreter, so a constant of an earlier preparation is removed first and the constant
/// is defined once, without the warning of a constant defined again.
fn rgss3_version_source(version: &str) -> String {
    format!(
        "Object.send(:remove_const, :RGSS_VERSION) if defined?(RGSS_VERSION)\n\
         RGSS_VERSION = \"{version}\".force_encoding(\"ASCII-8BIT\")\n"
    )
}

/// The source that sets the globals of test mode. RGSS1 has `$DEBUG` and `$BTEST`. The
/// later runtimes have `$TEST` instead of `$DEBUG`, which stays `false`, and a battle test
/// sets `$TEST` as well: measured on the original runtime of VX.
fn test_mode_globals(runtime: Runtime, debug: bool, battle_test: bool) -> String {
    match runtime {
        Runtime::Rgss1 => format!("$DEBUG = {debug}\n$BTEST = {battle_test}\n"),
        Runtime::Rgss2 | Runtime::Rgss3 => format!(
            "$DEBUG = false\n$TEST = {}\n$BTEST = {battle_test}\n",
            debug || battle_test
        ),
    }
}

/// The size of the frame of a game of RGSS1.
const RGSS1_FRAME_SIZE: Size = Size::new(640, 480);
/// The frame rate a game of RGSS1 starts with.
const RGSS1_FRAME_RATE: i64 = 40;
/// The size of the frame of a game of RGSS2, measured on the original runtime.
const RGSS2_FRAME_SIZE: Size = Size::new(544, 416);
/// The frame rate a game of RGSS2 starts with, measured on the original runtime.
const RGSS2_FRAME_RATE: i64 = 60;

/// The size of the frame a game of `runtime` starts with, which is the logical size of its
/// frame messages.
pub fn frame_size(runtime: Runtime) -> Size {
    match runtime {
        Runtime::Rgss1 => RGSS1_FRAME_SIZE,
        Runtime::Rgss2 | Runtime::Rgss3 => RGSS2_FRAME_SIZE,
    }
}

/// The rules of the core the scene and the input state of a game of `runtime` follow where the
/// runtimes differ.
pub(crate) fn rules(runtime: Runtime) -> Rules {
    match runtime {
        Runtime::Rgss1 => Rules::Rgss1,
        Runtime::Rgss2 => Rules::Rgss2,
        Runtime::Rgss3 => Rules::Rgss3,
    }
}

/// The empty scene a game of `runtime` starts with: its rules, its frame size and its frame
/// rate.
pub(crate) fn new_scene(runtime: Runtime) -> Scene {
    let mut scene = Scene::with_rules(rules(runtime), frame_size(runtime));
    scene.set_frame_rate(match runtime {
        Runtime::Rgss1 => RGSS1_FRAME_RATE,
        Runtime::Rgss2 | Runtime::Rgss3 => RGSS2_FRAME_RATE,
    });
    scene
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
}

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
fn evaluate(ruby: &Ruby, source: VALUE, name: &str) -> Result<(), Raised> {
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

/// The index of a script in the archive from the file name it is evaluated under in RGSS3:
/// `{` and the index as four digits and `}`, found anywhere in the name.
fn script_index_rgss3(file: &str) -> Option<usize> {
    let (_, rest) = file.split_once('{')?;
    let (digits, _) = rest.split_once('}')?;
    (!digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
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

/// The position at which the report of RGSS3 divides a line: the last `:<digits>:` that has
/// something before it, as `(.+):(\d+):(.*)` of Ruby matches. Returns the file, the line and
/// what follows.
fn split_position(line: &str) -> Option<(&str, u32, &str)> {
    let mut search = line.len();
    while let Some(end) = line[..search].rfind(':') {
        let digits_start = line[..end]
            .bytes()
            .rposition(|byte| !byte.is_ascii_digit())
            .map_or(0, |index| index + 1);
        if digits_start < end && digits_start >= 2 && line.as_bytes()[digits_start - 1] == b':' {
            let line_number = line[digits_start..end].parse().unwrap_or(u32::MAX);
            return Some((&line[..digits_start - 1], line_number, &line[end + 1..]));
        }
        search = end;
    }
    None
}

/// The file and line of a backtrace entry as the report of RGSS3 reads them: an entry
/// without a position is the file, and its line is the number it starts with, 0 for most.
fn entry_position(entry: &str) -> (&str, u32) {
    match split_position(entry) {
        Some((file, line, _)) => (file, line),
        None => {
            let digits = entry.bytes().take_while(u8::is_ascii_digit).count();
            (entry, entry[..digits].parse().unwrap_or(0))
        }
    }
}

/// Whether a line starts with the position of a script of RGSS3, `{<digits>}:<digits>:`.
fn starts_with_script_position(line: &str) -> bool {
    let Some((index, rest)) = line
        .strip_prefix('{')
        .and_then(|rest| rest.split_once("}:"))
    else {
        return false;
    };
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    digits(index) && rest.split_once(':').is_some_and(|(line, _)| digits(line))
}

/// The message of a `SyntaxError` as the report of RGSS3 cuts it: the position taken out of
/// the first line that has one, then the first line and the second unless that starts with
/// the position of a script, without the words ` syntax error, `, a leading space and a
/// line of nothing but `^`.
fn syntax_message(message: &str) -> String {
    let mut lines: Vec<String> = message.split('\n').map(str::to_owned).collect();
    if let Some(line) = lines.iter_mut().find(|line| split_position(line).is_some()) {
        *line = split_position(line).map_or_else(String::new, |(_, _, rest)| rest.to_owned());
    }
    let mut text = lines.first().cloned().unwrap_or_default();
    if let Some(second) = lines
        .get(1)
        .filter(|line| !starts_with_script_position(line))
    {
        text.push('\n');
        text.push_str(second);
    }
    for prefix in [" syntax error, ", " "] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = rest.to_owned();
        }
    }
    match text.split_once('\n') {
        Some((first, second)) if second.trim() == "^" => first.to_owned(),
        _ => text,
    }
}

/// The report of an uncaught exception in a game of RGSS3, in the form measured on the
/// original runtime (`ace-v1-api-REPORT.md` section 1.2): the position is that of the first
/// backtrace entry that does not start with `(eval)`, or the last entry; a `SyntaxError`
/// without a line there takes it from its message; a script is named by
/// `$RGSS_SCRIPTS[i][1]` as it is now. Not followed: the empty box for a text longer than
/// 1023 UTF-16 units.
fn report_rgss3(ruby: &Ruby, error: &RubyError) -> String {
    let class = error.class_name.as_str();
    let entry = error
        .backtrace
        .iter()
        .position(|entry| !entry.starts_with("(eval)"))
        .or_else(|| error.backtrace.len().checked_sub(1))
        .map_or("", |index| error.backtrace[index].as_str());
    let (mut name, mut line) = entry_position(entry);
    let mut message = error.message.clone();
    if class == "SyntaxError" {
        // The frame of the call of `eval` itself, `ruby:in `eval'` as on the original,
        // has no line.
        if name.contains(":in `") || line == 0 {
            let first = error.message.lines().find_map(split_position);
            (name, line) = first.map_or((name, line), |(file, number, _)| (file, number));
        }
        message = syntax_message(&error.message);
    } else if class == "Errno::ENOENT" {
        let file = message
            .strip_prefix("No such file or directory - ")
            .unwrap_or(&message);
        return format!("Unable to find file:\n\n{file}");
    }
    let name = match script_index_rgss3(name) {
        Some(index) => script_name(ruby, index).unwrap_or_else(|| name.to_owned()),
        None => name.to_owned(),
    };
    let mut text = if line != 0 {
        format!("Script '{name}' line {line}: {class} occurred.")
    } else {
        format!("{class} occurred.")
    };
    if !message.is_empty() {
        text.push_str("\n\n");
        text.push_str(&message);
    }
    text
}

/// The report of an uncaught exception in a game of the guest's runtime.
fn report_of_runtime(ruby: &Ruby, error: &RubyError) -> String {
    if Guest::with(|guest| guest.runtime.run.runtime) == Runtime::Rgss3 {
        report_rgss3(ruby, error)
    } else {
        report(ruby, error)
    }
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
    let runtime = Guest::with(|guest| guest.runtime.run.runtime);
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
    let count = unsafe { ruby.funcall(ruby.top_self(), c"vkxp_load_scripts", &[path]) };
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
    let runtime = Guest::with(|guest| {
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
    Guest::with(|guest| {
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
fn run_scripts(ruby: &Ruby, preload: &[Preload], count: usize) -> Result<(), RunError> {
    let runtime = Guest::with(|guest| guest.runtime.run.runtime);
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

/// The modal message of a game of RGSS3 whose script archive cannot be loaded, measured on
/// the original runtime (`ace-v1-api-REPORT.md` section 1.1). The error that names the
/// archive is the caller's to write.
const FAILED_TO_LOAD_SCRIPT: &str = "Failed to load script.";

/// Plays the game of the installed guest's [`RunConfig`] over `link`: everything of
/// [`run_game`] after the installation of the guest.
pub(crate) fn play(ruby: &Ruby, link: GuestEnd) -> Result<(), RunError> {
    Guest::with(|guest| {
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
    let shown = match &outcome {
        Err(RunError::Script(report)) => Some(report.clone()),
        Err(RunError::Archive { .. })
            if Guest::with(|guest| guest.runtime.run.runtime) == Runtime::Rgss3 =>
        {
            Some(FAILED_TO_LOAD_SCRIPT.to_owned())
        }
        _ => None,
    };
    if let Some(text) = shown {
        eprintln!("{text}");
        Guest::with(|guest| guest.show_message(text));
    }
    Guest::with(|guest| {
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
/// Panics when a guest is already installed on this thread.
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
