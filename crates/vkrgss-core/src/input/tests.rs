use super::*;

fn holding(buttons: &[Button]) -> InputSnapshot {
    buttons
        .iter()
        .fold(InputSnapshot::EMPTY, |snapshot, &button| {
            snapshot.with(button)
        })
}

/// An input state after one update with `buttons` held.
fn after(buttons: &[Button]) -> InputState {
    let mut input = InputState::new();
    input.receive(holding(buttons));
    input.update();
    input
}

#[test]
fn nothing_is_held_before_any_input() {
    let mut input = InputState::new();
    for round in 0..3 {
        for button in Button::ALL {
            assert!(!input.is_pressed(button), "{button:?} {round}");
            assert!(!input.is_triggered(button));
            assert!(!input.is_repeated(button));
        }
        assert_eq!((input.dir4(), input.dir8()), (0, 0));
        input.update();
    }
}

#[test]
fn a_snapshot_takes_effect_at_the_next_update() {
    let mut input = InputState::new();
    input.update();
    input.receive(holding(&[Button::C]));
    for _ in 0..3 {
        assert!(!input.is_pressed(Button::C));
        assert!(!input.is_triggered(Button::C));
        assert!(!input.is_repeated(Button::C));
    }
    input.update();
    for _ in 0..3 {
        assert!(input.is_pressed(Button::C));
        assert!(input.is_triggered(Button::C));
        assert!(input.is_repeated(Button::C));
    }
    // A release waits for its update as well.
    input.receive(InputSnapshot::EMPTY);
    assert!(input.is_pressed(Button::C));
    input.update();
    assert!(!input.is_pressed(Button::C));
}

#[test]
fn without_a_new_snapshot_every_button_keeps_its_state() {
    let mut input = after(&[Button::C, Button::Left, Button::F9]);
    for _ in 0..5 {
        input.update();
        for button in Button::ALL {
            let expected = matches!(button, Button::C | Button::Left | Button::F9);
            assert_eq!(input.is_pressed(button), expected, "{button:?}");
        }
        assert!(!input.is_triggered(Button::C), "it only stays held");
    }
}

#[test]
fn an_update_uses_the_last_of_several_snapshots() {
    let mut input = InputState::new();
    input.receive(holding(&[Button::A]));
    input.receive(holding(&[Button::B]));
    input.receive(holding(&[Button::C]));
    input.update();
    assert!(!input.is_pressed(Button::A));
    assert!(!input.is_pressed(Button::B));
    assert!(input.is_pressed(Button::C));

    // A press and release between two updates is never seen.
    input.receive(holding(&[Button::C, Button::X]));
    input.receive(holding(&[Button::C]));
    input.update();
    assert!(!input.is_pressed(Button::X));
    assert!(!input.is_triggered(Button::X));
    assert!(input.is_pressed(Button::C));
}

#[test]
fn every_button_is_tracked_on_its_own() {
    for held in Button::ALL {
        let input = after(&[held]);
        for button in Button::ALL {
            assert_eq!(
                input.is_pressed(button),
                button == held,
                "{held:?} {button:?}"
            );
            assert_eq!(input.is_triggered(button), button == held);
            assert_eq!(input.is_repeated(button), button == held);
        }
    }
}

#[test]
fn a_trigger_lasts_one_update() {
    let mut input = InputState::new();
    input.receive(holding(&[Button::C]));
    input.update();
    assert!(input.is_triggered(Button::C) && input.is_pressed(Button::C));
    input.receive(holding(&[Button::C]));
    input.update();
    assert!(!input.is_triggered(Button::C) && input.is_pressed(Button::C));
    input.update();
    assert!(!input.is_triggered(Button::C) && input.is_pressed(Button::C));
}

#[test]
fn releasing_and_pressing_again_triggers_again() {
    let mut input = after(&[Button::C]);
    input.update();
    assert!(!input.is_triggered(Button::C));
    input.receive(InputSnapshot::EMPTY);
    input.update();
    assert!(!input.is_pressed(Button::C) && !input.is_triggered(Button::C));
    input.receive(holding(&[Button::C]));
    input.update();
    assert!(input.is_triggered(Button::C));

    // Another button changing does not trigger it again.
    input.receive(holding(&[Button::C, Button::B]));
    input.update();
    assert!(!input.is_triggered(Button::C));
    assert!(input.is_triggered(Button::B));
}

