//! Running a game: the scenarios of `rgss-runtime` that need one. Each test makes a game
//! directory with a script archive of its own and plays it against a scripted host.
//!
//! The games of this binary run in one interpreter, one after another, so every test uses
//! global variables of its own.

use std::fs;

use vkrgss_binding::testing::host::{self, Game, Played, Script};
use vkrgss_binding::testing::{self, Harness};
use vkrgss_binding::{RunConfig, RunError};
use vkrgss_proto::{AudioChannel, AudioCommand, GuestToHost, HostToGuest, Size};
use vkrgss_vfs::Runtime;

/// The harness, started as a game of RGSS1.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss1)
}

fn run(game: &Game, script: Script, after: &str) -> Played {
    host::run_game(harness(), game, game.run_config(), script, after)
}

fn stop(channel: AudioChannel) -> GuestToHost {
    GuestToHost::Audio(AudioCommand::Stop { channel })
}

/// What every game ends with: the four channels stop, then the outcome and the shutdown
/// message.
fn ending_of(ok: bool) -> Vec<GuestToHost> {
    vec![
        stop(AudioChannel::Bgm),
        stop(AudioChannel::Bgs),
        stop(AudioChannel::Me),
        stop(AudioChannel::Se),
        GuestToHost::Outcome { ok },
        GuestToHost::Shutdown,
    ]
}

/// The ending of a game that did not fail.
fn ending() -> Vec<GuestToHost> {
    ending_of(true)
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
    vkrgss_binding::pending_over_ruby_1_9_2!();
    let game = Game::new(
        harness(),
        "globals",
        "Globals",
        &[("Main", "$vkrgss_globals = [$KCODE, $:, $DEBUG, $BTEST]")],
    );
    let played = run(&game, Script::new(), "$vkrgss_globals");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok(r#"["UTF8", [], false, false]"#));
}

#[test]
fn debug_and_battle_test_mode_are_the_callers() {
    let game = Game::new(
        harness(),
        "modes",
        "Modes",
        &[("Main", "$vkrgss_modes = [$DEBUG, $BTEST]")],
    );
    let config = RunConfig {
        debug: true,
        battle_test: true,
        ..game.run_config()
    };
    let played = host::run_game(harness(), &game, config, Script::new(), "$vkrgss_modes");
    assert_eq!(played.after.as_deref(), Ok("[true, true]"));
}

/// Scenario "Globals of VX, measured on the original".
#[test]
fn the_globals_of_test_mode_of_vx_are_those_of_the_original() {
    let game = Game::of(
        harness(),
        Runtime::Rgss2,
        "modes_of_vx",
        "Modes",
        &[("Main", "$vkrgss_modes_of_vx = [$TEST, $BTEST, $DEBUG]")],
    );
    assert_eq!(game.run_config().runtime, Runtime::Rgss2);
    for (debug, battle_test, globals) in [
        (false, false, "[false, false, false]"),
        (true, false, "[true, false, false]"),
        (false, true, "[true, true, false]"),
    ] {
        let config = RunConfig {
            debug,
            battle_test,
            ..game.run_config()
        };
        let played = host::run_game(
            harness(),
            &game,
            config,
            Script::new(),
            "$vkrgss_modes_of_vx",
        );
        assert!(played.outcome.is_ok(), "{:?}", played.outcome);
        assert_eq!(played.after.as_deref(), Ok(globals));
    }
}

/// Scenarios "Starting rate of XP", "Size of XP" and "No such function in XP".
#[test]
fn a_game_of_xp_starts_at_640_by_480_and_40_without_the_size_functions() {
    let source = "$vkrgss_frame_of_xp = [Graphics.frame_rate]
        Graphics.update
        [:width, :height].each do |name|
          begin
            Graphics.send(name)
          rescue NoMethodError => error
            $vkrgss_frame_of_xp << error.class
          end
        end";
    let game = Game::new(harness(), "frame_of_xp", "Frame", &[("Main", source)]);
    let played = run(&game, Script::new(), "$vkrgss_frame_of_xp");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(
        played.after.as_deref(),
        Ok("[40, NoMethodError, NoMethodError]")
    );
    let frames = played.record.frames();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].logical_size, Size::new(640, 480));
    assert_eq!(frames[0].frame_rate.get(), 40);
}

