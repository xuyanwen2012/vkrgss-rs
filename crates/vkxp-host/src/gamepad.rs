//! Gamepads: the map from what a pad holds to RGSS1 buttons, and the pads of the machine.
//!
//! [`GamepadMap`] is the logic: a pure function from the state of the connected pads, as
//! plain [`PadState`] values, to the buttons they hold. [`Gamepads`] is the thin layer over
//! `gilrs` that produces those values; nothing else in the crate touches a device.

use std::collections::BTreeMap;

use gilrs::{Axis, Gilrs};
use vkxp_proto::{Button, InputSnapshot};

use crate::MapError;
use crate::keys::{same_bindings, with_overrides};

/// How far the left stick has to be pushed along an axis to hold a direction.
pub const DEAD_ZONE: f32 = 0.5;

/// A button of a pad, by its position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PadButton {
    South,
    East,
    West,
    North,
    LeftShoulder,
    RightShoulder,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    LeftStick,
    RightStick,
    DPadDown,
    DPadLeft,
    DPadRight,
    DPadUp,
}

/// The pad buttons a configuration can map, with their names. The directional pad is not
/// among them: it is always the directions.
const NAMES: [(&str, PadButton); 12] = [
    ("South", PadButton::South),
    ("East", PadButton::East),
    ("West", PadButton::West),
    ("North", PadButton::North),
    ("LeftShoulder", PadButton::LeftShoulder),
    ("RightShoulder", PadButton::RightShoulder),
    ("LeftTrigger", PadButton::LeftTrigger),
    ("RightTrigger", PadButton::RightTrigger),
    ("Select", PadButton::Select),
    ("Start", PadButton::Start),
    ("LeftStick", PadButton::LeftStick),
    ("RightStick", PadButton::RightStick),
];

/// The directional pad.
const DPAD: [(PadButton, Button); 4] = [
    (PadButton::DPadDown, Button::Down),
    (PadButton::DPadLeft, Button::Left),
    (PadButton::DPadRight, Button::Right),
    (PadButton::DPadUp, Button::Up),
];

/// The default bindings.
const DEFAULTS: [(PadButton, Button); 8] = [
    (PadButton::South, Button::C),
    (PadButton::East, Button::B),
    (PadButton::West, Button::A),
    (PadButton::North, Button::X),
    (PadButton::LeftShoulder, Button::L),
    (PadButton::RightShoulder, Button::R),
    (PadButton::Start, Button::C),
    (PadButton::Select, Button::B),
];

/// What one connected pad holds at a moment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PadState {
    /// The buttons that are down.
    pub held: Vec<PadButton>,
    /// The left stick, each axis from -1 to 1; right and up are positive.
    pub stick: (f32, f32),
}

/// Which buttons each pad button holds down.
#[derive(Debug, Clone)]
pub struct GamepadMap {
    bindings: Vec<(PadButton, Button)>,
}

/// Two maps are equal when they hold the same bindings, in whatever order.
impl PartialEq for GamepadMap {
    fn eq(&self, other: &GamepadMap) -> bool {
        same_bindings(&self.bindings, &other.bindings)
    }
}

impl Eq for GamepadMap {}

impl GamepadMap {
    /// The default map with the pad buttons of every button in `overrides` replaced: the
    /// button, by its `Input` name, is held by exactly the pad buttons listed for it, besides
    /// the directional pad and the left stick, which always are the directions.
    pub fn with_overrides(
        overrides: &BTreeMap<String, Vec<String>>,
    ) -> Result<GamepadMap, MapError> {
        let bindings = with_overrides(&DEFAULTS, overrides, |name| {
            NAMES
                .iter()
                .find(|(known, _)| *known == name)
                .map(|(_, button)| *button)
                .ok_or_else(|| MapError::UnknownPadButton(name.to_owned()))
        })?;
        Ok(GamepadMap { bindings })
    }