#[test]
fn a_held_button_repeats_after_a_delay() {
    let mut input = InputState::new();
    input.receive(holding(&[Button::Down]));
    let mut repeats = Vec::new();
    for update in 1..=24 {
        input.update();
        if input.is_repeated(Button::Down) {
            repeats.push(update);
        }
        assert!(input.is_pressed(Button::Down));
    }
    assert_eq!(repeats, [1, 16, 20, 24]);

    for update in 25..=40 {
        input.update();
        assert_eq!(input.is_repeated(Button::Down), update % 4 == 0, "{update}");
    }
}

#[test]
fn a_release_starts_the_repeat_pattern_over() {
    let mut input = InputState::new();
    input.receive(holding(&[Button::Down]));
    for _ in 0..10 {
        input.update();
    }
    assert!(!input.is_repeated(Button::Down));
    input.receive(InputSnapshot::EMPTY);
    input.update();
    assert!(!input.is_repeated(Button::Down));

    input.receive(holding(&[Button::Down]));
    let mut repeats = Vec::new();
    for update in 1..=20 {
        input.update();
        if input.is_repeated(Button::Down) {
            repeats.push(update);
        }
    }
    assert_eq!(repeats, [1, 16, 20]);
}

/// The buttons that repeat at each update of `steps`, where a step is the buttons held at
/// one update.
fn repeats(steps: impl IntoIterator<Item = Vec<Button>>) -> Vec<Vec<Button>> {
    let mut input = InputState::new();
    steps
        .into_iter()
        .map(|held| {
            input.receive(holding(&held));
            input.update();
            let repeated = Button::ALL.into_iter().filter(|&b| input.is_repeated(b));
            repeated.collect()
        })
        .collect()
}

#[test]
fn a_second_button_takes_the_repeat() {
    let steps = (1..=31).map(|update| {
        if update < 8 {
            vec![Button::Down]
        } else {
            vec![Button::Down, Button::Left]
        }
    });
    for (update, repeated) in (1..).zip(repeats(steps)) {
        let expected = match update {
            1 => vec![Button::Down],
            8 | 23 | 27 | 31 => vec![Button::Left],
            _ => vec![],
        };
        assert_eq!(repeated, expected, "update {update}");
    }
}

#[test]
fn the_first_button_does_not_get_the_repeat_back() {
    let steps = (1..=40).map(|update| {
        if update == 2 {
            vec![Button::Down, Button::Left]
        } else {
            vec![Button::Down]
        }
    });
    for (update, repeated) in (1..).zip(repeats(steps)) {
        let expected = match update {
            1 => vec![Button::Down],
            2 => vec![Button::Left],
            _ => vec![],
        };
        assert_eq!(repeated, expected, "update {update}");
    }
}

#[test]
fn of_buttons_triggered_together_the_highest_number_repeats() {
    for (low, high) in [
        (Button::Down, Button::Left),
        (Button::Left, Button::Up),
        (Button::B, Button::C),
    ] {
        let input = after(&[low, high]);
        assert!(input.is_triggered(low) && input.is_triggered(high));
        assert!(!input.is_repeated(low), "{low:?} with {high:?}");
        assert!(input.is_repeated(high), "{high:?} with {low:?}");
    }
}

#[test]
fn no_button_repeats_after_the_repeating_one_is_released() {
    let mut input = after(&[Button::C]);
    input.receive(InputSnapshot::EMPTY);
    input.update();
    input.receive(holding(&[Button::C]));
    input.update();
    assert!(input.is_repeated(Button::C), "a new trigger repeats");
    // Fifteen updates later nothing remembers the first press.
    for update in 2..=16 {
        input.update();
        assert_eq!(input.is_repeated(Button::C), update == 16, "{update}");
    }
}

