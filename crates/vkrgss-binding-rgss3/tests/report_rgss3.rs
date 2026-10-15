//! The report of an uncaught exception and the message boxes of a game of RGSS3 against a
//! scripted host: the scenarios of "Uncaught errors of RGSS3" and of "Message boxes and
//! console output of RGSS3" of `rgss-runtime`, measured on the original runtime of RGSS3
//! (`ace-v1-api-REPORT.md` sections 1.2 and 1.7 of the measurements kept outside the
//! repository).
#![cfg(not(vkrgss_interpreter_absent))]

use vkrgss_binding_rgss3::RunError;
use vkrgss_binding_rgss3::testing::host::{self, Game, Played, Script, with_error_stream};
use vkrgss_binding_rgss3::testing::{self, Harness};
use vkrgss_proto::GuestToHost;
use vkrgss_vfs::Runtime;

/// The harness, started as a game of RGSS3.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss3)
}

/// Plays `game` as `host::run_game` does, with the current directory of the process in the
/// game directory, where the glue of a guest puts it: `load_data` of RGSS3 looks for a loose
/// file from there. One game at a time enters its directory.
fn run_game_in(
    game: &Game,
    run: vkrgss_binding_rgss3::RunConfig,
    script: Script,
    after: &str,
) -> host::Played {
    static PLAYING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _playing = PLAYING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::env::set_current_dir(game.dir()).expect("the game directory is entered");
    host::run_game(harness(), game, run, script, after)
}

/// Plays a game of RGSS3 of these scripts that ends with an uncaught exception, and returns
/// its report and what the host saw.
fn report(name: &str, scripts: &[(&str, &str)]) -> (String, Played) {
    let game = Game::of(harness(), Runtime::Rgss3, name, "Ace", scripts);
    let played = run_game_in(&game, game.run_config(), Script::new(), "nil");
    match &played.outcome {
        Err(RunError::Script(report)) => (report.clone(), played),
        other => panic!("{name}: {other:?}"),
    }
}

#[test]
fn a_runtime_error_names_the_script_and_the_line() {
    let (text, played) = report(
        "runtime_error",
        &[
            ("Main", "nil"),
            ("Scene_Title", "a = 1\nb = 2\nnil.vkrgss_undefined\n"),
        ],
    );
    assert!(
        text.starts_with("Script 'Scene_Title' line 3: NoMethodError occurred.\n\nundefined method `vkrgss_undefined' for nil"),
        "{text}"
    );
    // Shown before the shutdown message, after the outcome.
    let others = played.record.others();
    let modal = others
        .iter()
        .position(|message| matches!(message, GuestToHost::Modal { text: shown } if *shown == text))
        .expect("the report is shown");
    let shutdown = others
        .iter()
        .position(|message| **message == GuestToHost::Shutdown)
        .expect("the shutdown message");
    assert!(modal < shutdown);
    assert!(others.contains(&&GuestToHost::Outcome { ok: false }));
}

#[test]
fn a_method_of_another_script_names_that_script() {
    let (text, _) = report(
        "other_script",
        &[
            (
                "Helpers",
                "def vkrgss_helper\n  raise 'from the helper'\nend\n",
            ),
            ("Main", "\nvkrgss_helper\n"),
        ],
    );
    assert_eq!(
        text,
        "Script 'Helpers' line 2: RuntimeError occurred.\n\nfrom the helper"
    );
}

#[test]
fn an_error_inside_eval_names_the_line_of_the_eval() {
    let (text, _) = report(
        "inside_eval",
        &[(
            "Evaluating",
            "a = 1\nb = 2\nc = 3\neval(\"raise 'in eval'\")\n",
        )],
    );
    assert_eq!(
        text,
        "Script 'Evaluating' line 4: RuntimeError occurred.\n\nin eval"
    );
}

#[test]
fn a_missing_file_is_reported_by_its_name() {
    let (text, _) = report(
        "missing_file",
        &[("Main", "load_data('Data/Nope.rvdata2')")],
    );
    assert_eq!(text, "Unable to find file:\n\nData/Nope.rvdata2");
}

#[test]
fn an_empty_message_leaves_the_report_at_its_first_line() {
    let (text, _) = report("empty_message", &[("s", "raise RuntimeError, \"\"")]);
    assert_eq!(text, "Script 's' line 1: RuntimeError occurred.");
}

#[test]
fn an_exception_without_a_backtrace_has_no_line() {
    let (text, _) = report(
        "no_line",
        &[(
            "Main",
            "e = RuntimeError.new('adrift')\ne.set_backtrace([])\nraise e",
        )],
    );
    assert_eq!(text, "RuntimeError occurred.\n\nadrift");
}

