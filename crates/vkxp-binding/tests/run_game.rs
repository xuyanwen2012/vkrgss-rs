//! Running a game: the scenarios of `rgss-runtime` that need one. Each test makes a game
//! directory with a script archive of its own and plays it against a scripted host.
//!
//! The games of this binary run in one interpreter, one after another, so every test uses
//! global variables of its own.

use std::fs;

use vkxp_binding::testing::host::{self, Game, Played, Script};
use vkxp_binding::{RunConfig, RunError, testing};
use vkxp_proto::{AudioChannel, AudioCommand, GuestToHost, HostToGuest};

fn run(game: &Game, script: Script, after: &str) -> Played {
    host::run_game(game, game.run_config(), script, after)
}

fn stop(channel: AudioChannel) -> GuestToHost {
    GuestToHost::Audio(AudioCommand::Stop { channel })
}

/// What every game ends with: the four channels stop, then the shutdown message.
fn ending() -> Vec<GuestToHost> {
    vec![
        stop(AudioChannel::Bgm),
        stop(AudioChannel::Bgs),
        stop(AudioChannel::Me),
        stop(AudioChannel::Se),
        GuestToHost::Shutdown,
    ]
}

fn title(text: &str) -> GuestToHost {
    GuestToHost::SetTitle {
        title: text.to_owned(),
    }
}

/// The report of a run that failed on a script.
fn report(played: &Played) -> &str {
    match &played.outcome {
        Err(RunError::Script(report)) => report,
        other => panic!("{other:?} is not a script failure"),
    }
}

#[test]
fn the_first_script_sees_the_prepared_globals() {
    let game = Game::new(
        "globals",
        "Globals",
        &[("Main", "$vkxp_globals = [$KCODE, $:, $DEBUG, $BTEST]")],
    );
    let played = run(&game, Script::new(), "$vkxp_globals");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok(r#"["UTF8", [], false, false]"#));
}

#[test]
fn debug_and_battle_test_mode_are_the_callers() {
    let game = Game::new(
        "modes",
        "Modes",
        &[("Main", "$vkxp_modes = [$DEBUG, $BTEST]")],
    );
    let config = RunConfig {
        debug: true,
        battle_test: true,
        ..game.run_config()
    };
    let played = host::run_game(&game, config, Script::new(), "$vkxp_modes");
    assert_eq!(played.after.as_deref(), Ok("[true, true]"));
}

#[test]
fn the_title_reaches_the_host_before_the_first_frame() {
    let game = Game::new("title", "Demo", &[("Main", "Graphics.update")]);
    let played = run(&game, Script::new(), "nil");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.record.received[0], title("Demo"));
    assert!(matches!(played.record.received[1], GuestToHost::Frame(_)));
    assert_eq!(played.record.received[2..], ending());
}

