//! Fakes for testing a [`Session`](crate::Session) without a window, a GPU or an audio device:
//! a display that records what it was asked to do, an audio sink that records its commands,
//! and an error stream that can be read back.
//!
//! The fake clock is [`ManualClock`](crate::ManualClock), which the headless runner uses too.

use std::collections::HashSet;
use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use vkxp_proto::{AudioCommand, BitmapDelta, BitmapId, FramePacket};

use crate::{AudioSink, Display, ManualClock};

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
    Mask(Option<(BitmapId, u32)>),
    Redraw,
    Title(String),
}

/// An image as a [`RecordingDisplay`] models it, with frames named by their `frame_index`.
#[derive(Debug, Clone, PartialEq)]
pub enum Shown {
    /// Black: nothing was ever rendered, or nothing was ever held.
    Nothing,
    /// One frame alone.
    Frame(u64),
    /// The mix of a held image and the newest frame. The held image is itself a mix when it
    /// was captured in the middle of a cross-fade.
    Mix {
        held: Box<Shown>,
        newest: u64,
        weight: f32,
    },
    /// The masked mix of a held image and the newest frame.
    Masked {
        held: Box<Shown>,
        newest: u64,
        progress: f32,
        mask: BitmapId,
        vagueness: u32,
    },
}

impl Shown {
    /// The mix of `held` and the frame `newest` at `weight`.
    pub fn mix(held: Shown, newest: u64, weight: f32) -> Shown {
        Shown::Mix {
            held: Box::new(held),
            newest,
            weight,
        }
    }
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
#[derive(Debug, Clone)]
pub struct RecordingDisplay {
    /// Every request, oldest first.
    pub events: Vec<DisplayEvent>,
    newest: Option<u64>,
    held: Shown,
    weight: f32,
    mask: Option<(BitmapId, u32)>,
    /// The bitmaps the packets so far have created and not destroyed.
    bitmaps: HashSet<BitmapId>,
    shown: Shown,
    title: String,
    /// A clock to move forward by a fixed time whenever a packet is rendered.
    render_time: Option<(ManualClock, Duration)>,
}

impl RecordingDisplay {
    pub fn new() -> RecordingDisplay {
        RecordingDisplay {
            events: Vec::new(),
            newest: None,
            held: Shown::Nothing,
            weight: 1.0,
            mask: None,
            bitmaps: HashSet::new(),
            shown: Shown::Nothing,
            title: String::new(),
            render_time: None,
        }
    }

    /// A display on which rendering a packet, accepted or not, takes `time` on `clock`.
    pub fn with_render_time(clock: ManualClock, time: Duration) -> RecordingDisplay {
        RecordingDisplay {
            render_time: Some((clock, time)),
            ..RecordingDisplay::new()
        }
    }

    /// What is on screen: the image that was last shown.
    pub fn shown(&self) -> Shown {
        self.shown.clone()
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

    /// The image the current state gives: the mix of the held image and the newest frame,
    /// masked while a mask is set that names a bitmap the display has.
    fn compose(&self) -> Shown {
        let mask = self.mask.filter(|(id, _)| self.bitmaps.contains(id));
        match (self.newest, mask) {
            (Some(newest), Some((mask, vagueness))) if self.weight < 1.0 => {
                return Shown::Masked {
                    held: Box::new(self.held.clone()),
                    newest,
                    progress: self.weight,
                    mask,
                    vagueness,
                };
            }
            _ => {}
        }
        match self.newest {
            None => Shown::Nothing,
            Some(newest) if self.weight >= 1.0 => Shown::Frame(newest),
            Some(_) if self.weight <= 0.0 => self.held.clone(),
            Some(newest) => Shown::mix(self.held.clone(), newest, self.weight),
        }
    }

    fn show(&mut self) {
        self.shown = self.compose();
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
        if let Some((clock, time)) = &self.render_time {
            clock.advance(*time);
        }
        // Like the compositors, which apply the bitmap changes of a packet they then reject.
        for delta in &packet.bitmaps {
            match delta {
                BitmapDelta::Create { id, .. } => {
                    self.bitmaps.insert(*id);
                }
                BitmapDelta::Destroy { id } => {
                    self.bitmaps.remove(id);
                }
                BitmapDelta::Update { .. } => {}
            }
        }
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
        self.held = self.compose();
        Ok(())
    }

    fn set_mix(&mut self, weight: f32) {
        self.events.push(DisplayEvent::Mix(weight));
        self.weight = weight;
    }

    fn set_mask(&mut self, mask: Option<(BitmapId, u32)>) {
        self.events.push(DisplayEvent::Mask(mask));
        self.mask = mask;
    }

    fn has_bitmap(&self, id: BitmapId) -> bool {
        self.bitmaps.contains(&id)
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

/// An audio sink that keeps the commands it is sent and plays nothing. Clones share one list.
#[derive(Debug, Clone, Default)]
pub struct RecordingSink {
    commands: Arc<Mutex<Vec<AudioCommand>>>,
}

impl RecordingSink {
    pub fn new() -> RecordingSink {
        RecordingSink::default()
    }

    /// Every command received so far, oldest first.
    pub fn commands(&self) -> Vec<AudioCommand> {
        self.lock().clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<AudioCommand>> {
        self.commands.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl AudioSink for RecordingSink {
    fn command(&mut self, command: AudioCommand) {
        self.lock().push(command);
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
