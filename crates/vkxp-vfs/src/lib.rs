//! Path resolution, RTP lookup, encrypted archive.
//!
//! Side: guest.

#![forbid(unsafe_code)]

mod dir;
mod error;
mod ini;
mod rtp;

pub use error::VfsError;
pub use ini::{Encoding, GameIni};
pub use rtp::resolve_rtps;
