//! The guest executable of vkxp for games of RPG Maker VX Ace.
//!
//! Side: guest.
//!
//! The scripts of a game of RGSS3 run over Ruby 1.9.2, which cannot be linked into one
//! program with the Ruby 1.8.7 of `vkxp`. So `vkxp` starts this executable, which lies
//! beside it, as the guest process of such a game. It is the guest role and the main of a
//! guest over the second interpreter (`vkxp-guest-main-rgss3`) and nothing of the host. It
//! reads no option: started without the hidden option of the guest role, it says what it is
//! and ends.

use std::process::ExitCode;

#[cfg(not(vkxp_interpreter_absent))]
fn main() -> ExitCode {
    vkxp_guest_main_rgss3::guest_process(env!("CARGO_PKG_VERSION")).unwrap_or_else(not_a_guest)
}

/// What an executable started by anything but `vkxp` says, with status 2.
#[cfg(not(vkxp_interpreter_absent))]
fn not_a_guest() -> ExitCode {
    eprintln!(
        "vkxp-guest-rgss3: this is the guest executable of vkxp for games of RPG Maker VX Ace; \
         `vkxp <game directory>` starts it"
    );
    ExitCode::from(2)
}

/// A target without Ruby 1.9.2.
#[cfg(vkxp_interpreter_absent)]
fn main() -> ExitCode {
    eprintln!(
        "vkxp-guest-rgss3: this executable is not built for this target: Ruby 1.9.2, which \
         games of RPG Maker VX Ace need, is built for x86_64-unknown-linux-gnu only"
    );
    ExitCode::FAILURE
}
