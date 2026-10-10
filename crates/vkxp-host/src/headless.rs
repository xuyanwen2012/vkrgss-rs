//! The headless runner: a session without a window or display.
//!
//! It runs on a virtual clock that never waits: when a tick is due in the future the clock is
//! simply moved there. Input comes from an [`InputScript`], and chosen frames are written as
//! PNG files at the logical size, showing what a window of that size would: the held frame
//! while the display is frozen and the mix during a transition. Nothing in a run depends on
//! real time, so two runs of the same guest with the same script write identical files.
//!
//! Frames are numbered from 0 in the order their messages arrive, which for a guest that
//! counts its frames from 0 is the packet's `frame_index`.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

use thiserror::Error;
use vkxp_proto::link::{Disconnected, GuestEnd, HostEnd, link};
use vkxp_proto::{BitmapId, FramePacket, GuestToHost, HostToGuest, Size};
use vkxp_vk::{DeviceChoice, Frame, FrameTiming, Mix, VkCompositor, VkError};

use crate::report::ErrorStream;
use crate::{
    Clock, Display, InputScript, InputSource, ManualClock, Reporter, Session, SessionEnd,
    SessionOptions,
};

/// The device a headless run uses unless told otherwise: the CPU rasteriser, so that a run
/// neither depends on nor disturbs the GPUs of the machine.
pub const DEFAULT_DEVICE: &str = "llvmpipe";

/// The host's end of a link, for a host that has nothing to do but wait for the guest.
pub struct HostLink {
    host: HostEnd,
    woken: Receiver<()>,
}

/// Creates a link whose host end can block until the guest has sent something.
pub fn headless_link() -> (GuestEnd, HostLink) {
    let (wake, woken) = channel();
    let (guest, host) = link(move || {
        // Nobody listens once the host end has gone.
        let _ = wake.send(());
    });
    (guest, HostLink { host, woken })
}

impl HostLink {
    /// Waits for the next message from the guest.
    pub fn recv(&self) -> Result<GuestToHost, Disconnected> {
        loop {
            if let Some(message) = self.host.try_recv()? {
                return Ok(message);
            }
            if self.woken.recv().is_err() {
                // The guest end has gone; whatever it sent last is still in the link.
                return self.host.try_recv()?.ok_or(Disconnected);
            }
        }
    }

    /// Sends a message to the guest.
    pub fn send(&self, message: HostToGuest) -> Result<(), Disconnected> {
        self.host.send(message)
    }
}

/// How a headless run is set up.
pub struct HeadlessOptions {
    /// The device to composite on.
    pub device: DeviceChoice,
    /// The number of frames after which the run asks the guest to quit and ends. Without a
    /// limit the run lasts until the guest shuts down.
    pub frames: Option<u64>,
    /// The buttons held from given frames on.
    pub script: InputScript,
    /// A recorder or a replayer of the input. The script is the live input a recorder writes
    /// down, and what applies after the last frame of a replayed recording.
    pub input: Option<Box<dyn InputSource>>,
    /// Frames to write as PNG files, by frame number.
    pub dumps: Vec<(u64, PathBuf)>,
    /// Whether the frame time statistics are collected and their summary written to
    /// `errors` when the run ends.
    pub frame_stats: bool,
    /// Where modal texts and errors are reported.
    pub errors: Box<dyn Reporter>,
}

impl Default for HeadlessOptions {
    fn default() -> HeadlessOptions {
        HeadlessOptions {
            device: DeviceChoice::Name(DEFAULT_DEVICE.to_owned()),
            frames: None,
            script: InputScript::new(),
            input: None,
            dumps: Vec::new(),
            frame_stats: false,
            errors: Box::new(ErrorStream),
        }
    }
}

/// Why a headless run that did not fail came to its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadlessEnd {
    /// The guest sent its shutdown message.
    Shutdown,
    /// The frame limit was reached and the guest was asked to quit.
    FrameLimit,
}

/// What a successful headless run did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessReport {
    pub end: HeadlessEnd,
    /// The number of frames the guest sent.
    pub frames: u64,
    /// The time that passed on the virtual clock.
    pub elapsed: std::time::Duration,
    /// The title the guest set last.
    pub title: String,
}

