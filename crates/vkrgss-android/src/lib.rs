//! The Android app: the native library of an APK whose activities are subclasses of
//! Android's `NativeActivity`, through winit's Android backend (`android-activity`). A small
//! Java class, `java/io/github/xuyanwen2012/vkrgss/VkrgssActivity.java`, adds what the native
//! code cannot do alone: the system's file picker and the start of a game's activity; the
//! native code reaches it through JNI (`java`).
//!
//! Side: platform.
//!
//! `android_main` (`app`) runs in one of two activities, each in a process of its own:
//!
//! - `VkrgssActivity`, the library ([`library`], `library_app`): the screen of the imported
//!   games ([`screen`]), drawn by the app itself as a picture over an empty frame
//!   ([`canvas`]). "Import" and "Import RTP" open the picker for a zip, which is copied into
//!   the cache directory and unpacked into the app's private files directory ([`unpack`]);
//!   "Import built-in scripts" opens it for a built-in scripts file, which is stored there
//!   under the name of its runtime; a tap on the selected game starts it in the activity of
//!   games; the back key leaves.
//! - `VkrgssActivity$Game`, a game, whose directory the intent names, or the directory `game`
//!   of the app's external files directory when it names none (the game of tests, filled by
//!   `adb push`). It runs the game as `vkrgss` does in a window, with the guest in a process of
//!   its own and on-screen buttons over it ([`buttons`]), and ends with the game; the library
//!   comes back.
//!
//! A game runs in these steps:
//!
//! 1. The standard output and the error stream are made to lead to logcat (tag `vkrgss`) and
//!    to the file `vkrgss.log` of the app's external files directory (`log`; the library
//!    writes `library.log`), and the subscriber of the diagnostics is installed at the level
//!    info.
//! 2. The game directory's `vkrgss.toml` gives the options of the run ([`options`]). The
//!    run-time packages are those imported into the library, and those the file names. The
//!    built-in scripts file is by default the file of the game's runtime in the external files
//!    directory (`files/rpg_builtin.rb` for RPG Maker XP), which tests fill, or else the one
//!    imported into the library, and the system's fonts in `/system/fonts` are searched after
//!    the font directories of the file.
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
//!    thread. The on-screen buttons and the keys of gamepads hold the buttons of `Input`; F3
//!    and F4, on the screen or a keyboard, save and load states, which are copies of the guest
//!    process, as on the desktop. When the activity is paused the surface goes and the game
//!    waits, and the sound pauses; when it resumes the game and the sound go on. With the
//!    extra `validation` of the intent the compositor runs under the Vulkan validation
//!    layer, which the APK of `cargo xtask android-run --validation` carries, and what it
//!    reported is written to the log when the game has ended. An input script of the file holds the buttons,
//!    frame by frame ([`input`]), as a headless run's does.
//! 5. When the game has ended, the relay ends the guest processes, the activity is finished
//!    and its process exits, so that the next start of the activity starts afresh. When the
//!    app's process is ended from outside, the guest process ends with it (its parent-death
//!    signal) and the parked copies end when their connections do.
//!
//! The app does not start the interpreter itself: it links the crates of the guest for what
//! it checks of a game before the guest process starts, and the linker leaves the
//! interpreter out.

pub mod buttons;
pub mod canvas;
pub mod input;
pub mod library;
pub mod options;
pub mod screen;
pub mod unpack;

#[cfg(target_os = "android")]
mod app;
#[cfg(target_os = "android")]
mod java;
#[cfg(target_os = "android")]
mod library_app;
#[cfg(target_os = "android")]
mod log;
#[cfg(target_os = "android")]
#[path = "../../vkrgss/src/process.rs"]
mod process;

/// How long the app waits for the game's scripts to end after the host has.
#[cfg(target_os = "android")]
const GUEST_WAIT: std::time::Duration = std::time::Duration::from_secs(5);
