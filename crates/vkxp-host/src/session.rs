//! The session: what the host does with guest messages and user input.

use std::collections::{HashSet, VecDeque};
use std::io::Write;
use std::time::Duration;

use vkxp_proto::link::HostEnd;
use vkxp_proto::{BitmapId, Button, FramePacket, GuestToHost, HostToGuest, InputSnapshot};
use winit::keyboard::KeyCode;

use crate::{AudioSink, Clock, Display, KeyMap, NullSink};

/// The key that asks the guest to restart.
const RESET_KEY: KeyCode = KeyCode::F12;
/// The key that shows the measured frame rate in the window title.
const FPS_KEY: KeyCode = KeyCode::F2;
/// The key that engages fast-forward while it is held. A placeholder until the key is settled.
const FAST_FORWARD_KEY: KeyCode = KeyCode::Tab;

/// How often the frame rate in the title is refreshed.
const FPS_INTERVAL: Duration = Duration::from_secs(1);

/// How a session ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEnd {
    /// The guest sent its shutdown message.
    Shutdown,
    /// The guest's end of the link went away without a shutdown message.
    Disconnected,
    /// A frame was rejected in a session that does not continue after that. The frame still
    /// got its tick.
    Failed,
}

impl SessionEnd {
    /// Whether the session ended the way a healthy one does.
    pub fn is_success(self) -> bool {
        self == SessionEnd::Shutdown
    }
}

/// Something a key press asks of the window, which the session itself has no access to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    None,
    /// Alt+Enter: switch between windowed and borderless fullscreen.
    ToggleFullscreen,
}

/// How a session is set up.
pub struct SessionOptions {
    /// The physical keys that hold each button.
    pub keys: KeyMap,
    /// Whether a rejected frame ends the session with [`SessionEnd::Failed`], as a headless
    /// run wants, instead of being reported and skipped. The session ends once the tick of
    /// that frame has been sent: a rejected frame is answered like any other.
    pub fail_on_rejected_frame: bool,
    /// Where modal texts and errors are written.
    pub errors: Box<dyn Write + Send>,
    /// Where audio commands go.
    pub audio: Box<dyn AudioSink>,
}

impl Default for SessionOptions {
    fn default() -> SessionOptions {
        SessionOptions {
            keys: KeyMap::default(),
            fail_on_rejected_frame: false,
            errors: Box::new(std::io::stderr()),
            audio: Box::new(NullSink),
        }
    }
}

/// Where the display is between a freeze and the end of the transition that follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fade {
    /// Frames are shown as they come.
    Normal,
    /// The held frame is shown.
    Frozen,
    /// `step` of `duration` equal steps from the held frame to the newest have been taken.
    Fading { step: u32, duration: u32 },
}

/// The frame rate shown in the title: ticks counted since `since`.
#[derive(Debug, Clone, Copy)]
struct FpsCounter {
    since: Duration,
    ticks: u32,
}

/// A running session. See the crate documentation for its place among the layers.
///
/// # Pacing
///
/// Every frame message is answered with exactly one tick. With `p` the period of the frame
/// being answered and `t_prev` the time the previous tick was actually sent, the tick is due
/// at `max(t_prev + p, t_arrival)`: one period after the previous one, or at once when that
/// has passed. Lateness is never made up, whether the frame arrived late, rendering it took
/// long or the host woke late for the tick: the next tick is a full period after the late one.
/// Fast-forward makes it due at `t_arrival`; after a frame reset it is due at
/// `t_arrival + p`.
pub struct Session<D: Display, C: Clock> {
    display: D,
    clock: C,
    keys: KeyMap,
    fail_on_rejected_frame: bool,
    errors: Box<dyn Write + Send>,
    audio: Box<dyn AudioSink>,
    /// Messages for the guest that have not been handed to the link yet.
    outbox: VecDeque<HostToGuest>,
    /// When the tick of the frame being answered is due.
    pending_tick: Option<Duration>,
    /// When the previous tick was sent.
    previous_tick: Option<Duration>,
    /// A frame message that [`pump`](Session::pump) took from the link while the tick of the
    /// frame before it was still pending.
    waiting_frame: Option<GuestToHost>,
    /// Whether a frame reset is waiting for the frame it applies to.
    frame_reset: bool,
    /// Fast-forward engaged by its key, and by [`Session::set_fast_forward`].
    fast_forward_key: bool,
    fast_forward_forced: bool,
    /// Physical keys currently down.
    held_keys: HashSet<KeyCode>,
    /// Keys that are down as part of a host shortcut and hold no button until released.
    swallowed_keys: HashSet<KeyCode>,
    /// The snapshot the guest was last sent.
    buttons: InputSnapshot,
    fade: Fade,
    /// The mix weight the display is at.
    mix: f32,
    /// The mask bitmap and the vagueness of the transition under way, if it has a mask.
    mask: Option<(BitmapId, u32)>,
    title: String,
    fps: Option<FpsCounter>,
    frames: u64,
    /// Whether sending to the guest has failed. What the guest sent before it went is still
    /// read, so a shutdown message that is already in the link is not lost.
    guest_gone: bool,
    /// Whether a frame was rejected in a session that does not continue after that. The
    /// session ends as failed when the tick of that frame goes out.
    failing: bool,
    end: Option<SessionEnd>,
}