#[test]
fn the_title_reaches_the_host_before_the_first_frame() {
    let game = Game::new(harness(), "title", "Demo", &[("Main", "Graphics.update")]);
    let played = run(&game, Script::new(), "nil");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.record.received[0], title("Demo"));
    assert!(matches!(played.record.received[1], GuestToHost::Frame(_)));
    assert_eq!(played.record.received[2..], ending());
}

#[test]
fn the_built_in_scripts_are_defined_before_the_game() {
    let game = Game::new(
        harness(),
        "builtin",
        "Builtin",
        &[(
            "Main",
            "$vkrgss_builtin = [defined?(RPG::Cache), defined?(RPG::Sprite), defined?(RPG::Weather)]",
        )],
    );
    let played = run(&game, Script::new(), "$vkrgss_builtin");
    assert_eq!(
        played.after.as_deref(),
        Ok(r#"["constant", "constant", "constant"]"#)
    );
}

#[test]
fn a_missing_built_in_scripts_file_fails_before_any_script() {
    let game = Game::new(
        harness(),
        "no_builtin",
        "None",
        &[("Main", "$vkrgss_no_builtin = :ran")],
    );
    let missing = game.dir().join("nowhere").join("rpg.rb");
    let config = RunConfig {
        builtin_scripts: missing.clone(),
        ..game.run_config()
    };
    let played = host::run_game(
        harness(),
        &game,
        config,
        Script::new(),
        "$vkrgss_no_builtin",
    );
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
    assert_eq!(played.record.received[1..], ending_of(false));
}

#[test]
fn an_error_in_the_built_in_scripts_is_reported_under_their_name() {
    let game = Game::new(
        harness(),
        "bad_builtin",
        "Bad",
        &[("Main", "$vkrgss_bad_builtin = :ran")],
    );
    fs::write(game.builtin_scripts(), "module RPG\nend\nRPG.nothing\n").unwrap();
    let played = run(&game, Script::new(), "$vkrgss_bad_builtin");
    assert!(
        report(&played).starts_with("Script '(builtin)' line 3: NoMethodError occurred."),
        "{}",
        report(&played)
    );
    assert_eq!(played.after.as_deref(), Ok("nil"));
}

#[test]
fn scripts_run_in_archive_order_under_their_section_names() {
    let first = "($vkrgss_order ||= []) << 'One'";
    let game = Game::new(
        harness(),
        "order",
        "Order",
        &[
            ("One", first),
            ("Two", "$vkrgss_order << 'Two'\n$vkrgss_file = __FILE__"),
            (
                "Three",
                "$vkrgss_order << 'Three'\n$vkrgss_table = $RGSS_SCRIPTS.map { |script| script[1] } \
                 + [$RGSS_SCRIPTS[0][3], $RGSS_SCRIPTS[0].size, $RGSS_SCRIPTS[0][0]]",
            ),
        ],
    );
    let played = run(
        &game,
        Script::new(),
        "[$vkrgss_order, $vkrgss_file, $vkrgss_table]",
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
    let game = Game::new(
        harness(),
        "no_archive",
        "None",
        &[("Main", "$vkrgss_no_archive = :ran")],
    );
    fs::remove_file(game.dir().join("Data").join("Scripts.rxdata")).unwrap();
    let played = run(&game, Script::new(), "$vkrgss_no_archive");
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
    assert_eq!(played.record.received[1..], ending_of(false));
}

#[test]
fn an_archive_that_is_not_one_fails() {
    let game = Game::new(
        harness(),
        "bad_archive",
        "Bad",
        &[("Main", "$vkrgss_bad_archive = :ran")],
    );
    fs::write(game.dir().join("Data").join("Scripts.rxdata"), b"junk").unwrap();
    let played = run(&game, Script::new(), "$vkrgss_bad_archive");
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
        harness(),
        "finish",
        "Finish",
        &[("Main", "Audio.bgm_stop\n$vkrgss_finish = :ran")],
    );
    let played = run(&game, Script::new(), "$vkrgss_finish");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok(":ran"));
    let mut expected = vec![title("Finish"), stop(AudioChannel::Bgm)];
    expected.extend(ending());
    assert_eq!(played.record.received, expected);
}

