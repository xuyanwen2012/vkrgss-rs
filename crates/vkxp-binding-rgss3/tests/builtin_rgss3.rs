//! A built-in scripts file of RGSS3 without `rgss_main`: the binding defines neither
//! `rgss_main` nor `rgss_stop`, so a game that calls one ends with a `NoMethodError`. A
//! binary of its own, because the interpreter of a test process keeps what an earlier
//! built-in scripts file defined.
#![cfg(not(vkxp_interpreter_absent))]

use vkxp_binding_rgss3::RunError;
use vkxp_binding_rgss3::testing;
use vkxp_binding_rgss3::testing::host::{self, Game, Script};
use vkxp_vfs::Runtime;

#[test]
fn a_file_without_the_functions_ends_the_game_at_their_call() {
    testing::start_as(Runtime::Rgss3);
    let game = Game::of(
        Runtime::Rgss3,
        "without_main",
        "Ace",
        &[("Opening", "nil"), ("Main", "rgss_main { }")],
    );
    std::fs::write(game.builtin_scripts(), "module RPG\nend\n").unwrap();
    let played = host::run_game(&game, game.run_config(), Script::new(), "nil");
    match played.outcome {
        Err(RunError::Script(report)) => assert!(
            report.starts_with("Script 'Main' line 1: NoMethodError occurred."),
            "{report}"
        ),
        other => panic!("{other:?}"),
    }
}
