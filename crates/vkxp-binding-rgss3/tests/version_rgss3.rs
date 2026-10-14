//! `RGSS_VERSION` of a game of RGSS3 by the version of its DLL, `RunConfig::library_version`:
//! the scenarios "Versions, measured on the original", "The table of the decision" and
//! "Before this change" of "Version of RGSS3 by the library of the game" of `rgss-runtime`,
//! through runs of a game against a scripted host as in `run_game_rgss3.rs`. The original
//! runtimes 3.0.0.1 and 3.0.1.1 state their own versions (2026-10-13, kept outside the
//! repository).
#![cfg(not(vkxp_interpreter_absent))]

use vkxp_binding_rgss3::testing;
use vkxp_binding_rgss3::testing::host::{self, Game, Script};
use vkxp_vfs::{LibraryVersion, Runtime};

/// `[RGSS_VERSION, RGSS_VERSION.encoding.name]` in a game of RGSS3 named `name` whose DLL
/// has `version`.
fn version_of(name: &str, version: Option<[u16; 4]>) -> String {
    testing::start_as(Runtime::Rgss3);
    let game = Game::of(
        Runtime::Rgss3,
        name,
        "Ace",
        &[(
            "Main",
            "$vkxp_version = [RGSS_VERSION, RGSS_VERSION.encoding.name]",
        )],
    );
    let mut run = game.run_config();
    run.library_version = version.map(LibraryVersion);
    let played = host::run_game(&game, run, Script::new(), "$vkxp_version");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    played.after.expect("the version is read")
}

/// Scenario "Versions, measured on the original".
#[test]
fn the_version_is_that_of_the_dll() {
    assert_eq!(
        version_of("version_3001", Some([3, 0, 0, 1])),
        r#"["3.0.0", "ASCII-8BIT"]"#
    );
    assert_eq!(
        version_of("version_3011", Some([3, 0, 1, 1])),
        r#"["3.0.1", "ASCII-8BIT"]"#
    );
}

/// Scenario "The table of the decision": other versions and none, each run defining the
/// constant once. What the runs write to `$stderr` is kept; a constant defined twice on
/// purpose shows that a warning would be among it.
#[test]
fn every_other_version_is_3_0_1_and_the_constant_is_defined_once() {
    testing::start_as(Runtime::Rgss3);
    testing::eval(
        "$vkxp_stderr = $stderr
         $vkxp_written = ''
         capture = Object.new
         def capture.write(text)
           $vkxp_written << text.to_s
           text.to_s.size
         end
         $stderr = capture",
    )
    .expect("the error stream is captured");
    let versions = [
        version_of("version_3007", Some([3, 0, 0, 7])),
        version_of("version_2021", Some([2, 0, 2, 1])),
        version_of("version_none", None),
    ];
    let written = testing::eval(
        "VKXP_TWICE = 1
         VKXP_TWICE = 2
         $stderr = $vkxp_stderr
         $vkxp_written",
    )
    .expect("the error stream is put back");
    assert_eq!(
        versions,
        [
            r#"["3.0.0", "ASCII-8BIT"]"#,
            r#"["3.0.1", "ASCII-8BIT"]"#,
            r#"["3.0.1", "ASCII-8BIT"]"#,
        ]
    );
    assert!(written.contains("VKXP_TWICE"), "{written}");
    assert!(!written.contains("RGSS_VERSION"), "{written}");
}
