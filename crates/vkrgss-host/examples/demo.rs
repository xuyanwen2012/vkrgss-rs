//! The demo guest in a window.
//!
//! ```sh
//! cargo run -p vkrgss-host --example demo -- <device>
//! ```
//!
//! `<device>` is the index of a Vulkan device or a part of its name, as
//! `vulkaninfo --summary` prints them. It has no default: the caller says which GPU drives
//! the window.
//!
//! Arrow keys move the sprite (faster with Shift), Space, Enter or C tint the middle of the
//! screen, Escape or X swaps the background with a transition. F12 resets, F2 shows the frame
//! rate in the title, Tab held fast-forwards, Alt+Enter toggles fullscreen.

use std::process::ExitCode;

use vkrgss_host::demo::{self, DemoOptions};
use vkrgss_host::{WindowedOptions, device_from_text, run_windowed};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [device] = arguments.as_slice() else {
        eprintln!("usage: demo <device>");
        eprintln!("  <device>  index of a Vulkan device, or a part of its name");
        return ExitCode::from(2);
    };

    let options = WindowedOptions::new(device_from_text(device));
    let run = run_windowed(options, |link| {
        // The host going away is how a session that failed on its side ends for the guest.
        let _ = demo::run(&link, DemoOptions::default());
    });
    match run {
        Ok(end) if end.is_success() => ExitCode::SUCCESS,
        Ok(end) => {
            eprintln!("demo: the session ended without a shutdown: {end:?}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("demo: {error}");
            ExitCode::FAILURE
        }
    }
}
