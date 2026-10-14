//! The start of a game of RGSS3 against a scripted host: the preparation, the built-in
//! scripts, the script archive and the endings, the scenarios of `rgss-runtime` measured on
//! the original runtime of RGSS3 (`ace-v1-api-REPORT.md` sections 1.1, 1.3, 1.6 and 1.7 of
//! the measurements kept outside the repository).
#![cfg(not(vkxp_interpreter_absent))]

use vkxp_binding_rgss3::RunError;
use vkxp_binding_rgss3::testing;
use vkxp_binding_rgss3::testing::host::{self, Game, Played, RGSS3_STAND_IN, Script};
use vkxp_proto::GuestToHost;
use vkxp_vfs::Runtime;

/// A game of RGSS3 named `name` with these scripts.
fn game(name: &str, scripts: &[(&str, &str)]) -> Game {
    testing::start_as(Runtime::Rgss3);
    Game::of(Runtime::Rgss3, name, "Ace", scripts)
}

/// Plays `game` with its own settings and reads `after`.
fn play(game: &Game, after: &str) -> Played {
    host::run_game(game, game.run_config(), Script::new(), after)
}

/// Plays a game of these scripts and returns what `after` reads, after a run that ended
/// without a failure.
fn run(name: &str, scripts: &[(&str, &str)], after: &str) -> String {
    let played = play(&game(name, scripts), after);
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    match played.after {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

// --- Preparation ------------------------------------------------------------------------------

#[test]
fn the_globals_are_those_measured_in_a_plain_run_and_in_debug_mode() {
    let source = "$vkxp_globals = [$KCODE, $:, $TEST, $BTEST, $DEBUG]";
    let plain = game("globals_plain", &[("Main", source)]);
    let played = play(&plain, "$vkxp_globals");
    assert_eq!(
        played.after.as_deref(),
        Ok(r#"[nil, ["."], false, false, false]"#)
    );

    let debug = game("globals_debug", &[("Main", source)]);
    let mut run = debug.run_config();
    run.debug = true;
    let played = host::run_game(&debug, run, Script::new(), "$vkxp_globals");
    assert_eq!(
        played.after.as_deref(),
        Ok(r#"[nil, ["."], true, false, false]"#)
    );
}

#[test]
fn the_reset_class_is_rgss_reset_and_neither_reset_nor_hangup_is_defined() {
    assert_eq!(
        run(
            "reset_class",
            &[(
                "Main",
                "$vkxp_reset = [RGSSReset.superclass, RGSSReset.new.message, defined?(Reset), \
                 defined?(Hangup)]"
            )],
            "$vkxp_reset"
        ),
        r#"[Exception, "RGSSReset", nil, nil]"#
    );
}

#[test]
fn the_version_is_3_0_1_as_bytes() {
    assert_eq!(
        run(
            "version",
            &[(
                "Main",
                "$vkxp_version = [RGSS_VERSION, RGSS_VERSION.encoding.name]"
            )],
            "$vkxp_version"
        ),
        r#"["3.0.1", "ASCII-8BIT"]"#
    );
}

#[test]
fn the_title_comes_before_the_first_frame() {
    testing::start_as(Runtime::Rgss3);
    let game = Game::of(
        Runtime::Rgss3,
        "title",
        "Demo",
        &[("Main", "Graphics.update")],
    );
    let played = play(&game, "nil");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    let title = played
        .record
        .received
        .iter()
        .position(|message| matches!(message, GuestToHost::SetTitle { title } if title == "Demo"))
        .expect("the title was sent");
    let frame = played
        .record
        .received
        .iter()
        .position(|message| matches!(message, GuestToHost::Frame(_)))
        .expect("a frame was sent");
    assert!(title < frame);
}

// --- Built-in scripts -------------------------------------------------------------------------

#[test]
fn the_three_blocks_of_the_built_in_scripts_are_there() {
    assert_eq!(
        run(
            "three_blocks",
            &[(
                "Main",
                "$vkxp_blocks = [defined?(RPG), respond_to?(:rgss_main, true), \
                 respond_to?(:rgss_stop, true)]"
            )],
            "$vkxp_blocks"
        ),
        r#"["constant", true, true]"#
    );
}

#[test]
fn the_stand_in_says_that_it_was_written_for_the_tests() {
    let game = game("stand_in", &[("Main", "nil")]);
    let builtin = std::fs::read_to_string(game.builtin_scripts()).unwrap();
    assert_eq!(builtin, RGSS3_STAND_IN);
    assert!(
        builtin.starts_with(
            "# Stand-in for the built-in scripts of RGSS3, written for the tests of vkxp.\n"
        ),
        "{builtin}"
    );
}

// --- Script archive ---------------------------------------------------------------------------

#[test]
fn the_scripts_run_in_order_under_four_digits_in_braces() {
    let append = "($vkxp_files ||= []) << __FILE__";
    assert_eq!(
        run(
            "names",
            &[("A", append), ("B", append), ("C", append)],
            "$vkxp_files"
        ),
        r#"["{0000}", "{0001}", "{0002}"]"#
    );
}

#[test]
fn the_source_is_compiled_as_utf_8_and_stored_as_bytes() {
    assert_eq!(
        run(
            "encodings",
            &[
                ("First", "nil"),
                (
                    "Second",
                    "$vkxp_encodings = [__ENCODING__.name, \"測\".encoding.name, \"測\".size, \
                     __FILE__.encoding.name, $RGSS_SCRIPTS[1][3].encoding.name]"
                ),
            ],
            "$vkxp_encodings"
        ),
        r#"["UTF-8", "UTF-8", 1, "ASCII-8BIT", "ASCII-8BIT"]"#
    );
}

#[test]
fn the_source_is_inserted_at_index_3_and_a_fourth_value_moves_to_index_4() {
    let game = game("table", &[("First", "nil")]);
    // Written as bytes, as an editor that does not label its strings would.
    let archive = game.dir().join("Data/Scripts.rvdata2");
    let write = format!(
        "File.open({}, 'wb') {{ |file| Marshal.dump([\
           [0, 'First'.force_encoding('ASCII-8BIT'), Zlib::Deflate.deflate('$vkxp_table = $RGSS_SCRIPTS[1]')], \
           [7, 'Second'.force_encoding('ASCII-8BIT'), Zlib::Deflate.deflate('nil'), :extra]], file) }}",
        host::ruby_literal(&archive.to_string_lossy())
    );
    testing::run(move |ruby| ruby.eval_protected(&write, "testing", 1).map(|_| ()))
        .expect("the archive is written");
    let played = play(
        &game,
        "t = $vkxp_table; [t.size, t[0], t[1], t[3], t[4], t[1].encoding.name, t[2].encoding.name]",
    );
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(
        played.after.as_deref(),
        Ok(r#"[5, 7, "Second", "nil", :extra, "UTF-8", "UTF-8"]"#)
    );
}

#[test]
fn the_source_is_read_when_its_turn_comes_and_the_count_is_read_once() {
    assert_eq!(
        run(
            "turn",
            &[
                (
                    "First",
                    "$RGSS_SCRIPTS[1][3] = \"$vkxp_seen = :changed\"
                     $RGSS_SCRIPTS << [9, \"Fourth\", Zlib::Deflate.deflate(\"$vkxp_extra = true\"), \"$vkxp_extra = true\"]"
                ),
                ("Second", "$vkxp_seen = :original"),
                ("Third", "nil"),
            ],
            "[$vkxp_seen, $vkxp_extra]"
        ),
        "[:changed, nil]"
    );
}

#[test]
fn constants_methods_and_instance_variables_of_main_pass_and_locals_do_not() {
    assert_eq!(
        run(
            "passes",
            &[
                (
                    "First",
                    "local = 1; @ivar = 2; CONST = 3; def helper; 4; end"
                ),
                (
                    "Second",
                    "$vkxp_passes = [defined?(local), @ivar, CONST, helper, self.to_s]"
                ),
            ],
            "$vkxp_passes"
        ),
        r#"[nil, 2, 3, 4, "main"]"#
    );
}

#[test]
fn empty_scripts_comments_and_crlf_line_ends_run() {
    assert_eq!(
        run(
            "line_ends",
            &[
                ("Empty", ""),
                ("Comments", "# a comment\n# and another"),
                ("Lines", "a = 1\r\nb = 2\r\n$vkxp_crlf = a + b\r\n"),
            ],
            "$vkxp_crlf"
        ),
        "3"
    );
}

#[test]
fn a_missing_or_broken_archive_names_its_path_and_runs_nothing() {
    let missing = game("missing_archive", &[("Main", "$vkxp_missing_ran = true")]);
    std::fs::remove_file(missing.dir().join("Data/Scripts.rvdata2")).unwrap();
    let played = play(&missing, "$vkxp_missing_ran");
    match &played.outcome {
        Err(RunError::Archive { path, .. }) => assert_eq!(path, "Data\\Scripts.rvdata2"),
        other => panic!("{other:?}"),
    }
    assert_eq!(played.after.as_deref(), Ok("nil"));

    let broken = game("broken_archive", &[("Main", "$vkxp_broken_ran = true")]);
    std::fs::write(broken.dir().join("Data/Scripts.rvdata2"), b"not a dump").unwrap();
    let played = play(&broken, "$vkxp_broken_ran");
    let message = played.outcome.unwrap_err().to_string();
    assert!(message.contains("Data\\Scripts.rvdata2"), "{message}");
    assert_eq!(played.after.as_deref(), Ok("nil"));
}

// --- Endings ----------------------------------------------------------------------------------

/// Whether the game ended as one that did not fail: the audio stopped, the outcome and the
/// shutdown message sent.
fn ended_with_success(played: &Played) -> bool {
    let others = played.record.others();
    played.outcome.is_ok()
        && others.contains(&&GuestToHost::Outcome { ok: true })
        && others.last() == Some(&&GuestToHost::Shutdown)
}

#[test]
fn exit_in_every_form_ends_the_game_with_success_and_no_later_script_runs() {
    for (index, ending) in [
        "exit",
        "exit(3)",
        "exit(false)",
        "abort",
        "raise SystemExit",
    ]
    .into_iter()
    .enumerate()
    {
        let game = game(
            &format!("exit_{index}"),
            &[("Ends", ending), ("Later", "$vkxp_later = true")],
        );
        let played = play(&game, "$vkxp_later");
        assert!(ended_with_success(&played), "{ending}: {played:?}");
        assert_eq!(played.after.as_deref(), Ok("nil"), "{ending}");
    }
}

#[test]
fn exit_inside_the_main_function_and_inside_a_fiber_ends_with_success() {
    for (name, source) in [
        ("exit_main", "rgss_main { exit }\n$vkxp_after_exit = true"),
        (
            "exit_fiber",
            "Fiber.new { exit }.resume\n$vkxp_after_exit = true",
        ),
    ] {
        let played = play(&game(name, &[("Main", source)]), "$vkxp_after_exit");
        assert!(ended_with_success(&played), "{name}: {played:?}");
        assert_eq!(played.after.as_deref(), Ok("nil"), "{name}");
    }
}

#[test]
fn the_end_of_the_last_script_ends_with_success() {
    let played = play(&game("last_script", &[("Main", "nil")]), "nil");
    assert!(ended_with_success(&played), "{played:?}");
}

#[test]
fn a_loop_driven_by_a_fiber_shows_its_frames() {
    let source = "
bitmap = Bitmap.new(8, 8)
sprite = Sprite.new
sprite.bitmap = bitmap
mover = Fiber.new do
  loop do
    sprite.x += 1
    Fiber.yield
  end
end
10.times do
  mover.resume
  Graphics.update
end
$vkxp_moved = sprite.x
";
    let played = play(&game("fiber_loop", &[("Main", source)]), "$vkxp_moved");
    assert!(ended_with_success(&played), "{played:?}");
    assert_eq!(played.after.as_deref(), Ok("10"));
    assert_eq!(played.record.frames().len(), 10);
}