#[test]
fn exit_ends_the_game_before_later_scripts() {
    let game = Game::new(
        harness(),
        "exit",
        "Exit",
        &[
            (
                "First",
                "$vkrgss_exit = [:first]\nexit\n$vkrgss_exit << :after",
            ),
            ("Second", "$vkrgss_exit << :second"),
        ],
    );
    let played = run(&game, Script::new(), "$vkrgss_exit");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok("[:first]"));
    assert_eq!(played.record.received[1..], ending());
}

#[test]
fn an_uncaught_error_is_reported_with_script_name_and_line() {
    let game = Game::new(
        harness(),
        "error",
        "Error",
        &[
            ("Helpers", "$vkrgss_error = []"),
            ("Scene_Title", "$vkrgss_error << :title\n\nnil.frobnicate\n"),
            ("Main", "$vkrgss_error << :main"),
        ],
    );
    let played = run(&game, Script::new(), "$vkrgss_error");
    let report = report(&played).to_owned();
    assert_eq!(
        report,
        "Script 'Scene_Title' line 3: NoMethodError occurred.\n\n\
         undefined method `frobnicate' for nil:NilClass"
    );
    assert_eq!(played.after.as_deref(), Ok("[:title]"));
    // The host shows the report before it is told that the guest has ended. The frame with
    // the message panel is between the two.
    let mut expected = vec![title("Error"), GuestToHost::Modal { text: report }];
    expected.extend(ending_of(false));
    let others: Vec<GuestToHost> = played.record.others().into_iter().cloned().collect();
    assert_eq!(others, expected);
    assert!(matches!(played.record.received[2], GuestToHost::Frame(_)));
    assert_eq!(played.record.frames().len(), 1);
}

#[test]
fn an_error_is_reported_where_it_was_raised() {
    let game = Game::new(
        harness(),
        "error_elsewhere",
        "Error",
        &[
            (
                "Game_Thing",
                "def vkrgss_fail_here\n  raise ArgumentError, 'no'\nend",
            ),
            ("Main", "\n\n\nvkrgss_fail_here"),
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
        harness(),
        "syntax",
        "Syntax",
        &[
            ("Fine", "$vkrgss_syntax = [:fine]"),
            ("Broken", "$vkrgss_syntax << :broken\ndef broken(\n"),
            ("Main", "$vkrgss_syntax << :main"),
        ],
    );
    let played = run(&game, Script::new(), "$vkrgss_syntax");
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
        harness(),
        "reset",
        "Reset",
        &[
            (
                "Count",
                "$vkrgss_reset_runs = ($vkrgss_reset_runs || 0) + 1",
            ),
            (
                "Main",
                "if $vkrgss_reset_runs == 1
                   $vkrgss_reset_bitmap = Bitmap.new(4, 4)
                   Audio.bgm_stop
                   loop { Graphics.update }
                 end
                 $vkrgss_reset_seen = $vkrgss_reset_bitmap.disposed?
                 Graphics.update",
            ),
        ],
    );
    let played = run(
        &game,
        Script::new().after_frame(2, HostToGuest::Reset),
        "[$vkrgss_reset_runs, $vkrgss_reset_seen]",
    );
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok("[2, true]"));
    assert_eq!(played.bitmaps, 0);

    // Two frames, the reset with its audio stops, then the one frame of the second run.
    let received = &played.record.received;
    assert_eq!(received.len(), 15);
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
fn the_first_run_has_no_current_error() {
    let game = Game::new(
        harness(),
        "error_first",
        "Reset",
        &[("Main", "$vkrgss_error_first = [$!, $@]")],
    );
    let played = run(&game, Script::new(), "$vkrgss_error_first");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok("[nil, nil]"));
}