impl<D: Display, C: Clock> Session<D, C> {
    pub fn new(display: D, clock: C, options: SessionOptions) -> Session<D, C> {
        Session {
            display,
            clock,
            keys: options.keys,
            fail_on_rejected_frame: options.fail_on_rejected_frame,
            errors: options.errors,
            audio: options.audio,
            outbox: VecDeque::new(),
            pending_tick: None,
            previous_tick: None,
            waiting_frame: None,
            frame_reset: false,
            fast_forward_key: false,
            fast_forward_forced: false,
            held_keys: HashSet::new(),
            swallowed_keys: HashSet::new(),
            buttons: InputSnapshot::EMPTY,
            fade: Fade::Normal,
            mix: 1.0,
            mask: None,
            title: String::new(),
            fps: None,
            frames: 0,
            guest_gone: false,
            failing: false,
            end: None,
        }
    }

    pub fn display(&self) -> &D {
        &self.display
    }

    pub fn display_mut(&mut self) -> &mut D {
        &mut self.display
    }

    /// How the session ended, or `None` while it runs.
    pub fn end(&self) -> Option<SessionEnd> {
        self.end
    }

    /// Whether a frame has been rejected in a session that does not continue after that, and
    /// the session is only waiting to send that frame's tick before it ends.
    pub fn is_failing(&self) -> bool {
        self.failing
    }

    /// The number of frame messages handled so far.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// When [`poll`](Session::poll) next has something to do, on the session's clock, or
    /// `None` when only a guest message or user input can move the session on.
    pub fn next_wake(&self) -> Option<Duration> {
        self.pending_tick.filter(|_| self.end.is_none())
    }

    /// Whether fast-forward is engaged.
    pub fn fast_forward(&self) -> bool {
        self.fast_forward_key || self.fast_forward_forced
    }

    /// Engages or releases fast-forward independently of its key.
    pub fn set_fast_forward(&mut self, engaged: bool) {
        self.fast_forward_forced = engaged;
        self.hurry();
    }

    /// The messages for the guest produced so far, oldest first, removing them.
    pub fn take_messages(&mut self) -> Vec<HostToGuest> {
        self.outbox.drain(..).collect()
    }

    /// Handles one message from the guest.
    pub fn handle_message(&mut self, message: GuestToHost) {
        if self.end.is_some() {
            return;
        }
        match message {
            GuestToHost::Frame(packet) => self.frame(&packet),
            GuestToHost::Freeze => self.freeze(),
            GuestToHost::Transition {
                duration,
                mask,
                vague,
            } => {
                // A vagueness of 0 is treated as 1.
                let mask = mask.map(|id| (id, u32::from(vague).max(1)));
                self.transition(duration, mask);
            }
            GuestToHost::Audio(command) => self.audio.command(command),
            GuestToHost::Modal { text } => {
                self.report(format_args!("{text}"));
                self.outbox.push_back(HostToGuest::ModalDismissed);
            }
            GuestToHost::SetTitle { title } => {
                self.title = title;
                self.display.set_title(&self.title);
                if let Some(fps) = &mut self.fps {
                    // The rate reappears in the title at the next refresh.
                    *fps = FpsCounter {
                        since: self.clock.now(),
                        ticks: 0,
                    };
                }
            }
            GuestToHost::Shutdown => self.end = Some(SessionEnd::Shutdown),
            GuestToHost::FrameReset => self.frame_reset = true,
        }
    }

    /// The guest's end of the link has gone.
    pub fn guest_disconnected(&mut self) {
        self.end.get_or_insert(SessionEnd::Disconnected);
    }

    /// Sends the pending tick when it is due.
    pub fn poll(&mut self) {
        if self.end.is_some() {
            return;
        }
        let now = self.clock.now();
        if self.pending_tick.is_some_and(|due| due <= now) {
            self.send_tick(now);
        }
    }

