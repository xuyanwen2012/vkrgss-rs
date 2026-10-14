//! The report of an uncaught exception and the message boxes of a game of RGSS3 against a
//! scripted host: the scenarios of "Uncaught errors of RGSS3" and of "Message boxes and
//! console output of RGSS3" of `rgss-runtime`, measured on the original runtime of RGSS3
//! (`ace-v1-api-REPORT.md` sections 1.2 and 1.7 of the measurements kept outside the
//! repository).
#![cfg(not(vkxp_interpreter_absent))]

use vkxp_binding_rgss3::RunError;
use vkxp_binding_rgss3::testing;
use vkxp_binding_rgss3::testing::host::{self, Game, Played, Script, with_error_stream};
use vkxp_proto::GuestToHost;
use vkxp_vfs::Runtime;

/// Plays a game of RGSS3 of these scripts that ends with an uncaught exception, and returns
/// its report and what the host saw.
fn report(name: &str, scripts: &[(&str, &str)]) -> (String, Played) {
    testing::start_as(Runtime::Rgss3);
    let game = Game::of(Runtime::Rgss3, name, "Ace", scripts);
    let played = host::run_game(&game, game.run_config(), Script::new(), "nil");
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
            ("Scene_Title", "a = 1\nb = 2\nnil.vkxp_undefined\n"),
        ],
    );
    assert!(
        text.starts_with("Script 'Scene_Title' line 3: NoMethodError occurred.\n\nundefined method `vkxp_undefined' for nil"),
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
                "def vkxp_helper\n  raise 'from the helper'\nend\n",
            ),
            ("Main", "\nvkxp_helper\n"),
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
            ("Before", "$vkxp_before_syntax = true"),
            ("Bad Syntax", "a = 1\nb = 2\nc = (1 +\n"),
            ("After", "$vkxp_after_syntax = true"),
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
        testing::eval("[$vkxp_before_syntax, $vkxp_after_syntax]").as_deref(),
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
        &[("magic", "# coding: us-ascii\n$vkxp_text = \"測\"\n")],
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
        let source = format!("# coding: {name}\n$vkxp_ran = true\n");
        let (text, _) = report(&format!("magic_unknown_{index}"), &[("Main", &source)]);
        assert_eq!(
            text,
            format!("ArgumentError occurred.\n\n{message}"),
            "{name}"
        );
    }
}

// --- Message boxes ----------------------------------------------------------------------------

/// The texts of the modal messages the host received for a snippet, with its value.
fn boxes(source: &str) -> (String, Vec<String>) {
    testing::start_as(Runtime::Rgss3);
    let (value, record) = host::play(source, Script::new());
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
