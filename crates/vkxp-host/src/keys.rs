//! The map from physical keys to RGSS1 buttons.

use vkxp_proto::Button;
use winit::keyboard::KeyCode;

/// Which buttons each physical key holds down. One key may hold several buttons and several
/// keys may hold the same one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap {
    bindings: Vec<(KeyCode, Button)>,
}

/// The default bindings, those of the original runtime.
const DEFAULTS: [(KeyCode, Button); 37] = [
    (KeyCode::ArrowDown, Button::Down),
    (KeyCode::ArrowLeft, Button::Left),
    (KeyCode::ArrowRight, Button::Right),
    (KeyCode::ArrowUp, Button::Up),
    (KeyCode::Numpad2, Button::Down),
    (KeyCode::Numpad4, Button::Left),
    (KeyCode::Numpad6, Button::Right),
    (KeyCode::Numpad8, Button::Up),
    (KeyCode::Space, Button::C),
    (KeyCode::Enter, Button::C),
    (KeyCode::NumpadEnter, Button::C),
    (KeyCode::KeyC, Button::C),
    (KeyCode::Escape, Button::B),
    (KeyCode::Numpad0, Button::B),
    (KeyCode::Insert, Button::B),
    (KeyCode::KeyX, Button::B),
    (KeyCode::ShiftLeft, Button::A),
    (KeyCode::ShiftRight, Button::A),
    (KeyCode::KeyZ, Button::A),
    (KeyCode::KeyA, Button::X),
    (KeyCode::KeyS, Button::Y),
    (KeyCode::KeyD, Button::Z),
    (KeyCode::KeyQ, Button::L),
    (KeyCode::PageUp, Button::L),
    (KeyCode::KeyW, Button::R),
    (KeyCode::PageDown, Button::R),
    (KeyCode::ShiftLeft, Button::Shift),
    (KeyCode::ShiftRight, Button::Shift),
    (KeyCode::ControlLeft, Button::Ctrl),
    (KeyCode::ControlRight, Button::Ctrl),
    (KeyCode::AltLeft, Button::Alt),
    (KeyCode::AltRight, Button::Alt),
    (KeyCode::F5, Button::F5),
    (KeyCode::F6, Button::F6),
    (KeyCode::F7, Button::F7),
    (KeyCode::F8, Button::F8),
    (KeyCode::F9, Button::F9),
];

impl KeyMap {
    /// A map without any binding.
    pub fn empty() -> KeyMap {
        KeyMap {
            bindings: Vec::new(),
        }
    }

    /// Makes `key` hold `button`, in addition to whatever it holds already.
    pub fn bind(&mut self, key: KeyCode, button: Button) {
        if !self.bindings.contains(&(key, button)) {
            self.bindings.push((key, button));
        }
    }

    /// The buttons `key` holds down.
    pub fn buttons(&self, key: KeyCode) -> impl Iterator<Item = Button> + '_ {
        self.bindings
            .iter()
            .filter(move |(bound, _)| *bound == key)
            .map(|(_, button)| *button)
    }
}

impl Default for KeyMap {
    fn default() -> KeyMap {
        KeyMap {
            bindings: DEFAULTS.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buttons(key: KeyCode) -> Vec<Button> {
        KeyMap::default().buttons(key).collect()
    }

    #[test]
    fn default_map_matches_the_specification() {
        use Button as B;
        use KeyCode as K;
        let expected: [(&[KeyCode], &[Button]); 15] = [
            (&[K::ArrowDown, K::Numpad2], &[B::Down]),
            (&[K::ArrowLeft, K::Numpad4], &[B::Left]),
            (&[K::ArrowRight, K::Numpad6], &[B::Right]),
            (&[K::ArrowUp, K::Numpad8], &[B::Up]),
            (&[K::Space, K::Enter, K::NumpadEnter, K::KeyC], &[B::C]),
            (&[K::Escape, K::Numpad0, K::Insert, K::KeyX], &[B::B]),
            (&[K::ShiftLeft, K::ShiftRight], &[B::A, B::Shift]),
            (&[K::KeyZ], &[B::A]),
            (&[K::KeyA], &[B::X]),
            (&[K::KeyS], &[B::Y]),
            (&[K::KeyD], &[B::Z]),
            (&[K::KeyQ, K::PageUp], &[B::L]),
            (&[K::KeyW, K::PageDown], &[B::R]),
            (&[K::ControlLeft, K::ControlRight], &[B::Ctrl]),
            (&[K::AltLeft, K::AltRight], &[B::Alt]),
        ];
        for (keys, held) in expected {
            for key in keys {
                assert_eq!(buttons(*key), held, "{key:?}");
            }
        }
        let function_keys = [
            (K::F5, B::F5),
            (K::F6, B::F6),
            (K::F7, B::F7),
            (K::F8, B::F8),
            (K::F9, B::F9),
        ];
        for (key, button) in function_keys {
            assert_eq!(buttons(key), [button], "{key:?}");
        }
    }

    #[test]
    fn every_button_is_reachable_and_host_keys_hold_none() {
        let map = KeyMap::default();
        for button in Button::ALL {
            assert!(
                map.bindings.iter().any(|(_, bound)| *bound == button),
                "{button:?}"
            );
        }
        for key in [KeyCode::F12, KeyCode::F2, KeyCode::Tab, KeyCode::KeyP] {
            assert_eq!(buttons(key), [], "{key:?}");
        }
    }

    #[test]
    fn binding_adds_to_what_a_key_holds() {
        let mut map = KeyMap::empty();
        assert_eq!(map.buttons(KeyCode::KeyJ).count(), 0);
        map.bind(KeyCode::KeyJ, Button::C);
        map.bind(KeyCode::KeyJ, Button::C);
        map.bind(KeyCode::KeyJ, Button::A);
        assert_eq!(
            map.buttons(KeyCode::KeyJ).collect::<Vec<_>>(),
            [Button::C, Button::A]
        );
    }
}
