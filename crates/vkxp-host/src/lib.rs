//! Event loop, input mapping, thread orchestration, configuration.
//!
//! Side: host.
//!
//! The crate has three layers:
//!
//! 1. [`Session`] is the logic of a running game seen from the host: it consumes the guest's
//!    messages and the user's input and produces host messages, the time of the next wake-up
//!    and requests to a [`Display`]. It holds no window, thread or GPU, and reads time from a
//!    [`Clock`], so every pacing, input and transition rule is tested with a fake clock and a
//!    recording display (see [`testing`]).
//! 2. The runners drive a session: one in a window on `winit` with the real clock, one headless
//!    on a virtual clock with scripted input and frame dumps.
//! 3. The [`Display`] implementations: the Vulkan presenter for a window, an off-screen one for
//!    headless runs, and the recording fake.

#![forbid(unsafe_code)]

mod clock;
mod display;
mod keys;
mod session;

pub mod testing;

pub use clock::{Clock, ManualClock, SystemClock};
pub use display::Display;
pub use keys::KeyMap;
pub use session::{KeyAction, Session, SessionEnd, SessionOptions};
