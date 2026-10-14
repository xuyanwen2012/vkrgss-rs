//! The input script of a headless run.
//!
//! A script is text: entries `frame:BUTTONS` separated by commas or line breaks. `BUTTONS` is
//! a `+`-joined list of button names, or nothing. Each entry sets the buttons held from that
//! frame on, until the next entry. For example `60:C, 62:, 120:DOWN+SHIFT, 124:` presses C for
//! two frames and later holds DOWN and SHIFT for four.
//!
//! Frames count from 0. Button names are those of the `Input` constants (`DOWN`, `LEFT`,
//! `RIGHT`, `UP`, `A`, `B`, `C`, `X`, `Y`, `Z`, `L`, `R`, `SHIFT`, `CTRL`, `ALT`, `F5` to
//! `F9`) in either case. Blanks around entries and names are ignored, and so are empty entries
//! such as the one after a trailing comma.

use std::collections::BTreeMap;
use std::str::FromStr;

use thiserror::Error;
use vkrgss_proto::{Button, InputSnapshot};

/// The buttons held from given frames on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputScript {
    entries: BTreeMap<u64, InputSnapshot>,
}

/// Why a script could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScriptError {
    #[error("script entry `{entry}` has no `:` between its frame and its buttons")]
    MissingColon { entry: String },
    #[error("script entry `{entry}` does not start with a frame number")]
    InvalidFrame { entry: String },
    #[error("script entry `{entry}` names the unknown button `{name}`")]
    UnknownButton { entry: String, name: String },
    #[error("script entry `{entry}` has an empty button name")]
    EmptyButton { entry: String },
    #[error("the script has more than one entry for frame {frame}")]
    DuplicateFrame { frame: u64 },
}

impl InputScript {
    /// A script without entries: nothing is ever held.
    pub fn new() -> InputScript {
        InputScript::default()
    }

    /// Reads a script; see the module documentation for the syntax.
    pub fn parse(text: &str) -> Result<InputScript, ScriptError> {
        let mut entries = BTreeMap::new();
        for entry in text.split([',', '\n']).map(str::trim) {
            if entry.is_empty() {
                continue;
            }
            let owned = || entry.to_owned();
            let (frame, buttons) = entry
                .split_once(':')
                .ok_or_else(|| ScriptError::MissingColon { entry: owned() })?;
            let frame: u64 = frame
                .trim()
                .parse()
                .map_err(|_| ScriptError::InvalidFrame { entry: owned() })?;
            let mut snapshot = InputSnapshot::EMPTY;
            if !buttons.trim().is_empty() {
                for name in buttons.split('+').map(str::trim) {
                    if name.is_empty() {
                        return Err(ScriptError::EmptyButton { entry: owned() });
                    }
                    let button = button_named(name).ok_or_else(|| ScriptError::UnknownButton {
                        entry: owned(),
                        name: name.to_owned(),
                    })?;
                    snapshot = snapshot.with(button);
                }
            }
            if entries.insert(frame, snapshot).is_some() {
                return Err(ScriptError::DuplicateFrame { frame });
            }
        }
        Ok(InputScript { entries })
    }

    /// The buttons that start being held at `frame`, if the script has an entry for it.
    pub fn at(&self, frame: u64) -> Option<InputSnapshot> {
        self.entries.get(&frame).copied()
    }

