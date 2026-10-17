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
//!    recording display (see `testing`).
//! 2. The runners drive a session: one in a window on `winit` with the real clock, one headless
//!    on a virtual clock with scripted input and frame dumps. The windowed one reads the
//!    gamepads ([`Gamepads`]); a headless run reads none. Either can record the input of
//!    every frame into a file or replay such a file ([`Recorder`], [`Replayer`]). A headless
//!    run can ask for states to be saved and loaded at given frames ([`StateSchedule`]),
//!    which a guest in a process of its own answers.
//! 3. The [`Display`] implementations: the Vulkan presenter for a window, an off-screen one for
//!    headless runs, and the recording fake.
//!
//! Audio commands go to the session's [`AudioSink`]: the output of `vkrgss-audio` in a window
//! unless audio is disabled, [`NullSink`] otherwise and in every headless run.
//!
//! `demo` is a guest written in Rust, used by the tests and the examples until the Ruby side
//! exists. It and `testing` are compiled only for tests and with the feature `testing`, which
//! the executable does not enable.

#![forbid(unsafe_code)]

mod audio;
mod clock;
mod display;
mod gamepad;
mod headless;
mod keys;
mod overlay;
mod raw;
mod record;
mod report;
mod script;
mod session;
mod states;
mod stats;
mod windowed;

#[cfg(any(test, feature = "testing"))]
pub mod demo;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use audio::{AudioOptions, AudioSink, NullSink};
pub use clock::{Clock, ManualClock, SystemClock};
pub use display::Display;
pub use gamepad::{DEAD_ZONE, GamepadMap, Gamepads, PadButton, PadState};
pub use headless::{
    HeadlessDisplay, HeadlessEnd, HeadlessError, HeadlessOptions, HeadlessReport, HostLink,
    default_device, headless_link, run_headless,
};
pub use keys::{KeyDefaults, KeyMap, MapError};
pub use overlay::{Overlay, OverlayAction};
pub use raw::{frame_pointer, mouse_key, virtual_key};
pub use record::{InputSource, Recorder, Recording, RecordingError, Replayer};
pub use report::{Reporter, Severity};
pub use script::{InputScript, ScriptError, button_name};
pub use session::{KeyAction, Session, SessionEnd, SessionOptions};
pub use states::{ScheduleError, StateRequest, StateSchedule};
pub use vkrgss_vk::{Filter, FrameTiming, OverlayImage};
#[cfg(target_os = "android")]
pub use windowed::run_windowed_on_android;
pub use windowed::{Driver, WindowedError, WindowedOptions, initial_window_size, run_windowed};

/// Reads a device from a command-line argument or an environment variable: a number is the
/// device's index in the enumeration order, anything else is a part of its name.
pub fn device_from_text(text: &str) -> vkrgss_vk::DeviceChoice {
    match text.parse() {
        Ok(index) => vkrgss_vk::DeviceChoice::Index(index),
        Err(_) => vkrgss_vk::DeviceChoice::Name(text.to_owned()),
    }
}
