//! The map from physical keys to the buttons of `Input`, with the defaults of each runtime.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::IntoDeserializer;
use serde::de::value::Error as NameError;
use thiserror::Error;
use vkrgss_proto::Button;
use winit::keyboard::KeyCode;

use crate::script::button_named;

/// Why a map could not be built from the names in a configuration.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MapError {
    #[error("unknown button `{0}`")]
    UnknownButton(String),
    #[error("unknown key `{0}`")]
    UnknownKey(String),
    #[error("unknown gamepad button `{0}`")]
    UnknownPadButton(String),
}

/// The key with the name the windowing library gives it, such as `KeyJ`, `Space` or
/// `ArrowDown`.
fn key_named(name: &str) -> Option<KeyCode> {
    KeyCode::deserialize(IntoDeserializer::<NameError>::into_deserializer(name)).ok()
}

/// Whether two lists without repetitions hold the same bindings.
pub(crate) fn same_bindings<T: PartialEq>(one: &[T], other: &[T]) -> bool {
    one.len() == other.len() && one.iter().all(|binding| other.contains(binding))
}

/// `defaults` with the bindings of every button in `overrides` replaced by the ones listed
/// for it. Shared by the key map and the gamepad map.
pub(crate) fn with_overrides<T: Copy + PartialEq>(
    defaults: &[(T, Button)],
    overrides: &BTreeMap<String, Vec<String>>,
    named: impl Fn(&str) -> Result<T, MapError>,
) -> Result<Vec<(T, Button)>, MapError> {
    let mut bindings = defaults.to_vec();
    for (name, sources) in overrides {
        let button = button_named(name).ok_or_else(|| MapError::UnknownButton(name.to_owned()))?;
        bindings.retain(|(_, bound)| *bound != button);
        for source in sources {
            let binding = (named(source)?, button);
            if !bindings.contains(&binding) {
                bindings.push(binding);
            }
        }
    }
    Ok(bindings)
}

/// Which buttons each physical key holds down. One key may hold several buttons and several
/// keys may hold the same one.
#[derive(Debug, Clone)]
pub struct KeyMap {
    bindings: Vec<(KeyCode, Button)>,
}

/// Two maps are equal when they hold the same bindings, in whatever order.
impl PartialEq for KeyMap {
    fn eq(&self, other: &KeyMap) -> bool {
        same_bindings(&self.bindings, &other.bindings)
    }
}

impl Eq for KeyMap {}

/// Which original runtime's keys a map starts from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KeyDefaults {
    /// Those of RPG Maker XP (RGSS1).
    #[default]
    Xp,
    /// Those of RPG Maker VX (RGSS2).
    Vx,
}

impl KeyDefaults {
    fn bindings(self) -> &'static [(KeyCode, Button)] {
        match self {
            KeyDefaults::Xp => &DEFAULTS,
            KeyDefaults::Vx => &DEFAULTS_VX,
        }
    }
}

/// The default bindings of a game of RGSS1, those of the original runtime.
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

