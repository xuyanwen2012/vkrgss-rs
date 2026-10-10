//! The states schedule of a headless run.
//!
//! A schedule is text: entries `frame:save N` and `frame:load N` separated by commas or line
//! breaks, with `N` a state slot from 0 to 9. At the frame of an entry the session is asked
//! to save a state into the slot or to load the state of the slot, as the state keys ask in
//! a window. For example `120:save 0, 200:load 0` saves at frame 120 and goes back there at
//! frame 200.
//!
//! Frames count from 0. Blanks around entries and their parts are ignored, and so are empty
//! entries such as the one after a trailing comma. A frame may have several entries; they
//! are made in the order written.

use std::str::FromStr;

use thiserror::Error;

/// The highest state slot.
const LAST_SLOT: u8 = 9;

/// What a schedule asks for at a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateRequest {
    /// Save a state into the slot.
    Save(u8),
    /// Load the state of the slot.
    Load(u8),
}

/// The state requests of a headless run by frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateSchedule {
    entries: Vec<(u64, StateRequest)>,
}

/// Why a schedule could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScheduleError {
    #[error("states entry `{entry}` has no `:` between its frame and its request")]
    MissingColon { entry: String },
    #[error("states entry `{entry}` does not start with a frame number")]
    InvalidFrame { entry: String },
    #[error("states entry `{entry}` asks for neither `save` nor `load` of a slot")]
    UnknownRequest { entry: String },
    #[error("states entry `{entry}` does not name a slot from 0 to 9")]
    InvalidSlot { entry: String },
}

impl StateSchedule {
    /// A schedule without entries.
    pub fn new() -> StateSchedule {
        StateSchedule::default()
    }

    /// Reads a schedule; see the module documentation for the syntax.
    pub fn parse(text: &str) -> Result<StateSchedule, ScheduleError> {
        let mut entries = Vec::new();
        for entry in text.split([',', '\n']).map(str::trim) {
            if entry.is_empty() {
                continue;
            }
            let owned = || entry.to_owned();
            let (frame, request) = entry
                .split_once(':')
                .ok_or_else(|| ScheduleError::MissingColon { entry: owned() })?;
            let frame: u64 = frame
                .trim()
                .parse()
                .map_err(|_| ScheduleError::InvalidFrame { entry: owned() })?;
            let mut words = request.split_whitespace();
            let request = match words.next().map(str::to_ascii_lowercase).as_deref() {
                Some("save") => StateRequest::Save,
                Some("load") => StateRequest::Load,
                _ => return Err(ScheduleError::UnknownRequest { entry: owned() }),
            };
            let slot = words.next().and_then(|slot| slot.parse::<u8>().ok());
            let slot = slot
                .filter(|slot| *slot <= LAST_SLOT && words.next().is_none())
                .ok_or_else(|| ScheduleError::InvalidSlot { entry: owned() })?;
            entries.push((frame, request(slot)));
        }
        Ok(StateSchedule { entries })
    }

    /// The requests to make at `frame`, in the order written.
    pub fn at(&self, frame: u64) -> impl Iterator<Item = StateRequest> {
        let entries = self.entries.iter();
        entries.filter_map(move |(at, request)| (*at == frame).then_some(*request))
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl FromStr for StateSchedule {
    type Err = ScheduleError;

    fn from_str(text: &str) -> Result<StateSchedule, ScheduleError> {
        StateSchedule::parse(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_schedule_gives_the_requests_of_each_frame_in_order() {
        let schedule = StateSchedule::parse("120:save 0, 200: LOAD 0\n200:save 9,").unwrap();
        let at = |frame| schedule.at(frame).collect::<Vec<_>>();
        assert_eq!(at(120), [StateRequest::Save(0)]);
        assert_eq!(at(200), [StateRequest::Load(0), StateRequest::Save(9)]);
        assert_eq!(at(121), []);
        assert!(!schedule.is_empty());
        assert!(StateSchedule::parse(" , ").unwrap().is_empty());
    }

    /// Scenario "Malformed schedule", as far as the reading goes.
    #[test]
    fn a_malformed_entry_is_named() {
        let error = |text| StateSchedule::parse(text).unwrap_err();
        assert_eq!(
            error("5:keep 0"),
            ScheduleError::UnknownRequest {
                entry: "5:keep 0".to_owned()
            }
        );
        assert!(error("5:keep 0").to_string().contains("`5:keep 0`"));
        for text in ["5:save 10", "5:load", "5:save -1", "5:save 1 2", "5:load x"] {
            assert_eq!(
                error(text),
                ScheduleError::InvalidSlot {
                    entry: text.to_owned()
                },
                "{text}"
            );
        }
        assert_eq!(
            error("1:save 0, save 1"),
            ScheduleError::MissingColon {
                entry: "save 1".to_owned()
            }
        );
        assert_eq!(
            error("x:save 1"),
            ScheduleError::InvalidFrame {
                entry: "x:save 1".to_owned()
            }
        );
    }
}