    /// Runs the session against its link as far as it can go without waiting: sends what is
    /// due, hands over the messages for the guest and handles the guest's messages.
    ///
    /// A pending tick does not hold up what the guest sends after its frame: a modal is
    /// answered at once, a title is set, a freeze applies to the frame already shown. Only
    /// the next frame waits. It is taken from the link and set aside until the tick has gone
    /// out, and nothing behind it is read before then, so every message is handled in the
    /// order it was sent and every frame is answered in turn.
    ///
    /// Call it again at [`next_wake`](Session::next_wake) and whenever the link's wake hook
    /// has run or the user did something.
    pub fn pump(&mut self, link: &HostEnd) {
        loop {
            if self.guest_gone && self.pending_tick.is_some() {
                // Nobody waits for this tick; do not let it hold up the rest of the link.
                self.send_tick(self.clock.now());
            }
            self.poll();
            self.flush(link);
            if self.end.is_some() {
                return;
            }
            if self.pending_tick.is_none()
                && let Some(frame) = self.waiting_frame.take()
            {
                self.handle_message(frame);
                continue;
            }
            if self.waiting_frame.is_some() {
                // A frame waits for the tick of the one before it; see above.
                return;
            }
            match link.try_recv() {
                Ok(Some(message @ GuestToHost::Frame(_))) if self.pending_tick.is_some() => {
                    self.waiting_frame = Some(message);
                }
                Ok(Some(message)) => self.handle_message(message),
                Ok(None) => break,
                Err(_) => self.guest_disconnected(),
            }
        }
        self.flush(link);
    }

    /// A physical key went down or up. Repeats of a key that is already down are ignored.
    pub fn key(&mut self, key: KeyCode, pressed: bool) -> KeyAction {
        if self.end.is_some() {
            return KeyAction::None;
        }
        let mut action = KeyAction::None;
        if pressed {
            if !self.held_keys.insert(key) {
                return KeyAction::None;
            }
            let alt = [KeyCode::AltLeft, KeyCode::AltRight]
                .iter()
                .any(|alt| self.held_keys.contains(alt));
            match key {
                RESET_KEY => self.outbox.push_back(HostToGuest::Reset),
                FPS_KEY => self.toggle_fps(),
                FAST_FORWARD_KEY => {
                    self.fast_forward_key = true;
                    self.hurry();
                }
                KeyCode::Enter | KeyCode::NumpadEnter if alt => {
                    self.swallowed_keys.insert(key);
                    action = KeyAction::ToggleFullscreen;
                }
                _ => {}
            }
        } else {
            self.held_keys.remove(&key);
            self.swallowed_keys.remove(&key);
            if key == FAST_FORWARD_KEY {
                self.fast_forward_key = false;
            }
        }
        self.update_buttons();
        action
    }

    /// The window lost keyboard focus: every key counts as released.
    pub fn focus_lost(&mut self) {
        self.held_keys.clear();
        self.swallowed_keys.clear();
        self.fast_forward_key = false;
        self.update_buttons();
    }

    /// Sets the held buttons directly, bypassing the key map, as a scripted run does.
    pub fn set_buttons(&mut self, snapshot: InputSnapshot) {
        if self.end.is_none() && snapshot != self.buttons {
            self.buttons = snapshot;
            self.outbox.push_back(HostToGuest::Input(snapshot));
        }
    }

    /// The user asked to close the window. The session runs on until the guest shuts down.
    pub fn close_requested(&mut self) {
        self.request_quit();
    }

    /// Asks the guest to shut down.
    pub fn request_quit(&mut self) {
        if self.end.is_none() {
            self.outbox.push_back(HostToGuest::Quit);
        }
    }

    fn frame(&mut self, packet: &FramePacket) {
        let arrival = self.clock.now();
        if self.pending_tick.is_some() {
            // A guest that keeps to the handshake never gets here. Answering the earlier frame
            // first keeps one tick per frame.
            self.send_tick(arrival);
            if self.end.is_some() {
                // That was the tick of a rejected frame in a session that ends with it.
                return;
            }
        }
        self.frames += 1;

        // The step of a cross-fade this frame takes, with the weight that shows it. The
        // display needs the weight before it renders; the session only counts the step once
        // the frame has been shown.
        let step = match self.fade {
            Fade::Fading { step, duration } => {
                let step = step + 1;
                Some((step, duration, step as f32 / duration as f32))
            }
            Fade::Normal | Fade::Frozen => None,
        };
        if let Some((_, _, weight)) = step {
            self.display.set_mix(weight);
        }
        match self.display.render(packet) {
            Ok(()) => {
                if let Some((step, duration, weight)) = step {
                    self.mix = weight;
                    // The mask comes with the frames after the transition message, so only
                    // now is it known whether there is one. Without it the display shows a
                    // cross-fade.
                    if let Some((id, _)) = self.mask
                        && !self.display.has_bitmap(id)
                    {
                        self.report(format_args!(
                            "the transition's mask, bitmap {} of generation {}, does not \
                             exist; showing a plain cross-fade",
                            id.index, id.generation
                        ));
                        self.set_mask(None);
                    }
                    self.fade = if step >= duration {
                        self.set_mask(None);
                        Fade::Normal
                    } else {
                        Fade::Fading { step, duration }
                    };
                }
            }
            Err(error) => {
                if step.is_some() {
                    // The image on screen stays, so its weight does too: a redraw must not
                    // show the frame before at the weight meant for this one.
                    self.display.set_mix(self.mix);
                }
                self.report(format_args!(
                    "frame {} was rejected: {error}",
                    packet.frame_index
                ));
                // The frame is answered with its tick all the same; see `send_tick`.
                self.failing |= self.fail_on_rejected_frame;
            }
        }

        let period = Duration::from_secs(1) / u32::from(packet.frame_rate.get());
        let due = if self.fast_forward() {
            arrival
        } else if self.frame_reset {
            arrival + period
        } else {
            match self.previous_tick {
                Some(previous) => (previous + period).max(arrival),
                None => arrival,
            }
        };
        self.frame_reset = false;
        self.pending_tick = Some(due);
        self.poll();
    }

