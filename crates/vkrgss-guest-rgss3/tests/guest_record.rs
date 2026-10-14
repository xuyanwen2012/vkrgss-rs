//! The built `vkrgss-guest-rgss3` executable started without a record it can use: the
//! scenarios "Refused records of the guest executable", "Not started as a guest" and "The
//! guest executable of RGSS3 given a game of XP" of the requirement "An executable runs the
//! runtimes of its interpreter" of `game-launch`.
//!
//! The executable ends before it opens a game file, starts an interpreter or connects, so
//! these tests need neither a program that listens nor a Vulkan driver.
#![cfg(not(vkrgss_interpreter_absent))]

use std::path::Path;
use std::process::{Command, Output};

use vkrgss_guest::record::{Diagnostics, GuestRecord};
use vkrgss_guest::role::{GUEST_ROLE, GuestRole};
use vkrgss_guest::settings::GuestSettings;

/// The version of the executable.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Runs the executable with these arguments in `work`.
fn guest(work: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vkrgss-guest-rgss3"))
        .args(arguments)
        .current_dir(work)
        .output()
        .expect("the executable starts")
}

/// The error stream of a run that was refused with a status that is not 0.
#[track_caller]
fn refusal(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(!output.status.success(), "{stderr}");
    assert!(output.stdout.is_empty());
    stderr
}

/// A record of a game in `dir`, whose socket nobody listens on.
fn record(dir: &Path) -> GuestRecord {
    GuestRecord {
        role: GuestRole {
            seed: 7,
            socket: dir.join("guest.sock"),
        },
        game_dir: dir.to_owned(),
        settings: GuestSettings::default(),
        diagnostics: Diagnostics::default(),
    }
}

#[test]
fn a_record_that_does_not_exist_is_refused_naming_its_path() {
    let work = tempfile::tempdir().unwrap();
    let absent = work.path().join("absent.record");
    let stderr = refusal(&guest(work.path(), &[GUEST_ROLE, absent.to_str().unwrap()]));
    assert!(stderr.contains(absent.to_str().unwrap()), "{stderr}");
}

#[test]
fn a_record_of_another_version_is_refused_naming_both_versions() {
    let work = tempfile::tempdir().unwrap();
    let file = work.path().join("guest.record");
    record(work.path()).write(&file, "0.0.0").unwrap();
    let stderr = refusal(&guest(work.path(), &[GUEST_ROLE, file.to_str().unwrap()]));
    assert!(stderr.contains("0.0.0"), "{stderr}");
    assert!(stderr.contains(VERSION), "{stderr}");
    assert!(stderr.contains(file.to_str().unwrap()), "{stderr}");
}

#[test]
fn a_file_that_is_no_record_is_refused_naming_it() {
    let work = tempfile::tempdir().unwrap();
    let file = work.path().join("notes.record");
    let written = format!("vkrgss-guest-record {VERSION}\nnot a record at all");
    std::fs::write(&file, written).unwrap();
    let stderr = refusal(&guest(work.path(), &[GUEST_ROLE, file.to_str().unwrap()]));
    assert!(stderr.contains(file.to_str().unwrap()), "{stderr}");
}

#[test]
fn a_game_of_xp_is_refused_before_the_socket_is_tried() {
    let work = tempfile::tempdir().unwrap();
    std::fs::write(
        work.path().join("Game.ini"),
        "[Game]\r\nLibrary=RGSS104E.dll\r\nScripts=Data\\Scripts.rxdata\r\n",
    )
    .unwrap();
    let builtin = work.path().join("rpg.rb");
    std::fs::write(&builtin, "module RPG\nend\n").unwrap();
    let mut record = record(work.path());
    record.settings.builtin_scripts = Some(builtin);
    let file = work.path().join("guest.record");
    record.write(&file, VERSION).unwrap();
    let stderr = refusal(&guest(work.path(), &[GUEST_ROLE, file.to_str().unwrap()]));
    assert!(stderr.contains("RPG Maker XP"), "{stderr}");
    assert!(!stderr.contains("guest.sock"), "{stderr}");
}

#[test]
fn without_the_option_of_the_guest_role_it_says_what_it_is() {
    let work = tempfile::tempdir().unwrap();
    for arguments in [&[][..], &["--help"], &["--version"]] {
        let output = guest(work.path(), arguments);
        let stderr = refusal(&output);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}: {stderr}");
        assert_eq!(stderr.lines().count(), 1, "{stderr}");
        assert!(stderr.contains("`vkrgss "), "{stderr}");
    }
}
