//! The guest executable of vkrgss for games of RPG Maker XP and VX.
//!
//! Side: guest.
//!
//! The scripts of a game of RGSS1 or RGSS2 run over Ruby 1.8.7. `vkrgss` starts this
//! executable, which lies beside it, as the guest process of such a game, so that the
//! process that save states copy with `fork` links nothing of the host: no window
//! framework, no Vulkan, no audio. It is the guest role and the main of a guest
//! (`vkrgss-guest-main`) and nothing else. It reads no option: started without the hidden
//! option of the guest role, it says what it is and ends.

use std::process::ExitCode;

fn main() -> ExitCode {
    vkrgss_guest_main::guest_process(env!("CARGO_PKG_VERSION")).unwrap_or_else(not_a_guest)
}

/// What an executable started by anything but `vkrgss` says, with status 2.
fn not_a_guest() -> ExitCode {
    eprintln!(
        "vkrgss-guest-rgss12: this is the guest executable of vkrgss for games of RPG Maker XP \
         and VX; `vkrgss <game directory>` starts it"
    );
    ExitCode::from(2)
}