/// The default bindings of a game of RGSS2: every key that holds a button in the original
/// runtime of VX, and no other. Against those of RGSS1, Z confirms and the C key holds
/// nothing. Shift holds A and SHIFT together, as measured.
///
/// Provenance: oracle measurement 2026-10-12, RGSS202E versions 2.0.0.1 and 2.0.2.1,
/// `vx-v1-sprite-REPORT.md` section 7 (`vx-v1-input-keys.txt`: every virtual key from 8 to
/// 254 but F1, F12, the Windows keys and PrintScreen, one at a time). The Enter key of the
/// numeric pad has the virtual key of Enter.
const DEFAULTS_VX: [(KeyCode, Button); 36] = [
    (KeyCode::ArrowDown, Button::Down),
    (KeyCode::ArrowLeft, Button::Left),
    (KeyCode::ArrowRight, Button::Right),
    (KeyCode::ArrowUp, Button::Up),
    (KeyCode::Numpad2, Button::Down),
    (KeyCode::Numpad4, Button::Left),
    (KeyCode::Numpad6, Button::Right),
    (KeyCode::Numpad8, Button::Up),
    (KeyCode::Enter, Button::C),
    (KeyCode::NumpadEnter, Button::C),
    (KeyCode::Space, Button::C),
    (KeyCode::KeyZ, Button::C),
    (KeyCode::Escape, Button::B),
    (KeyCode::Insert, Button::B),
    (KeyCode::KeyX, Button::B),
    (KeyCode::Numpad0, Button::B),
    (KeyCode::ShiftLeft, Button::A),
    (KeyCode::ShiftRight, Button::A),
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

    /// The default map of a game of RGSS1 with the keys of every button in `overrides`
    /// replaced: the button, by its `Input` name, is held by exactly the keys listed for it,
    /// by the names of the windowing library's key codes. Buttons that are not mentioned keep
    /// their default keys.
    pub fn with_overrides(overrides: &BTreeMap<String, Vec<String>>) -> Result<KeyMap, MapError> {
        KeyMap::of(KeyDefaults::Xp, overrides)
    }

    /// The map `defaults` with the keys of every button in `overrides` replaced, as
    /// [`KeyMap::with_overrides`] replaces them.
    pub fn of(
        defaults: KeyDefaults,
        overrides: &BTreeMap<String, Vec<String>>,
    ) -> Result<KeyMap, MapError> {
        let bindings = with_overrides(defaults.bindings(), overrides, |name| {
            key_named(name).ok_or_else(|| MapError::UnknownKey(name.to_owned()))
        })?;
        Ok(KeyMap { bindings })
    }

    /// The default map of a game of RGSS2: the keys of the original runtime of VX.
    pub fn vx() -> KeyMap {
        KeyMap {
            bindings: DEFAULTS_VX.to_vec(),
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

    /// Requirement "Keys of VX": the keys the original runtime of VX maps, and no other.
    #[test]
    fn the_map_of_vx_matches_the_specification() {
        use Button as B;
        use KeyCode as K;
        let map = KeyMap::vx();
        let held = |key| map.buttons(key).collect::<Vec<_>>();
        let expected: [(&[KeyCode], &[Button]); 14] = [
            (&[K::ArrowDown, K::Numpad2], &[B::Down]),
            (&[K::ArrowLeft, K::Numpad4], &[B::Left]),
            (&[K::ArrowRight, K::Numpad6], &[B::Right]),
            (&[K::ArrowUp, K::Numpad8], &[B::Up]),
            (&[K::Enter, K::NumpadEnter, K::Space, K::KeyZ], &[B::C]),
            (&[K::Escape, K::Insert, K::KeyX, K::Numpad0], &[B::B]),
            (&[K::ShiftLeft, K::ShiftRight], &[B::A, B::Shift]),
            (&[K::KeyA], &[B::X]),
            (&[K::KeyS], &[B::Y]),
            (&[K::KeyD], &[B::Z]),
            (&[K::KeyQ, K::PageUp], &[B::L]),
            (&[K::KeyW, K::PageDown], &[B::R]),
            (&[K::ControlLeft, K::ControlRight], &[B::Ctrl]),
            (&[K::AltLeft, K::AltRight], &[B::Alt]),
        ];
        let mut bound = 0;
        for (keys, buttons) in expected {
            for key in keys {
                assert_eq!(held(*key), buttons, "{key:?}");
                bound += buttons.len();
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
            assert_eq!(held(key), [button], "{key:?}");
            bound += 1;
        }
        assert_eq!(map.bindings.len(), bound, "no other key holds a button");
        // Keys measured to hold nothing, among them the C key that confirms in XP.
        for key in [K::KeyC, K::KeyV, K::KeyB, K::Numpad1, K::Numpad5, K::Tab] {
            assert_eq!(held(key), [], "{key:?}");
        }
        for button in Button::ALL {
            assert!(map.bindings.iter().any(|(_, bound)| *bound == button));
        }
    }

    #[test]
    fn overrides_replace_the_keys_of_the_chosen_defaults() {
        let none = BTreeMap::new();
        assert_eq!(KeyMap::of(KeyDefaults::Vx, &none), Ok(KeyMap::vx()));
        assert_eq!(KeyMap::of(KeyDefaults::Xp, &none), Ok(KeyMap::default()));
        assert_ne!(KeyMap::vx(), KeyMap::default());
        let map = KeyMap::of(KeyDefaults::Vx, &overrides(&[("C", &["KeyJ"])])).unwrap();
        let held = |key| map.buttons(key).collect::<Vec<_>>();
        assert_eq!(held(KeyCode::KeyJ), [Button::C]);
        assert_eq!(held(KeyCode::KeyZ), []);
        assert_eq!(held(KeyCode::KeyX), [Button::B]);
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

    fn overrides(entries: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        entries
            .iter()
            .map(|(button, keys)| {
                let keys = keys.iter().map(|key| (*key).to_owned()).collect();
                ((*button).to_owned(), keys)
            })
            .collect()
    }

    #[test]
    fn no_overrides_is_the_default_map() {
        assert_eq!(
            KeyMap::with_overrides(&BTreeMap::new()),
            Ok(KeyMap::default())
        );
    }

    /// Scenario "Remapped button".
    #[test]
    fn a_remapped_button_is_held_by_exactly_its_keys() {
        let map = KeyMap::with_overrides(&overrides(&[("C", &["KeyJ"])])).unwrap();
        let held = |key| map.buttons(key).collect::<Vec<_>>();
        assert_eq!(held(KeyCode::KeyJ), [Button::C]);
        for key in [
            KeyCode::Space,
            KeyCode::Enter,
            KeyCode::NumpadEnter,
            KeyCode::KeyC,
        ] {
            assert_eq!(held(key), [], "{key:?}");
        }
        assert_eq!(held(KeyCode::ArrowDown), [Button::Down]);
        assert_eq!(held(KeyCode::ArrowLeft), [Button::Left]);
        assert_eq!(held(KeyCode::ArrowRight), [Button::Right]);
        assert_eq!(held(KeyCode::ArrowUp), [Button::Up]);
        assert_eq!(held(KeyCode::Escape), [Button::B]);
    }

    #[test]
    fn a_remapped_button_leaves_the_other_buttons_of_its_keys() {
        let map = KeyMap::with_overrides(&overrides(&[("a", &["KeyZ"]), ("B", &[])])).unwrap();
        let held = |key| map.buttons(key).collect::<Vec<_>>();
        assert_eq!(held(KeyCode::ShiftLeft), [Button::Shift]);
        assert_eq!(held(KeyCode::KeyZ), [Button::A]);
        assert_eq!(held(KeyCode::Escape), []);
    }

    /// Scenario "Unknown key name".
    #[test]
    fn an_unknown_key_name_is_an_error_naming_it() {
        let error = KeyMap::with_overrides(&overrides(&[("C", &["KeyJ", "Jay"])])).unwrap_err();
        assert_eq!(error, MapError::UnknownKey("Jay".to_owned()));
        assert!(error.to_string().contains("Jay"), "{error}");
    }

    #[test]
    fn an_unknown_button_name_is_an_error_naming_it() {
        let error = KeyMap::with_overrides(&overrides(&[("JUMP", &["KeyJ"])])).unwrap_err();
        assert_eq!(error, MapError::UnknownButton("JUMP".to_owned()));
        assert!(error.to_string().contains("JUMP"), "{error}");
    }
}