#[test]
fn the_run_after_a_reset_has_the_reset_as_the_current_error() {
    let game = Game::new(
        harness(),
        "error_reset",
        "Reset",
        &[
            ("Record", "($vkrgss_error_reset ||= []) << [$!.class, $@]"),
            (
                "Main",
                "loop { Graphics.update } if $vkrgss_error_reset.size == 1",
            ),
        ],
    );
    let played = run(
        &game,
        Script::new().after_frame(2, HostToGuest::Reset),
        "[$vkrgss_error_reset.size, $vkrgss_error_reset[0], $vkrgss_error_reset[1][0], \
         $vkrgss_error_reset[1][1].class, $vkrgss_error_reset[1][1][0] =~ /in `update'/]",
    );
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    let after = played.after.expect("the snippet evaluates");
    assert!(
        after.starts_with("[2, [NilClass, nil], Reset, Array, ") && !after.ends_with("nil]"),
        "{after}"
    );
}

#[test]
fn an_alias_guarded_by_the_backtrace_is_made_once() {
    let game = Game::new(
        harness(),
        "error_alias",
        "Reset",
        &[
            (
                "Guarded",
                "class Sprite
                   alias vkrgss_error_alias_dispose dispose unless $@
                   def dispose
                     vkrgss_error_alias_dispose
                   end
                 end",
            ),
            (
                "Main",
                "$vkrgss_error_alias_runs = ($vkrgss_error_alias_runs || 0) + 1
                 loop { Graphics.update } if $vkrgss_error_alias_runs < 3
                 sprite = Sprite.new
                 sprite.dispose
                 $vkrgss_error_alias = sprite.disposed?",
            ),
        ],
    );
    let played = run(
        &game,
        Script::new()
            .after_frame(2, HostToGuest::Reset)
            .after_frame(4, HostToGuest::Reset),
        "[$vkrgss_error_alias_runs, $vkrgss_error_alias]",
    );
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok("[3, true]"));
}

#[test]
fn a_rescue_in_the_run_after_a_reset_leaves_the_reset() {
    vkrgss_binding::pending_over_ruby_1_9_2!();
    let game = Game::new(
        harness(),
        "error_rescue",
        "Reset",
        &[
            (
                "Count",
                "$vkrgss_error_rescue_runs = ($vkrgss_error_rescue_runs || 0) + 1",
            ),
            (
                "Rescue",
                "begin
                   raise 'own'
                 rescue
                   $vkrgss_error_rescue_own = $!.class
                 end",
            ),
            (
                "Main",
                "$vkrgss_error_rescue = $!.class
                 loop { Graphics.update } if $vkrgss_error_rescue_runs == 1",
            ),
        ],
    );
    let played = run(
        &game,
        Script::new().after_frame(2, HostToGuest::Reset),
        "[$vkrgss_error_rescue_runs, $vkrgss_error_rescue_own, $vkrgss_error_rescue]",
    );
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok("[2, RuntimeError, Reset]"));
}

#[test]
fn a_quit_request_ends_the_game() {
    let game = Game::new(
        harness(),
        "quit",
        "Quit",
        &[
            (
                "Main",
                "$vkrgss_quit = 0\nloop { Graphics.update; $vkrgss_quit += 1 }",
            ),
            ("Never", "$vkrgss_quit = :later"),
        ],
    );
    let played = run(
        &game,
        Script::new().after_frame(3, HostToGuest::Quit),
        "$vkrgss_quit",
    );
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok("2"));
    assert_eq!(played.record.frames().len(), 3);
    assert_eq!(played.record.received[4..], ending());
}

