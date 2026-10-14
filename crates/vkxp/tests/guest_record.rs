//! The built `vkxp` executable given the hidden option of the guest role with a record it
//! cannot use: the scenarios of the requirement "Refused guest record" of `game-launch`.
//!
//! An executable refuses such a record before it opens a game or a connection, so these
//! tests need neither a game nor a Vulkan driver.

use std::path::Path;
use std::process::{Command, Output};

use vkxp_guest::record::{Diagnostics, GuestRecord};
use vkxp_guest::role::{GUEST_ROLE, GuestRole};
use vkxp_guest::settings::GuestSettings;

/// The version of the program.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Runs the executable with these arguments in `work`.
fn vkxp(work: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vkxp"))
        .args(arguments)
        .current_dir(work)
        .output()
        .expect("the program starts")
}

/// The error stream of a run that was refused with status 1: one line.
#[track_caller]
fn refusal(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(stderr.starts_with("vkxp: "), "{stderr}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
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

/// Scenario "No such file".
#[test]
fn a_record_that_does_not_exist_is_refused_naming_its_path() {
    let work = tempfile::tempdir().unwrap();
    let absent = work.path().join("absent.record");
    let stderr = refusal(&vkxp(work.path(), &[GUEST_ROLE, absent.to_str().unwrap()]));
    assert!(stderr.contains(absent.to_str().unwrap()), "{stderr}");
    assert!(stderr.contains("cannot read the guest record"), "{stderr}");

    // The option alone, or with more than the record, names nothing to read.
    let more = [GUEST_ROLE, absent.to_str().unwrap(), "--verbose"];
    for arguments in [&[GUEST_ROLE][..], &more] {
        let stderr = refusal(&vkxp(work.path(), arguments));
        assert!(stderr.contains(GUEST_ROLE), "{stderr}");
        assert!(stderr.contains("nothing else"), "{stderr}");
    }
}

/// Scenario "The value of version 0.2.0".
#[test]
fn the_value_of_an_earlier_version_is_refused_as_a_record_that_cannot_be_read() {
    let work = tempfile::tempdir().unwrap();
    let stderr = refusal(&vkxp(work.path(), &[GUEST_ROLE, "7=/tmp/guest.sock"]));
    assert!(
        stderr.contains("cannot read the guest record 7=/tmp/guest.sock"),
        "{stderr}"
    );
}

/// Scenario "Other version".
#[test]
fn a_record_of_another_version_is_refused_naming_both_versions() {
    let work = tempfile::tempdir().unwrap();
    let file = work.path().join("guest.record");
    record(work.path()).write(&file, "0.0.0").unwrap();
    let stderr = refusal(&vkxp(work.path(), &[GUEST_ROLE, file.to_str().unwrap()]));
    assert!(stderr.contains("0.0.0"), "{stderr}");
    assert!(stderr.contains(VERSION), "{stderr}");
    assert!(stderr.contains(file.to_str().unwrap()), "{stderr}");
}

/// Scenario "Not a record".
#[test]
fn a_file_that_is_no_record_is_refused_naming_it() {
    let work = tempfile::tempdir().unwrap();
    let file = work.path().join("notes.record");
    let written = format!("vkxp-guest-record {VERSION}\nnot a record at all");
    std::fs::write(&file, written).unwrap();
    let stderr = refusal(&vkxp(work.path(), &[GUEST_ROLE, file.to_str().unwrap()]));
    assert!(stderr.contains(file.to_str().unwrap()), "{stderr}");
    assert!(stderr.contains("is not a guest record"), "{stderr}");
}

/// Scenario "Among other arguments".
#[test]
fn the_option_after_another_argument_is_an_unknown_option() {
    let work = tempfile::tempdir().unwrap();
    let file = work.path().join("guest.record");
    record(work.path()).write(&file, VERSION).unwrap();
    let game = work.path().to_str().unwrap();
    let output = vkxp(work.path(), &[game, GUEST_ROLE, file.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains(&format!("unknown option `{GUEST_ROLE}`")),
        "{stderr}"
    );
    assert!(stderr.contains("usage: vkxp"), "{stderr}");
}

/// Scenario "The program given a record of VX Ace" of the requirement "An executable runs
/// the runtimes of its interpreter".
#[test]
fn a_game_of_vx_ace_is_refused_before_the_socket_is_tried() {
    let work = tempfile::tempdir().unwrap();
    std::fs::write(
        work.path().join("Game.ini"),
        "[Game]\r\nLibrary=System\\RGSS301.dll\r\nScripts=Data\\Scripts.rvdata2\r\n",
    )
    .unwrap();
    let builtin = work.path().join("rpg.rb");
    std::fs::write(&builtin, "module RPG\nend\n").unwrap();
    let mut record = record(work.path());
    record.settings.builtin_scripts = Some(builtin);
    let file = work.path().join("guest.record");
    record.write(&file, VERSION).unwrap();
    let stderr = refusal(&vkxp(work.path(), &[GUEST_ROLE, file.to_str().unwrap()]));
    assert!(stderr.contains("RPG Maker VX Ace"), "{stderr}");
    assert!(!stderr.contains("guest.sock"), "{stderr}");
}
