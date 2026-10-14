//! A built-in scripts file of RGSS3 without `rgss_main`: the binding defines neither
//! `rgss_main` nor `rgss_stop`, so a game that calls one ends with a `NoMethodError`. A
//! binary of its own, because the interpreter of a test process keeps what an earlier
//! built-in scripts file defined.
#![cfg(not(vkxp_interpreter_absent))]

use vkxp_binding_rgss3::RunError;
use vkxp_binding_rgss3::testing;
use vkxp_binding_rgss3::testing::host::{self, Game, Script};
use vkxp_vfs::Runtime;

/// Plays `game` as `host::run_game` does, with the current directory of the process in the
/// game directory, where the glue of a guest puts it: `load_data` of RGSS3 looks for a loose
/// file from there. One game at a time enters its directory.
fn run_game_in(
    game: &Game,
    run: vkxp_binding_rgss3::RunConfig,
    script: Script,
    after: &str,
) -> host::Played {
    static PLAYING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _playing = PLAYING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::env::set_current_dir(game.dir()).expect("the game directory is entered");
    host::run_game(game, run, script, after)
}

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
    let played = run_game_in(&game, game.run_config(), Script::new(), "nil");
    match played.outcome {
        Err(RunError::Script(report)) => assert!(
            report.starts_with("Script 'Main' line 1: NoMethodError occurred."),
            "{report}"
        ),
        other => panic!("{other:?}"),
    }
}