    /// Sends the pending tick at the time `now`, from which the next one is timed.
    fn send_tick(&mut self, now: Duration) {
        self.pending_tick = None;
        self.previous_tick = Some(now);
        self.outbox.push_back(HostToGuest::Tick);
        if self.failing {
            // The rejected frame has its one tick; nothing else is answered.
            self.end = Some(SessionEnd::Failed);
            return;
        }
        if let Some(fps) = &mut self.fps {
            fps.ticks += 1;
            let elapsed = now.saturating_sub(fps.since);
            if elapsed >= FPS_INTERVAL {
                let rate = f64::from(fps.ticks) / elapsed.as_secs_f64();
                *fps = FpsCounter {
                    since: now,
                    ticks: 0,
                };
                let title = format!("{} - {rate:.1} FPS", self.title);
                self.display.set_title(&title);
            }
        }
    }

    /// Makes a pending tick due now if fast-forward is engaged.
    fn hurry(&mut self) {
        if self.fast_forward()
            && let Some(due) = &mut self.pending_tick
        {
            *due = (*due).min(self.clock.now());
        }
    }

    fn freeze(&mut self) {
        if self.fade == Fade::Frozen {
            // Already showing a held frame; that one stays.
            return;
        }
        // In the middle of a cross-fade this captures the mixed image on screen.
        if let Err(error) = self.display.hold() {
            self.report(format_args!("cannot freeze the display: {error}"));
        }
        self.set_mix(0.0);
        self.set_mask(None);
        self.fade = Fade::Frozen;
    }

    fn transition(&mut self, duration: u32, mask: Option<(BitmapId, u32)>) {
        match self.fade {
            // Nothing is held, so there is nothing to fade from.
            Fade::Normal => {}
            Fade::Frozen | Fade::Fading { .. } if duration == 0 => {
                self.fade = Fade::Normal;
                self.set_mix(1.0);
                self.set_mask(None);
                if let Err(error) = self.display.redraw() {
                    self.report(format_args!("cannot redraw the display: {error}"));
                }
            }
            Fade::Frozen | Fade::Fading { .. } => {
                self.fade = Fade::Fading { step: 0, duration };
                self.set_mask(mask);
            }
        }
    }

    /// Sets the mask of the transition under way, telling the display when it changes.
    fn set_mask(&mut self, mask: Option<(BitmapId, u32)>) {
        if self.mask != mask {
            self.mask = mask;
            self.display.set_mask(mask);
        }
    }

    fn set_mix(&mut self, weight: f32) {
        self.mix = weight;
        self.display.set_mix(weight);
    }

    fn toggle_fps(&mut self) {
        self.fps = match self.fps {
            Some(_) => {
                self.display.set_title(&self.title);
                None
            }
            None => Some(FpsCounter {
                since: self.clock.now(),
                ticks: 0,
            }),
        };
    }

    /// Sends the guest a snapshot if the set of held buttons has changed.
    fn update_buttons(&mut self) {
        let snapshot = self
            .held_keys
            .iter()
            .filter(|key| !self.swallowed_keys.contains(key))
            .flat_map(|key| self.keys.buttons(*key))
            .fold(
                InputSnapshot::EMPTY,
                |snapshot: InputSnapshot, button: Button| snapshot.with(button),
            );
        self.set_buttons(snapshot);
    }

    /// Hands the messages produced so far to the link.
    fn flush(&mut self, link: &HostEnd) {
        while let Some(message) = self.outbox.pop_front() {
            if self.guest_gone || link.send(message).is_err() {
                self.guest_gone = true;
                self.outbox.clear();
            }
        }
    }

    fn report(&mut self, message: std::fmt::Arguments<'_>) {
        // Nothing useful can be done when the error stream itself fails.
        let _ = writeln!(self.errors, "vkxp: {message}");
    }
}

#[cfg(test)]
mod tests;
