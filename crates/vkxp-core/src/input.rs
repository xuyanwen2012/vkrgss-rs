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
    /// The snapshot the queries answered from before the last update.
    previous: InputSnapshot,
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
        self.previous = self.latched;
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
mod tests;
