//! The session: what the host does with guest messages and user input.

use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

use vkxp_proto::link::HostEnd;
use vkxp_proto::{
    AudioChannel, AudioCommand, BitmapId, Button, FramePacket, GuestToHost, HostToGuest,
    InputSnapshot, Size,
};
use vkxp_vk::Mix;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::raw::with_raw_keys;
use crate::report::ErrorStream;
use crate::stats::FrameStats;
use crate::{AudioSink, Clock, Display, InputSource, KeyMap, NullSink, Reporter, Severity};

/// The key that asks the guest to restart.
const RESET_KEY: KeyCode = KeyCode::F12;
/// The key that shows the measured frame rate in the window title.
const FPS_KEY: KeyCode = KeyCode::F2;
/// The key that, like Alt+Enter, switches between windowed and borderless fullscreen.
const FULLSCREEN_KEY: KeyCode = KeyCode::F11;
/// The key that engages fast-forward while it is held. A placeholder until the key is settled.
const FAST_FORWARD_KEY: KeyCode = KeyCode::Tab;

/// The key that saves a state into the chosen slot, and with Shift chooses the next slot.
const SAVE_STATE_KEY: KeyCode = KeyCode::F3;
/// The key that loads the state of the chosen slot, and with Shift chooses the slot before.
const LOAD_STATE_KEY: KeyCode = KeyCode::F4;
/// The number of state slots.
const STATE_SLOTS: u8 = 10;
/// How long a notice about states stays in the title.
const NOTICE_TIME: Duration = Duration::from_secs(2);

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
    /// Alt+Enter or F11: switch between windowed and borderless fullscreen.
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
    /// Whether a modal message stays pending until the player dismisses it, as a windowed
    /// session wants, instead of being dismissed at once.
    pub wait_for_modal: bool,
    /// Where modal texts and errors are reported.
    pub errors: Box<dyn Reporter>,
    /// Where audio commands go.
    pub audio: Box<dyn AudioSink>,
    /// Where the input snapshot of each frame comes from: a recorder or a replayer. With a
    /// source the guest is sent the source's snapshot for a frame, if it differs from the
    /// one before, ahead of that frame's tick and at no other time. Without one, live input
    /// is sent as it changes.
    pub input: Option<Box<dyn InputSource>>,
    /// Whether the session collects the frame time statistics: how long it waits for the
    /// guest between frames, and what the display measures of each frame. Their summary is
    /// written to `errors` when the session is dropped.
    pub frame_stats: bool,
    /// With `frame_stats`: the summary is also written after every so many frames, and the
    /// statistics start anew each time.
    pub frame_stats_interval: Option<usize>,
    /// Whether the guest can be saved and loaded: it runs in a process of its own, behind
    /// something that keeps its states. Without it the state keys only say so.
    pub states: bool,
}

impl Default for SessionOptions {
    fn default() -> SessionOptions {
        SessionOptions {
            keys: KeyMap::default(),
            fail_on_rejected_frame: false,
            wait_for_modal: false,
            errors: Box::new(ErrorStream),
            audio: Box::new(NullSink),
            input: None,
            frame_stats: false,
            frame_stats_interval: None,
            states: false,
        }
    }
}

/// The frame time statistics of a session that collects them.
struct Measuring {
    stats: FrameStats,
    /// The frames after which a summary is written, besides the one at the end.
    interval: Option<usize>,
    /// When the tick that answers the frame before was handed over for the guest, on the
    /// host's monotonic clock and not on the session's, which a headless run moves by hand.
    /// `None` until that tick has left the session: what the host does between queueing a
    /// tick and handing it over, such as writing a dump, is not the guest's time.
    answered: Option<Instant>,
    /// Whether a summary has been written.
    reported: bool,
}