/// Why a headless run failed.
#[derive(Debug, Error)]
pub enum HeadlessError {
    #[error("cannot start the compositor: {0}")]
    Compositor(#[source] VkError),
    /// The details were written to the error stream.
    #[error("frame {frame} was rejected by the compositor")]
    FrameRejected { frame: u64 },
    #[error("the guest went away without shutting down, after {frames} frame(s)")]
    GuestDisconnected { frames: u64 },
    #[error("cannot read back frame {frame}: {source}")]
    ReadFrame {
        frame: u64,
        #[source]
        source: VkError,
    },
    #[error("cannot write frame {frame} to {}: {message}", path.display())]
    Dump {
        frame: u64,
        path: PathBuf,
        message: String,
    },
    #[error("frame {frame} was to be dumped but the run ended after {frames} frame(s)")]
    DumpNotReached { frame: u64, frames: u64 },
}

/// The display of a headless run: the compositor and nothing that shows its frames.
///
/// Packets are composited exactly as in a window, and the held frame and the mix are kept as
/// in a window, so that [`read_shown`](HeadlessDisplay::read_shown) gives what one would show.
pub struct HeadlessDisplay {
    compositor: VkCompositor,
    /// The logical size of the newest frame.
    size: Option<Size>,
    title: String,
}

impl HeadlessDisplay {
    /// Opens the device without a surface.
    pub fn new(device: DeviceChoice) -> Result<HeadlessDisplay, VkError> {
        Ok(HeadlessDisplay {
            compositor: VkCompositor::new(device)?,
            size: None,
            title: String::new(),
        })
    }

    /// The newest frame, read back from the device.
    pub fn read_frame(&mut self) -> Result<Option<Frame>, VkError> {
        self.compositor.read_frame()
    }

    /// What a window of the newest frame's size would show: that frame as it was composited,
    /// or while a freeze or a transition is under way the presenter's mix of it and the held
    /// frame, drawn off screen at scale 1.
    pub fn read_shown(&mut self) -> Result<Option<Frame>, VkError> {
        match self.size {
            Some(size) if self.compositor.mix() != Mix::Newest => {
                self.compositor.scale_frame(size).map(Some)
            }
            _ => self.compositor.read_frame(),
        }
    }

    /// The title the guest set last.
    pub fn title(&self) -> &str {
        &self.title
    }
}

impl Display for HeadlessDisplay {
    type Error = VkError;

    fn render(&mut self, packet: &FramePacket) -> Result<(), VkError> {
        self.compositor.render(packet)?;
        self.size = Some(packet.logical_size);
        Ok(())
    }

    fn hold(&mut self) -> Result<(), VkError> {
        self.compositor.freeze()
    }

    fn set_mix(&mut self, mix: Mix) {
        self.compositor.set_mix(mix);
    }

    fn set_mask(&mut self, mask: Option<(BitmapId, u32)>) {
        self.compositor.set_mask(mask);
    }

    fn has_bitmap(&self, id: BitmapId) -> bool {
        self.compositor.has_bitmap(id)
    }

    fn take_frame_timing(&mut self) -> Option<FrameTiming> {
        self.compositor.take_frame_timing()
    }

    fn redraw(&mut self) -> Result<(), VkError> {
        Ok(())
    }