    /// The buttons held during `frame`: those of the last entry at or before it.
    pub fn held_at(&self, frame: u64) -> InputSnapshot {
        self.entries
            .range(..=frame)
            .next_back()
            .map_or(InputSnapshot::EMPTY, |(_, snapshot)| *snapshot)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl FromStr for InputScript {
    type Err = ScriptError;

    fn from_str(text: &str) -> Result<InputScript, ScriptError> {
        InputScript::parse(text)
    }
}

/// The name of a button in a script.
pub fn button_name(button: Button) -> &'static str {
    match button {
        Button::Down => "DOWN",
        Button::Left => "LEFT",
        Button::Right => "RIGHT",
        Button::Up => "UP",
        Button::A => "A",
        Button::B => "B",
        Button::C => "C",
        Button::X => "X",
        Button::Y => "Y",
        Button::Z => "Z",
        Button::L => "L",
        Button::R => "R",
        Button::Shift => "SHIFT",
        Button::Ctrl => "CTRL",
        Button::Alt => "ALT",
        Button::F5 => "F5",
        Button::F6 => "F6",
        Button::F7 => "F7",
        Button::F8 => "F8",
        Button::F9 => "F9",
    }
}

pub(crate) fn button_named(name: &str) -> Option<Button> {
    Button::ALL
        .into_iter()
        .find(|button| button_name(*button).eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(buttons: &[Button]) -> InputSnapshot {
        buttons
            .iter()
            .fold(InputSnapshot::EMPTY, |snapshot, button| {
                snapshot.with(*button)
            })
    }

    #[test]
    fn the_example_of_the_design_parses() {
        let script = InputScript::parse("60:C, 62:, 120:DOWN+SHIFT, 124:").unwrap();
        assert_eq!(script.at(60), Some(held(&[Button::C])));
        assert_eq!(script.at(61), None);
        assert_eq!(script.at(62), Some(InputSnapshot::EMPTY));
        assert_eq!(script.at(120), Some(held(&[Button::Down, Button::Shift])));
        assert_eq!(script.at(124), Some(InputSnapshot::EMPTY));

        assert_eq!(script.held_at(0), InputSnapshot::EMPTY);
        assert_eq!(script.held_at(59), InputSnapshot::EMPTY);
        assert_eq!(script.held_at(61), held(&[Button::C]));
        assert_eq!(script.held_at(62), InputSnapshot::EMPTY);
        assert_eq!(script.held_at(123), held(&[Button::Down, Button::Shift]));
        assert_eq!(script.held_at(9999), InputSnapshot::EMPTY);
    }

    #[test]
    fn entries_may_be_separated_by_line_breaks_and_padded_with_blanks() {
        let text = "  0 : right \n\n 3:Right + Shift ,\r\n 7:\n,";
        let script: InputScript = text.parse().unwrap();
        assert_eq!(script.at(0), Some(held(&[Button::Right])));
        assert_eq!(script.at(3), Some(held(&[Button::Right, Button::Shift])));
        assert_eq!(script.at(7), Some(InputSnapshot::EMPTY));
        assert_eq!(script.entries.len(), 3);
    }

    #[test]
    fn empty_text_is_an_empty_script() {
        for text in ["", "  ", "\n\n", ",,"] {
            let script = InputScript::parse(text).unwrap();
            assert!(script.is_empty(), "{text:?}");
            assert_eq!(script, InputScript::new());
            assert_eq!(script.held_at(100), InputSnapshot::EMPTY);
        }
    }

    #[test]
    fn every_button_has_a_name_that_parses_back() {
        for button in Button::ALL {
            let name = button_name(button);
            for text in [name.to_owned(), name.to_ascii_lowercase()] {
                let script = InputScript::parse(&format!("5:{text}")).unwrap();
                assert_eq!(script.at(5), Some(held(&[button])), "{text}");
            }
        }
        let all: Vec<&str> = Button::ALL.into_iter().map(button_name).collect();
        let script = InputScript::parse(&format!("1:{}", all.join("+"))).unwrap();
        assert_eq!(script.at(1), Some(held(&Button::ALL)));
    }

    #[test]
    fn entries_may_come_in_any_order() {
        let script = InputScript::parse("20:, 10:RIGHT").unwrap();
        assert_eq!(script.held_at(15), held(&[Button::Right]));
        assert_eq!(script.held_at(20), InputSnapshot::EMPTY);
    }

    #[test]
    fn malformed_entries_are_refused_with_the_entry_named() {
        let entry = |text: &str| text.to_owned();
        let cases = [
            ("10", ScriptError::MissingColon { entry: entry("10") }),
            (
                "0:A, RIGHT",
                ScriptError::MissingColon {
                    entry: entry("RIGHT"),
                },
            ),
            (
                "ten:A",
                ScriptError::InvalidFrame {
                    entry: entry("ten:A"),
                },
            ),
            (":A", ScriptError::InvalidFrame { entry: entry(":A") }),
            (
                "-1:A",
                ScriptError::InvalidFrame {
                    entry: entry("-1:A"),
                },
            ),
            (
                "4:JUMP",
                ScriptError::UnknownButton {
                    entry: entry("4:JUMP"),
                    name: entry("JUMP"),
                },
            ),
            (
                "4:A+F4",
                ScriptError::UnknownButton {
                    entry: entry("4:A+F4"),
                    name: entry("F4"),
                },
            ),
            (
                "4:A+",
                ScriptError::EmptyButton {
                    entry: entry("4:A+"),
                },
            ),
            (
                "4:A++B",
                ScriptError::EmptyButton {
                    entry: entry("4:A++B"),
                },
            ),
            ("4:A, 4:", ScriptError::DuplicateFrame { frame: 4 }),
        ];
        for (text, expected) in cases {
            assert_eq!(InputScript::parse(text), Err(expected), "{text:?}");
        }
        assert_eq!(
            InputScript::parse("4:JUMP").unwrap_err().to_string(),
            "script entry `4:JUMP` names the unknown button `JUMP`"
        );
    }
}