    /// The buttons the connected pads hold together.
    pub fn held(&self, pads: &[PadState]) -> InputSnapshot {
        let mut snapshot = InputSnapshot::EMPTY;
        for pad in pads {
            for (_, button) in DPAD
                .iter()
                .chain(&self.bindings)
                .filter(|(held, _)| pad.held.contains(held))
            {
                snapshot = snapshot.with(*button);
            }
            let (x, y) = pad.stick;
            let directions = [
                (x > DEAD_ZONE, Button::Right),
                (x < -DEAD_ZONE, Button::Left),
                (y > DEAD_ZONE, Button::Up),
                (y < -DEAD_ZONE, Button::Down),
            ];
            for (_, button) in directions.iter().filter(|(pushed, _)| *pushed) {
                snapshot = snapshot.with(*button);
            }
        }
        snapshot
    }
}

impl Default for GamepadMap {
    fn default() -> GamepadMap {
        GamepadMap {
            bindings: DEFAULTS.to_vec(),
        }
    }
}

/// The buttons of the library and ours.
const LIBRARY_BUTTONS: [(gilrs::Button, PadButton); 16] = [
    (gilrs::Button::South, PadButton::South),
    (gilrs::Button::East, PadButton::East),
    (gilrs::Button::West, PadButton::West),
    (gilrs::Button::North, PadButton::North),
    (gilrs::Button::LeftTrigger, PadButton::LeftShoulder),
    (gilrs::Button::RightTrigger, PadButton::RightShoulder),
    (gilrs::Button::LeftTrigger2, PadButton::LeftTrigger),
    (gilrs::Button::RightTrigger2, PadButton::RightTrigger),
    (gilrs::Button::Select, PadButton::Select),
    (gilrs::Button::Start, PadButton::Start),
    (gilrs::Button::LeftThumb, PadButton::LeftStick),
    (gilrs::Button::RightThumb, PadButton::RightStick),
    (gilrs::Button::DPadDown, PadButton::DPadDown),
    (gilrs::Button::DPadLeft, PadButton::DPadLeft),
    (gilrs::Button::DPadRight, PadButton::DPadRight),
    (gilrs::Button::DPadUp, PadButton::DPadUp),
];

/// The pads of the machine, through `gilrs`.
pub struct Gamepads {
    gilrs: Gilrs,
    map: GamepadMap,
}

impl Gamepads {
    /// Starts reading pads. On a machine without gamepad support this says so once on the
    /// error stream and returns `None`.
    pub fn open(map: GamepadMap) -> Option<Gamepads> {
        match Gilrs::new() {
            Ok(gilrs) => Some(Gamepads { gilrs, map }),
            Err(error) => {
                eprintln!("vkxp: no gamepad is read: {error}");
                None
            }
        }
    }

