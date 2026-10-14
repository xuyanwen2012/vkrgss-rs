//! The module `Input` in a game of RGSS3 over Ruby 1.9.2: Ruby snippets evaluated against a
//! scripted host that sends input snapshots. The scenarios of "Arguments of Input of RGSS3",
//! "When Input of RGSS3 sees a key", "Repeat and directions of Input of RGSS3", "Method
//! surface of Input in RGSS3" and "Button constants of RGSS3, measured on the original" of
//! `rgss-modules`. A file of this package alone, so that it never runs as RGSS1
//! or RGSS2 over Ruby 1.8.7.
//!
//! The expected values are those measured on the original runtime of VX Ace (DLL versions
//! 3.0.0.1 and 3.0.1.1, 2026-10-13, `ace-v1-sprite-REPORT.md` section 6 with its samples and
//! logs, and the method list of RGSS3, kept outside the repository).
//!
//! Each snippet starts in a new connection, so its input state is new: nothing held and
//! nothing received. A message the host sends after a frame arrives before the tick of that
//! frame, as in a scheduled run.
#![cfg(not(vkrgss_interpreter_absent))]
extern crate vkrgss_binding_rgss3 as vkrgss_binding;

use vkrgss_binding::testing;
use vkrgss_binding::testing::host::{self, Script};
use vkrgss_proto::{Button, HostToGuest, InputSnapshot};
use vkrgss_vfs::Runtime;

/// The twenty buttons of RGSS3 in the order of [`Button::ALL`], with their numbers of RGSS2.
const BUTTONS: [(&str, i32); 20] = [
    ("DOWN", 2),
    ("LEFT", 4),
    ("RIGHT", 6),
    ("UP", 8),
    ("A", 11),
    ("B", 12),
    ("C", 13),
    ("X", 14),
    ("Y", 15),
    ("Z", 16),
    ("L", 17),
    ("R", 18),
    ("SHIFT", 21),
    ("CTRL", 22),
    ("ALT", 23),
    ("F5", 25),
    ("F6", 26),
    ("F7", 27),
    ("F8", 28),
    ("F9", 29),
];