    fn set_title(&mut self, title: &str) {
        title.clone_into(&mut self.title);
    }
}

/// Runs a session against `link` without a window, until the guest shuts down or the frame
/// limit is reached. See the module documentation.
///
/// The snapshot of the script's entry for frame `n` is sent before that frame's tick. After
/// the last frame of a limited run the guest gets that frame's tick and then a quit request,
/// and the run ends without waiting for the guest's answer.
///
/// A rejected frame fails the run: its error is reported, the guest gets
/// that frame's tick like any other, and then the run ends and the guest is disconnected.
pub fn run_headless(
    options: HeadlessOptions,
    link: HostLink,
) -> Result<HeadlessReport, HeadlessError> {
    let HeadlessOptions {
        device,
        frames: limit,
        script,
        input,
        dumps,
        frame_stats,
        errors,
    } = options;
    let mut display = HeadlessDisplay::new(device).map_err(HeadlessError::Compositor)?;
    display.compositor.set_frame_timing(frame_stats);
    let chosen = display.compositor.device();
    tracing::info!(
        "compositing on {} ({}), on a virtual clock",
        chosen.name,
        chosen.driver
    );
    let clock = ManualClock::new();
    let session_options = SessionOptions {
        fail_on_rejected_frame: true,
        errors,
        input,
        frame_stats,
        ..SessionOptions::default()
    };
    let mut session = Session::new(display, clock.clone(), session_options);
    let report = |session: &Session<HeadlessDisplay, ManualClock>, end, frames| match dumps
        .iter()
        .find(|(frame, _)| *frame >= frames)
    {
        Some((frame, _)) => Err(HeadlessError::DumpNotReached {
            frame: *frame,
            frames,
        }),
        None => Ok(HeadlessReport {
            end,
            frames,
            elapsed: clock.now(),
            title: session.display().title().to_owned(),
        }),
    };

    let mut frames = 0;
    if limit == Some(0) {
        let _ = link.send(HostToGuest::Quit);
        return report(&session, HeadlessEnd::FrameLimit, frames);
    }
    loop {
        let Ok(message) = link.recv() else {
            return Err(HeadlessError::GuestDisconnected { frames });
        };
        let is_frame = matches!(message, GuestToHost::Frame(_));
        if is_frame && let Some(buttons) = script.at(frames) {
            // Queued ahead of the tick that handling the frame produces.
            session.set_buttons(buttons);
        }
        session.handle_message(message);

        // The number of the frame just handled, if it was one.
        let handled = frames;
        if is_frame {
            // A rejected frame left the previous one on the display; that is not dumped under
            // this frame's number.
            if !session.is_failing() {
                for (_, path) in dumps.iter().filter(|(frame, _)| *frame == handled) {
                    dump(session.display_mut(), handled, path)?;
                }
            }
            frames += 1;
            if let Some(due) = session.next_wake() {
                // The virtual clock never waits: it is moved to the tick.
                clock.advance_to(due);
                session.poll();
            }
        }
        // Everything the session produced goes out before the run looks at how it stands, so
        // a rejected frame has its tick before the run fails and the guest is cut off.
        let mut gone = false;
        for message in session.take_messages() {
            gone |= link.send(message).is_err();
        }
        match session.end() {
            Some(SessionEnd::Shutdown) => return report(&session, HeadlessEnd::Shutdown, frames),
            Some(SessionEnd::Failed) => {
                return Err(HeadlessError::FrameRejected { frame: handled });
            }
            Some(SessionEnd::Disconnected) => {
                return Err(HeadlessError::GuestDisconnected { frames });
            }
            None if gone => return Err(HeadlessError::GuestDisconnected { frames }),
            None => {}
        }
        if is_frame && limit == Some(frames) {
            let _ = link.send(HostToGuest::Quit);
            return report(&session, HeadlessEnd::FrameLimit, frames);
        }
    }
}

/// Writes what the display shows to `path` as a PNG file.
fn dump(display: &mut HeadlessDisplay, frame: u64, path: &Path) -> Result<(), HeadlessError> {
    let image = display
        .read_shown()
        .map_err(|source| HeadlessError::ReadFrame { frame, source })?
        .expect("a frame was just rendered");
    write_png(path, &image).map_err(|message| HeadlessError::Dump {
        frame,
        path: path.to_owned(),
        message,
    })
}

/// Writes `frame` as an 8-bit RGBA PNG. The encoder settings are fixed, so the same frame
/// always gives the same bytes.
fn write_png(path: &Path, frame: &Frame) -> Result<(), String> {
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), frame.size.width, frame.size.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
    writer
        .write_image_data(&frame.rgba)
        .map_err(|error| error.to_string())?;
    writer.finish().map_err(|error| error.to_string())
}