#[test]
fn a_single_direction_is_reported_by_both_queries() {
    for (button, value) in [
        (Button::Down, 2),
        (Button::Left, 4),
        (Button::Right, 6),
        (Button::Up, 8),
    ] {
        let mut input = after(&[button]);
        assert_eq!((input.dir4(), input.dir8()), (value, value), "{button:?}");
        input.update();
        assert_eq!((input.dir4(), input.dir8()), (value, value), "still held");
    }
    // Other buttons are no direction.
    let input = after(&[Button::C, Button::Shift, Button::F5]);
    assert_eq!((input.dir4(), input.dir8()), (0, 0));
}

#[test]
fn opposite_directions_cancel() {
    let input = after(&[Button::Left, Button::Right]);
    assert_eq!((input.dir4(), input.dir8()), (0, 0));
    let input = after(&[Button::Up, Button::Down]);
    assert_eq!((input.dir4(), input.dir8()), (0, 0));
    let input = after(&[Button::Up, Button::Down, Button::Left, Button::Right]);
    assert_eq!((input.dir4(), input.dir8()), (0, 0));
    // The direction left over on the other axis still counts.
    let input = after(&[Button::Left, Button::Right, Button::Down]);
    assert_eq!((input.dir4(), input.dir8()), (2, 2));
    let input = after(&[Button::Up, Button::Down, Button::Right]);
    assert_eq!((input.dir4(), input.dir8()), (6, 6));
}

/// The answers of both direction queries at each update of `steps`, where a step is the
/// buttons held at one update.
fn directions(steps: &[&[Button]]) -> Vec<(u8, u8)> {
    let mut input = InputState::new();
    steps
        .iter()
        .map(|held| {
            input.receive(holding(held));
            input.update();
            (input.dir4(), input.dir8())
        })
        .collect()
}

#[test]
fn a_diagonal_gives_dir4_the_axis_that_was_not_the_last_held_alone() {
    use Button::{Down, Left, Right, Up};
    let answers = directions(&[
        &[Down],
        &[Down, Left],
        &[Left],
        &[Down, Left],
        &[],
        &[Left, Up],
        &[Up],
        &[],
        &[Right, Up],
    ]);
    assert_eq!(
        answers,
        [
            (2, 2),
            (4, 1),
            (4, 4),
            (2, 1),
            (0, 0),
            (8, 7),
            (8, 8),
            (0, 0),
            (6, 9)
        ]
    );
}

#[test]
fn a_cancelled_axis_does_not_count_for_dir4() {
    use Button::{Down, Left, Up};
    let answers = directions(&[
        &[Down],
        &[Down, Up],
        &[Down, Up, Left],
        &[Up, Left],
        &[Left],
    ]);
    assert_eq!(answers, [(2, 2), (0, 0), (4, 4), (8, 7), (4, 4)]);
}

#[test]
fn a_diagonal_keeps_its_dir4_while_both_are_held() {
    let mut input = after(&[Button::Down]);
    input.receive(holding(&[Button::Down, Button::Right]));
    input.update();
    assert_eq!((input.dir4(), input.dir8()), (6, 3));
    for _ in 0..20 {
        input.update();
        assert_eq!((input.dir4(), input.dir8()), (6, 3));
    }
}

#[test]
fn the_horizontal_direction_wins_dir4_before_any_axis_was_held_alone() {
    for (vertical, horizontal) in [
        (Button::Down, Button::Left),
        (Button::Down, Button::Right),
        (Button::Up, Button::Left),
        (Button::Up, Button::Right),
    ] {
        let input = after(&[vertical, horizontal]);
        assert_eq!(
            input.dir4(),
            horizontal as u8,
            "{vertical:?} {horizontal:?}"
        );
    }
}

#[test]
fn all_four_diagonals_are_reported_by_dir8() {
    for (first, second, diagonal) in [
        (Button::Down, Button::Left, 1),
        (Button::Down, Button::Right, 3),
        (Button::Up, Button::Left, 7),
        (Button::Up, Button::Right, 9),
    ] {
        for (earlier, later) in [(first, second), (second, first)] {
            let mut input = after(&[earlier]);
            input.receive(holding(&[earlier, later]));
            input.update();
            assert_eq!(input.dir8(), diagonal, "{earlier:?} then {later:?}");
        }
        // Pressed in the same update, the diagonal is the same.
        assert_eq!(after(&[first, second]).dir8(), diagonal);
    }
}