/// Where the display is between a freeze and the end of the transition that follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fade {
    /// Frames are shown as they come.
    Normal,
    /// The held frame is shown.
    Frozen,
    /// `shown` frames have been shown since the transition message. The next one is shown at
    /// step `shown` of `duration` steps from the held frame to the newest, with the values of
    /// [`step_mix`], and the one after the `duration`-th is shown alone and ends the
    /// transition. A `still` transition,
    /// one that arrived while nothing was frozen, shows the held frame at every step.
    Fading {
        shown: u32,
        duration: u32,
        still: bool,
    },
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
///
/// The display is told the frame rate of every frame before it renders it, and that its own
/// pacing starts anew with a frame that does not follow the one before it by a period: the
/// frame after a frame reset, every frame while fast-forward is engaged and the first after
/// it was engaged or released, and the first after a modal message that waited for the
/// player.
///
/// # Frame time statistics
///
/// A session that was asked to (see [`SessionOptions::frame_stats`]) measures the time from
/// answering a frame, which is when its tick leaves the session through
/// [`take_messages`](Session::take_messages) or [`pump`](Session::pump), to receiving the
/// next frame packet, and takes what its display measured
/// of each frame it rendered. That reads the host's clock and nothing else: no tick is due
/// earlier or later for it and the guest is sent nothing else.
///
/// # States
///
/// The session has ten state slots, 0 to 9, and starts with slot 0 chosen. F3 asks the guest
/// to save a state into the chosen slot and F4 asks for the state of that slot to be loaded
/// ([`save_state`](Session::save_state), [`load_state`](Session::load_state)); with Shift
/// they choose the next and the previous slot. What became of a request, and a change of
/// slot, is shown in the title for two seconds and reported as [`Severity::Info`]. The keys
/// send nothing, and the title says why, in a session whose guest cannot be saved
/// ([`SessionOptions::states`]) and in one that records or replays its input. While a modal
/// message waits they do nothing.
///
/// From a request to load until its answer no tick is sent: a tick sent then would answer
/// a frame of the guest that the load ends, and reach the one that takes its place. When a
/// state was loaded the session holds nothing of the guest that goes on from it: the display
/// forgets every bitmap and table, a freeze or transition ends, a modal message is dropped,
/// the four audio channels stop, the pacing starts anew as after a frame reset and the
/// guest is sent the current input again.
///
/// The audio sink is asked to remember a slot when a state was saved into it, and to recall
/// it when its state was loaded, behind the stop of the channels: the music and the
/// background sound that the guest then plays again go on where they were.
pub struct Session<D: Display, C: Clock> {
    display: D,
    clock: C,
    keys: KeyMap,
    fail_on_rejected_frame: bool,
    wait_for_modal: bool,
    /// Whether a modal message waits for the player to dismiss it.
    pending_modal: bool,
    errors: Box<dyn Reporter>,
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
    /// Whether the display is to start its pacing anew with the next frame.
    pacing_restart: bool,
    /// Fast-forward engaged by its key, and by [`Session::set_fast_forward`].
    fast_forward_key: bool,
    fast_forward_forced: bool,
    /// Physical keys currently down.
    held_keys: HashSet<KeyCode>,
    /// Keys that are down as part of a host shortcut and hold no button until released.
    swallowed_keys: HashSet<KeyCode>,
    /// Mouse buttons currently down.
    held_mouse: HashSet<MouseButton>,
    /// Where the pointer is in the frame, when it is over the picture.
    pointer: Option<(i16, i16)>,
    /// The logical size of the newest frame the guest sent.
    frame_size: Option<Size>,
    /// The buttons the gamepads hold.
    pad: InputSnapshot,
    /// The snapshot the guest was last sent.
    buttons: InputSnapshot,
    /// The buttons live input holds, which is what the guest is sent unless `input` says
    /// otherwise.
    live: InputSnapshot,
    input: Option<Box<dyn InputSource>>,
    fade: Fade,
    /// The mix the display is at.
    mix: Mix,
    /// The mask bitmap and the vagueness of the transition under way, if it has a mask.
    mask: Option<(BitmapId, u32)>,
    /// The mask the display was told last. It is that of the transition from step 1 on:
    /// step 0 shows the held frame alone, which no threshold does under a mask.
    display_mask: Option<(BitmapId, u32)>,
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
    /// The frame time statistics, when the session collects them.
    measuring: Option<Measuring>,
    /// Whether the guest can be saved and loaded.
    states: bool,
    /// The chosen state slot.
    slot: u8,
    /// The slot of the request to load a state that has not been answered yet.
    loading: Option<u8>,
    /// The notice about states that the title shows, and until when.
    notice: Option<(String, Duration)>,
}

