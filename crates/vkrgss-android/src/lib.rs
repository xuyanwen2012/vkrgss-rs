//! The Android app: the native library of an APK whose activity is Android's
//! `NativeActivity`, through winit's Android backend (`android-activity`). There is no Java
//! code.
//!
//! Side: platform.
//!
//! `android_main` (`app`) runs a game as `vkrgss` does in a window, with the guest in a
//! process of its own:
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
//! 3. The guest process is started as `vkrgss` starts it (`process`, the file
//!    `process.rs` of `vkrgss` compiled into this crate): the guest executable of the game's
//!    runtime, which the APK installs in the app's native library directory beside this
//!    library as `libvkrgss_guest_rgss12.so` or `libvkrgss_guest_rgss3.so`, is started with
//!    its guest record, and its socket lies in the app's private files directory. An app
//!    cannot execute files of its data directories, but it can execute what the installer
//!    put in that directory. The guest confines itself with the seccomp filter; the kernels
//!    of the phones have no Landlock. Its lines reach logcat and the log file through the
//!    error stream it inherits.
//! 4. The host runs on the thread of `android_main` with the window of the activity
//!    (`run_windowed_on_android` of `vkrgss-host`), with sound through cpal's AAudio backend
//!    unless the file says `audio = false`, and the relay to the guest process on the guest
//!    thread. F3 and F4 of a keyboard save and load states, which are copies of the guest
//!    process, as on the desktop. When the activity is paused the surface goes and the game
//!    waits; when it resumes the game goes on. An input script of the file holds the
//!    buttons, frame by frame ([`input`]), as a headless run's does.
//! 5. When the game has ended, the relay ends the guest processes and the app's process
//!    exits, so that the next start of the activity starts afresh. When the app's process
//!    is ended from outside, the guest process ends with it (its parent-death signal) and
//!    the parked copies end when their connections do.
//!
//! The app does not start the interpreter itself: it links the crates of the guest for what
//! it checks of a game before the guest process starts, and the linker leaves the
//! interpreter out.

pub mod input;
pub mod options;

#[cfg(target_os = "android")]
mod app;
#[cfg(target_os = "android")]
mod log;
#[cfg(target_os = "android")]
#[path = "../../vkrgss/src/process.rs"]
mod process;

/// How long the app waits for the game's scripts to end after the host has.
#[cfg(target_os = "android")]
const GUEST_WAIT: std::time::Duration = std::time::Duration::from_secs(5);