#[test]
fn a_diagonal_with_a_cancelled_axis_is_no_diagonal() {
    let mut input = after(&[Button::Down, Button::Right]);
    assert_eq!(input.dir8(), 3);
    input.receive(holding(&[Button::Down, Button::Right, Button::Left]));
    input.update();
    assert_eq!((input.dir4(), input.dir8()), (2, 2));
}

#[test]
fn raw_keys_and_the_pointer_follow_the_latched_snapshot() {
    let mut input = InputState::new();
    input.update();
    assert!(!input.is_key_down(0x41));
    assert_eq!(input.pointer(), None);

    let held = InputSnapshot::EMPTY
        .with_key(0x41)
        .with_pointer(Some((100, 50)));
    input.receive(held);
    assert!(!input.is_key_down(0x41));
    assert_eq!(input.pointer(), None);
    input.update();
    assert!(input.is_key_down(0x41) && input.is_key_new(0x41));
    assert!(!input.is_key_down(0x42) && !input.is_key_new(0x42));
    assert_eq!(input.pointer(), Some((100, 50)));

    // Held on without a new snapshot, and with one: no longer new.
    input.update();
    assert!(input.is_key_down(0x41) && !input.is_key_new(0x41));
    input.receive(held.with_key(0x42));
    input.update();
    assert!(input.is_key_down(0x41) && !input.is_key_new(0x41));
    assert!(input.is_key_new(0x42));

    input.receive(InputSnapshot::EMPTY);
    input.update();
    assert!(!input.is_key_down(0x41) && !input.is_key_new(0x41));
    assert_eq!(input.pointer(), None);
}

#[test]
fn the_same_snapshots_give_the_same_answers() {
    let script = [
        Some(holding(&[Button::Down])),
        None,
        Some(holding(&[Button::Down, Button::Right, Button::C])),
        None,
        Some(holding(&[Button::Right])),
        Some(InputSnapshot::EMPTY),
        Some(holding(&[Button::Up, Button::Left])),
    ];
    let run = || {
        let mut input = InputState::new();
        let mut answers = Vec::new();
        for snapshot in script {
            if let Some(snapshot) = snapshot {
                input.receive(snapshot);
            }
            input.update();
            answers.push((
                input.dir4(),
                input.dir8(),
                Button::ALL.map(|button| {
                    (
                        input.is_pressed(button),
                        input.is_triggered(button),
                        input.is_repeated(button),
                    )
                }),
            ));
        }
        (answers, input)
    };
    assert_eq!(run(), run());
    let directions: Vec<(u8, u8)> = run().0.iter().map(|a| (a.0, a.1)).collect();
    assert_eq!(
        directions,
        [(2, 2), (2, 2), (6, 3), (6, 3), (6, 6), (0, 0), (8, 7)]
    );
}

// The timing of RGSS2.

/// The updates, counted from 1, at which `button` held alone repeats within `updates`.
fn repeating_updates(rules: Rules, updates: u32) -> Vec<u32> {
    let mut input = InputState::with_rules(rules);
    input.receive(holding(&[Button::C]));
    (1..=updates)
        .filter(|_| {
            input.update();
            input.is_repeated(Button::C)
        })
        .collect()
}

/// Scenarios "Repeat pattern, measured on the original" and "XP" of "Repeat timing of
/// RGSS2".
#[test]
fn the_repeat_timing_is_that_of_the_rules() {
    assert_eq!(repeating_updates(Rules::Rgss2, 36), [1, 24, 30, 36]);
    assert_eq!(repeating_updates(Rules::Rgss1, 24), [1, 16, 20, 24]);
    assert_eq!(InputState::new().rules(), Rules::Rgss1);
    assert_eq!(InputState::default().rules(), Rules::Rgss1);
}

