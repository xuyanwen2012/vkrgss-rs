//! Fakes for testing a [`Session`](crate::Session) without a window or a GPU: a display that
//! records what it was asked to do, and an error stream that can be read back.
//!
//! The fake clock is [`ManualClock`](crate::ManualClock), which the headless runner uses too.

use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};

use vkxp_proto::FramePacket;

use crate::Display;

/// One request a [`RecordingDisplay`] received.
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayEvent {
    /// A packet was rendered and shown.
    Render {
        frame_index: u64,
    },
    /// A packet was rejected.
    Rejected {
        frame_index: u64,
    },
    Hold,
    Mix(f32),
    Redraw,
    Title(String),
}

/// What a [`RecordingDisplay`] has on screen, with frames named by their `frame_index`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shown {
    /// Nothing was ever shown, or a held frame that does not exist: black.
    Nothing,
    /// One frame alone.
    Frame(u64),
    /// A mix of the held frame (`None` is black) and the newest frame.
    Mix {
        held: Option<u64>,
        newest: u64,
        weight: f32,
    },
}

/// Why a [`RecordingDisplay`] rejected a packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rejected {
    /// Position of the op that is not supported.
    pub op: usize,
}

impl std::fmt::Display for Rejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "op {} is of a reserved kind", self.op)
    }
}

/// A display that renders nothing. It records every request and models which frame would be
/// on screen. Like the compositors, it rejects a packet that contains an op of a reserved kind.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordingDisplay {
    /// Every request, oldest first.
    pub events: Vec<DisplayEvent>,
    newest: Option<u64>,
    held: Option<u64>,
    weight: f32,
    shown: Shown,
    title: String,
}

impl RecordingDisplay {
    pub fn new() -> RecordingDisplay {
        RecordingDisplay {
            events: Vec::new(),
            newest: None,
            held: None,
            weight: 1.0,
            shown: Shown::Nothing,
            title: String::new(),
        }
    }

    /// What is on screen.
    pub fn shown(&self) -> Shown {
        self.shown
    }

    /// The current window title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The frame indices of the packets rendered so far, in order.
    pub fn rendered(&self) -> Vec<u64> {
        self.events
            .iter()
            .filter_map(|event| match event {
                DisplayEvent::Render { frame_index } => Some(*frame_index),
                _ => None,
            })
            .collect()
    }

    fn show(&mut self) {
        self.shown = match (self.newest, self.held) {
            (Some(newest), _) if self.weight >= 1.0 => Shown::Frame(newest),
            (_, held) if self.weight <= 0.0 => held.map_or(Shown::Nothing, Shown::Frame),
            (Some(newest), held) => Shown::Mix {
                held,
                newest,
                weight: self.weight,
            },
            (None, _) => Shown::Nothing,
        };
    }
}

impl Default for RecordingDisplay {
    fn default() -> RecordingDisplay {
        RecordingDisplay::new()
    }
}

impl Display for RecordingDisplay {
    type Error = Rejected;

    fn render(&mut self, packet: &FramePacket) -> Result<(), Rejected> {
        let frame_index = packet.frame_index;
        if let Some(op) = packet.ops.iter().position(|op| op.kind().is_reserved()) {
            self.events.push(DisplayEvent::Rejected { frame_index });
            return Err(Rejected { op });
        }
        self.events.push(DisplayEvent::Render { frame_index });
        self.newest = Some(frame_index);
        self.show();
        Ok(())
    }

    fn hold(&mut self) -> Result<(), Rejected> {
        self.events.push(DisplayEvent::Hold);
        self.held = self.newest;
        Ok(())
    }

    fn set_mix(&mut self, weight: f32) {
        self.events.push(DisplayEvent::Mix(weight));
        self.weight = weight;
    }

    fn redraw(&mut self) -> Result<(), Rejected> {
        self.events.push(DisplayEvent::Redraw);
        self.show();
        Ok(())
    }

    fn set_title(&mut self, title: &str) {
        self.events.push(DisplayEvent::Title(title.to_owned()));
        self.title = title.to_owned();
    }
}

/// An error stream that keeps what is written to it. Clones share one buffer.
#[derive(Debug, Clone, Default)]
pub struct ErrorBuffer {
    bytes: Arc<Mutex<Vec<u8>>>,
}

impl ErrorBuffer {
    pub fn new() -> ErrorBuffer {
        ErrorBuffer::default()
    }

    /// Everything written so far.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.lock()).into_owned()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<u8>> {
        self.bytes.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Write for ErrorBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.lock().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