#[test]
fn the_built_in_scripts_are_defined_before_the_game() {
    let game = Game::new(
        "builtin",
        "Builtin",
        &[(
            "Main",
            "$vkxp_builtin = [defined?(RPG::Cache), defined?(RPG::Sprite), defined?(RPG::Weather)]",
        )],
    );
    let played = run(&game, Script::new(), "$vkxp_builtin");
    assert_eq!(
        played.after.as_deref(),
        Ok(r#"["constant", "constant", "constant"]"#)
    );
}

#[test]
fn a_missing_built_in_scripts_file_fails_before_any_script() {
    let game = Game::new("no_builtin", "None", &[("Main", "$vkxp_no_builtin = :ran")]);
    let missing = game.dir().join("nowhere").join("rpg.rb");
    let config = RunConfig {
        builtin_scripts: missing.clone(),
        ..game.run_config()
    };
    let played = host::run_game(&game, config, Script::new(), "$vkxp_no_builtin");
    let error = played.outcome.expect_err("the run fails");
    assert!(
        matches!(&error, RunError::BuiltinScripts { path, .. } if *path == missing),
        "{error:?}"
    );
    let message = error.to_string();
    assert!(
        message.contains(&missing.display().to_string()),
        "{message}"
    );
    assert!(message.contains("extraction tool"), "{message}");
    assert_eq!(played.after.as_deref(), Ok("nil"));
    assert_eq!(played.record.received[0], title("None"));
    assert_eq!(played.record.received[1..], ending());
}

#[test]
fn an_error_in_the_built_in_scripts_is_reported_under_their_name() {
    let game = Game::new(
        "bad_builtin",
        "Bad",
        &[("Main", "$vkxp_bad_builtin = :ran")],
    );
    fs::write(game.builtin_scripts(), "module RPG\nend\nRPG.nothing\n").unwrap();
    let played = run(&game, Script::new(), "$vkxp_bad_builtin");
    assert!(
        report(&played).starts_with("Script '(builtin)' line 3: NoMethodError occurred."),
        "{}",
        report(&played)
    );
    assert_eq!(played.after.as_deref(), Ok("nil"));
}

#[test]
fn scripts_run_in_archive_order_under_their_section_names() {
    let first = "($vkxp_order ||= []) << 'One'";
    let game = Game::new(
        "order",
        "Order",
        &[
            ("One", first),
            ("Two", "$vkxp_order << 'Two'\n$vkxp_file = __FILE__"),
            (
                "Three",
                "$vkxp_order << 'Three'\n$vkxp_table = $RGSS_SCRIPTS.map { |script| script[1] } \
                 + [$RGSS_SCRIPTS[0][3], $RGSS_SCRIPTS[0].size, $RGSS_SCRIPTS[0][0]]",
            ),
        ],
    );
    let played = run(
        &game,
        Script::new(),
        "[$vkxp_order, $vkxp_file, $vkxp_table]",
    );
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(
        played.after,
        Ok(format!(
            r#"[["One", "Two", "Three"], "Section001", ["One", "Two", "Three", "{first}", 4, 0]]"#
        ))
    );
}

#[test]
fn a_missing_archive_fails_before_any_script() {
    let game = Game::new("no_archive", "None", &[("Main", "$vkxp_no_archive = :ran")]);
    fs::remove_file(game.dir().join("Data").join("Scripts.rxdata")).unwrap();
    let played = run(&game, Script::new(), "$vkxp_no_archive");
    let error = played.outcome.expect_err("the run fails");
    assert!(
        matches!(&error, RunError::Archive { path, .. } if path == "Data\\Scripts.rxdata"),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "cannot load the script archive Data\\Scripts.rxdata: \
         Errno::ENOENT: No such file or directory - Data\\Scripts.rxdata"
    );
    assert_eq!(played.after.as_deref(), Ok("nil"));
    assert_eq!(played.record.received[1..], ending());
}

#[test]
fn an_archive_that_is_not_one_fails() {
    let game = Game::new(
        "bad_archive",
        "Bad",
        &[("Main", "$vkxp_bad_archive = :ran")],
    );
    fs::write(game.dir().join("Data").join("Scripts.rxdata"), b"junk").unwrap();
    let played = run(&game, Script::new(), "$vkxp_bad_archive");
    assert!(
        matches!(played.outcome, Err(RunError::Archive { .. })),
        "{:?}",
        played.outcome
    );
    assert_eq!(played.after.as_deref(), Ok("nil"));
}

#[test]
fn the_game_ends_when_the_last_script_returns() {
    let game = Game::new(
        "finish",
        "Finish",
        &[("Main", "Audio.bgm_stop\n$vkxp_finish = :ran")],
    );
    let played = run(&game, Script::new(), "$vkxp_finish");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok(":ran"));
    let mut expected = vec![title("Finish"), stop(AudioChannel::Bgm)];
    expected.extend(ending());
    assert_eq!(played.record.received, expected);
}

#[test]
fn exit_ends_the_game_before_later_scripts() {
    let game = Game::new(
        "exit",
        "Exit",
        &[
            ("First", "$vkxp_exit = [:first]\nexit\n$vkxp_exit << :after"),
            ("Second", "$vkxp_exit << :second"),
        ],
    );
    let played = run(&game, Script::new(), "$vkxp_exit");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok("[:first]"));
    assert_eq!(played.record.received[1..], ending());
}

