//! `InputState`: what `Input` answers, derived from the snapshots the host delivers.

use vkxp_proto::{Button, InputSnapshot};

/// Number of consecutive updates a button is held before it starts to repeat.
const REPEAT_DELAY: u64 = 16;
/// Number of updates between two repeats.
const REPEAT_INTERVAL: u64 = 4;

/// One counter per bit of a snapshot; the discriminant of a [`Button`] is its bit.
const BUTTON_BITS: usize = 32;

/// The button state scripts query through `Input`.
///
/// The host delivers snapshots whenever it likes through [`InputState::receive`]. The state
/// changes only in [`InputState::update`], which takes the most recent snapshot, so every
/// query between two updates gives the same answer, and the same snapshots at the same
/// updates always give the same answers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputState {
    /// The most recent snapshot received since the last update.
    received: Option<InputSnapshot>,
    /// The snapshot the queries answer from.
    latched: InputSnapshot,
    /// For each button, the number of consecutive updates it has been held.
    held: [u64; BUTTON_BITS],
    /// For each button, the number of the update at which it was last triggered.
    triggered_at: [u64; BUTTON_BITS],
    /// Number of updates so far.
    updates: u64,
}

impl InputState {
    /// The state before any input: nothing held.
    pub fn new() -> InputState {
        InputState::default()
    }

    /// Takes a snapshot from the host. It has no effect until the next update, and only the
    /// last snapshot received before an update counts.
    pub fn receive(&mut self, snapshot: InputSnapshot) {
        self.received = Some(snapshot);
    }

    /// One `Input.update`: latches the most recent snapshot, or keeps the button state when
    /// none was received, and advances every button by one step.
    pub fn update(&mut self) {
        if let Some(snapshot) = self.received.take() {
            self.latched = snapshot;
        }
        self.updates += 1;
        for button in Button::ALL {
            let bit = button as usize;
            if self.latched.is_down(button) {
                self.held[bit] += 1;
                if self.held[bit] == 1 {
                    self.triggered_at[bit] = self.updates;
                }
            } else {
                self.held[bit] = 0;
            }
        }
    }

    /// Number of consecutive updates `button` has been held, 0 when it is not held.
    fn held(&self, button: Button) -> u64 {
        self.held[button as usize]
    }

    /// Whether `button` is held.
    pub fn is_pressed(&self, button: Button) -> bool {
        self.held(button) > 0
    }

    /// Whether `button` went from not held to held in the last update.
    pub fn is_triggered(&self, button: Button) -> bool {
        self.held(button) == 1
    }

    /// Whether `button` was triggered in the last update, or has been held long enough to
    /// repeat: on the 15th update after the trigger, and on every 4th after that.
    ///
    /// PLACEHOLDER: the repeat timing is not yet checked against the original runtime.
    pub fn is_repeated(&self, button: Button) -> bool {
        let held = self.held(button);
        held == 1 || (held >= REPEAT_DELAY && (held - REPEAT_DELAY).is_multiple_of(REPEAT_INTERVAL))
    }

    /// Of two opposite directions, the one held alone.
    fn axis(&self, negative: Button, positive: Button) -> Option<Button> {
        match (self.is_pressed(negative), self.is_pressed(positive)) {
            (true, false) => Some(negative),
            (false, true) => Some(positive),
            _ => None,
        }
    }

    /// The direction held on each axis, horizontal then vertical. Opposite directions held
    /// together cancel.
    fn axes(&self) -> (Option<Button>, Option<Button>) {
        (
            self.axis(Button::Left, Button::Right),
            self.axis(Button::Up, Button::Down),
        )
    }

    /// The four-way direction: 2 for down, 4 for left, 6 for right, 8 for up, 0 for none.
    /// Opposite directions cancel; of two perpendicular directions the one pressed more
    /// recently counts.
    ///
    /// PLACEHOLDER: the priority among directions, in particular that the vertical one wins
    /// when both were pressed in the same update, is not yet checked against the original
    /// runtime.
    pub fn dir4(&self) -> u8 {
        let direction = match self.axes() {
            (Some(horizontal), Some(vertical)) => {
                let pressed = |button: Button| self.triggered_at[button as usize];
                if pressed(horizontal) > pressed(vertical) {
                    Some(horizontal)
                } else {
                    Some(vertical)
                }
            }
            (horizontal, vertical) => horizontal.or(vertical),
        };
        direction.map_or(0, |button| button as u8)
    }

    /// The eight-way direction: 1 for down-left, 3 for down-right, 7 for up-left and 9 for
    /// up-right when two perpendicular directions are held, otherwise [`InputState::dir4`].
    pub fn dir8(&self) -> u8 {
        match self.axes() {
            (Some(Button::Left), Some(Button::Down)) => 1,
            (Some(Button::Right), Some(Button::Down)) => 3,
            (Some(Button::Left), Some(Button::Up)) => 7,
            (Some(Button::Right), Some(Button::Up)) => 9,
            _ => self.dir4(),
        }
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn of_two_perpendicular_directions_the_later_press_wins_dir4() {
        let mut input = after(&[Button::Down]);
        input.receive(holding(&[Button::Down, Button::Right]));
        input.update();
        assert_eq!((input.dir4(), input.dir8()), (6, 3));
        // It stays that way while both are held.
        for _ in 0..20 {
            input.update();
            assert_eq!((input.dir4(), input.dir8()), (6, 3));
        }

        let mut input = after(&[Button::Right]);
        input.receive(holding(&[Button::Down, Button::Right]));
        input.update();
        assert_eq!((input.dir4(), input.dir8()), (2, 3));

        // Pressing the first one again makes it the more recent.
        input.receive(holding(&[Button::Down]));
        input.update();
        assert_eq!((input.dir4(), input.dir8()), (2, 2));
        input.receive(holding(&[Button::Down, Button::Right]));
        input.update();
        assert_eq!((input.dir4(), input.dir8()), (6, 3));
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
                assert_eq!(input.dir4(), later as u8, "{earlier:?} then {later:?}");
            }
            // Pressed in the same update, the diagonal is the same.
            let input = after(&[first, second]);
            assert_eq!(input.dir8(), diagonal);
            assert_eq!(input.dir4(), first as u8, "the vertical direction");
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
}