impl<D: Display, C: Clock> Session<D, C> {
    pub fn new(display: D, clock: C, options: SessionOptions) -> Session<D, C> {
        Session {
            display,
            clock,
            keys: options.keys,
            fail_on_rejected_frame: options.fail_on_rejected_frame,
            wait_for_modal: options.wait_for_modal,
            pending_modal: false,
            errors: options.errors,
            audio: options.audio,
            outbox: VecDeque::new(),
            pending_tick: None,
            previous_tick: None,
            waiting_frame: None,
            frame_reset: false,
            pacing_restart: false,
            fast_forward_key: false,
            fast_forward_forced: false,
            held_keys: HashSet::new(),
            swallowed_keys: HashSet::new(),
            held_mouse: HashSet::new(),
            pointer: None,
            frame_size: None,
            pad: InputSnapshot::EMPTY,
            buttons: InputSnapshot::EMPTY,
            live: InputSnapshot::EMPTY,
            input: options.input,
            fade: Fade::Normal,
            mix: Mix::Newest,
            mask: None,
            display_mask: None,
            title: String::new(),
            fps: None,
            frames: 0,
            guest_gone: false,
            failing: false,
            end: None,
            measuring: options.frame_stats.then(|| Measuring {
                stats: FrameStats::default(),
                interval: options.frame_stats_interval,
                answered: None,
                reported: false,
            }),
            states: options.states,
            slot: 0,
            loading: None,
            notice: None,
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

    /// When [`poll`](Session::poll) next has something to do, on the session's clock: the
    /// earlier of the time the pending tick is due and the end of the notice in the title.
    /// `None` when only a guest message or user input can move the session on. While a
    /// request to load a state waits for its answer the pending tick is not due, but the
    /// end of a notice still is.
    pub fn next_wake(&self) -> Option<Duration> {
        if self.end.is_some() {
            return None;
        }
        let tick = self.pending_tick.filter(|_| self.loading.is_none());
        let notice = self.notice.as_ref().map(|(_, until)| *until);
        tick.into_iter().chain(notice).min()
    }

    /// Whether fast-forward is engaged.
    pub fn fast_forward(&self) -> bool {
        self.fast_forward_key || self.fast_forward_forced
    }

    /// Engages or releases fast-forward independently of its key.
    pub fn set_fast_forward(&mut self, engaged: bool) {
        self.fast_forward_forced = engaged;
        self.pacing_restart = true;
        self.hurry();
    }

    /// The messages for the guest produced so far, oldest first, removing them.
    pub fn take_messages(&mut self) -> Vec<HostToGuest> {
        let messages: Vec<HostToGuest> = self.outbox.drain(..).collect();
        if messages.contains(&HostToGuest::Tick) {
            self.tick_delivered();
        }
        messages
    }

    /// The tick that answers a frame has left the session: the wait for the guest's next
    /// frame starts here, when it is measured.
    fn tick_delivered(&mut self) {
        if let Some(measuring) = &mut self.measuring {
            measuring.answered = Some(Instant::now());
        }
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
                let mask = mask.map(|id| (id, u32::from(vague)));
                self.transition(duration, mask);
            }
            GuestToHost::Audio(command) => self.audio.command(command),
            GuestToHost::Modal { text } => {
                self.report(Severity::Always, format_args!("{text}"));
                if self.wait_for_modal {
                    self.pending_modal = true;
                } else {
                    self.outbox.push_back(HostToGuest::ModalDismissed);
                }
            }
            GuestToHost::SetTitle { title } => {
                self.title = title;
                // A notice that is not over stays behind the new title: a resumed guest
                // sends its title right after the answer that its state was loaded.
                let now = self.clock.now();
                self.notice.take_if(|(_, until)| *until <= now);
                match &self.notice {
                    Some((notice, _)) => {
                        let title = format!("{} - {notice}", self.title);
                        self.display.set_title(&title);
                    }
                    None => self.display.set_title(&self.title),
                }
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
            // What stands between the session and a guest in a process keeps these.
            GuestToHost::Attach { .. } | GuestToHost::Outcome { .. } => {}
            GuestToHost::StateSaved { slot } => {
                self.audio.remember(slot);
                self.show_notice(format_args!("state saved into slot {slot}"));
            }
            GuestToHost::StateLoaded { slot } => self.state_loaded(slot),
            GuestToHost::StateFailed { slot, load: true } => {
                self.show_notice(format_args!("slot {slot} holds no state"));
                if self.loading == Some(slot) {
                    self.loading = None;
                    // The tick that waited for the answer may be due.
                    self.poll();
                }
            }
            GuestToHost::StateFailed { slot, load: false } => {
                self.show_notice(format_args!("no state could be saved into slot {slot}"));
            }
            GuestToHost::InjectKey { .. } | GuestToHost::PointerVisible { .. } => {}
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
        if self.notice.as_ref().is_some_and(|(_, until)| *until <= now) {
            self.notice = None;
            // The frame rate comes back at its next refresh.
            self.display.set_title(&self.title);
        }
        // No tick while a request to load a state waits for its answer; see "States".
        if self.loading.is_none() && self.pending_tick.is_some_and(|due| due <= now) {
            self.send_tick(now);
        }
    }

    /// Asks the guest to save a state into `slot`, as F3 does for the chosen slot. The
    /// answer is shown as a notice. Nothing is asked while a modal message waits, and a
    /// session whose guest cannot be saved shows why instead.
    pub fn save_state(&mut self, slot: u8) {
        if self.states_offered() {
            self.outbox.push_back(HostToGuest::SaveState { slot });
        }
    }

    /// Asks for the state of `slot` to be loaded, as F4 does for the chosen slot, under the
    /// conditions of [`save_state`](Session::save_state). No tick is sent until the answer
    /// has arrived, and while it has not, another request to load is dropped.
    pub fn load_state(&mut self, slot: u8) {
        if self.loading.is_none() && self.states_offered() {
            self.loading = Some(slot);
            self.outbox.push_back(HostToGuest::LoadState { slot });
        }
    }

    /// The chosen state slot.
    pub fn state_slot(&self) -> u8 {
        self.slot
    }

    /// Whether a state request can be made now. When the session has no states to offer,
    /// a notice says why.
    fn states_offered(&mut self) -> bool {
        if self.end.is_some() || self.pending_modal {
            return false;
        }
        let reason = if !self.states {
            "states need the guest in a process of its own (--guest process)"
        } else if self.input.is_some() {
            "no states while input is recorded or replayed"
        } else {
            return true;
        };
        self.show_notice(format_args!("{reason}"));
        false
    }

    /// Chooses the slot `step` after the chosen one, wrapping around.
    fn choose_slot(&mut self, step: u8) {
        if self.pending_modal {
            return;
        }
        self.slot = (self.slot + step) % STATE_SLOTS;
        let slot = self.slot;
        self.show_notice(format_args!("state slot {slot}"));
    }

    /// Shows `notice` in the title for [`NOTICE_TIME`] and reports it.
    fn show_notice(&mut self, notice: std::fmt::Arguments<'_>) {
        let notice = notice.to_string();
        let title = format!("{} - {notice}", self.title);
        self.display.set_title(&title);
        self.report(Severity::Info, format_args!("{notice}"));
        self.notice = Some((notice, self.clock.now() + NOTICE_TIME));
    }

    /// The state of `slot` was loaded: the guest that sends from here on is one the session
    /// holds nothing of. See "States".
    fn state_loaded(&mut self, slot: u8) {
        self.loading = None;
        if let Err(error) = self.display.forget_resources() {
            self.report(
                Severity::Error,
                format_args!("cannot forget the bitmaps and tables of the display: {error}"),
            );
        }
        self.fade = Fade::Normal;
        self.set_mix(Mix::Newest);
        self.drop_mask();
        self.pending_modal = false;
        for channel in [
            AudioChannel::Bgm,
            AudioChannel::Bgs,
            AudioChannel::Me,
            AudioChannel::Se,
        ] {
            self.audio.command(AudioCommand::Stop { channel });
        }
        // The guest that goes on plays its music again, which then goes on too.
        self.audio.recall(slot);
        // The frame that waited for its tick was one of the guest that is gone.
        self.pending_tick = None;
        self.waiting_frame = None;
        self.frame_reset = true;
        self.outbox.push_back(HostToGuest::Input(self.buttons));
        self.show_notice(format_args!("state loaded from slot {slot}"));
    }

    /// Runs the session against its link as far as it can go without waiting: sends what is
    /// due, hands over the messages for the guest and handles the guest's messages.
    ///
    /// A pending tick does not hold up what the guest sends after its frame: a modal is
    /// answered at once unless the session waits for the player, a title is set, a freeze applies to the frame already shown. Only
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
            let shift = [KeyCode::ShiftLeft, KeyCode::ShiftRight]
                .iter()
                .any(|shift| self.held_keys.contains(shift));
            match key {
                RESET_KEY => self.outbox.push_back(HostToGuest::Reset),
                // Whatever the key map says of these keys, this press holds no button.
                FULLSCREEN_KEY => {
                    self.swallowed_keys.insert(key);
                    action = KeyAction::ToggleFullscreen;
                }
                KeyCode::Enter | KeyCode::NumpadEnter if alt => {
                    self.swallowed_keys.insert(key);
                    action = KeyAction::ToggleFullscreen;
                }
                // The press that dismisses a modal message holds no button either, and is
                // not the shortcut the key otherwise is: the key map may put C or B on the
                // keys below.
                _ if self.pending_modal && self.dismisses(key) => {
                    self.swallowed_keys.insert(key);
                    self.dismiss_modal();
                }
                FPS_KEY => self.toggle_fps(),
                SAVE_STATE_KEY | LOAD_STATE_KEY => {
                    self.swallowed_keys.insert(key);
                    match (key == SAVE_STATE_KEY, shift) {
                        (true, false) => self.save_state(self.slot),
                        (false, false) => self.load_state(self.slot),
                        (true, true) => self.choose_slot(1),
                        (false, true) => self.choose_slot(STATE_SLOTS - 1),
                    }
                }
                FAST_FORWARD_KEY => {
                    self.fast_forward_key = true;
                    self.pacing_restart = true;
                    self.hurry();
                }
                _ => {}
            }
        } else {
            self.held_keys.remove(&key);
            self.swallowed_keys.remove(&key);
            if key == FAST_FORWARD_KEY {
                self.fast_forward_key = false;
                self.pacing_restart = true;
            }
        }
        self.update_buttons();
        action
    }

