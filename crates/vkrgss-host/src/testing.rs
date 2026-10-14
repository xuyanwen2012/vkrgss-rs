//! Fakes for testing a [`Session`](crate::Session) without a window, a GPU or an audio device:
//! a display that records what it was asked to do, an audio sink that records its commands,
//! and a reporter whose reports can be read back.
//!
//! The fake clock is [`ManualClock`], which the headless runner uses too.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use vkrgss_proto::{AudioCommand, BitmapDelta, BitmapId, FramePacket, FrameRate, FreezeRule, Op};
use vkrgss_vk::Mix;

use crate::{AudioSink, Display, ManualClock, Reporter, Severity};

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
    Mix(Mix),
    Mask(Option<(BitmapId, u32)>),
    Redraw,
    Title(String),
    /// Whether the pointer is to be seen.
    Pointer(bool),
    /// Every bitmap and table was forgotten.
    Forget,
}

/// What a [`RecordingDisplay`] was told about the pacing of its frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacingEvent {
    /// The frame rate of the packet rendered next.
    Rate(u8),
    /// Pacing starts anew with the packet rendered next.
    Restart,
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
        /// The weight of the newest frame, of 256.
        weight: u8,
    },
    /// The masked mix of a held image and the newest frame.
    Masked {
        held: Box<Shown>,
        newest: u64,
        threshold: u8,
        mask: BitmapId,
        vagueness: u32,
    },
}

impl Shown {
    /// The mix of `held` and the frame `newest` at `weight`.
    pub fn mix(held: Shown, newest: u64, weight: u8) -> Shown {
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
    /// Position of the op that draws a bitmap that does not exist.
    pub op: usize,
}

impl std::fmt::Display for Rejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "op {} draws an unknown bitmap", self.op)
    }
}

/// A display that renders nothing. It records every request and models which frame would be
/// on screen. Like the compositors, it rejects a packet that contains a quad of a bitmap that
/// does not exist.
#[derive(Debug, Clone)]
pub struct RecordingDisplay {
    /// Every request, oldest first.
    pub events: Vec<DisplayEvent>,
    /// What it was told about pacing, oldest first. Kept apart from the requests above.
    pub pacing: Vec<PacingEvent>,
    /// The rule of every [`DisplayEvent::Hold`], oldest first.
    pub holds: Vec<FreezeRule>,
    newest: Option<u64>,
    held: Shown,
    mix: Mix,
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
            pacing: Vec::new(),
            holds: Vec::new(),
            newest: None,
            held: Shown::Nothing,
            mix: Mix::Newest,
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
        let Some(newest) = self.newest else {
            return Shown::Nothing;
        };
        let Mix::Step { weight, threshold } = self.mix else {
            return Shown::Frame(newest);
        };
        match self.mask.filter(|(id, _)| self.bitmaps.contains(id)) {
            Some((mask, vagueness)) => Shown::Masked {
                held: Box::new(self.held.clone()),
                newest,
                threshold,
                mask,
                vagueness,
            },
            None if weight == 0 => self.held.clone(),
            None => Shown::mix(self.held.clone(), newest, weight),
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
        let unknown =
            |op: &Op| matches!(op, Op::Quad(quad) if !self.bitmaps.contains(&quad.bitmap));
        if let Some(op) = packet.ops.iter().position(unknown) {
            self.events.push(DisplayEvent::Rejected { frame_index });
            return Err(Rejected { op });
        }
        self.events.push(DisplayEvent::Render { frame_index });
        self.newest = Some(frame_index);
        self.show();
        Ok(())
    }

    fn hold(&mut self, rule: FreezeRule) -> Result<(), Rejected> {
        self.events.push(DisplayEvent::Hold);
        self.holds.push(rule);
        self.held = self.compose();
        Ok(())
    }

    fn set_mix(&mut self, mix: Mix) {
        self.events.push(DisplayEvent::Mix(mix));
        self.mix = mix;
    }

    fn set_mask(&mut self, mask: Option<(BitmapId, u32)>) {
        self.events.push(DisplayEvent::Mask(mask));
        self.mask = mask;
    }

    fn has_bitmap(&self, id: BitmapId) -> bool {
        self.bitmaps.contains(&id)
    }

    fn forget_resources(&mut self) -> Result<(), Rejected> {
        self.events.push(DisplayEvent::Forget);
        self.bitmaps.clear();
        Ok(())
    }

    fn set_frame_rate(&mut self, rate: FrameRate) {
        self.pacing.push(PacingEvent::Rate(rate.get()));
    }

    fn restart_pacing(&mut self) {
        self.pacing.push(PacingEvent::Restart);
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

    fn set_pointer_visible(&mut self, visible: bool) {
        self.events.push(DisplayEvent::Pointer(visible));
    }
}

/// One thing a [`RecordingSink`] was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SinkEvent {
    Command(AudioCommand),
    Remember(u8),
    Recall(u8),
}

/// An audio sink that keeps what it is asked for and plays nothing. Clones share one list.
#[derive(Debug, Clone, Default)]
pub struct RecordingSink {
    events: Arc<Mutex<Vec<SinkEvent>>>,
}

impl RecordingSink {
    pub fn new() -> RecordingSink {
        RecordingSink::default()
    }

    /// Every command received so far, oldest first.
    pub fn commands(&self) -> Vec<AudioCommand> {
        let events = self.lock();
        let commands = events.iter().filter_map(|event| match event {
            SinkEvent::Command(command) => Some(command.clone()),
            SinkEvent::Remember(_) | SinkEvent::Recall(_) => None,
        });
        commands.collect()
    }

    /// Everything the sink was asked for so far, oldest first.
    pub fn events(&self) -> Vec<SinkEvent> {
        self.lock().clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<SinkEvent>> {
        self.events.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl AudioSink for RecordingSink {
    fn command(&mut self, command: AudioCommand) {
        self.lock().push(SinkEvent::Command(command));
    }

    fn remember(&mut self, slot: u8) {
        self.lock().push(SinkEvent::Remember(slot));
    }

    fn recall(&mut self, slot: u8) {
        self.lock().push(SinkEvent::Recall(slot));
    }
}

/// A reporter that keeps what is reported to it. Clones share one buffer.
#[derive(Debug, Clone, Default)]
pub struct ErrorBuffer {
    reports: Arc<Mutex<Vec<(Severity, String)>>>,
}

impl ErrorBuffer {
    pub fn new() -> ErrorBuffer {
        ErrorBuffer::default()
    }

    /// Everything reported so far, as the lines `vkrgss: <text>` of an error stream.
    pub fn text(&self) -> String {
        self.lock()
            .iter()
            .map(|(_, text)| format!("vkrgss: {text}\n"))
            .collect()
    }

    /// The severity of each report so far whose text contains `part`, in order.
    pub fn severities(&self, part: &str) -> Vec<Severity> {
        self.lock()
            .iter()
            .filter(|(_, text)| text.contains(part))
            .map(|(severity, _)| *severity)
            .collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<(Severity, String)>> {
        self.reports.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Reporter for ErrorBuffer {
    fn report(&mut self, severity: Severity, text: &str) {
        self.lock().push((severity, text.to_owned()));
    }
}
