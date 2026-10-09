//! Audio commands sent from the guest to the host.

use serde::{Deserialize, Serialize};

/// The four playback channels of RGSS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AudioChannel {
    /// Background music: one looping track.
    Bgm,
    /// Background sound: one looping track.
    Bgs,
    /// Music effect: one track that pauses the background music while it plays.
    Me,
    /// Sound effects: any number at once.
    Se,
}

/// A request to the host's audio side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCommand {
    /// Starts playback on a channel.
    Play {
        channel: AudioChannel,
        /// The encoded audio file, read by the guest. The host never opens game files.
        data: Vec<u8>,
        /// 0 to 100.
        volume: u8,
        /// Playback speed in percent, 50 to 150.
        pitch: u16,
    },
    /// Stops a channel at once.
    Stop { channel: AudioChannel },
    /// Fades a channel out over a duration, then stops it.
    Fade {
        channel: AudioChannel,
        duration_ms: u32,
    },
}