    /// A mouse button was clicked.
    pub fn click(&mut self) {
        if self.end.is_none() {
            self.dismiss_modal();
        }
    }

    /// A mouse button went down or up.
    pub fn mouse_button(&mut self, button: MouseButton, pressed: bool) {
        if self.end.is_some() {
            return;
        }
        if pressed {
            self.held_mouse.insert(button);
        } else {
            self.held_mouse.remove(&button);
        }
        self.update_buttons();
    }

    /// The pointer is at `pointer` in the frame, or not over the picture.
    pub fn pointer_moved(&mut self, pointer: Option<(i16, i16)>) {
        if self.end.is_none() && pointer != self.pointer {
            self.pointer = pointer;
            self.update_buttons();
        }
    }

    /// The logical size of the newest frame the guest sent, which is the picture a pointer
    /// position is mapped into.
    pub fn frame_size(&self) -> Option<Size> {
        self.frame_size
    }

    /// Whether `key` holds a button that dismisses a modal message.
    fn dismisses(&self, key: KeyCode) -> bool {
        self.keys
            .buttons(key)
            .any(|button| matches!(button, Button::C | Button::B))
    }

    /// Answers the pending modal message, if there is one.
    fn dismiss_modal(&mut self) {
        if self.pending_modal {
            self.pending_modal = false;
            // The guest stood still for as long as the message did.
            self.pacing_restart = true;
            self.outbox.push_back(HostToGuest::ModalDismissed);
        }
    }

