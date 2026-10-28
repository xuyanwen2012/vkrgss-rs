//! The guest's side of the link to the host: what goes out, the pump that consumes what
//! comes in, the modal message, the copies of the guest for states, and the requests the
//! host made that are raised.

use std::ffi::CStr;
use std::fmt::Debug;

use vkrgss_core::InputState;
use vkrgss_core::split_deltas;
use vkrgss_proto::framing::BITMAPS_AHEAD_LEN;
use vkrgss_proto::link::GuestEnd;
use vkrgss_proto::{
    AudioChannel, AudioCommand, BitmapDelta, FramePacket, GuestToHost, HostToGuest,
};
use vkrgss_ruby_sys::ffi::{self, VALUE};
use vkrgss_ruby_sys::{Raise, Ruby};
use vkrgss_vfs::Runtime;

use crate::guest::Guest;
use crate::panel::Panel;
use crate::stats::{self, Part, Span};
use crate::{audio, rb};

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
        self.runtime.failure = None;
        self.runtime.music = None;
        self.runtime.background_sound = None;
        self.audio = audio::State::default();
        self.input = InputState::with_rules(self.input.rules());
    }

    /// Drops the link, which tells the host that the guest has gone.
    pub(crate) fn disconnect(&mut self) {
        self.runtime.link = None;
    }

    /// Extracts the next frame of the scene. Every op names a live bitmap, so the arena has no
    /// reason to fail; an error all the same fails the session: no packet is made, the game
    /// ends as on a quit request, and the run returns [`RunError::Extraction`].
    ///
    /// [`RunError::Extraction`]: super::RunError::Extraction
    pub(crate) fn extract(&mut self) -> Option<FramePacket> {
        match self.scene.extract(&mut self.bitmaps) {
            Ok(packet) => Some(packet),
            Err(error) => {
                self.runtime.failure = Some(error);
                self.runtime.pending = Some(Pending::Quit);
                None
            }
        }
    }

    /// Sends a frame packet to the host. When the pixels of its bitmap changes pass
    /// [`BITMAPS_AHEAD_LEN`], the changes go ahead of it in `Bitmaps` messages that carry at
    /// most that many bytes each, so that every message stays under the bound of the framing
    /// and every bitmap the arena accepts reaches the host.
    pub(crate) fn send_frame(&mut self, mut packet: FramePacket) {
        let pixels: usize = packet
            .bitmaps
            .iter()
            .map(|delta| match delta {
                BitmapDelta::Update { rgba, .. } => rgba.len(),
                BitmapDelta::Create { .. } | BitmapDelta::Destroy { .. } => 0,
            })
            .sum();
        if pixels > BITMAPS_AHEAD_LEN {
            let deltas = std::mem::take(&mut packet.bitmaps);
            split_deltas(deltas, BITMAPS_AHEAD_LEN, |part| {
                self.send(GuestToHost::Bitmaps(part));
            });
        }
        self.send(GuestToHost::Frame(packet));
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
        let packet = self.extract();
        self.scene.set_frame_count(frame_count);
        if let Some(packet) = packet {
            self.send_frame(packet);
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
        self.runtime.modal = true;
        self.send(GuestToHost::Modal { text });
        loop {
            let frame_count = self.scene.frame_count();
            let extraction = Span::enter(Part::Extraction);
            let packet = self.extract();
            self.scene.set_frame_count(frame_count);
            let Some(mut packet) = packet else {
                break;
            };
            if let Some(panel) = &panel {
                panel.bring_to_front(&mut packet.ops);
            }
            self.send_frame(packet);
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
pub(super) fn system_exit(_ruby: &Ruby) -> VALUE {
    // SAFETY: `&Ruby` proves the thread; the class globals are set once the interpreter runs.
    unsafe { ffi::rb_eSystemExit }
}

/// The class of the exception that resets a game of `runtime`: `Reset`, or `RGSSReset` in
/// RGSS3.
pub(super) fn reset_class_name(runtime: Runtime) -> &'static CStr {
    match runtime {
        Runtime::Rgss1 | Runtime::Rgss2 => c"Reset",
        Runtime::Rgss3 => c"RGSSReset",
    }
}

/// Raises what the host asked for while the guest pumped: `Reset` with the message `reset`
/// for a reset request, or in RGSS3 `RGSSReset` with an empty message, and `SystemExit` for
/// a quit request. Called after [`Guest::pump`] has returned and the guest is released.
pub(crate) fn raise_pending(ruby: &Ruby) -> Result<(), Raise> {
    let (pending, runtime) = Guest::with(ruby, |guest| {
        (guest.runtime.pending.take(), guest.runtime.run.runtime)
    });
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
