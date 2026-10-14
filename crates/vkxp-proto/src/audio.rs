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
        /// Playback speed in percent, 1 to 65535.
        pitch: u16,
        /// The sample frame of the file at which the sound begins.
        start: u64,
        /// For a music effect: the time in milliseconds over which the music returns to
        /// its volume when the effect has ended. 0 for none, and for the other channels.
        music_return_ms: u32,
    },
    /// Stops a channel at once.
    Stop { channel: AudioChannel },
    /// Fades a channel out over a duration, then stops it.
    Fade {
        channel: AudioChannel,
        duration_ms: u32,
    },
    /// Sets the volume, 0 to 100, of what a channel plays, without starting it again.
    Volume { channel: AudioChannel, volume: u8 },
}