    /// The buttons the pads connected right now hold.
    pub fn poll(&mut self) -> InputSnapshot {
        // The library follows connections, removals and the state of every pad as its events
        // are taken.
        while self.gilrs.next_event().is_some() {}
        let pads: Vec<PadState> = self
            .gilrs
            .gamepads()
            .map(|(_, pad)| PadState {
                held: LIBRARY_BUTTONS
                    .iter()
                    .filter(|(button, _)| pad.is_pressed(*button))
                    .map(|(_, button)| *button)
                    .collect(),
                stick: (pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY)),
            })
            .collect();
        self.map.held(&pads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(held: &[PadButton], stick: (f32, f32)) -> PadState {
        PadState {
            held: held.to_vec(),
            stick,
        }
    }

    fn snapshot(buttons: &[Button]) -> InputSnapshot {
        buttons
            .iter()
            .fold(InputSnapshot::EMPTY, |snapshot, button| {
                snapshot.with(*button)
            })
    }

    fn held(pads: &[PadState]) -> InputSnapshot {
        GamepadMap::default().held(pads)
    }

    /// Scenario "Directional pad".
    #[test]
    fn the_directional_pad_holds_the_directions() {
        let directions = [
            (PadButton::DPadRight, Button::Right),
            (PadButton::DPadLeft, Button::Left),
            (PadButton::DPadUp, Button::Up),
            (PadButton::DPadDown, Button::Down),
        ];
        for (pressed, button) in directions {
            assert_eq!(
                held(&[pad(&[pressed], (0.0, 0.0))]),
                snapshot(&[button]),
                "{pressed:?}"
            );
        }
    }

    /// Scenario "Stick dead zone".
    #[test]
    fn the_left_stick_holds_a_direction_beyond_the_dead_zone() {
        assert_eq!(held(&[pad(&[], (0.1, 0.0))]), InputSnapshot::EMPTY);
        assert_eq!(held(&[pad(&[], (0.9, 0.0))]), snapshot(&[Button::Right]));

        assert_eq!(held(&[pad(&[], (-0.9, 0.0))]), snapshot(&[Button::Left]));
        assert_eq!(held(&[pad(&[], (0.0, 0.9))]), snapshot(&[Button::Up]));
        assert_eq!(
            held(&[pad(&[], (0.9, -0.9))]),
            snapshot(&[Button::Right, Button::Down])
        );
        assert_eq!(
            held(&[pad(&[], (DEAD_ZONE, -DEAD_ZONE))]),
            InputSnapshot::EMPTY
        );
    }

    #[test]
    fn the_default_table() {
        let table = [
            (PadButton::South, Some(Button::C)),
            (PadButton::East, Some(Button::B)),
            (PadButton::West, Some(Button::A)),
            (PadButton::North, Some(Button::X)),
            (PadButton::LeftShoulder, Some(Button::L)),
            (PadButton::RightShoulder, Some(Button::R)),
            (PadButton::Start, Some(Button::C)),
            (PadButton::Select, Some(Button::B)),
            (PadButton::LeftTrigger, None),
            (PadButton::RightTrigger, None),
            (PadButton::LeftStick, None),
            (PadButton::RightStick, None),
        ];
        for (pressed, button) in table {
            let expected = button.map_or(InputSnapshot::EMPTY, |button| snapshot(&[button]));
            assert_eq!(
                held(&[pad(&[pressed], (0.0, 0.0))]),
                expected,
                "{pressed:?}"
            );
        }
    }

    #[test]
    fn the_configuration_replaces_the_table_per_button() {
        let overrides = BTreeMap::from([
            ("C".to_owned(), vec!["East".to_owned()]),
            ("Y".to_owned(), vec!["LeftTrigger".to_owned()]),
        ]);
        let map = GamepadMap::with_overrides(&overrides).unwrap();
        let held = |pressed| map.held(&[pad(&[pressed], (0.0, 0.0))]);
        assert_eq!(held(PadButton::East), snapshot(&[Button::B, Button::C]));
        assert_eq!(held(PadButton::South), InputSnapshot::EMPTY);
        assert_eq!(held(PadButton::Start), InputSnapshot::EMPTY);
        assert_eq!(held(PadButton::LeftTrigger), snapshot(&[Button::Y]));
        assert_eq!(held(PadButton::West), snapshot(&[Button::A]));
        assert_eq!(held(PadButton::DPadUp), snapshot(&[Button::Up]));

        assert_eq!(
            GamepadMap::with_overrides(&BTreeMap::new()),
            Ok(GamepadMap::default())
        );
    }

    #[test]
    fn unknown_names_are_errors_naming_them() {
        let overrides = |button: &str, pad: &str| {
            GamepadMap::with_overrides(&BTreeMap::from([(button.to_owned(), vec![pad.to_owned()])]))
        };
        assert_eq!(
            overrides("C", "Middle"),
            Err(MapError::UnknownPadButton("Middle".to_owned()))
        );
        assert_eq!(
            overrides("JUMP", "South"),
            Err(MapError::UnknownButton("JUMP".to_owned()))
        );
        // The directional pad is the directions and nothing else.
        assert!(overrides("C", "DPadUp").is_err());
    }

    #[test]
    fn several_pads_hold_buttons_together() {
        let pads = [
            pad(&[PadButton::South], (0.0, 0.0)),
            pad(&[PadButton::DPadLeft], (0.0, 0.0)),
        ];
        assert_eq!(held(&pads), snapshot(&[Button::C, Button::Left]));
    }

    /// Scenario "Removal": a pad that is gone is no longer among the states.
    #[test]
    fn a_removed_pad_releases_what_it_held() {
        let pads = vec![pad(&[PadButton::South], (0.9, 0.0))];
        assert_eq!(held(&pads), snapshot(&[Button::C, Button::Right]));
        assert_eq!(held(&[]), InputSnapshot::EMPTY);
    }
}
