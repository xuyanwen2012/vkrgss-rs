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
    F9 = 29,
}

impl Button {
    /// Every button, in ascending order of its `Input` constant.
    pub const ALL: [Button; 20] = [
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
        Button::F9,
    ];

    const fn mask(self) -> u32 {
        1 << self as u8
    }
}

/// The complete input state at one sampling point.
///
/// The guest samples input once per `Input.update`; a recording is the sequence of those
/// snapshots. Bit `n` of `buttons` is set while the button whose `Input` constant is `n` is
/// held.
///
/// `keys` is the raw state behind the emulated `Win32API` keyboard functions: one bit for
/// each of the 256 Windows virtual-key codes, mouse buttons included, code `n` at bit
/// `n % 64` of word `n / 64`. `pointer` is the position of the mouse pointer in logical frame
/// pixels, or `None` when it is not over the picture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InputSnapshot {
    pub buttons: u32,
    pub keys: [u64; 4],
    pub pointer: Option<(i16, i16)>,
}

impl InputSnapshot {
    /// No button and no key held, no pointer.
    pub const EMPTY: InputSnapshot = InputSnapshot {
        buttons: 0,
        keys: [0; 4],
        pointer: None,
    };

    /// Whether `button` is held.
    pub const fn is_down(&self, button: Button) -> bool {
        self.buttons & button.mask() != 0
    }

    /// This snapshot with `button` held.
    #[must_use]
    pub const fn with(self, button: Button) -> InputSnapshot {
        InputSnapshot {
            buttons: self.buttons | button.mask(),
            ..self
        }
    }

    /// Whether the key with the Windows virtual-key code `key` is held.
    pub const fn is_key_down(&self, key: u8) -> bool {
        self.keys[key as usize / 64] & (1 << (key % 64)) != 0
    }

    /// This snapshot with the key with the Windows virtual-key code `key` held.
    #[must_use]
    pub const fn with_key(mut self, key: u8) -> InputSnapshot {
        self.keys[key as usize / 64] |= 1 << (key % 64);
        self
    }

    /// This snapshot with the pointer at `pointer`.
    #[must_use]
    pub const fn with_pointer(self, pointer: Option<(i16, i16)>) -> InputSnapshot {
        InputSnapshot { pointer, ..self }
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
    fn there_are_twenty_buttons_with_the_values_of_their_constants() {
        let values = Button::ALL.map(|button| button as u8);
        assert_eq!(
            values,
            [
                2, 4, 6, 8, 11, 12, 13, 14, 15, 16, 17, 18, 21, 22, 23, 25, 26, 27, 28, 29
            ]
        );
        assert_eq!(Button::F9 as u8, 29);
        assert_eq!(Button::ALL[19], Button::F9);
    }

    #[test]
    fn each_button_alone_is_the_only_one_held() {
        for held in Button::ALL {
            let snapshot = InputSnapshot::EMPTY.with(held);
            assert_eq!(snapshot.buttons, 1 << held as u8);
            for button in Button::ALL {
                assert_eq!(snapshot.is_down(button), button == held, "{button:?}");
            }
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

    #[test]
    fn the_empty_snapshot_has_no_key_and_no_pointer() {
        assert_eq!(InputSnapshot::EMPTY, InputSnapshot::default());
        assert!((0..=255).all(|key| !InputSnapshot::EMPTY.is_key_down(key)));
        assert_eq!(InputSnapshot::EMPTY.pointer, None);
    }

    #[test]
    fn each_key_alone_is_the_only_one_held() {
        for held in 0..=255u8 {
            let snapshot = InputSnapshot::EMPTY.with_key(held);
            assert_eq!(snapshot.buttons, 0);
            assert_eq!(snapshot.keys.map(u64::count_ones).iter().sum::<u32>(), 1);
            for key in 0..=255u8 {
                assert_eq!(snapshot.is_key_down(key), key == held, "{key:#04x}");
            }
        }
    }
}
