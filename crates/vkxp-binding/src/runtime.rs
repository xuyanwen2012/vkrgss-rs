//! The guest's side of the link, and the preparation of the interpreter for a game.
//!
//! The guest holds the [`GuestEnd`] of the link. Everything it tells the host goes through
//! [`Guest::send`], and everything the host tells it is consumed in one place,
//! [`Guest::pump`].

use std::path::PathBuf;

use vkxp_core::InputState;
use vkxp_proto::link::GuestEnd;
use vkxp_proto::{GuestToHost, HostToGuest};
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{Raise, Raised, Ruby, RubyError};
use vkxp_vfs::GameIni;

use crate::guest::Guest;
use crate::{audio, graphics, input, kernel, rb};

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
    /// The dismissal of the modal message just sent.
    Dismissal,
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
    /// An input snapshot is kept for the next `Input.update`. A reset or quit request is
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
                HostToGuest::ModalDismissed if wait == Await::Dismissal => return,
                HostToGuest::Tick | HostToGuest::ModalDismissed => {}
                HostToGuest::Reset => {
                    self.runtime.pending.get_or_insert(Pending::Reset);
                }
                HostToGuest::Quit => self.runtime.pending = Some(Pending::Quit),
            }
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
