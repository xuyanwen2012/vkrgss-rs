//! The guest executable of vkrgss for games of RPG Maker VX Ace.
//!
//! Side: guest.
//!
//! The scripts of a game of RGSS3 run over Ruby 1.9.2, which cannot be linked into one
//! program with the Ruby 1.8.7 of `vkrgss`. So `vkrgss` starts this executable, which lies
//! beside it, as the guest process of such a game. It is the guest role and the main of a
//! guest over the second interpreter (`vkrgss-guest-main-rgss3`) and nothing of the host. It
//! reads no option: started without the hidden option of the guest role, it says what it is
//! and ends. `vkrgss --script` replaces itself with this executable for a script of RGSS3,
//! with the hidden option of script mode (`script_process`), which it runs instead.

use std::process::ExitCode;

#[cfg(not(vkrgss_interpreter_absent))]
fn main() -> ExitCode {
    vkrgss_guest_main_rgss3::script_process()
        .or_else(|| vkrgss_guest_main_rgss3::guest_process(env!("CARGO_PKG_VERSION")))
        .unwrap_or_else(not_a_guest)
}

/// What an executable started by anything but `vkrgss` says, with status 2.
#[cfg(not(vkrgss_interpreter_absent))]
fn not_a_guest() -> ExitCode {
    eprintln!(
        "vkrgss-guest-rgss3: this is the guest executable of vkrgss for games of RPG Maker VX Ace; \
         `vkrgss <game directory>` starts it"
    );
    ExitCode::from(2)
}

/// A target without Ruby 1.9.2.
#[cfg(vkrgss_interpreter_absent)]
fn main() -> ExitCode {
    eprintln!(
        "vkrgss-guest-rgss3: this executable is not built for this target: Ruby 1.9.2, which \
         games of RPG Maker VX Ace need, is built for x86_64-unknown-linux-gnu, \
         aarch64-unknown-linux-gnu, aarch64-apple-darwin and aarch64-linux-android only"
    );
    ExitCode::FAILURE
}