/// The value of a snippet played against a host that sends what `script` says, or its
/// exception as `Class: message`.
fn play(source: &str, script: Script) -> String {
    testing::start_as(Runtime::Rgss3);
    match host::play(source, script).0 {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

fn eval(source: &str) -> String {
    play(source, Script::new())
}

/// A message of the host with `buttons` held.
fn holding(buttons: &[Button]) -> HostToGuest {
    let snapshot = buttons
        .iter()
        .fold(InputSnapshot::EMPTY, |snapshot, &button| {
            snapshot.with(button)
        });
    HostToGuest::Input(snapshot)
}

// --- Arguments ------------------------------------------------------------------------------

/// Scenario "Names and numbers, measured on the original": the k-th button is held alone from
/// the k-th frame on, and the k-th update follows that frame.
#[test]
fn a_name_or_the_number_of_rgss2_names_the_button() {
    let mut script = Script::new();
    for (index, &button) in Button::ALL.iter().enumerate() {
        script = script.after_frame(index + 1, holding(&[button]));
    }
    let names: Vec<String> = BUTTONS.iter().map(|(name, _)| format!(":{name}")).collect();
    let value = play(
        &format!(
            "[{}].map do |name|
               Graphics.update
               Input.update
               constant = Input.const_get(name)
               [:press?, :trigger?, :repeat?].map do |query|
                 [Input.send(query, constant), Input.send(query, name),
                  (-300..300).select {{ |number| Input.send(query, number) }}]
               end
             end",
            names.join(", ")
        ),
        script,
    );
    for (&button, (name, _)) in Button::ALL.iter().zip(BUTTONS) {
        assert_eq!(format!("{button:?}").to_uppercase(), name);
    }
    let expected: Vec<String> = BUTTONS
        .iter()
        .map(|(_, number)| {
            format!(
                "[{}]",
                vec![format!("[true, true, [{number}]]"); 3].join(", ")
            )
        })
        .collect();
    assert_eq!(value, format!("[{}]", expected.join(", ")));
}

/// Scenario "Anything else, measured on the original".
#[test]
fn anything_else_is_false_and_raises_nothing() {
    let value = play(
        r#"Graphics.update
           Input.update
           values = [:down, :Down, :NOPE, :"", "DOWN", 2.0, 2.9, 258, 2**31 - 1, 2**31, 2**40,
                     nil, true, false, [2], Object.new, Input]
           answers = [:press?, :trigger?, :repeat?].map do |query|
             values.map { |value| Input.send(query, value) }
           end
           [answers.flatten.uniq, answers.flatten.size, Input.press?(:DOWN)]"#,
        Script::new().at_start(holding(&Button::ALL)),
    );
    assert_eq!(value, "[[false], 51, true]");
}

/// Scenario "Count, measured on the original".
#[test]
fn the_queries_take_one_argument() {
    let value = eval(
        "[proc { Input.press? }, proc { Input.trigger?(:A, :B) }, proc { Input.repeat? }]
           .map { |call| begin; call.call; :none; rescue ArgumentError => error; error.class; end }",
    );
    assert_eq!(value, "[ArgumentError, ArgumentError, ArgumentError]");
}

/// Scenario "The call of the standard scripts".
#[test]
fn the_call_of_the_standard_scripts_is_false() {
    assert_eq!(eval("Input.update; Input.repeat?(:DOWN)"), "false");
}

// --- When a key is seen ---------------------------------------------------------------------

/// Scenario "An update alone sees nothing new, measured on the original".
#[test]
fn an_update_alone_sees_nothing_new() {
    let value = play(
        "alone = (1..12).map { Input.update; Input.press?(:C) }
         Graphics.update
         Input.update
         [alone.uniq, alone.size, Input.press?(:C), Input.trigger?(:C)]",
        Script::new().at_start(holding(&[Button::C])),
    );
    assert_eq!(value, "[[false], 12, true, true]");
}

/// Scenario "Seen after the next frame, measured on the original".
#[test]
fn a_key_is_seen_after_the_next_frame() {
    let value = play(
        "Graphics.update
         Input.update
         [Input.press?(:DOWN), Input.trigger?(:DOWN)]",
        Script::new().after_frame(1, holding(&[Button::Down])),
    );
    assert_eq!(value, "[true, true]");
}

/// Scenario "Calls that show no frame, measured on the original".
#[test]
fn calls_that_show_no_frame_do_not_show_a_key() {
    let value = play(
        "Graphics.frame_reset
         Graphics.brightness = 255
         Graphics.snap_to_bitmap.dispose
         Graphics.freeze
         Graphics.transition(0)
         Graphics.wait(0)
         Graphics.fadeout(0)
         Graphics.resize_screen(544, 416)
         Input.update
         Input.press?(:C)",
        Script::new().at_start(holding(&[Button::C])),
    );
    assert_eq!(value, "false");
    // The brightness the fade left.
    assert_eq!(eval("Graphics.brightness = 255"), "255");
}

/// Scenario "Frames of waits and transitions, measured on the original".
#[test]
fn the_frames_of_waits_and_transitions_show_a_key() {
    let query = "Input.update
                 [Input.press?(:C), Input.trigger?(:C)]";
    let value = play(
        &format!("Graphics.wait(3)\n{query}"),
        Script::new().after_frame(3, holding(&[Button::C])),
    );
    assert_eq!(value, "[true, true]");
    let value = play(
        &format!("Graphics.freeze\nGraphics.transition(2)\n{query}"),
        Script::new().after_frame(2, holding(&[Button::C])),
    );
    assert_eq!(value, "[true, true]");
}

// --- Repeat and directions --------------------------------------------------------------------

/// The numbers of the updates, each after a shown frame, at which each of `queries` is true,
/// with `script` sending the input.
fn updates_at_which(updates: usize, queries: &[&str], script: Script) -> String {
    play(
        &format!(
            "queries = [{}]
             found = queries.map {{ [] }}
             (1..{updates}).each do |number|
               Graphics.update
               Input.update
               queries.each_with_index {{ |query, index| found[index] << number if eval(query) }}
             end
             found",
            queries
                .iter()
                .map(|query| format!("{query:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        script,
    )
}

/// Scenario "Repeat, measured on the original".
#[test]
fn a_held_button_repeats_as_in_rgss2() {
    assert_eq!(
        updates_at_which(
            36,
            &["Input.repeat?(:DOWN)"],
            Script::new().at_start(holding(&[Button::Down]))
        ),
        "[[1, 24, 30, 36]]"
    );
}

/// Scenario "The newest button repeats, measured on the original".
#[test]
fn the_newest_button_repeats() {
    let value = updates_at_which(
        40,
        &[
            "Input.repeat?(:RIGHT)",
            "Input.repeat?(:DOWN) && number >= 11",
        ],
        Script::new()
            .at_start(holding(&[Button::Down]))
            .after_frame(11, holding(&[Button::Down, Button::Right])),
    );
    assert_eq!(value, "[[11, 34, 40], []]");
}

/// Scenario "A and SHIFT, measured on the original".
#[test]
fn shift_repeats_and_a_does_not() {
    assert_eq!(
        updates_at_which(
            36,
            &["Input.repeat?(:SHIFT)", "Input.repeat?(:A)"],
            Script::new().at_start(holding(&[Button::A, Button::Shift]))
        ),
        "[[1, 24, 30, 36], []]"
    );
}

/// Scenario "Directions, measured on the original".
#[test]
fn the_directions_are_those_of_rgss2() {
    let once = "Graphics.update
                Input.update
                [Input.dir4, Input.dir8]";
    let value = play(
        once,
        Script::new().at_start(holding(&[Button::Left, Button::Up])),
    );
    assert_eq!(value, "[4, 7]");
    let value = play(
        &format!("Graphics.update\nInput.update\n{once}"),
        Script::new()
            .at_start(holding(&[Button::Down]))
            .after_frame(2, holding(&[Button::Down, Button::Right])),
    );
    assert_eq!(value, "[6, 3]");
    let value = play(
        once,
        Script::new().at_start(holding(&[Button::Left, Button::Right])),
    );
    assert_eq!(value, "[0, 0]");
}

// --- Surface ----------------------------------------------------------------------------------

/// Scenario "Button constants of RGSS3, measured on the original".
#[test]
fn the_constants_are_the_symbols_of_their_names() {
    assert_eq!(
        eval(
            "[Input::DOWN, Input::LEFT, Input::RIGHT, Input::UP, Input::A, Input::C, Input::F9,
              Input::DOWN == :DOWN]"
        ),
        "[:DOWN, :LEFT, :RIGHT, :UP, :A, :C, :F9, true]"
    );
}

/// Scenario "Functions and constants, measured on the original".
#[test]
fn the_functions_and_constants_are_those_of_the_list() {
    assert_eq!(
        eval("[Input.singleton_methods(false).sort, Input.constants.sort]"),
        "[[:dir4, :dir8, :press?, :repeat?, :trigger?, :update], [:A, :ALT, :B, :C, :CTRL, \
         :DOWN, :F5, :F6, :F7, :F8, :F9, :L, :LEFT, :R, :RIGHT, :SHIFT, :UP, :X, :Y, :Z]]"
    );
}
