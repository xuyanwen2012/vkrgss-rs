//! Input delivered from the host to the guest.

use serde::{Deserialize, Serialize};

/// A logical RGSS button. The discriminant is the value of the matching `Input` constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Button {
    Down = 2,
    Left = 4,
    Right = 6,
    Up = 8,
    A = 11,
    B = 12,
    C = 13,
    X = 14,
    Y = 15,
    Z = 16,
    L = 17,
    R = 18,
    Shift = 21,
    Ctrl = 22,
    Alt = 23,
    F5 = 25,
    F6 = 26,
    F7 = 27,
    F8 = 28,
}

impl Button {
    /// Every button, in ascending order of its `Input` constant.
    pub const ALL: [Button; 19] = [
        Button::Down,
        Button::Left,
        Button::Right,
        Button::Up,
        Button::A,
        Button::B,
        Button::C,
        Button::X,
        Button::Y,
        Button::Z,
        Button::L,
        Button::R,
        Button::Shift,
        Button::Ctrl,
        Button::Alt,
        Button::F5,
        Button::F6,
        Button::F7,
        Button::F8,
    ];

    const fn mask(self) -> u32 {
        1 << self as u8
    }
}

/// The complete logical button state at one sampling point.
///
/// The guest samples input once per `Input.update`; a recording is the sequence of those
/// snapshots. Bit `n` of `buttons` is set while the button whose `Input` constant is `n` is
/// held.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InputSnapshot {
    pub buttons: u32,
}

impl InputSnapshot {
    /// No button held.
    pub const EMPTY: InputSnapshot = InputSnapshot { buttons: 0 };

    /// Whether `button` is held.
    pub const fn is_down(&self, button: Button) -> bool {
        self.buttons & button.mask() != 0
    }

    /// This snapshot with `button` held.
    #[must_use]
    pub const fn with(self, button: Button) -> InputSnapshot {
        InputSnapshot {
            buttons: self.buttons | button.mask(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_are_independent_bits() {
        let mut seen = 0u32;
        for button in Button::ALL {
            assert_eq!(seen & button.mask(), 0, "{button:?} shares a bit");
            seen |= button.mask();
        }
    }

    #[test]
    fn snapshot_tracks_each_button() {
        let snapshot = InputSnapshot::EMPTY.with(Button::Down).with(Button::C);
        for button in Button::ALL {
            let expected = matches!(button, Button::Down | Button::C);
            assert_eq!(snapshot.is_down(button), expected, "{button:?}");
        }
        assert_eq!(snapshot.buttons, (1 << 2) | (1 << 13));
    }
}