/// A game that writes ten values of `rand(1000)` to `file`, played with `seed`.
fn rand_values(name: &str, seed: u64) -> String {
    let file = harness().game_dir().with_file_name(format!("{name}.txt"));
    let source = format!(
        "File.open({}, 'w') {{ |file| 10.times {{ file.puts(rand(1000)) }} }}",
        host::ruby_literal(&file.to_string_lossy())
    );
    let game = Game::new(harness(), name, "Rand", &[("Main", &source)]);
    let config = RunConfig {
        seed: Some(seed),
        ..game.run_config()
    };
    let played = host::run_game(harness(), &game, config, Script::new(), "nil");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    let values = fs::read_to_string(file).unwrap();
    assert_eq!(values.lines().count(), 10, "{values}");
    values
}

#[test]
fn the_same_seed_gives_the_same_values() {
    assert_eq!(rand_values("seed_a", 7), rand_values("seed_b", 7));
}

#[test]
fn different_seeds_give_different_values() {
    assert_ne!(rand_values("seed_c", 7), rand_values("seed_d", 8));
}

#[test]
fn the_seed_is_set_before_the_built_in_scripts() {
    let game = Game::new(harness(), "seed_builtin", "Rand", &[("Main", "")]);
    let builtin = format!(
        "{}$vkrgss_seed_builtin = rand(1000000)\n",
        host::RPG_STAND_IN
    );
    fs::write(game.builtin_scripts(), builtin).unwrap();
    let play = || {
        let config = RunConfig {
            seed: Some(7),
            ..game.run_config()
        };
        let played = host::run_game(
            harness(),
            &game,
            config,
            Script::new(),
            "$vkrgss_seed_builtin",
        );
        assert!(played.outcome.is_ok(), "{:?}", played.outcome);
        played.after.unwrap()
    };
    assert_eq!(play(), play());
}

#[test]
fn a_reset_seeds_again_and_evaluates_the_preload_scripts_again() {
    let game = Game::new(
        harness(),
        "seed_reset",
        "Reset",
        &[(
            "Main",
            "($vkrgss_seed_reset ||= []) << [rand(1000000), rand(1000000), $vkrgss_preload_runs]
             loop { Graphics.update } if $vkrgss_seed_reset.size == 1",
        )],
    );
    let preload = game.dir().join("count.rb");
    fs::write(
        &preload,
        "$vkrgss_preload_runs = ($vkrgss_preload_runs || 0) + 1\n",
    )
    .unwrap();
    let config = RunConfig {
        seed: Some(7),
        preload: vec![preload],
        ..game.run_config()
    };
    let played = host::run_game(
        harness(),
        &game,
        config,
        Script::new().after_frame(2, HostToGuest::Reset),
        "[$vkrgss_seed_reset.size, $vkrgss_seed_reset[0][0, 2] == $vkrgss_seed_reset[1][0, 2], \
         $vkrgss_seed_reset.map { |run| run[2] }]",
    );
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok("[2, true, [1, 2]]"));
}