#[test]
fn an_uncaught_error_is_reported_with_script_name_and_line() {
    let game = Game::new(
        "error",
        "Error",
        &[
            ("Helpers", "$vkxp_error = []"),
            ("Scene_Title", "$vkxp_error << :title\n\nnil.frobnicate\n"),
            ("Main", "$vkxp_error << :main"),
        ],
    );
    let played = run(&game, Script::new(), "$vkxp_error");
    let report = report(&played).to_owned();
    assert_eq!(
        report,
        "Script 'Scene_Title' line 3: NoMethodError occurred.\n\n\
         undefined method `frobnicate' for nil:NilClass"
    );
    assert_eq!(played.after.as_deref(), Ok("[:title]"));
    // The host shows the report before it is told that the guest has ended.
    let mut expected = vec![title("Error"), GuestToHost::Modal { text: report }];
    expected.extend(ending());
    assert_eq!(played.record.received, expected);
}

#[test]
fn an_error_is_reported_where_it_was_raised() {
    let game = Game::new(
        "error_elsewhere",
        "Error",
        &[
            (
                "Game_Thing",
                "def vkxp_fail_here\n  raise ArgumentError, 'no'\nend",
            ),
            ("Main", "\n\n\nvkxp_fail_here"),
        ],
    );
    let played = run(&game, Script::new(), "nil");
    assert_eq!(
        report(&played),
        "Script 'Game_Thing' line 2: ArgumentError occurred.\n\nno"
    );
}

#[test]
fn a_script_that_does_not_parse_is_reported() {
    let game = Game::new(
        "syntax",
        "Syntax",
        &[
            ("Fine", "$vkxp_syntax = [:fine]"),
            ("Broken", "$vkxp_syntax << :broken\ndef broken(\n"),
            ("Main", "$vkxp_syntax << :main"),
        ],
    );
    let played = run(&game, Script::new(), "$vkxp_syntax");
    let report = report(&played);
    assert!(
        report.starts_with("Script 'Broken' line 2: SyntaxError occurred.\n\n"),
        "{report}"
    );
    assert_eq!(played.after.as_deref(), Ok("[:fine]"));
    assert!(played.record.received.contains(&GuestToHost::Modal {
        text: report.to_owned()
    }));
}

#[test]
fn a_reset_request_starts_the_scripts_over() {
    let game = Game::new(
        "reset",
        "Reset",
        &[
            ("Count", "$vkxp_reset_runs = ($vkxp_reset_runs || 0) + 1"),
            (
                "Main",
                "if $vkxp_reset_runs == 1
                   $vkxp_reset_bitmap = Bitmap.new(4, 4)
                   Audio.bgm_stop
                   loop { Graphics.update }
                 end
                 $vkxp_reset_seen = $vkxp_reset_bitmap.disposed?
                 Graphics.update",
            ),
        ],
    );
    let played = run(
        &game,
        Script::new().after_frame(2, HostToGuest::Reset),
        "[$vkxp_reset_runs, $vkxp_reset_seen]",
    );
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok("[2, true]"));
    assert_eq!(testing::bitmap_count(), 0);

    // Two frames, the reset with its audio stops, then the one frame of the second run.
    let received = &played.record.received;
    assert_eq!(received.len(), 14);
    assert_eq!(received[1], stop(AudioChannel::Bgm));
    assert!(matches!(received[2], GuestToHost::Frame(_)));
    assert!(matches!(received[3], GuestToHost::Frame(_)));
    assert_eq!(received[4..8], ending()[..4]);
    let GuestToHost::Frame(after_reset) = &received[8] else {
        panic!("{:?} is not a frame", received[8]);
    };
    assert!(after_reset.ops.is_empty(), "the scene is empty");
    assert_eq!(received[9..], ending());
}

#[test]
fn a_quit_request_ends_the_game() {
    let game = Game::new(
        "quit",
        "Quit",
        &[
            (
                "Main",
                "$vkxp_quit = 0\nloop { Graphics.update; $vkxp_quit += 1 }",
            ),
            ("Never", "$vkxp_quit = :later"),
        ],
    );
    let played = run(
        &game,
        Script::new().after_frame(3, HostToGuest::Quit),
        "$vkxp_quit",
    );
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok("2"));
    assert_eq!(played.record.frames().len(), 3);
    assert_eq!(played.record.received[4..], ending());
}
