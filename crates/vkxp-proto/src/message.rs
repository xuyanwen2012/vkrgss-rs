//! The two message types that cross the guest/host boundary.
//!
//! # Pacing handshake
//!
//! The host answers every [`GuestToHost::Frame`] with exactly one [`HostToGuest::Tick`], sent
//! when the time of that frame has passed. The guest does not send its next `Frame` before it
//! has received that `Tick`. A `Tick` carries no data, so what the guest does never depends on
//! wall-clock time, only on the number of ticks it has seen.

use serde::{Deserialize, Serialize};

use crate::{AudioCommand, BitmapId, FramePacket, InputSnapshot};

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
}