const RGSS2_SAMPLES: &str = include_str!("../../tests/data/rgss2-input-samples.txt");

/// The buttons the digits of the samples stand for, in their order.
const SAMPLED: [Button; 9] = [
    Button::Down,
    Button::Left,
    Button::Right,
    Button::Up,
    Button::C,
    Button::B,
    Button::A,
    Button::X,
    Button::Shift,
];

/// One digit for each sampled button that `answer` holds for.
fn digits(answer: impl Fn(Button) -> bool) -> String {
    let digit = |button: &Button| if answer(*button) { '1' } else { '0' };
    SAMPLED.iter().map(digit).collect()
}

/// Scenario "Sequences of the measurement".
#[test]
fn the_sequences_measured_on_the_original_of_vx_give_the_measured_answers() {
    let mut input = InputState::with_rules(Rules::Rgss2);
    let mut stream = "";
    let mut checked = 0;
    for line in RGSS2_SAMPLES.lines().filter(|line| !line.starts_with('#')) {
        if let Some(name) = line.strip_prefix("stream ") {
            stream = name;
            input.receive(InputSnapshot::EMPTY);
            for _ in 0..8 {
                input.update();
            }
            continue;
        }
        let words: Vec<&str> = line.split(' ').collect();
        let times = words.get(6).map_or(1, |times| times[1..].parse().unwrap());
        for _ in 0..times {
            if words[0] == "update" {
                let held = SAMPLED.iter().zip(words[1].chars());
                let held: Vec<Button> = held
                    .filter(|(_, digit)| *digit == '1')
                    .map(|(button, _)| *button)
                    .collect();
                input.receive(holding(&held));
                input.update();
            } else {
                assert_eq!(words[0], "keep", "{line}");
            }
            let found = [
                digits(|button| input.is_pressed(button)),
                digits(|button| input.is_triggered(button)),
                digits(|button| input.is_repeated(button)),
                input.dir4().to_string(),
                input.dir8().to_string(),
            ];
            assert_eq!(found[..], words[1..6], "update {checked}, in `{stream}`");
            checked += 1;
        }
    }
    assert_eq!(checked, 12084);
}

// The rules of RGSS3.

/// The answers of `rules` to the samples of `text`, and the measured ones, for every
/// listed update: `press?`, `trigger?`, `repeat?`, `dir4` and `dir8`. A frame is shown
/// before each update.
fn replayed(text: &str, rules: Rules) -> Vec<([String; 5], [String; 5])> {
    let mut input = InputState::with_rules(rules);
    let mut answers = Vec::new();
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        if line.starts_with("stream ") {
            input.receive(InputSnapshot::EMPTY);
            for _ in 0..8 {
                input.frame_shown();
                input.update();
            }
            continue;
        }
        let words: Vec<&str> = line.split(' ').collect();
        let times = words.get(6).map_or(1, |times| times[1..].parse().unwrap());
        for _ in 0..times {
            if words[0] == "update" {
                let held = SAMPLED.iter().zip(words[1].chars());
                let held: Vec<Button> = held
                    .filter(|(_, digit)| *digit == '1')
                    .map(|(button, _)| *button)
                    .collect();
                input.receive(holding(&held));
                input.frame_shown();
                input.update();
            } else {
                assert_eq!(words[0], "keep", "{line}");
            }
            let found = [
                digits(|button| input.is_pressed(button)),
                digits(|button| input.is_triggered(button)),
                digits(|button| input.is_repeated(button)),
                input.dir4().to_string(),
                input.dir8().to_string(),
            ];
            let measured = std::array::from_fn(|index| words[1 + index].to_owned());
            answers.push((found, measured));
        }
    }
    answers
}

const RGSS3_SAMPLES: &str = include_str!("../../tests/data/rgss3-input-samples.txt");

/// Scenario "Sequences of the measurement" of "Input of RGSS3".
#[test]
fn the_sequences_measured_on_the_original_of_vx_ace_give_the_measured_answers() {
    let answers = replayed(RGSS3_SAMPLES, Rules::Rgss3);
    assert_eq!(answers.len(), 12084);
    for (update, (found, measured)) in answers.iter().enumerate() {
        assert_eq!(found, measured, "update {update}");
    }
}