#[test]
fn preload_scripts_run_in_order_between_the_built_in_scripts_and_the_game() {
    let game = Game::new(
        harness(),
        "preload",
        "Preload",
        &[(
            "Main",
            "$vkrgss_preload = [VKRGSS_PRELOAD_PATCH, $vkrgss_preload_order]",
        )],
    );
    let first = game.dir().join("first.rb");
    let second = game.dir().join("second.rb");
    fs::write(
        &first,
        "VKRGSS_PRELOAD_PATCH = defined?(RPG::Cache)\n$vkrgss_preload_order = [__FILE__]\n",
    )
    .unwrap();
    fs::write(&second, "$vkrgss_preload_order << :second\n").unwrap();
    let config = RunConfig {
        preload: vec![first.clone(), second],
        ..game.run_config()
    };
    let played = host::run_game(harness(), &game, config, Script::new(), "$vkrgss_preload");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(
        played.after,
        Ok(format!(r#"["constant", ["{}", :second]]"#, first.display()))
    );
}

#[test]
fn an_error_in_a_preload_script_is_reported_with_its_file_and_line() {
    let game = Game::new(
        harness(),
        "preload_error",
        "Preload",
        &[("Main", "$vkrgss_preload_error = :ran")],
    );
    let patch = game.dir().join("patch.rb");
    fs::write(&patch, "x = 1\nraise ArgumentError, 'bad patch'\n").unwrap();
    let config = RunConfig {
        preload: vec![patch.clone()],
        ..game.run_config()
    };
    let played = host::run_game(
        harness(),
        &game,
        config,
        Script::new(),
        "$vkrgss_preload_error",
    );
    assert_eq!(
        report(&played),
        format!(
            "Script '{}' line 2: ArgumentError occurred.\n\nbad patch",
            patch.display()
        )
    );
    assert_eq!(played.after.as_deref(), Ok("nil"));
}

#[test]
fn a_missing_preload_script_fails_before_any_script() {
    let game = Game::new(
        harness(),
        "preload_missing",
        "Preload",
        &[("Main", "$vkrgss_preload_missing = :ran")],
    );
    fs::write(
        game.builtin_scripts(),
        "$vkrgss_preload_missing = :builtin\n",
    )
    .unwrap();
    let missing = game.dir().join("nowhere.rb");
    let config = RunConfig {
        preload: vec![missing.clone()],
        ..game.run_config()
    };
    let played = host::run_game(
        harness(),
        &game,
        config,
        Script::new(),
        "$vkrgss_preload_missing",
    );
    let error = played.outcome.expect_err("the run fails");
    assert!(
        matches!(&error, RunError::Preload { path, .. } if *path == missing),
        "{error:?}"
    );
    assert!(
        error.to_string().contains(&missing.display().to_string()),
        "{error}"
    );
    assert_eq!(played.after.as_deref(), Ok("nil"));
    assert_eq!(played.record.received[1..], ending_of(false));
}

/// Scenario "A picture a script wrote".
#[test]
fn a_picture_a_script_wrote_is_loaded() {
    let picture = harness().game_dir().join("Graphics/Pictures/Dot.png");
    // The first `Bitmap.new` looks into the game directory before `Save` exists. The game is
    // not started in its directory here, as the executable starts it, so the script goes
    // there itself.
    let source = format!(
        "bytes = File.open({}, 'rb') {{ |file| file.read }}
         Dir.chdir($vkrgss_written_dir) do
           $vkrgss_written = [(Bitmap.new('Save/1.cdat') rescue $!.class)]
           Dir.mkdir('Save')
           File.open('Save/1.cdat', 'wb') {{ |file| file.write(bytes) }}
           bitmap = Bitmap.new('Save/1.cdat')
           $vkrgss_written << bitmap.width << bitmap.height
           # The guest is shared with the tests that count its bitmaps.
           bitmap.dispose
         end",
        host::ruby_literal(&picture.to_string_lossy())
    );
    let game = Game::new(
        harness(),
        "written_picture",
        "Written",
        &[("Main", &source)],
    );
    let dir = host::ruby_literal(&game.dir().to_string_lossy());
    harness()
        .eval(&format!("$vkrgss_written_dir = {dir}"))
        .unwrap();
    // The game directory was last changed when the game was made: the directory `Save` gets
    // a later time stamp whatever the resolution of the file system's.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let played = run(&game, Script::new(), "$vkrgss_written");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok("[Errno::ENOENT, 2, 2]"));
}