    /// The window lost keyboard focus: every key and mouse button counts as released.
    pub fn focus_lost(&mut self) {
        self.held_keys.clear();
        self.held_mouse.clear();
        self.swallowed_keys.clear();
        self.pacing_restart |= self.fast_forward_key;
        self.fast_forward_key = false;
        self.update_buttons();
    }

    /// The gamepads hold `snapshot` from now on, together with what the keyboard holds.
    pub fn pad_buttons(&mut self, snapshot: InputSnapshot) {
        if self.end.is_none() && snapshot != self.pad {
            self.pad = snapshot;
            self.update_buttons();
        }
    }

    /// Sets the held buttons directly, bypassing the key map, as a scripted run does.
    pub fn set_buttons(&mut self, snapshot: InputSnapshot) {
        self.live = snapshot;
        if self.input.is_none() {
            self.deliver(snapshot);
        }
    }

    /// Sends the guest a snapshot if it differs from the one it was last sent.
    fn deliver(&mut self, snapshot: InputSnapshot) {
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
        if let Some(measuring) = &mut self.measuring
            && let Some(answered) = measuring.answered.take()
        {
            measuring.stats.add_guest(answered.elapsed());
        }
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
        self.frame_size = Some(packet.logical_size);
        if let Some(input) = &mut self.input {
            // Queued ahead of this frame's tick.
            let snapshot = input.next(self.live);
            self.deliver(snapshot);
        }

        // The step of a transition this frame is shown at, with the mix that shows it. The
        // display needs the mix before it renders; the session only counts the frame once it
        // has been shown.
        let step = match self.fade {
            Fade::Fading {
                shown,
                duration,
                still,
            } => {
                let mix = if shown >= duration {
                    Mix::Newest
                } else if still {
                    Mix::HELD
                } else {
                    step_mix(shown, duration)
                };
                Some((shown, duration, still, mix))
            }
            Fade::Normal | Fade::Frozen => None,
        };
        let mask_before = self.display_mask;
        if let Some((shown, duration, _, mix)) = step {
            self.display.set_mix(mix);
            // Step 0 shows the held frame alone, so not under the mask of a transition that
            // this one took over from either.
            if shown == 0 {
                self.show_mask(None);
            } else if shown < duration {
                self.show_mask(self.mask);
            }
        }
        let restart = std::mem::take(&mut self.pacing_restart);
        if restart || self.frame_reset || self.fast_forward() {
            self.display.restart_pacing();
        }
        self.display.set_frame_rate(packet.frame_rate);
        match self.display.render(packet) {
            Ok(()) => {
                if self.measuring.is_some() {
                    self.frame_measured();
                }
                if let Some((shown, duration, still, mix)) = step {
                    self.mix = mix;
                    // The mask comes with the frames after the transition message, so only
                    // now is it known whether there is one. Without it the display shows a
                    // cross-fade. The frame past the last step is shown alone, and the guest
                    // has disposed of the mask by then.
                    if shown < duration
                        && let Some((id, _)) = self.mask
                        && !self.display.has_bitmap(id)
                    {
                        self.report(
                            Severity::Warning,
                            format_args!(
                                "the transition's mask, bitmap {} of generation {}, does not \
                                 exist; showing a plain cross-fade",
                                id.index, id.generation
                            ),
                        );
                        self.drop_mask();
                    }
                    self.fade = if shown >= duration {
                        self.drop_mask();
                        Fade::Normal
                    } else {
                        Fade::Fading {
                            shown: shown + 1,
                            duration,
                            still,
                        }
                    };
                }
            }
            Err(error) => {
                if step.is_some() {
                    // The image on screen stays, so its mix does too: a redraw must not show
                    // the frame before in the mix meant for this one.
                    self.display.set_mix(self.mix);
                    self.show_mask(mask_before);
                }
                self.report(
                    Severity::Error,
                    format_args!("frame {} was rejected: {error}", packet.frame_index),
                );
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
                if self.notice.is_none() {
                    let title = format!("{} - {rate:.1} FPS", self.title);
                    self.display.set_title(&title);
                }
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
            self.report(
                Severity::Error,
                format_args!("cannot freeze the display: {error}"),
            );
        }
        self.set_mix(Mix::HELD);
        self.drop_mask();
        self.fade = Fade::Frozen;
    }

    fn transition(&mut self, duration: u32, mask: Option<(BitmapId, u32)>) {
        match self.fade {
            Fade::Normal if duration == 0 => {}
            // Nothing is frozen: the picture on screen stays for the transition's frames.
            Fade::Normal => {
                if let Err(error) = self.display.hold() {
                    self.report(
                        Severity::Error,
                        format_args!("cannot freeze the display: {error}"),
                    );
                }
                self.set_mix(Mix::HELD);
                self.fade = Fade::Fading {
                    shown: 0,
                    duration,
                    still: true,
                };
            }
            // With duration 0 nothing changes on the display: the next frame is past the
            // last step and is shown alone.
            Fade::Frozen | Fade::Fading { .. } => {
                self.fade = Fade::Fading {
                    shown: 0,
                    duration,
                    still: false,
                };
                // The display is told at step 1: step 0 shows the held frame alone.
                self.mask = mask.filter(|_| duration > 0);
            }
        }
    }

    /// Drops the mask of the transition under way, on the display too.
    fn drop_mask(&mut self) {
        self.mask = None;
        self.show_mask(None);
    }

    /// Tells the display the mask when it changes.
    fn show_mask(&mut self, mask: Option<(BitmapId, u32)>) {
        if self.display_mask != mask {
            self.display_mask = mask;
            self.display.set_mask(mask);
        }
    }

    fn set_mix(&mut self, mix: Mix) {
        self.mix = mix;
        self.display.set_mix(mix);
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

    /// Sends the guest a snapshot if the held buttons, the raw keys or the pointer have
    /// changed.
    fn update_buttons(&mut self) {
        let buttons = self
            .held_keys
            .iter()
            .filter(|key| !self.swallowed_keys.contains(key))
            .flat_map(|key| self.keys.buttons(*key))
            .fold(self.pad, |snapshot: InputSnapshot, button: Button| {
                snapshot.with(button)
            });
        // The raw state is that of the keyboard: a key a host shortcut took is held in it.
        let snapshot = with_raw_keys(
            buttons,
            self.held_keys.iter().copied(),
            self.held_mouse.iter().copied(),
        );
        self.set_buttons(snapshot.with_pointer(self.pointer));
    }

    /// Hands the messages produced so far to the link.
    fn flush(&mut self, link: &HostEnd) {
        while let Some(message) = self.outbox.pop_front() {
            let tick = message == HostToGuest::Tick;
            if self.guest_gone || link.send(message).is_err() {
                self.guest_gone = true;
                self.outbox.clear();
            } else if tick {
                self.tick_delivered();
            }
        }
    }

    fn report(&mut self, severity: Severity, message: std::fmt::Arguments<'_>) {
        self.errors.report(severity, &message.to_string());
    }

    /// Adds the frame just rendered to the statistics, and writes their summary when the
    /// interval is over.
    fn frame_measured(&mut self) {
        let timing = self.display.take_frame_timing();
        let Some(measuring) = &mut self.measuring else {
            return;
        };
        measuring.stats.add_frame(timing);
        if measuring
            .interval
            .is_some_and(|interval| measuring.stats.frames() >= interval)
        {
            self.report_frame_stats();
        }
    }

    /// Writes the summary of the frame time statistics and starts them anew.
    fn report_frame_stats(&mut self) {
        let Some(measuring) = &mut self.measuring else {
            return;
        };
        measuring.reported = true;
        for line in std::mem::take(&mut measuring.stats).lines() {
            self.report(Severity::Always, format_args!("{line}"));
        }
    }
}

impl<D: Display, C: Clock> Drop for Session<D, C> {
    fn drop(&mut self) {
        // The summary of a run that reports once, or of the frames since the last report.
        if self
            .measuring
            .as_ref()
            .is_some_and(|measuring| !measuring.reported || measuring.stats.frames() > 0)
        {
            self.report_frame_stats();
        }
    }
}

/// The mix of step `step` of a transition of `duration` frames, `step` below `duration`: the
/// cross-fade weight `255 * step / duration` and the mask threshold
/// `256 * step / duration - 1`, floored at 0, as measured on the original. Step 0 is
/// [`Mix::HELD`].
fn step_mix(step: u32, duration: u32) -> Mix {
    let step = u64::from(step);
    let duration = u64::from(duration);
    // Both are below 255, since the step is below the duration.
    Mix::Step {
        weight: (255 * step / duration) as u8,
        threshold: (256 * step / duration).saturating_sub(1) as u8,
    }
}

#[cfg(test)]
mod tests;
