//! The functions of `Graphics` in a game of RGSS2: Ruby snippets evaluated against a scripted
//! host, and what that host saw. A binary of its own, because `Graphics` is defined once, as
//! the runtime the harness starts with has it.

use vkxp_binding::testing::host::{self, Record, Script};
use vkxp_binding::testing::{self};
use vkxp_proto::{GuestToHost, TransitionRule};
use vkxp_vfs::Runtime;

/// The value of a snippet, or its exception as `Class: message`, and what the host saw. The
/// brightness is 255 and nothing is frozen when the snippet starts.
fn eval(source: &str) -> (String, Record) {
    testing::start_as(Runtime::Rgss2);
    let source = format!("Graphics.brightness = 255\n{source}");
    let (outcome, record) = host::play(&source, Script::new());
    let outcome = match outcome {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    };
    (outcome, record)
}

/// The brightness of every frame the host received.
fn levels(record: &Record) -> Vec<u8> {
    let frames = record.frames();
    frames.iter().map(|frame| frame.brightness).collect()
}

// --- Brightness of RGSS2 ----------------------------------------------------------------------

/// Scenario "Limits, measured on the original".
#[test]
fn the_brightness_starts_at_255_and_is_limited() {
    let (value, record) = eval(
        "start = Graphics.brightness
         Graphics.brightness = -5
         low = Graphics.brightness
         Graphics.brightness = 300
         [start, low, Graphics.brightness]",
    );
    assert_eq!(value, "[255, 0, 255]");
    assert!(record.received.is_empty());
}

/// Scenario "In the packet".
#[test]
fn the_frames_carry_the_brightness() {
    let (_, record) = eval("Graphics.brightness = 100; Graphics.update; Graphics.update");
    assert_eq!(levels(&record), [100, 100]);
}

// --- Waits and fades of RGSS2 -----------------------------------------------------------------

/// Scenario "Wait, measured on the original".
#[test]
fn wait_shows_its_frames_and_none_for_a_count_below_1() {
    let (value, record) = eval(
        "Graphics.frame_count = 0
         waited = [Graphics.wait(7), Graphics.wait(0), Graphics.wait(-3)]
         [waited, Graphics.frame_count]",
    );
    assert_eq!(value, "[[nil, nil, nil], 7]");
    assert_eq!(record.frames().len(), 7);
    assert_eq!(record.received.len(), 7);
    assert_eq!(record.ticks, 7);
}

/// Scenario "Fade out, measured on the original".
#[test]
fn fadeout_steps_down_to_0() {
    let (value, record) = eval(
        "Graphics.brightness = 200
         [Graphics.fadeout(3), Graphics.brightness]",
    );
    assert_eq!(value, "[nil, 0]");
    assert_eq!(levels(&record), [134, 67, 0]);
}

/// Scenario "Fade in, measured on the original".
#[test]
fn fadein_steps_up_to_255() {
    let (value, record) = eval(
        "Graphics.brightness = 100
         [Graphics.fadein(4), Graphics.brightness]",
    );
    assert_eq!(value, "[nil, 255]");
    assert_eq!(levels(&record), [138, 177, 216, 255]);
}

/// Scenario "No frame". A count below 0 is taken as 0, which was not measured.
#[test]
fn a_fade_without_a_frame_leaves_its_end_value() {
    let (value, record) = eval(
        "Graphics.fadeout(0)
         out = Graphics.brightness
         Graphics.fadein(0)
         back = Graphics.brightness
         Graphics.fadeout(-2)
         [out, back, Graphics.brightness]",
    );
    assert_eq!(value, "[0, 255, 0]");
    assert!(record.received.is_empty());
}

/// Scenario "A redefined update is not called".
#[test]
fn waits_and_fades_do_not_call_a_redefined_update() {
    let (value, record) = eval(
        "$vkxp_updates = 0
         class << Graphics
           alias vkxp_update update
           def update
             $vkxp_updates += 1
             vkxp_update
           end
         end
         begin
           Graphics.wait(5)
           Graphics.fadeout(5)
         ensure
           class << Graphics
             alias update vkxp_update
           end
         end
         $vkxp_updates",
    );
    assert_eq!(value, "0");
    assert_eq!(record.frames().len(), 10);
}

// --- Transition of RGSS2 ----------------------------------------------------------------------

/// Scenario "Default, measured on the original".
#[test]
fn freeze_and_the_default_transition() {
    let (value, record) = eval("Graphics.freeze; Graphics.transition");
    assert_eq!(value, "nil");
    assert_eq!(record.received.len(), 12);
    assert_eq!(record.received[0], GuestToHost::Freeze);
    assert_eq!(
        record.received[1],
        GuestToHost::Transition {
            duration: 10,
            mask: None,
            vague: 20,
            rule: TransitionRule::Rgss2,
        }
    );
    assert_eq!(record.frames().len(), 10);
    assert_eq!(record.ticks, 10);
}

/// Scenario "Nothing frozen, measured on the original". The picture is not looked for
/// either, and the transition that ended leaves nothing frozen.
#[test]
fn a_transition_with_nothing_frozen_does_nothing() {
    let (value, record) = eval(
        "Graphics.brightness = 100
         count = Graphics.frame_count
         shown = [Graphics.transition(20), Graphics.transition(20, 'Graphics/Transitions/Nope')]
         [shown, Graphics.frame_count - count, Graphics.brightness]",
    );
    assert_eq!(value, "[[nil, nil], 0, 100]");
    assert!(record.received.is_empty());

    let (value, record) = eval(
        "Graphics.freeze
         Graphics.transition(2)
         Graphics.brightness = 100
         Graphics.transition(3)
         Graphics.brightness",
    );
    assert_eq!(value, "100");
    assert_eq!(record.frames().len(), 2);
    assert_eq!(record.others().len(), 2);
}

/// Scenario "Brightness after a freeze, measured on the original".
#[test]
fn a_transition_after_a_freeze_sets_the_brightness_to_255() {
    let (value, record) = eval(
        "Graphics.fadeout(1)
         Graphics.freeze
         Graphics.transition(0)
         Graphics.brightness",
    );
    assert_eq!(value, "255");
    assert_eq!(levels(&record), [0]);
    assert_eq!(
        record.received[1..],
        [
            GuestToHost::Freeze,
            GuestToHost::Transition {
                duration: 0,
                mask: None,
                vague: 20,
                rule: TransitionRule::Rgss2,
            }
        ]
    );
}

/// The frames of a transition carry the brightness it set.
#[test]
fn the_frames_of_a_transition_are_at_full_brightness() {
    let (_, record) = eval(
        "Graphics.brightness = 40
         Graphics.freeze
         Graphics.transition(3, 'Graphics/Transitions/004-Blind04')",
    );
    assert_eq!(levels(&record), [255, 255, 255]);
    assert!(matches!(
        record.received[1],
        GuestToHost::Transition {
            duration: 3,
            mask: Some(_),
            vague: 20,
            rule: TransitionRule::Rgss2,
        }
    ));
}
