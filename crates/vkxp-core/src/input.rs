//! `InputState`: what `Input` answers, derived from the snapshots the host delivers.

use vkxp_proto::{Button, InputSnapshot};

use crate::Rules;

/// The repeat timing of `rules`: the number of consecutive updates the repeating button is
/// held before it starts to repeat, and the number of updates between two repeats.
///
/// Provenance of RGSS2: oracle measurement 2026-10-12, RGSS202E versions 2.0.0.1 and 2.0.2.1,
/// `vx-v1-sprite-REPORT.md` section 7 (12084 updates without a mismatch; the timing of RGSS1
/// leaves 821 mismatches of `repeat?`).
fn repeat_timing(rules: Rules) -> (u64, u64) {
    match rules {
        Rules::Rgss1 => (16, 4),
        Rules::Rgss2 | Rules::Rgss3 => (24, 6),
    }
}

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
    /// The snapshot the queries answered from before the last update.
    previous: InputSnapshot,
    /// For each button, the number of consecutive updates it has been held.
    held: [u64; BUTTON_BITS],
    /// The one button that repeats: the one triggered most recently, while it stays held.
    rep_button: Option<Button>,
    /// Number of consecutive updates the repeating button has been held since its trigger.
    rep_count: u64,
    /// Which direction a diagonal gives the four-way query: the horizontal one when unset,
    /// the vertical one when set. A horizontal direction held alone sets it and a vertical
    /// one held alone clears it; nothing else changes it, not even a release of everything.
    axis_flag: bool,
    /// The four-way direction as of the last update.
    dir4: u8,
    /// The runtime whose repeat timing the state follows.
    rules: Rules,
}

impl InputState {
    /// The state before any input: nothing held. It follows the rules of RGSS1.
    pub fn new() -> InputState {
        InputState::default()
    }

    /// The state before any input, which follows `rules`.
    pub fn with_rules(rules: Rules) -> InputState {
        InputState {
            rules,
            ..InputState::default()
        }
    }

    /// The runtime whose rules the state follows.
    pub fn rules(&self) -> Rules {
        self.rules
    }

    /// Takes a snapshot from the host. It has no effect until the next update, and only the
    /// last snapshot received before an update counts.
    pub fn receive(&mut self, snapshot: InputSnapshot) {
        self.received = Some(snapshot);
    }

    /// One `Input.update`: latches the most recent snapshot, or keeps the button state when
    /// none was received, and advances every button by one step.
    pub fn update(&mut self) {
        self.previous = self.latched;
        if let Some(snapshot) = self.received.take() {
            self.latched = snapshot;
        }
        for button in Button::ALL {
            let bit = button as usize;
            if self.latched.is_down(button) {
                self.held[bit] += 1;
                // Of several buttons triggered in one update the highest number is the last
                // to be seen here, and is the one that repeats.
                if self.held[bit] == 1 {
                    self.rep_button = Some(button);
                    self.rep_count = 0;
                }
            } else {
                self.held[bit] = 0;
            }
        }
        if self
            .rep_button
            .is_some_and(|button| self.is_pressed(button))
        {
            self.rep_count += 1;
        } else {
            self.rep_button = None;
            self.rep_count = 0;
        }
        self.dir4 = self.update_dir4();
    }

    /// The four-way direction of this update, and the axis flag for the next diagonal.
    fn update_dir4(&mut self) -> u8 {
        let direction = match self.axes() {
            (Some(horizontal), Some(vertical)) => {
                if self.axis_flag {
                    Some(vertical)
                } else {
                    Some(horizontal)
                }
            }
            (Some(horizontal), None) => {
                self.axis_flag = true;
                Some(horizontal)
            }
            (None, Some(vertical)) => {
                self.axis_flag = false;
                Some(vertical)
            }
            (None, None) => None,
        };
        direction.map_or(0, |button| button as u8)
    }

    /// Whether the key with the Windows virtual-key code `key` is held.
    pub fn is_key_down(&self, key: u8) -> bool {
        self.latched.is_key_down(key)
    }

    /// Whether the key with the Windows virtual-key code `key` is held and was not before the
    /// last update.
    pub fn is_key_new(&self, key: u8) -> bool {
        self.latched.is_key_down(key) && !self.previous.is_key_down(key)
    }

    /// The pointer position in logical frame pixels, or `None` when the pointer is not over
    /// the picture.
    pub fn pointer(&self) -> Option<(i16, i16)> {
        self.latched.pointer
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

    /// Whether `button` is the repeating button and repeats at the last update: on the update
    /// in which it was triggered, on the 15th update after that, and on every 4th after that;
    /// by the rules of RGSS2 on the 23rd update after it and on every 6th after that.
    /// Only one button repeats at a time: the one triggered most recently, and of several
    /// triggered in the same update the one with the highest number. A button that lost the
    /// repeat to another does not get it back while it stays held. Measured on the original
    /// runtime.
    pub fn is_repeated(&self, button: Button) -> bool {
        let count = self.rep_count;
        let (delay, interval) = repeat_timing(self.rules);
        self.rep_button == Some(button)
            && (count == 1 || (count >= delay && (count - delay).is_multiple_of(interval)))
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
    /// Opposite directions cancel. Of two perpendicular directions the one counts whose axis
    /// was not the last to be held alone, and the horizontal one when neither axis has been
    /// held alone yet. Measured on the original runtime.
    pub fn dir4(&self) -> u8 {
        self.dir4
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
mod tests;
