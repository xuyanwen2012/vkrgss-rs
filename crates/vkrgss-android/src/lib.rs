//! The Android app: the native library of an APK whose activity is Android's
//! `NativeActivity`, through winit's Android backend (`android-activity`). There is no Java
//! code.
//!
//! Side: platform.
//!
//! `android_main` (`app`) runs a game as `vkrgss --guest thread` does in a window:
//!
//! 1. The standard output and the error stream are made to lead to logcat (tag `vkrgss`) and
//!    to the file `vkrgss.log` of the app's external files directory (`log`), and the
//!    subscriber of the diagnostics is installed at the level info.
//! 2. The game is the directory `game` of that directory
//!    (`/sdcard/Android/data/<package>/files/game/`), which `adb push` can fill; its
//!    `vkrgss.toml` gives the options of the run ([`options`]). The built-in scripts file is
//!    by default the file of the game's runtime in the files directory
//!    (`files/rpg_builtin.rb` for RPG Maker XP), and the system's fonts in `/system/fonts` are
//!    searched after the font directories of the file.
//! 3. A game of RGSS3 is refused ([`refusal`]): its scripts need Ruby 1.9.2, which runs in a
//!    process of its own, and the guest on Android is a thread of the app for now.
//! 4. The interpreter thread runs the game's scripts and the host runs on the thread of
//!    `android_main` with the window of the activity (`run_windowed_on_android` of
//!    `vkrgss-host`), with sound through cpal's AAudio backend unless the file says
//!    `audio = false`. When the activity is paused the surface goes and the game waits;
//!    when it resumes the game goes on. An input script of the file holds the buttons, frame
//!    by frame ([`input`]), as a headless run's does.
//! 5. When the game has ended the process exits, so that the next start of the activity
//!    starts a new interpreter.
//!
//! Save states are off: they need the guest in a process.

pub mod input;
pub mod options;

#[cfg(target_os = "android")]
mod app;
#[cfg(target_os = "android")]
mod log;

use vkrgss_vfs::Runtime;

/// What the app says of a game of `runtime` that it does not run, or `None` for a runtime it
/// runs.
pub fn refusal(runtime: Runtime) -> Option<String> {
    match runtime {
        Runtime::Rgss1 | Runtime::Rgss2 => None,
        Runtime::Rgss3 => Some(format!(
            "a game of {} ({}) is not supported on Android yet: its scripts need Ruby 1.9.2 \
             in a guest process; games of RPG Maker XP and RPG Maker VX run",
            runtime.product(),
            runtime.name()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scenario "Game of RGSS3": the message.
    #[test]
    fn a_game_of_rgss3_is_refused_with_a_message_naming_its_runtime() {
        assert_eq!(refusal(Runtime::Rgss1), None);
        assert_eq!(refusal(Runtime::Rgss2), None);
        let message = refusal(Runtime::Rgss3).unwrap();
        assert!(message.contains("RPG Maker VX Ace"), "{message}");
        assert!(
            message.contains("not supported on Android yet"),
            "{message}"
        );
    }
}
