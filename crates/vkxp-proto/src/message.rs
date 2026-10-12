//! The two message types that cross the guest/host boundary.
//!
//! # Pacing handshake
//!
//! The host answers every [`GuestToHost::Frame`] with exactly one [`HostToGuest::Tick`], sent
//! when the time of that frame has passed. The guest does not send its next `Frame` before it
//! has received that `Tick`. A `Tick` carries no data, so what the guest does never depends on
//! wall-clock time, only on the number of ticks it has seen.
//!
//! # States
//!
//! A guest in a process of its own can be saved: it leaves a copy of its process behind, which
//! is parked until it is told to resume. Every guest process opens its connection with
//! [`GuestToHost::Attach`]. Whatever stands between the host and the guest processes keeps
//! the parked copies by slot; it takes [`HostToGuest::LoadState`] for itself, sends
//! [`HostToGuest::Resume`] to a parked copy, keeps [`GuestToHost::Outcome`], and makes the
//! answers [`GuestToHost::StateSaved`] and [`GuestToHost::StateLoaded`].

use serde::{Deserialize, Serialize};

use crate::{AudioCommand, BitmapId, FramePacket, InputSnapshot};

/// The rule by which the frames of a transition are stepped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionRule {
    /// RGSS1: the frames of a transition of N show the steps 0 to N-1, the first of which is
    /// the held frame alone.
    Rgss1,
    /// RGSS2: the frames of a transition of N show the steps 1 to N, the last of which is
    /// the new frame alone.
    Rgss2,
}

/// A message from the guest (script thread) to the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuestToHost {
    /// A finished frame to composite and present. The host answers with one
    /// [`HostToGuest::Tick`]; see the module documentation.
    Frame(FramePacket),
    /// Keep showing the current frame until the next transition.
    Freeze,
    /// Cross-fade from the frozen frame to the next one.
    Transition {
        /// Length in frames.
        duration: u32,
        /// Bitmap whose brightness orders the fade, if any.
        mask: Option<BitmapId>,
        /// Softness of the boundary when a mask is used.
        vague: u16,
        /// How the frames of the transition are stepped.
        rule: TransitionRule,
    },
    /// An audio request.
    Audio(AudioCommand),
    /// Show a blocking message panel. The guest waits for `HostToGuest::ModalDismissed`.
    Modal { text: String },
    /// Set the window title.
    SetTitle { title: String },
    /// The guest has ended.
    Shutdown,
    /// `Graphics.frame_reset`: restart the pacing schedule from now. The host times the next
    /// frame from the moment it handles this message instead of catching up on time the guest
    /// spent elsewhere, for example loading.
    FrameReset,
    /// The first message of a guest process on its connection.
    Attach {
        /// The id of the process.
        pid: u32,
        /// The slot whose state this process is: a copy that waits to be resumed. `None`
        /// for a process that runs.
        parked: Option<u8>,
    },
    /// How the game went, sent by a guest whose game has ended, before its
    /// [`GuestToHost::Shutdown`].
    Outcome {
        /// Whether the game ended without a failure.
        ok: bool,
    },
    /// A state was saved into `slot`.
    StateSaved { slot: u8 },
    /// The state of `slot` was loaded: what the guest sends from here on comes from it, and
    /// the host holds nothing of it yet.
    StateLoaded { slot: u8 },
    /// A state could not be saved into `slot` or loaded from it.
    StateFailed {
        slot: u8,
        /// Whether a load failed; false for a save.
        load: bool,
    },
    /// A key is pressed or released on the player's behalf, as `keybd_event` does. The host
    /// treats it as the physical key of that code.
    InjectKey {
        /// The Windows virtual-key code.
        key: u8,
        /// Whether the key goes down; false for a release.
        down: bool,
    },
    /// Whether the pointer is to be seen over the picture.
    PointerVisible { visible: bool },
}

/// A message from the host to the guest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostToGuest {
    /// The button state for the next `Input.update`.
    Input(InputSnapshot),
    /// The user closed the message panel.
    ModalDismissed,
    /// The time of one frame has passed. Sent exactly once for every [`GuestToHost::Frame`];
    /// see the module documentation.
    Tick,
    /// The user asked to close the game. The guest shuts down and sends
    /// [`GuestToHost::Shutdown`].
    Quit,
    /// The user asked to restart the game (F12). The guest restarts at its next frame boundary
    /// and keeps running; unlike [`HostToGuest::Quit`] it does not shut down.
    Reset,
    /// Save the game as it is at a frame boundary into `slot`.
    SaveState { slot: u8 },
    /// Go back to the state of `slot`. It is answered with [`GuestToHost::StateLoaded`] or
    /// [`GuestToHost::StateFailed`] and never reaches a guest that can be saved.
    LoadState { slot: u8 },
    /// To a parked copy: go on from where the copy was made.
    Resume,
}