/// Scenario "Not the timing of RGSS1".
#[test]
fn the_sequences_of_vx_ace_are_not_repeated_as_in_rgss1() {
    let answers = replayed(RGSS3_SAMPLES, Rules::Rgss1);
    assert!(
        answers
            .iter()
            .any(|(found, measured)| found[2] != measured[2])
    );
}

/// Scenario "Repeat pattern, measured on the original" of "Input of RGSS3".
#[test]
fn the_repeat_timing_of_rgss3_is_that_of_rgss2() {
    let mut input = InputState::with_rules(Rules::Rgss3);
    assert_eq!(input.rules(), Rules::Rgss3);
    input.receive(holding(&[Button::C]));
    let repeating: Vec<u32> = (1..=36)
        .filter(|_| {
            input.frame_shown();
            input.update();
            input.is_repeated(Button::C)
        })
        .collect();
    assert_eq!(repeating, [1, 24, 30, 36]);
}

/// Scenario "First diagonal of a new state, measured on the original" of "Input of RGSS3".
#[test]
fn the_first_diagonal_of_a_new_state_of_rgss3_gives_the_horizontal_direction() {
    let mut input = InputState::with_rules(Rules::Rgss3);
    input.receive(holding(&[Button::Left, Button::Up]));
    input.frame_shown();
    input.update();
    assert_eq!((input.dir4(), input.dir8()), (4, 7));
}

/// Scenarios "An update alone sees nothing new" and "Seen after a shown frame".
#[test]
fn in_rgss3_a_snapshot_is_seen_by_the_update_after_a_shown_frame() {
    let mut input = InputState::with_rules(Rules::Rgss3);
    input.receive(holding(&[Button::C]));
    for _ in 0..3 {
        input.update();
        assert!(!input.is_pressed(Button::C));
    }
    input.frame_shown();
    assert!(
        !input.is_pressed(Button::C),
        "only an update changes the state"
    );
    input.update();
    assert!(input.is_pressed(Button::C) && input.is_triggered(Button::C));
}

/// Scenario "Arrived after the frame".
#[test]
fn in_rgss3_a_snapshot_that_arrives_after_a_frame_waits_for_the_next() {
    let mut input = InputState::with_rules(Rules::Rgss3);
    input.frame_shown();
    input.receive(holding(&[Button::C]));
    input.update();
    assert!(!input.is_pressed(Button::C));
    input.frame_shown();
    input.update();
    assert!(input.is_pressed(Button::C) && input.is_triggered(Button::C));
}

/// Scenario "Several snapshots before one frame".
#[test]
fn in_rgss3_a_shown_frame_shows_the_last_of_several_snapshots() {
    let mut input = InputState::with_rules(Rules::Rgss3);
    input.receive(holding(&[Button::C]));
    input.receive(InputSnapshot::EMPTY);
    input.frame_shown();
    input.update();
    assert!(!input.is_pressed(Button::C) && !input.is_triggered(Button::C));
}

/// Scenario "Several frames before one update".
#[test]
fn in_rgss3_a_snapshot_stays_visible_over_several_frames() {
    let mut input = InputState::with_rules(Rules::Rgss3);
    input.receive(holding(&[Button::C]));
    for _ in 0..12 {
        input.frame_shown();
    }
    input.update();
    assert!(input.is_pressed(Button::C) && input.is_triggered(Button::C));
}

/// Scenario "RGSS1 and RGSS2" of "Snapshots are seen at shown frames in RGSS3".
#[test]
fn in_rgss1_and_rgss2_a_shown_frame_changes_nothing() {
    for rules in [Rules::Rgss1, Rules::Rgss2] {
        for shown in [false, true] {
            let mut input = InputState::with_rules(rules);
            input.receive(holding(&[Button::C]));
            if shown {
                input.frame_shown();
            }
            input.update();
            assert!(
                input.is_pressed(Button::C) && input.is_triggered(Button::C),
                "{rules:?}, shown {shown}"
            );
        }
    }
}
