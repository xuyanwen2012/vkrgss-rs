//! The two message types that cross the guest/host boundary.

use serde::{Deserialize, Serialize};

use crate::{AudioCommand, BitmapId, FramePacket, InputSnapshot};

/// A message from the guest (script thread) to the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuestToHost {
    /// A finished frame to composite and present.
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
}

/// A message from the host to the guest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostToGuest {
    /// The button state for the next `Input.update`.
    Input(InputSnapshot),
    /// The user closed the message panel.
    ModalDismissed,
    /// One frame of time has passed.
    Tick,
    /// The user asked to close the game.
    Quit,
}