#[test]
fn a_syntax_error_names_its_script_and_line_and_ends_the_scripts() {
    let (text, _) = report(
        "syntax_error",
        &[
            ("Before", "$vkrgss_before_syntax = true"),
            ("Bad Syntax", "a = 1\nb = 2\nc = (1 +\n"),
            ("After", "$vkrgss_after_syntax = true"),
        ],
    );
    assert!(
        text.starts_with("Script 'Bad Syntax' line 3: SyntaxError occurred.\n\n"),
        "{text}"
    );
    let message = text.split_once("\n\n").unwrap().1;
    assert!(!message.starts_with('{'), "{message}");
    assert!(!message.is_empty(), "{text}");
    assert_eq!(
        harness()
            .eval("[$vkrgss_before_syntax, $vkrgss_after_syntax]")
            .as_deref(),
        Ok("[true, nil]")
    );
}

#[test]
fn a_changed_name_is_the_name_reported() {
    let (text, _) = report(
        "changed_name",
        &[
            ("First", "$RGSS_SCRIPTS[1][1] = 'Renamed'"),
            ("Second", "raise 'named'"),
        ],
    );
    assert_eq!(
        text,
        "Script 'Renamed' line 1: RuntimeError occurred.\n\nnamed"
    );
}

#[test]
fn exceptions_that_are_no_standard_error_are_reported_the_same_way() {
    for (index, (source, starts)) in [
        (
            "raise Exception, 'plain'",
            "Script 'Main' line 1: Exception occurred.\n\nplain",
        ),
        (
            "raise Interrupt",
            "Script 'Main' line 1: Interrupt occurred.",
        ),
        (
            "raise NotImplementedError, 'later'",
            "Script 'Main' line 1: NotImplementedError occurred.\n\nlater",
        ),
        (
            "throw :sym",
            "Script 'Main' line 1: ArgumentError occurred.\n\n",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let (text, _) = report(&format!("not_standard_{index}"), &[("Main", source)]);
        assert!(text.starts_with(starts), "{source}: {text}");
        if source.starts_with("throw") {
            assert!(text.contains(":sym"), "{text}");
        }
    }
}

/// Requirement "Magic comments of the scripts of RGSS3", scenario "US-ASCII"
/// (`ace-v1-api-REPORT.md` section 1.1).
#[test]
fn a_byte_above_127_in_a_script_of_us_ascii_is_a_syntax_error() {
    let (text, _) = report(
        "magic_us_ascii",
        &[("magic", "# coding: us-ascii\n$vkrgss_text = \"測\"\n")],
    );
    assert_eq!(
        text,
        "Script 'magic' line 2: SyntaxError occurred.\n\ninvalid multibyte char (US-ASCII)"
    );
}

/// Requirement "Magic comments of the scripts of RGSS3", scenarios "Unknown names" and
/// "UTF-16LE" (`ace-v1-api-REPORT.md` section 1.1): the comment is refused by the call of
/// `eval`, which has no line.
#[test]
fn an_unknown_or_incompatible_encoding_is_an_argument_error_without_a_line() {
    for (index, (name, message)) in [
        ("euc-jp", "unknown encoding name: euc-jp"),
        ("binary", "unknown encoding name: binary"),
        ("nosuch", "unknown encoding name: nosuch"),
        ("utf-16le", "UTF-16LE is not ASCII compatible"),
    ]
    .into_iter()
    .enumerate()
    {
        let source = format!("# coding: {name}\n$vkrgss_ran = true\n");
        let (text, _) = report(&format!("magic_unknown_{index}"), &[("Main", &source)]);
        assert_eq!(
            text,
            format!("ArgumentError occurred.\n\n{message}"),
            "{name}"
        );
    }
}

/// Requirement "Long reports and the status after an error in RGSS3", scenario "A long
/// message": shown whole, where the original shows an empty box above 1023 UTF-16 units.
#[test]
fn a_long_report_is_shown_whole() {
    let (text, played) = report("long_report", &[("s", "raise RuntimeError, 'x' * 2000")]);
    let expected = format!(
        "Script 's' line 1: RuntimeError occurred.\n\n{}",
        "x".repeat(2000)
    );
    assert_eq!(text, expected);
    let shown =
        played.record.others().into_iter().any(
            |message| matches!(message, GuestToHost::Modal { text: shown } if *shown == expected),
        );
    assert!(shown, "{:?}", played.record.others());
}

// --- A script archive that cannot be loaded ---------------------------------------------------

/// The texts of the modal messages of a run.
fn modal_texts(played: &Played) -> Vec<String> {
    played
        .record
        .received
        .iter()
        .filter_map(|message| match message {
            GuestToHost::Modal { text } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Plays a game of RGSS3 whose one script would set `$vkrgss_loaded_ran`, with its archive
/// replaced by `archive` (Ruby source of the bytes), or removed for `None`.
fn with_archive(name: &str, archive: Option<&str>) -> Played {
    let game = Game::of(
        harness(),
        Runtime::Rgss3,
        name,
        "Ace",
        &[("Main", "$vkrgss_loaded_ran = true")],
    );
    let path = game.dir().join("Data/Scripts.rvdata2");
    match archive {
        None => std::fs::remove_file(&path).unwrap(),
        Some(bytes) => {
            let write = format!(
                "File.open({}, 'wb') {{ |file| file.write({bytes}) }}",
                host::ruby_literal(&path.to_string_lossy())
            );
            harness()
                .run(move |ruby| ruby.eval_protected(&write, "testing", 1).map(|_| ()))
                .expect("the archive is written");
        }
    }
    harness().eval("$vkrgss_loaded_ran = nil").unwrap();
    run_game_in(
        &game,
        game.run_config(),
        Script::new(),
        "$vkrgss_loaded_ran",
    )
}

/// Requirement "A script archive of RGSS3 that cannot be loaded", scenario "The cases"
/// (`ace-v1-api-REPORT.md` section 1.1).
#[test]
fn an_archive_that_cannot_be_loaded_shows_failed_to_load_script() {
    for (index, archive) in [
        None,
        Some("''"),
        Some("'not a dump'"),
        Some("Marshal.dump('a string')"),
        Some("Marshal.dump([[0, 'Main']])"),
        Some("Marshal.dump([[0, 'Main', 'not deflated']])"),
    ]
    .into_iter()
    .enumerate()
    {
        let played = with_archive(&format!("unloadable_{index}"), archive);
        assert!(
            matches!(played.outcome, Err(RunError::Archive { .. })),
            "{archive:?}: {:?}",
            played.outcome
        );
        assert_eq!(
            modal_texts(&played),
            ["Failed to load script."],
            "{archive:?}"
        );
        assert_eq!(played.after.as_deref(), Ok("nil"), "{archive:?}");
        let others = played.record.others();
        let modal = others
            .iter()
            .position(|message| matches!(message, GuestToHost::Modal { .. }));
        let outcome = others
            .iter()
            .position(|message| **message == GuestToHost::Outcome { ok: false });
        assert!(
            modal < outcome && outcome.is_some(),
            "{archive:?}: {others:?}"
        );
        assert_eq!(others.last(), Some(&&GuestToHost::Shutdown), "{archive:?}");
    }
}

/// Requirement "A script archive of RGSS3 that cannot be loaded", scenario "Empty archive"
/// (`ace-v1-api-REPORT.md` section 1.1).
#[test]
fn an_empty_archive_runs_nothing_and_ends_without_a_failure() {
    let played = with_archive("empty_archive", Some("Marshal.dump([])"));
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert!(modal_texts(&played).is_empty());
    assert!(
        played
            .record
            .others()
            .contains(&&GuestToHost::Outcome { ok: true })
    );
}

// --- Message boxes ----------------------------------------------------------------------------

/// The texts of the modal messages the host received for a snippet, with its value.
fn boxes(source: &str) -> (String, Vec<String>) {
    let (value, record) = host::play(harness(), source, Script::new());
    let texts = record
        .received
        .iter()
        .filter_map(|message| match message {
            GuestToHost::Modal { text } => Some(text.clone()),
            _ => None,
        })
        .collect();
    (value.expect("the snippet runs"), texts)
}

#[test]
fn a_box_shows_each_argument_as_print_of_ruby_1_8_writes_it() {
    let (_, texts) = boxes("msgbox 1, \"a\", nil, :b, 1.5, [1, \"x\"]");
    assert_eq!(texts, [r#"1anilb1.5[1, "x"]"#]);
}

#[test]
fn msgbox_p_shows_each_argument_inspected_on_a_line() {
    let (_, texts) = boxes("msgbox_p \"a\", nil");
    assert_eq!(texts, ["\"a\"\nnil"]);
}

#[test]
fn the_separator_joins_the_arguments_and_a_nul_byte_ends_the_text() {
    let (_, texts) = boxes("$, = \"-\"; msgbox 1, 2; $, = nil; msgbox \"ab\\0cd\"");
    assert_eq!(texts, ["1-2", "ab"]);
}

#[test]
fn both_return_nil_and_write_the_text_to_the_error_stream() {
    let (value, texts) = boxes(&with_error_stream("[msgbox(\"x\"), msgbox_p(1)]"));
    assert_eq!(value, r#"[[nil, nil], "x\n1\n"]"#);
    assert_eq!(texts, ["x", "1"]);
}
