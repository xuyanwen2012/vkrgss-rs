//! The scenarios of `specs/host-session` that need no window or GPU, and the tick handshake of
//! `specs/guest-host-protocol`, on a fake clock with the recording display.

use std::time::Duration;

use vkxp_proto::link::{GuestEnd, HostEnd, link};
use vkxp_proto::{
    AudioChannel, AudioCommand, BitmapDelta, BitmapId, BlendMode, Button, Color, FramePacket,
    FrameRate, GuestToHost, HostToGuest, InputSnapshot, Op, Quad, Rect, Sampling, Size, Tone,
};
use winit::keyboard::KeyCode;

use super::*;
use crate::ManualClock;
use crate::testing::{
    DisplayEvent, ErrorBuffer, PacingEvent, RecordingDisplay, RecordingSink, Shown,
};

/// A step of a transition whose weight and threshold are both `value`.
fn step(value: u8) -> Mix {
    Mix::Step {
        weight: value,
        threshold: value,
    }
}

const SIZE: Size = Size::new(32, 24);

fn ms(milliseconds: u64) -> Duration {
    Duration::from_millis(milliseconds)
}

fn packet(frame_index: u64, rate: u8) -> FramePacket {
    FramePacket {
        frame_index,
        logical_size: SIZE,
        frame_rate: FrameRate::new(rate).unwrap(),
        tables: vec![],
        bitmaps: Vec::new(),
        ops: vec![Op::Clear {
            clip: Rect::from_size(SIZE),
            color: Color::new(frame_index as u8, 0, 0, 255),
        }],
    }
}

fn frame(frame_index: u64) -> GuestToHost {
    GuestToHost::Frame(packet(frame_index, 40))
}

fn frame_at_rate(frame_index: u64, rate: u8) -> GuestToHost {
    GuestToHost::Frame(packet(frame_index, rate))
}

/// A frame whose packet every compositor rejects.
fn invalid_frame(frame_index: u64) -> GuestToHost {
    let mut packet = packet(frame_index, 40);
    packet.ops.push(Op::Quad(Quad {
        clip: Rect::from_size(SIZE),
        bitmap: BitmapId::new(900, 0),
        src: Rect::new(0, 0, 1, 1),
        sampling: Sampling::place_at(0, 0),
        tone: Tone::NEUTRAL,
        color: Color::new(0, 0, 0, 0),
        opacity: 255,
        blend: BlendMode::Normal,
    }));
    GuestToHost::Frame(packet)
}

fn transition(duration: u32) -> GuestToHost {
    GuestToHost::Transition {
        duration,
        mask: None,
        vague: 0,
    }
}

fn held(buttons: &[Button]) -> HostToGuest {
    HostToGuest::Input(
        buttons
            .iter()
            .fold(InputSnapshot::EMPTY, |snapshot, button| {
                snapshot.with(*button)
            }),
    )
}

/// A session on a fake clock with the recording display and a readable error stream.
struct Rig {
    session: Session<RecordingDisplay, ManualClock>,
    clock: ManualClock,
    errors: ErrorBuffer,
    /// The last snapshot [`Rig::taken`] saw, as the session sent it.
    last: InputSnapshot,
}

impl Rig {
    fn new() -> Rig {
        Rig::with(false)
    }

    fn with(fail_on_rejected_frame: bool) -> Rig {
        Rig::with_options(SessionOptions {
            fail_on_rejected_frame,
            ..SessionOptions::default()
        })
    }

    /// A session set up like a windowed one: modal messages wait for the player.
    fn windowed() -> Rig {
        Rig::with_options(SessionOptions {
            wait_for_modal: true,
            ..SessionOptions::default()
        })
    }

    fn with_options(options: SessionOptions) -> Rig {
        let clock = ManualClock::new();
        let errors = ErrorBuffer::new();
        let options = SessionOptions {
            errors: Box::new(errors.clone()),
            ..options
        };
        Rig {
            session: Session::new(RecordingDisplay::new(), clock.clone(), options),
            clock,
            errors,
            last: InputSnapshot::EMPTY,
        }
    }

    /// What the session has to send, as far as the buttons go: a snapshot is reduced to its
    /// buttons, and left out when only its raw keys or its pointer differ from the snapshot
    /// before it. A snapshot that repeats the one before it stays, so a session that sends
    /// one twice is found out.
    fn taken(&mut self) -> Vec<HostToGuest> {
        let mut messages = self.session.take_messages();
        messages.retain_mut(|message| {
            let HostToGuest::Input(snapshot) = message else {
                return true;
            };
            let only_raw = *snapshot != self.last && snapshot.buttons == self.last.buttons;
            self.last = *snapshot;
            *snapshot = InputSnapshot {
                buttons: snapshot.buttons,
                ..InputSnapshot::EMPTY
            };
            !only_raw
        });
        messages
    }

    fn send(&mut self, message: GuestToHost) -> Vec<HostToGuest> {
        self.session.handle_message(message);
        self.taken()
    }

    /// Moves the clock to the pending tick and returns the time it was sent at.
    fn wait_for_tick(&mut self) -> Duration {
        let due = self.session.next_wake().expect("a tick is pending");
        self.clock.advance_to(due);
        self.session.poll();
        assert_eq!(self.taken(), [HostToGuest::Tick]);
        self.clock.now()
    }

    /// Sends a frame and waits for its tick, like a guest that answers at once.
    fn frame(&mut self, message: GuestToHost) -> Duration {
        let before = self.send(message);
        if before == [HostToGuest::Tick] {
            return self.clock.now();
        }
        assert_eq!(before, []);
        self.wait_for_tick()
    }

    fn key(&mut self, key: KeyCode, pressed: bool) -> Vec<HostToGuest> {
        self.session.key(key, pressed);
        self.taken()
    }

    fn shown(&self) -> Shown {
        self.session.display().shown()
    }
}

// Frame pacing.

#[test]
fn steady_frames_at_rate_40_are_ticked_25_milliseconds_apart() {
    let mut rig = Rig::new();
    let ticks: Vec<Duration> = (0..6).map(|index| rig.frame(frame(index))).collect();
    assert_eq!(ticks, [ms(0), ms(25), ms(50), ms(75), ms(100), ms(125)]);
}

#[test]
fn a_tick_is_not_sent_before_it_is_due() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    assert_eq!(rig.send(frame(1)), []);
    assert_eq!(rig.session.next_wake(), Some(ms(25)));
    rig.clock.advance(ms(24));
    rig.session.poll();
    assert_eq!(rig.taken(), []);
    rig.clock.advance(ms(1));
    rig.session.poll();
    assert_eq!(rig.taken(), [HostToGuest::Tick]);
    assert_eq!(rig.session.next_wake(), None);
}

#[test]
fn guest_time_within_the_period_does_not_delay_the_tick() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.clock.advance(ms(10));
    assert_eq!(rig.frame(frame(1)), ms(25));
    rig.clock.advance(ms(24));
    assert_eq!(rig.frame(frame(2)), ms(50));
}

#[test]
fn late_frame_is_ticked_at_once_and_lateness_is_not_made_up() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.clock.advance(ms(100));
    assert_eq!(rig.send(frame(1)), [HostToGuest::Tick], "sent at once");
    assert_eq!(rig.send(frame(2)), []);
    assert_eq!(rig.session.next_wake(), Some(ms(125)));
    assert_eq!(rig.wait_for_tick(), ms(125));
    assert_eq!(rig.frame(frame(3)), ms(150));
}

#[test]
fn tick_sent_late_because_the_host_woke_late_is_not_made_up() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    assert_eq!(rig.send(frame(1)), []);
    assert_eq!(rig.session.next_wake(), Some(ms(25)));
    // The host comes back 75 milliseconds after the tick was due.
    rig.clock.advance_to(ms(100));
    rig.session.poll();
    assert_eq!(rig.taken(), [HostToGuest::Tick]);

    assert_eq!(
        rig.send(frame(2)),
        [],
        "a full period after the tick at 100"
    );
    assert_eq!(rig.session.next_wake(), Some(ms(125)));
    assert_eq!(rig.wait_for_tick(), ms(125));
    assert_eq!(rig.frame(frame(3)), ms(150));
}

#[test]
fn tick_sent_slightly_late_times_the_next_one_from_when_it_was_sent() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(frame(1));
    rig.clock.advance_to(ms(28));
    rig.session.poll();
    assert_eq!(rig.taken(), [HostToGuest::Tick]);
    assert_eq!(rig.frame(frame(2)), ms(53));
}

#[test]
fn rendering_that_outlasts_the_period_delays_the_tick_and_is_not_made_up() {
    let clock = ManualClock::new();
    let display = RecordingDisplay::with_render_time(clock.clone(), ms(40));
    let mut session = Session::new(display, clock.clone(), SessionOptions::default());
    let mut ticked_at = Vec::new();
    for index in 0..4 {
        session.handle_message(frame(index));
        if let Some(due) = session.next_wake() {
            clock.advance_to(due);
            session.poll();
        }
        assert_eq!(
            session.take_messages(),
            [HostToGuest::Tick],
            "frame {index}"
        );
        ticked_at.push(clock.now());
    }
    // Each frame arrives with the previous tick and takes 40 milliseconds to render, so its
    // tick goes out as soon as it is rendered: 40 apart, never two in a row to catch up.
    assert_eq!(ticked_at, [ms(40), ms(80), ms(120), ms(160)]);
}

#[test]
fn rendering_within_the_period_does_not_delay_the_tick() {
    let clock = ManualClock::new();
    let display = RecordingDisplay::with_render_time(clock.clone(), ms(10));
    let mut session = Session::new(display, clock.clone(), SessionOptions::default());
    let mut ticked_at = Vec::new();
    for index in 0..4 {
        session.handle_message(frame(index));
        if let Some(due) = session.next_wake() {
            clock.advance_to(due);
            session.poll();
        }
        assert_eq!(
            session.take_messages(),
            [HostToGuest::Tick],
            "frame {index}"
        );
        ticked_at.push(clock.now());
    }
    assert_eq!(ticked_at, [ms(10), ms(35), ms(60), ms(85)]);
}

#[test]
fn frame_rate_of_the_packet_sets_the_period_of_its_tick() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.frame(frame(1));
    assert_eq!(rig.frame(frame_at_rate(2, 20)), ms(75), "25 + 50");
    assert_eq!(rig.frame(frame_at_rate(3, 20)), ms(125));
    assert_eq!(rig.frame(frame(4)), ms(150), "back to 25");
    assert_eq!(rig.frame(frame_at_rate(5, 100)), ms(160));
}

// Fast-forward and frame reset.

#[test]
fn fast_forward_ticks_each_frame_without_waiting() {
    let mut rig = Rig::new();
    rig.session.set_fast_forward(true);
    for index in 0..5 {
        assert_eq!(rig.send(frame(index)), [HostToGuest::Tick]);
    }
    assert_eq!(rig.clock.now(), ms(0), "no time had to pass");
    assert_eq!(rig.session.display().rendered(), [0, 1, 2, 3, 4]);

    rig.session.set_fast_forward(false);
    assert_eq!(rig.send(frame(5)), [], "paced again");
    assert_eq!(rig.wait_for_tick(), ms(25));
}

#[test]
fn fast_forward_key_engages_while_held_and_hurries_a_pending_tick() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    assert_eq!(rig.send(frame(1)), []);
    rig.clock.advance(ms(5));
    assert_eq!(rig.key(KeyCode::Tab, true), [], "the key holds no button");
    assert!(rig.session.fast_forward());
    rig.session.poll();
    assert_eq!(rig.taken(), [HostToGuest::Tick]);
    assert_eq!(rig.send(frame(2)), [HostToGuest::Tick]);

    rig.key(KeyCode::Tab, false);
    assert!(!rig.session.fast_forward());
    assert_eq!(rig.send(frame(3)), []);
}

#[test]
fn frame_reset_makes_the_next_tick_due_one_period_after_a_late_frame() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.clock.advance(ms(300));
    assert_eq!(rig.send(GuestToHost::FrameReset), []);
    rig.clock.advance(ms(200));
    assert_eq!(rig.send(frame(1)), [], "not at once");
    assert_eq!(rig.session.next_wake(), Some(ms(525)));
    assert_eq!(rig.wait_for_tick(), ms(525));
    // The reset applies to one frame only.
    assert_eq!(rig.frame(frame(2)), ms(550));
    rig.clock.advance(ms(100));
    assert_eq!(rig.send(frame(3)), [HostToGuest::Tick]);
}

#[test]
fn frame_reset_also_delays_a_frame_that_is_not_late() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.clock.advance(ms(10));
    rig.send(GuestToHost::FrameReset);
    assert_eq!(rig.frame(frame(1)), ms(35));
}

// The tick handshake, through a link.

/// A session and both ends of its link, all on one thread.
fn linked() -> (Rig, GuestEnd, HostEnd) {
    let (guest, host) = link(|| {});
    (Rig::new(), guest, host)
}

#[test]
fn every_frame_gets_exactly_one_tick() {
    let (mut rig, guest, host) = linked();
    let mut ticks = 0;
    for index in 0..20 {
        guest.send(frame(index)).unwrap();
        rig.session.pump(&host);
        while let Some(due) = rig.session.next_wake() {
            rig.clock.advance_to(due);
            rig.session.pump(&host);
        }
        assert_eq!(
            guest.try_recv(),
            Ok(Some(HostToGuest::Tick)),
            "frame {index}"
        );
        assert_eq!(guest.try_recv(), Ok(None), "frame {index}");
        ticks += 1;
    }
    assert_eq!(ticks, 20);
    assert_eq!(rig.session.frames(), 20);
    assert_eq!(rig.clock.now(), ms(19 * 25));
}

#[test]
fn no_tick_is_sent_without_a_frame() {
    let (mut rig, guest, host) = linked();
    guest
        .send(GuestToHost::SetTitle {
            title: "quiet".to_owned(),
        })
        .unwrap();
    guest.send(GuestToHost::FrameReset).unwrap();
    for _ in 0..100 {
        rig.clock.advance(ms(1000));
        rig.session.pump(&host);
        assert_eq!(rig.session.next_wake(), None);
    }
    assert_eq!(guest.try_recv(), Ok(None));

    guest.send(frame(0)).unwrap();
    rig.session.pump(&host);
    while let Some(due) = rig.session.next_wake() {
        rig.clock.advance_to(due);
        rig.session.pump(&host);
    }
    rig.clock.advance(ms(60_000));
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));
    assert_eq!(guest.try_recv(), Ok(None), "one frame, one tick");
}

#[test]
fn messages_behind_a_frame_wait_for_its_tick() {
    let (mut rig, guest, host) = linked();
    guest.send(frame(0)).unwrap();
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));

    // A guest that breaks the handshake: a second and third frame without waiting.
    guest.send(frame(1)).unwrap();
    guest.send(frame(2)).unwrap();
    rig.session.pump(&host);
    assert_eq!(
        rig.session.display().rendered(),
        [0, 1],
        "frame 2 is not read yet"
    );
    assert_eq!(guest.try_recv(), Ok(None));
    rig.clock.advance(ms(25));
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));
    assert_eq!(rig.session.display().rendered(), [0, 1, 2]);
    assert_eq!(guest.try_recv(), Ok(None));
    rig.clock.advance(ms(25));
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));
}

#[test]
fn a_second_frame_handled_directly_still_gets_one_tick_each() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    assert_eq!(rig.send(frame(1)), []);
    assert_eq!(
        rig.send(frame(2)),
        [HostToGuest::Tick],
        "the tick of frame 1"
    );
    // That tick went out at 0, ahead of its time, and the next one is timed from there.
    assert_eq!(rig.wait_for_tick(), ms(25));
}

// Input delivery.

#[test]
fn press_and_release_send_one_snapshot_each() {
    let mut rig = Rig::new();
    assert_eq!(rig.key(KeyCode::ArrowDown, true), [held(&[Button::Down])]);
    assert_eq!(rig.key(KeyCode::ArrowDown, true), [], "key repeat");
    assert_eq!(rig.key(KeyCode::ArrowDown, false), [held(&[])]);
    assert_eq!(rig.key(KeyCode::ArrowDown, false), []);
}

#[test]
fn snapshots_are_complete() {
    let mut rig = Rig::new();
    rig.key(KeyCode::ArrowDown, true);
    assert_eq!(
        rig.key(KeyCode::KeyC, true),
        [held(&[Button::Down, Button::C])]
    );
    assert_eq!(rig.key(KeyCode::ArrowDown, false), [held(&[Button::C])]);
}

/// Scenarios "Keyboard and pad together" and "Removal".
#[test]
fn the_keyboard_and_the_pads_hold_buttons_together() {
    use crate::{GamepadMap, PadButton, PadState};

    let map = GamepadMap::default();
    let pad = PadState {
        held: vec![PadButton::South],
        stick: (0.0, 0.0),
    };
    let mut rig = Rig::new();
    assert_eq!(rig.key(KeyCode::ArrowLeft, true), [held(&[Button::Left])]);
    rig.session
        .pad_buttons(map.held(std::slice::from_ref(&pad)));
    assert_eq!(rig.taken(), [held(&[Button::Left, Button::C])]);
    rig.session.pad_buttons(map.held(&[pad]));
    assert_eq!(rig.taken(), [], "nothing changed");
    assert_eq!(rig.key(KeyCode::ArrowLeft, false), [held(&[Button::C])]);

    // The pad is removed: it is no longer among the states.
    rig.session.pad_buttons(map.held(&[]));
    assert_eq!(rig.taken(), [held(&[])]);
}

#[test]
fn releasing_one_of_two_keys_for_a_button_sends_nothing() {
    let mut rig = Rig::new();
    assert_eq!(rig.key(KeyCode::Space, true), [held(&[Button::C])]);
    assert_eq!(rig.key(KeyCode::Enter, true), [], "C is already held");
    assert_eq!(rig.key(KeyCode::Space, false), [], "C is still held");
    assert_eq!(rig.key(KeyCode::Enter, false), [held(&[])]);
}

#[test]
fn losing_focus_releases_every_button() {
    let mut rig = Rig::new();
    rig.key(KeyCode::ArrowLeft, true);
    rig.key(KeyCode::KeyZ, true);
    rig.key(KeyCode::Tab, true);
    rig.taken();

    rig.session.focus_lost();
    assert_eq!(rig.taken(), [held(&[])]);
    assert!(!rig.session.fast_forward());
    rig.session.focus_lost();
    assert_eq!(rig.taken(), [], "nothing changed");
    // The key was released while the window was not looking.
    assert_eq!(rig.key(KeyCode::ArrowLeft, false), []);
    assert_eq!(rig.key(KeyCode::ArrowLeft, true), [held(&[Button::Left])]);
}

#[test]
fn unmapped_keys_send_nothing() {
    let mut rig = Rig::new();
    for key in [KeyCode::KeyP, KeyCode::F1, KeyCode::Backspace] {
        assert_eq!(rig.key(key, true), [], "{key:?}");
        assert_eq!(rig.key(key, false), [], "{key:?}");
    }
}

#[test]
fn scripted_buttons_bypass_the_key_map_and_are_sent_on_change_only() {
    let mut rig = Rig::new();
    let right = InputSnapshot::EMPTY.with(Button::Right);
    rig.session.set_buttons(right);
    rig.session.set_buttons(right);
    rig.session.set_buttons(InputSnapshot::EMPTY);
    assert_eq!(
        rig.session.take_messages(),
        [HostToGuest::Input(right), held(&[])]
    );

    // The same goes for the raw keys and the pointer of a snapshot.
    let raw = right.with_key(0x41).with_pointer(Some((100, 50)));
    rig.session.set_buttons(raw);
    rig.session.set_buttons(raw);
    assert_eq!(rig.session.take_messages(), [HostToGuest::Input(raw)]);
}

// The default key map.

#[test]
fn shift_holds_both_a_and_shift() {
    let mut rig = Rig::new();
    assert_eq!(
        rig.key(KeyCode::ShiftLeft, true),
        [held(&[Button::A, Button::Shift])]
    );
}

#[test]
fn the_letter_x_is_b_and_the_letter_a_is_x() {
    let mut rig = Rig::new();
    assert_eq!(rig.key(KeyCode::KeyX, true), [held(&[Button::B])]);
    rig.key(KeyCode::KeyX, false);
    rig.taken();
    assert_eq!(rig.key(KeyCode::KeyA, true), [held(&[Button::X])]);
}

#[test]
fn function_keys_five_to_nine_are_themselves() {
    let keys = [
        (KeyCode::F5, Button::F5),
        (KeyCode::F6, Button::F6),
        (KeyCode::F7, Button::F7),
        (KeyCode::F8, Button::F8),
        (KeyCode::F9, Button::F9),
    ];
    for (key, button) in keys {
        let mut rig = Rig::new();
        assert_eq!(rig.key(key, true), [held(&[button])]);
    }
}

#[test]
fn alt_enter_toggles_fullscreen_without_pressing_c() {
    let mut rig = Rig::new();
    assert_eq!(rig.key(KeyCode::AltLeft, true), [held(&[Button::Alt])]);
    assert_eq!(
        rig.session.key(KeyCode::Enter, true),
        KeyAction::ToggleFullscreen
    );
    assert_eq!(rig.session.key(KeyCode::Enter, true), KeyAction::None);
    assert_eq!(rig.taken(), []);
    // Still swallowed after Alt goes up first.
    assert_eq!(rig.key(KeyCode::AltLeft, false), [held(&[])]);
    assert_eq!(rig.key(KeyCode::Enter, false), []);
    assert_eq!(rig.session.key(KeyCode::Enter, true), KeyAction::None);
    assert_eq!(rig.taken(), [held(&[Button::C])]);
}

#[test]
fn f11_toggles_fullscreen_once_per_press_and_holds_no_button() {
    let mut rig = Rig::new();
    for _ in 0..2 {
        assert_eq!(
            rig.session.key(KeyCode::F11, true),
            KeyAction::ToggleFullscreen
        );
        assert_eq!(rig.session.key(KeyCode::F11, true), KeyAction::None, "held");
        assert_eq!(rig.key(KeyCode::F11, false), []);
    }
    assert_eq!(rig.taken(), []);
}

// Reset, quit and shutdown.

#[test]
fn reset_key_sends_one_reset_per_press() {
    let mut rig = Rig::new();
    assert_eq!(rig.key(KeyCode::F12, true), [HostToGuest::Reset]);
    for _ in 0..5 {
        assert_eq!(rig.key(KeyCode::F12, true), [], "held");
    }
    assert_eq!(rig.key(KeyCode::F12, false), []);
    assert_eq!(rig.key(KeyCode::F12, true), [HostToGuest::Reset]);
}

#[test]
fn closing_the_window_asks_the_guest_to_quit_and_the_session_runs_on() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.session.close_requested();
    assert_eq!(rig.taken(), [HostToGuest::Quit]);
    assert_eq!(rig.session.end(), None);
    // The guest may still draw while it winds down.
    assert_eq!(rig.frame(frame(1)), ms(25));
    assert_eq!(rig.session.end(), None);

    rig.send(GuestToHost::Shutdown);
    assert_eq!(rig.session.end(), Some(SessionEnd::Shutdown));
}

#[test]
fn guest_shutdown_ends_the_session_with_success() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    assert_eq!(rig.send(GuestToHost::Shutdown), []);
    let end = rig.session.end().expect("the session has ended");
    assert!(end.is_success());

    // Nothing moves an ended session.
    assert_eq!(rig.send(frame(1)), []);
    assert_eq!(rig.key(KeyCode::F12, true), []);
    rig.session.close_requested();
    rig.session.poll();
    assert_eq!(rig.taken(), []);
    assert_eq!(rig.session.next_wake(), None);
    assert_eq!(rig.session.display().rendered(), [0]);
}

#[test]
fn dropped_guest_end_ends_the_session_after_its_last_messages() {
    let (mut rig, guest, host) = linked();
    guest.send(frame(0)).unwrap();
    guest
        .send(GuestToHost::SetTitle {
            title: "last words".to_owned(),
        })
        .unwrap();
    drop(guest);

    rig.session.pump(&host);
    assert_eq!(rig.session.display().rendered(), [0]);
    assert_eq!(rig.session.display().title(), "last words");
    let end = rig.session.end().expect("the session has ended");
    assert_eq!(end, SessionEnd::Disconnected);
    assert!(!end.is_success());
}

#[test]
fn dropped_guest_end_is_noticed_while_sending() {
    let (mut rig, guest, host) = linked();
    drop(guest);
    rig.session.key(KeyCode::F12, true);
    rig.session.pump(&host);
    assert_eq!(rig.session.end(), Some(SessionEnd::Disconnected));
}

#[test]
fn shutdown_already_in_the_link_wins_over_a_failed_send() {
    let (mut rig, guest, host) = linked();
    guest.send(frame(0)).unwrap();
    guest.send(frame(1)).unwrap();
    guest.send(GuestToHost::Shutdown).unwrap();
    drop(guest);
    rig.session.pump(&host);
    assert_eq!(rig.session.display().rendered(), [0, 1]);
    assert_eq!(rig.session.end(), Some(SessionEnd::Shutdown));
    assert_eq!(rig.clock.now(), ms(0), "no tick was waited for");
}

#[test]
fn shutdown_through_the_link_ends_with_success_even_if_the_guest_then_goes() {
    let (mut rig, guest, host) = linked();
    guest.send(GuestToHost::Shutdown).unwrap();
    drop(guest);
    rig.session.pump(&host);
    rig.session.pump(&host);
    assert_eq!(rig.session.end(), Some(SessionEnd::Shutdown));
}

// Freeze and transition.

#[test]
fn frozen_display_keeps_its_image_while_frames_are_rendered_and_ticked() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    assert_eq!(rig.shown(), Shown::Frame(0));
    assert_eq!(rig.send(GuestToHost::Freeze), []);

    let mut ticks = 0;
    for index in 1..=3 {
        rig.frame(frame(index));
        ticks += 1;
        assert_eq!(rig.shown(), Shown::Frame(0), "after frame {index}");
    }
    assert_eq!(ticks, 3);
    assert_eq!(rig.session.display().rendered(), [0, 1, 2, 3]);
}

#[test]
fn transition_cross_fades_one_step_behind_its_frames_then_shows_frames_normally() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(transition(4));
    assert_eq!(rig.shown(), Shown::Frame(0));

    let mix = |newest, weight| Shown::mix(Shown::Frame(0), newest, weight);
    rig.frame(frame(1));
    assert_eq!(rig.shown(), Shown::Frame(0), "the held frame alone");
    rig.frame(frame(2));
    assert_eq!(rig.shown(), mix(2, 63));
    rig.frame(frame(3));
    assert_eq!(rig.shown(), mix(3, 127), "the even mix");
    rig.frame(frame(4));
    assert_eq!(rig.shown(), mix(4, 191), "one quarter of the held frame");
    rig.frame(frame(5));
    assert_eq!(rig.shown(), Shown::Frame(5), "the newest frame alone");
    rig.frame(frame(6));
    assert_eq!(rig.shown(), Shown::Frame(6));

    let mixes: Vec<Mix> = rig
        .session
        .display()
        .events
        .iter()
        .filter_map(|event| match event {
            DisplayEvent::Mix(mix) => Some(*mix),
            _ => None,
        })
        .collect();
    // The freeze, then the steps 0 to 3 of 4, then the frame that is shown alone.
    assert_eq!(
        mixes,
        [
            Mix::HELD,
            Mix::HELD,
            step(63),
            step(127),
            step(191),
            Mix::Newest
        ]
    );
}

/// The thresholds of the steps from 1 on.
fn thresholds(duration: u32) -> Vec<u8> {
    (1..duration)
        .map(|at| match step_mix(at, duration) {
            Mix::Step { threshold, .. } => threshold,
            Mix::Newest => panic!("step {at} of {duration} is no step"),
        })
        .collect()
}

/// Scenario "Intermediate thresholds", and the table of the design.
#[test]
fn intermediate_steps_have_the_thresholds_measured_on_the_original() {
    assert_eq!(thresholds(2), [127]);
    assert_eq!(thresholds(3), [84, 169]);
    assert_eq!(thresholds(5), [50, 101, 152, 203]);
    assert_eq!(thresholds(8), [31, 63, 95, 127, 159, 191, 223]);
    assert_eq!(
        thresholds(20),
        [
            11, 24, 37, 50, 63, 75, 88, 101, 114, 127, 139, 152, 165, 178, 191, 203, 216, 229, 242
        ]
    );
    assert_eq!(thresholds(50)[..10], [4, 9, 14, 19, 24, 29, 34, 39, 45, 50]);
}

/// Scenario "First step of a masked transition".
#[test]
fn first_step_of_a_masked_transition_is_the_held_frame_alone() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(masked_transition(5, 40));
    rig.frame(frame_with_mask(1));
    // The display has the mask bitmap and is not told to use it yet.
    assert_eq!(rig.shown(), Shown::Frame(0));
    rig.frame(frame(2));
    assert_eq!(rig.shown(), masked(2, 50, 40));
    assert_eq!(rig.errors.text(), "");
}

/// Scenario "Last step" of the transition step values.
#[test]
fn last_step_has_the_weight_and_the_threshold_measured_on_the_original() {
    let last = |duration| step_mix(duration - 1, duration);
    assert!(matches!(last(8), Mix::Step { weight: 223, .. }));
    assert!(matches!(last(3), Mix::Step { threshold: 169, .. }));
    assert!(matches!(last(5), Mix::Step { threshold: 203, .. }));
    // The first step shows the held frame, and no duration overflows.
    assert_eq!(step_mix(0, 8), Mix::HELD);
    assert_eq!(step_mix(u32::MAX - 1, u32::MAX), step(254));
    // Above 256 the formula is below 0 at the first steps: the threshold is floored.
    assert!(matches!(step_mix(1, 300), Mix::Step { threshold: 0, .. }));

    // The eighth frame of a transition of duration 8 is shown with them.
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(transition(8));
    for index in 1..=8 {
        rig.frame(frame(index));
    }
    assert_eq!(rig.shown(), Shown::mix(Shown::Frame(0), 8, 223));
}

#[test]
fn transition_of_duration_one_shows_the_held_frame_then_the_newest() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(transition(1));
    rig.frame(frame(1));
    assert_eq!(rig.shown(), Shown::Frame(0));
    rig.frame(frame(2));
    assert_eq!(rig.shown(), Shown::Frame(2));
}

#[test]
fn frames_between_freeze_and_transition_do_not_advance_the_fade() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.frame(frame(1));
    rig.frame(frame(2));
    rig.send(transition(2));
    rig.frame(frame(3));
    assert_eq!(rig.shown(), Shown::Frame(0));
    rig.frame(frame(4));
    assert_eq!(rig.shown(), Shown::mix(Shown::Frame(0), 4, 127));
    rig.frame(frame(5));
    assert_eq!(rig.shown(), Shown::Frame(5));
}

#[test]
fn transition_of_duration_zero_keeps_the_held_frame_until_the_next_frame() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(transition(0));
    assert_eq!(rig.shown(), Shown::Frame(0));
    // Whatever makes the window draw again shows the held frame too.
    rig.session.display_mut().redraw().unwrap();
    assert_eq!(rig.shown(), Shown::Frame(0), "after a redraw");
    rig.frame(frame(1));
    assert_eq!(rig.shown(), Shown::Frame(1));
    rig.frame(frame(2));
    assert_eq!(rig.shown(), Shown::Frame(2));

    // The freeze has ended: the next one holds the frame then on screen.
    rig.send(GuestToHost::Freeze);
    rig.frame(frame(3));
    assert_eq!(rig.shown(), Shown::Frame(2));
}

#[test]
fn transition_without_freeze_keeps_the_picture_on_screen_for_its_frames() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(transition(4));
    assert_eq!(rig.shown(), Shown::Frame(0));
    for index in 1..=4 {
        rig.frame(frame(index));
        assert_eq!(rig.shown(), Shown::Frame(0), "after frame {index}");
    }
    rig.frame(frame(5));
    assert_eq!(rig.shown(), Shown::Frame(5));
    rig.frame(frame(6));
    assert_eq!(rig.shown(), Shown::Frame(6));
}

#[test]
fn masked_transition_without_freeze_keeps_the_picture_on_screen_too() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(masked_transition(2, 40));
    rig.frame(frame_with_mask(1));
    assert_eq!(rig.shown(), Shown::Frame(0));
    rig.frame(frame(2));
    assert_eq!(rig.shown(), Shown::Frame(0));
    rig.frame(frame(3));
    assert_eq!(rig.shown(), Shown::Frame(3));
    assert_eq!(rig.errors.text(), "");
}

#[test]
fn transition_of_duration_zero_without_freeze_changes_nothing() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(transition(0));
    for index in 1..=2 {
        rig.frame(frame(index));
        assert_eq!(rig.shown(), Shown::Frame(index));
    }
    assert!(
        !rig.session
            .display()
            .events
            .iter()
            .any(|event| matches!(event, DisplayEvent::Mix(_) | DisplayEvent::Hold))
    );
}

const MASK: BitmapId = BitmapId::new(3, 0);

fn masked_transition(duration: u32, vague: u16) -> GuestToHost {
    GuestToHost::Transition {
        duration,
        mask: Some(MASK),
        vague,
    }
}

/// A frame that brings the mask bitmap, black on the left half and white on the right half.
fn frame_with_mask(frame_index: u64) -> GuestToHost {
    let mut packet = packet(frame_index, 40);
    let rgba = (0..SIZE.width * SIZE.height)
        .flat_map(|index| {
            let level = if index % SIZE.width < SIZE.width / 2 {
                0
            } else {
                255
            };
            [level, level, level, 255]
        })
        .collect();
    packet.bitmaps = vec![
        BitmapDelta::Create {
            id: MASK,
            width: SIZE.width,
            height: SIZE.height,
        },
        BitmapDelta::Update {
            id: MASK,
            rect: Rect::from_size(SIZE),
            rgba,
        },
    ];
    GuestToHost::Frame(packet)
}

fn masked(newest: u64, threshold: u8, vagueness: u32) -> Shown {
    Shown::Masked {
        held: Box::new(Shown::Frame(0)),
        newest,
        threshold,
        mask: MASK,
        vagueness,
    }
}

#[test]
fn masked_transition_is_the_masked_mix_with_the_mask_and_the_vagueness() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(masked_transition(4, 40));
    // The mask arrives with the first frame after the transition message.
    rig.frame(frame_with_mask(1));
    assert_eq!(rig.shown(), Shown::Frame(0), "the held frame alone");
    rig.frame(frame(2));
    assert_eq!(rig.shown(), masked(2, 63, 40));
    rig.frame(frame(3));
    // At this threshold the display shows the newest frame where the mask is black and the
    // held frame where it is white; the presenter's tests look at the pixels.
    assert_eq!(rig.shown(), masked(3, 127, 40));
    rig.frame(frame(4));
    assert_eq!(rig.shown(), masked(4, 191, 40));
    rig.frame(frame(5));
    assert_eq!(rig.shown(), Shown::Frame(5));
    assert_eq!(rig.errors.text(), "");

    // The mask is dropped with the transition: the next one is a cross-fade.
    rig.send(GuestToHost::Freeze);
    rig.send(transition(2));
    rig.frame(frame(6));
    rig.frame(frame(7));
    assert_eq!(rig.shown(), Shown::mix(Shown::Frame(5), 7, 127));
    let masks: Vec<&DisplayEvent> = rig
        .session
        .display()
        .events
        .iter()
        .filter(|event| matches!(event, DisplayEvent::Mask(_)))
        .collect();
    assert_eq!(
        masks,
        [
            &DisplayEvent::Mask(Some((MASK, 40))),
            &DisplayEvent::Mask(None)
        ]
    );
}

/// The guest disposes of the picture when its transition returns, so the frame that ends the
/// transition carries the destroy delta.
#[test]
fn mask_destroyed_with_the_frame_that_ends_the_transition_is_not_reported() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(masked_transition(2, 40));
    rig.frame(frame_with_mask(1));
    rig.frame(frame(2));
    let mut last = packet(3, 40);
    last.bitmaps = vec![BitmapDelta::Destroy { id: MASK }];
    rig.frame(GuestToHost::Frame(last));
    assert_eq!(rig.shown(), Shown::Frame(3));
    assert_eq!(rig.errors.text(), "");
}

#[test]
fn vagueness_of_zero_is_passed_on_as_it_is() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(masked_transition(2, 0));
    rig.frame(frame_with_mask(1));
    rig.frame(frame(2));
    assert_eq!(rig.shown(), masked(2, 127, 0));
}

#[test]
fn transition_with_an_unknown_mask_is_a_cross_fade_and_is_reported_once() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(masked_transition(4, 40));
    rig.frame(frame(1));
    assert_eq!(rig.shown(), Shown::Frame(0));
    rig.frame(frame(2));
    assert_eq!(rig.shown(), Shown::mix(Shown::Frame(0), 2, 63));
    rig.frame(frame(3));
    assert_eq!(rig.shown(), Shown::mix(Shown::Frame(0), 3, 127));
    rig.frame(frame(4));
    rig.frame(frame(5));
    assert_eq!(rig.shown(), Shown::Frame(5));
    assert_eq!(rig.errors.text().matches("mask").count(), 1);
    assert_eq!(rig.errors.severities("mask"), [Severity::Warning]);
}

#[test]
fn freeze_during_a_masked_transition_holds_the_masked_image_and_drops_the_mask() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(masked_transition(4, 40));
    rig.frame(frame_with_mask(1));
    rig.frame(frame(2));
    let on_screen = rig.shown();
    assert_eq!(on_screen, masked(2, 63, 40));
    rig.send(GuestToHost::Freeze);
    rig.frame(frame(3));
    assert_eq!(rig.shown(), on_screen);
    rig.send(transition(2));
    rig.frame(frame(4));
    assert_eq!(rig.shown(), on_screen);
    rig.frame(frame(5));
    assert_eq!(rig.shown(), Shown::mix(on_screen, 5, 127));
}

#[test]
fn a_second_freeze_keeps_the_frame_already_held() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.frame(frame(1));
    rig.send(GuestToHost::Freeze);
    rig.frame(frame(2));
    assert_eq!(rig.shown(), Shown::Frame(0));
}

/// A rig one step into a cross-fade of four from frame 0 to frame 2.
fn fading() -> (Rig, Shown) {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(transition(4));
    rig.frame(frame(1));
    rig.frame(frame(2));
    let shown = Shown::mix(Shown::Frame(0), 2, 63);
    assert_eq!(rig.shown(), shown);
    (rig, shown)
}

#[test]
fn freeze_during_a_transition_holds_the_mixed_image_on_screen() {
    let (mut rig, on_screen) = fading();
    rig.send(GuestToHost::Freeze);
    for index in 3..=5 {
        rig.frame(frame(index));
        assert_eq!(rig.shown(), on_screen, "after frame {index}");
    }

    // The next transition fades from that image, not from either frame behind it.
    rig.send(transition(2));
    rig.frame(frame(6));
    assert_eq!(rig.shown(), on_screen);
    rig.frame(frame(7));
    assert_eq!(rig.shown(), Shown::mix(on_screen, 7, 127));
    rig.frame(frame(8));
    assert_eq!(rig.shown(), Shown::Frame(8));
}

#[test]
fn frame_rejected_during_a_transition_keeps_the_image_and_its_weight() {
    let (mut rig, on_screen) = fading();
    assert_eq!(rig.send(invalid_frame(3)), []);
    rig.wait_for_tick();
    assert_eq!(rig.shown(), on_screen);

    // Whatever makes the window draw again, a resize or an expose, shows the same image.
    rig.session.display_mut().redraw().unwrap();
    assert_eq!(rig.shown(), on_screen, "after a redraw");
    assert_eq!(rig.session.end(), None);

    // The fade goes on with the next frame that is shown, at the step the rejected one missed.
    rig.frame(frame(4));
    assert_eq!(rig.shown(), Shown::mix(Shown::Frame(0), 4, 127));
    rig.frame(frame(5));
    rig.frame(frame(6));
    assert_eq!(rig.shown(), Shown::Frame(6));
    rig.session.display_mut().redraw().unwrap();
    assert_eq!(rig.shown(), Shown::Frame(6));
}

#[test]
fn frame_rejected_at_the_end_of_a_transition_does_not_end_it() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(transition(1));
    rig.frame(frame(1));
    rig.send(invalid_frame(2));
    rig.wait_for_tick();
    rig.session.display_mut().redraw().unwrap();
    assert_eq!(rig.shown(), Shown::Frame(0), "still the held frame");
    rig.frame(frame(3));
    assert_eq!(rig.shown(), Shown::Frame(3));
}

#[test]
fn frame_rejected_while_frozen_keeps_the_held_frame() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(invalid_frame(1));
    rig.wait_for_tick();
    rig.session.display_mut().redraw().unwrap();
    assert_eq!(rig.shown(), Shown::Frame(0));
}

#[test]
fn freeze_before_any_frame_shows_nothing_until_the_transition() {
    let mut rig = Rig::new();
    rig.send(GuestToHost::Freeze);
    rig.frame(frame(0));
    assert_eq!(rig.shown(), Shown::Nothing);
    rig.send(transition(1));
    rig.frame(frame(1));
    assert_eq!(rig.shown(), Shown::Nothing);
    rig.frame(frame(2));
    assert_eq!(rig.shown(), Shown::Frame(2));
}

// Title and modal.

#[test]
fn title_message_sets_the_window_title() {
    let mut rig = Rig::new();
    rig.send(GuestToHost::SetTitle {
        title: "Project1".to_owned(),
    });
    assert_eq!(rig.session.display().title(), "Project1");
}

#[test]
fn modal_text_goes_to_the_error_stream_and_is_dismissed_at_once() {
    let mut rig = Rig::new();
    let answer = rig.send(GuestToHost::Modal {
        text: "Script 'Main' line 7: NameError occurred.".to_owned(),
    });
    assert_eq!(answer, [HostToGuest::ModalDismissed]);
    assert!(
        rig.errors
            .text()
            .contains("Script 'Main' line 7: NameError occurred."),
        "{}",
        rig.errors.text()
    );
}

#[test]
fn windowed_modal_waits_for_the_confirm_key_and_hides_that_press() {
    let mut rig = Rig::windowed();
    assert_eq!(rig.send(modal("Save failed.")), []);
    assert!(rig.errors.text().contains("Save failed."));
    assert_eq!(rig.errors.severities("Save failed."), [Severity::Always]);
    // Frames are shown and answered while the message is pending.
    for index in 0..3 {
        rig.frame(frame(index));
        assert_eq!(rig.shown(), Shown::Frame(index));
    }
    // A key that is neither confirm nor cancel reaches the game and dismisses nothing.
    assert_eq!(rig.key(KeyCode::ArrowDown, true), [held(&[Button::Down])]);
    assert_eq!(rig.key(KeyCode::ArrowDown, false), [held(&[])]);

    assert_eq!(rig.key(KeyCode::Enter, true), [HostToGuest::ModalDismissed]);
    // The press stays hidden for as long as the key is down.
    assert_eq!(rig.key(KeyCode::ArrowDown, true), [held(&[Button::Down])]);
    assert_eq!(rig.key(KeyCode::Enter, false), []);
    assert_eq!(
        rig.key(KeyCode::Enter, true),
        [held(&[Button::Down, Button::C])]
    );
}

#[test]
fn windowed_modal_is_dismissed_by_the_cancel_key_and_by_a_click() {
    let mut rig = Rig::windowed();
    rig.send(modal("first"));
    assert_eq!(
        rig.key(KeyCode::Escape, true),
        [HostToGuest::ModalDismissed]
    );
    assert_eq!(rig.key(KeyCode::Escape, false), []);

    rig.send(modal("second"));
    rig.session.click();
    assert_eq!(rig.taken(), [HostToGuest::ModalDismissed]);
    // Nothing is pending any more.
    rig.session.click();
    assert_eq!(rig.taken(), []);
}

#[test]
fn windowed_modal_is_dismissed_by_a_host_key_remapped_to_confirm_or_cancel() {
    for (key, button) in [
        (KeyCode::Tab, Button::C),
        (KeyCode::F2, Button::C),
        (KeyCode::Tab, Button::B),
    ] {
        let mut keys = KeyMap::default();
        keys.bind(key, button);
        let mut rig = Rig::with_options(SessionOptions {
            wait_for_modal: true,
            keys,
            ..SessionOptions::default()
        });
        rig.send(GuestToHost::SetTitle {
            title: "Game".to_owned(),
        });
        rig.send(modal("Save failed."));
        // The press dismisses once, and is neither a button nor the host's shortcut.
        assert_eq!(rig.key(key, true), [HostToGuest::ModalDismissed], "{key:?}");
        assert!(!rig.session.fast_forward());
        assert_eq!(rig.key(key, true), [], "a repeat of the held key");
        assert_eq!(rig.key(KeyCode::ArrowDown, true), [held(&[Button::Down])]);
        assert_eq!(rig.key(KeyCode::ArrowDown, false), [held(&[])]);
        assert_eq!(rig.key(key, false), []);
        // Pressed again it is the game's button, and the shortcut it was.
        assert_eq!(rig.key(key, true), [held(&[button])], "{key:?}");
        assert_eq!(rig.session.fast_forward(), key == KeyCode::Tab);
        assert_eq!(rig.key(key, false), [held(&[])]);
    }
}

#[test]
fn reset_key_mapped_to_confirm_still_asks_for_a_reset_while_a_modal_waits() {
    let mut keys = KeyMap::default();
    keys.bind(KeyCode::F12, Button::C);
    let mut rig = Rig::with_options(SessionOptions {
        wait_for_modal: true,
        keys,
        ..SessionOptions::default()
    });
    rig.send(modal("Save failed."));
    let answer = rig.key(KeyCode::F12, true);
    assert_eq!(answer[0], HostToGuest::Reset);
    assert!(!answer.contains(&HostToGuest::ModalDismissed));
}

#[test]
fn windowed_modal_is_not_dismissed_by_a_key_that_was_already_down() {
    let mut rig = Rig::windowed();
    assert_eq!(rig.key(KeyCode::Enter, true), [held(&[Button::C])]);
    rig.send(modal("Save failed."));
    // The platform repeats the held key.
    assert_eq!(rig.key(KeyCode::Enter, true), []);
    assert_eq!(rig.key(KeyCode::Enter, false), [held(&[])]);
    assert_eq!(rig.key(KeyCode::Enter, true), [HostToGuest::ModalDismissed]);
}

#[test]
fn frame_rate_key_puts_the_measured_rate_in_the_title_and_takes_it_out_again() {
    let mut rig = Rig::new();
    rig.send(GuestToHost::SetTitle {
        title: "Game".to_owned(),
    });
    rig.frame(frame(0));
    assert_eq!(rig.key(KeyCode::F2, true), [], "the key holds no button");
    rig.key(KeyCode::F2, false);
    for index in 1..=40 {
        rig.frame(frame(index));
    }
    assert_eq!(rig.session.display().title(), "Game - 40.0 FPS");

    rig.key(KeyCode::F2, true);
    assert_eq!(rig.session.display().title(), "Game");
    for index in 41..=90 {
        rig.frame(frame(index));
    }
    assert_eq!(rig.session.display().title(), "Game");
}

/// Scenario "Order" of the audio commands requirement.
#[test]
fn audio_commands_reach_the_sink_in_the_order_sent() {
    let sink = RecordingSink::new();
    let clock = ManualClock::new();
    let errors = ErrorBuffer::new();
    let options = SessionOptions {
        errors: Box::new(errors.clone()),
        audio: Box::new(sink.clone()),
        ..SessionOptions::default()
    };
    let mut session = Session::new(RecordingDisplay::new(), clock.clone(), options);
    let (guest, host) = link(|| {});

    let play = AudioCommand::Play {
        channel: AudioChannel::Bgm,
        data: vec![1, 2, 3],
        volume: 80,
        pitch: 100,
    };
    let fade = AudioCommand::Fade {
        channel: AudioChannel::Bgm,
        duration_ms: 500,
    };
    let stop = AudioCommand::Stop {
        channel: AudioChannel::Se,
    };
    guest.send(frame(0)).unwrap();
    guest.send(GuestToHost::Audio(play.clone())).unwrap();
    guest.send(GuestToHost::Audio(fade.clone())).unwrap();
    guest.send(frame(1)).unwrap();
    guest.send(GuestToHost::Audio(stop.clone())).unwrap();
    session.pump(&host);
    // None of them waits for a tick, not even the one behind the frame whose tick is pending.
    assert_eq!(session.next_wake(), Some(ms(25)));
    assert_eq!(sink.commands(), [play, fade, stop]);
    assert_eq!(errors.text(), "");
}

/// The sink of a session that was given none, as in a headless run, reports nothing.
#[test]
fn audio_commands_without_a_sink_are_dropped_silently() {
    let mut rig = Rig::new();
    for _ in 0..3 {
        let answer = rig.send(GuestToHost::Audio(AudioCommand::Stop {
            channel: AudioChannel::Bgm,
        }));
        assert_eq!(answer, []);
    }
    assert_eq!(rig.errors.text(), "");
}

// Rejected frames.

#[test]
fn rejected_frame_is_reported_keeps_the_image_and_still_gets_its_tick() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    assert_eq!(rig.send(invalid_frame(1)), []);
    assert_eq!(rig.wait_for_tick(), ms(25));

    assert_eq!(rig.shown(), Shown::Frame(0), "the display is unchanged");
    assert_eq!(rig.session.end(), None);
    let errors = rig.errors.text();
    assert!(errors.contains("frame 1 was rejected"), "{errors}");
    assert!(errors.contains("op 1 draws an unknown bitmap"), "{errors}");
    assert_eq!(rig.errors.severities("was rejected"), [Severity::Error]);

    assert_eq!(rig.frame(frame(2)), ms(50));
    assert_eq!(rig.shown(), Shown::Frame(2));
}

#[test]
fn rejected_frame_ends_a_session_that_does_not_continue_with_failure() {
    let mut rig = Rig::with(true);
    rig.frame(frame(0));
    assert_eq!(rig.send(invalid_frame(1)), []);
    // The rejected frame is answered first, at its time, like any other.
    assert_eq!(rig.session.end(), None);
    assert!(rig.session.is_failing());
    assert_eq!(rig.wait_for_tick(), ms(25));

    let end = rig.session.end().expect("the session has ended");
    assert_eq!(end, SessionEnd::Failed);
    assert!(!end.is_success());
    let errors = rig.errors.text();
    assert!(errors.contains("frame 1 was rejected"), "{errors}");
    assert!(errors.contains("unknown bitmap"), "{errors}");
}

#[test]
fn guest_of_a_failing_session_gets_the_one_tick_of_the_rejected_frame_and_no_more() {
    let (guest, host) = link(|| {});
    let mut rig = Rig::with(true);
    guest.send(frame(0)).unwrap();
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));

    guest.send(invalid_frame(1)).unwrap();
    // More from a guest that does not wait; none of it is answered.
    guest.send(frame(2)).unwrap();
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(None), "the tick is not due yet");
    assert_eq!(rig.session.end(), None);
    while let Some(due) = rig.session.next_wake() {
        rig.clock.advance_to(due);
        rig.session.pump(&host);
    }
    assert_eq!(rig.session.end(), Some(SessionEnd::Failed));
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));
    assert_eq!(guest.try_recv(), Ok(None), "exactly one");

    rig.clock.advance(ms(1000));
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(None));
    assert_eq!(
        rig.session.display().rendered(),
        [0],
        "frame 2 was never read"
    );
}

#[test]
fn rejected_frame_in_a_failing_session_is_ticked_at_once_when_it_is_late() {
    let mut rig = Rig::with(true);
    rig.frame(frame(0));
    rig.clock.advance(ms(100));
    assert_eq!(rig.send(invalid_frame(1)), [HostToGuest::Tick]);
    assert_eq!(rig.session.end(), Some(SessionEnd::Failed));
}

// Messages behind a frame whose tick is pending, through a link.

/// A linked rig at 10 frames per second with frame 0 ticked and frame 1 waiting for its tick,
/// which is due at 100 milliseconds. The clock is still at 0.
fn waiting_for_a_tick() -> (Rig, GuestEnd, HostEnd) {
    let (mut rig, guest, host) = linked();
    guest.send(frame_at_rate(0, 10)).unwrap();
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));
    guest.send(frame_at_rate(1, 10)).unwrap();
    rig.session.pump(&host);
    assert_eq!(rig.session.next_wake(), Some(ms(100)));
    assert_eq!(guest.try_recv(), Ok(None));
    (rig, guest, host)
}

fn modal(text: &str) -> GuestToHost {
    GuestToHost::Modal {
        text: text.to_owned(),
    }
}

#[test]
fn modal_behind_a_frame_is_answered_at_once_while_the_tick_still_waits() {
    let (mut rig, guest, host) = waiting_for_a_tick();
    guest.send(modal("Save failed.")).unwrap();
    rig.session.pump(&host);

    // No time has passed: the text is out and the dismissal is there, the tick is not.
    assert_eq!(rig.clock.now(), ms(0));
    assert!(
        rig.errors.text().contains("Save failed."),
        "{}",
        rig.errors.text()
    );
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::ModalDismissed)));
    assert_eq!(guest.try_recv(), Ok(None), "the tick is not due yet");
    assert_eq!(
        rig.session.next_wake(),
        Some(ms(100)),
        "and is due when it was"
    );

    rig.clock.advance(ms(99));
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(None));
    rig.clock.advance(ms(1));
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));
    assert_eq!(guest.try_recv(), Ok(None), "one tick for the frame");
}

#[test]
fn control_messages_behind_a_frame_take_effect_while_its_tick_waits() {
    let (mut rig, guest, host) = waiting_for_a_tick();
    guest
        .send(GuestToHost::SetTitle {
            title: "Loading".to_owned(),
        })
        .unwrap();
    guest.send(GuestToHost::Freeze).unwrap();
    guest.send(modal("first")).unwrap();
    guest.send(modal("second")).unwrap();
    rig.session.pump(&host);

    assert_eq!(rig.session.display().title(), "Loading");
    assert!(rig.session.display().events.contains(&DisplayEvent::Hold));
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::ModalDismissed)));
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::ModalDismissed)));
    assert_eq!(guest.try_recv(), Ok(None));
    assert_eq!(rig.session.next_wake(), Some(ms(100)));
}

#[test]
fn next_frame_and_what_follows_it_wait_for_the_tick_in_the_order_sent() {
    let (mut rig, guest, host) = waiting_for_a_tick();
    guest.send(modal("before the frame")).unwrap();
    // A guest that does not wait for its tick: the next frame, and a modal behind it.
    guest.send(frame_at_rate(2, 10)).unwrap();
    guest.send(modal("after the frame")).unwrap();
    guest.send(frame_at_rate(3, 10)).unwrap();
    rig.session.pump(&host);
    rig.session.pump(&host);

    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::ModalDismissed)));
    assert_eq!(guest.try_recv(), Ok(None));
    assert!(rig.errors.text().contains("before the frame"));
    assert!(
        !rig.errors.text().contains("after the frame"),
        "it is behind frame 2"
    );
    assert_eq!(rig.session.display().rendered(), [0, 1]);
    assert_eq!(rig.session.next_wake(), Some(ms(100)));

    // The tick of frame 1 lets frame 2 in, and the modal behind it; frame 3 waits in turn.
    rig.clock.advance_to(ms(100));
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::ModalDismissed)));
    assert_eq!(guest.try_recv(), Ok(None));
    assert!(rig.errors.text().contains("after the frame"));
    assert_eq!(rig.session.display().rendered(), [0, 1, 2]);
    assert_eq!(rig.session.next_wake(), Some(ms(200)));

    rig.clock.advance_to(ms(200));
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));
    assert_eq!(rig.session.display().rendered(), [0, 1, 2, 3]);
    rig.clock.advance_to(ms(300));
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));
    assert_eq!(guest.try_recv(), Ok(None), "four frames, four ticks");
}

#[test]
fn shutdown_behind_a_frame_ends_the_session_without_waiting_for_the_tick() {
    let (mut rig, guest, host) = waiting_for_a_tick();
    guest.send(GuestToHost::Shutdown).unwrap();
    rig.session.pump(&host);
    assert_eq!(rig.session.end(), Some(SessionEnd::Shutdown));
    assert_eq!(rig.clock.now(), ms(0));
}

#[test]
fn guest_that_goes_with_a_frame_waiting_ends_the_session_after_that_frame() {
    let (mut rig, guest, host) = waiting_for_a_tick();
    guest.send(frame_at_rate(2, 10)).unwrap();
    drop(guest);
    rig.session.pump(&host);
    while let Some(due) = rig.session.next_wake() {
        rig.clock.advance_to(due);
        rig.session.pump(&host);
    }
    rig.session.pump(&host);
    assert_eq!(rig.session.display().rendered(), [0, 1, 2]);
    assert_eq!(rig.session.end(), Some(SessionEnd::Disconnected));
}

// Input recording and replay.

impl Rig {
    /// A session that takes each frame's snapshot from `input`.
    fn with_input(input: impl crate::InputSource + 'static) -> Rig {
        let clock = ManualClock::new();
        let errors = ErrorBuffer::new();
        let options = SessionOptions {
            errors: Box::new(errors.clone()),
            input: Some(Box::new(input)),
            ..SessionOptions::default()
        };
        Rig {
            session: Session::new(RecordingDisplay::new(), clock.clone(), options),
            clock,
            errors,
            last: InputSnapshot::EMPTY,
        }
    }

    /// Sends a frame, waits for its tick and returns what the guest was sent before the tick.
    fn frame_input(&mut self, frame_index: u64) -> Vec<HostToGuest> {
        let mut sent = self.send(frame(frame_index));
        if sent.last() != Some(&HostToGuest::Tick) {
            let due = self.session.next_wake().expect("a tick is pending");
            self.clock.advance_to(due);
            self.session.poll();
            sent.extend(self.taken());
        }
        assert_eq!(sent.pop(), Some(HostToGuest::Tick));
        sent
    }
}

/// Records a session in which RIGHT is held during frames 1 and 2 and C during frame 4, and
/// returns what the guest was sent before each frame's tick.
fn record_a_session(path: &std::path::Path) -> Vec<Vec<HostToGuest>> {
    let mut rig = Rig::with_input(crate::Recorder::create(path, 7).unwrap());
    let mut delivered = vec![rig.frame_input(0)];
    // With a source, live input waits for the next frame.
    assert_eq!(rig.key(KeyCode::ArrowRight, true), []);
    delivered.push(rig.frame_input(1));
    delivered.push(rig.frame_input(2));
    assert_eq!(rig.key(KeyCode::ArrowRight, false), []);
    delivered.push(rig.frame_input(3));
    // Pressed and released between two frames: no frame sees it.
    assert_eq!(rig.key(KeyCode::KeyZ, true), []);
    assert_eq!(rig.key(KeyCode::KeyZ, false), []);
    assert_eq!(rig.key(KeyCode::KeyC, true), []);
    delivered.push(rig.frame_input(4));
    delivered
}

#[test]
fn a_recorder_writes_the_snapshot_delivered_for_every_frame() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.txt");
    let delivered = record_a_session(&path);
    assert_eq!(
        delivered,
        [
            vec![],
            vec![held(&[Button::Right])],
            vec![],
            vec![held(&[])],
            vec![held(&[Button::C])]
        ]
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "vkxp-input 2 seed=7\n1x\n2x RIGHT keys=27\n1x\n1x C keys=43\n"
    );
}

#[test]
fn a_replayed_recording_delivers_the_recorded_snapshots_frame_by_frame() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.txt");
    let recorded = record_a_session(&path);

    let recording = crate::Recording::read(&path).unwrap();
    assert_eq!(recording.seed, 7);
    let mut rig = Rig::with_input(crate::Replayer::new(recording));
    // Live input does not reach the guest during the replay.
    assert_eq!(rig.key(KeyCode::ArrowDown, true), []);
    let replayed: Vec<Vec<HostToGuest>> = (0..5).map(|index| rig.frame_input(index)).collect();
    assert_eq!(replayed, recorded);
    assert_eq!(rig.errors.text(), "");

    // After the last recorded frame live input applies again.
    assert_eq!(rig.frame_input(5), [held(&[Button::Down])]);
    assert_eq!(rig.key(KeyCode::ArrowDown, false), []);
    assert_eq!(rig.frame_input(6), [held(&[])]);
}

#[test]
fn a_replayed_recording_delivers_its_raw_keys_and_pointer() {
    let text = "vkxp-input 2 seed=0\n2x C keys=1,d pointer=100,50\n1x\n";
    let mut rig = Rig::with_input(crate::Replayer::new(crate::Recording::parse(text).unwrap()));
    let snapshot = InputSnapshot::EMPTY
        .with(Button::C)
        .with_key(0x01)
        .with_key(0x0D)
        .with_pointer(Some((100, 50)));
    rig.session.handle_message(frame(0));
    assert_eq!(
        rig.session.take_messages(),
        [HostToGuest::Input(snapshot), HostToGuest::Tick]
    );
}

#[test]
fn a_recorder_writes_the_raw_keys_and_the_pointer_of_the_window() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.txt");
    let mut rig = Rig::with_input(crate::Recorder::create(&path, 1).unwrap());
    rig.frame_input(0);
    rig.session.key(KeyCode::KeyJ, true);
    rig.session
        .mouse_button(winit::event::MouseButton::Left, true);
    rig.session.pointer_moved(Some((100, 50)));
    rig.frame_input(1);
    drop(rig);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "vkxp-input 2 seed=1\n1x\n1x keys=1,4a pointer=100,50\n"
    );
}

#[test]
fn after_a_replay_without_live_input_no_button_is_held() {
    let recording = crate::Recording::parse("vkxp-input 1 seed=0\n2x C\n").unwrap();
    let mut rig = Rig::with_input(crate::Replayer::new(recording));
    assert_eq!(rig.frame_input(0), [held(&[Button::C])]);
    assert_eq!(rig.frame_input(1), []);
    assert_eq!(rig.frame_input(2), [held(&[])]);
    assert_eq!(rig.frame_input(3), []);
}

// What the display is told about pacing.

/// What the display has been told about pacing since the last call.
fn pacing(rig: &mut Rig) -> Vec<PacingEvent> {
    std::mem::take(&mut rig.session.display_mut().pacing)
}

const RATE_40: PacingEvent = PacingEvent::Rate(40);
const RESTART: PacingEvent = PacingEvent::Restart;

#[test]
fn display_is_told_the_frame_rate_of_every_frame_before_it_renders_it() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.frame(frame_at_rate(1, 20));
    rig.frame(frame(2));
    // A rejected frame is one too.
    rig.frame(invalid_frame(3));
    assert_eq!(
        pacing(&mut rig),
        [RATE_40, PacingEvent::Rate(20), RATE_40, RATE_40]
    );
}

#[test]
fn display_restarts_its_pacing_with_the_frame_after_a_frame_reset() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::FrameReset);
    assert_eq!(pacing(&mut rig), [RATE_40], "not before the frame");
    rig.frame(frame(1));
    rig.frame(frame(2));
    assert_eq!(pacing(&mut rig), [RESTART, RATE_40, RATE_40]);
}

#[test]
fn display_restarts_its_pacing_with_every_frame_of_a_fast_forward_and_the_one_after() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.key(KeyCode::Tab, true);
    rig.frame(frame(1));
    rig.frame(frame(2));
    rig.key(KeyCode::Tab, false);
    rig.frame(frame(3));
    rig.frame(frame(4));
    assert_eq!(
        pacing(&mut rig),
        [
            RATE_40, RESTART, RATE_40, RESTART, RATE_40, RESTART, RATE_40, RATE_40
        ]
    );

    // Engaged and released between two frames, or released by a loss of focus.
    rig.session.set_fast_forward(true);
    rig.session.set_fast_forward(false);
    rig.frame(frame(5));
    rig.key(KeyCode::Tab, true);
    rig.session.focus_lost();
    rig.frame(frame(6));
    rig.session.focus_lost();
    rig.frame(frame(7));
    assert_eq!(
        pacing(&mut rig),
        [RESTART, RATE_40, RESTART, RATE_40, RATE_40]
    );
}

#[test]
fn display_restarts_its_pacing_with_the_frame_after_a_modal_wait() {
    let mut rig = Rig::windowed();
    rig.frame(frame(0));
    rig.send(GuestToHost::Modal {
        text: "wait".to_owned(),
    });
    rig.clock.advance(ms(3000));
    assert_eq!(rig.key(KeyCode::Enter, true), [HostToGuest::ModalDismissed]);
    rig.key(KeyCode::Enter, false);
    rig.frame(frame(1));
    rig.frame(frame(2));
    assert_eq!(pacing(&mut rig), [RATE_40, RESTART, RATE_40, RATE_40]);

    // A message that does not wait breaks no rhythm.
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Modal {
        text: "no wait".to_owned(),
    });
    rig.frame(frame(1));
    assert_eq!(pacing(&mut rig), [RATE_40, RATE_40]);
}

/// Sends `frames` frames, each answered with its tick a period after the one before. The
/// tick of each frame is handed over before the next frame comes, as the handshake has it,
/// and nothing else is.
fn run_frames(rig: &mut Rig, frames: u64) {
    for index in 0..frames {
        rig.session.handle_message(frame(index));
        rig.clock.advance(ms(25));
        rig.session.poll();
        assert_eq!(rig.session.take_messages(), [HostToGuest::Tick]);
    }
}

/// The mean of the `guest` row of the one summary in `text`, in milliseconds, with the
/// number of frames the row counts.
fn guest_row(text: &str) -> (u64, f64) {
    let row = text
        .lines()
        .find_map(|line| line.strip_prefix("vkxp:   guest: "))
        .unwrap_or_else(|| panic!("no guest row in: {text}"));
    let (count, rest) = row.split_once(" frame(s), mean ").expect("a measured row");
    let mean = rest.split(' ').next().unwrap().parse().unwrap();
    (count.parse().unwrap(), mean)
}

/// Two frames with `before` between queueing the first frame's tick and handing it over,
/// and `after` between handing it over and the second frame. Returns the `guest` row.
fn guest_wait(before: Duration, after: Duration) -> (u64, f64) {
    let mut rig = Rig::with_options(SessionOptions {
        frame_stats: true,
        ..SessionOptions::default()
    });
    rig.session.handle_message(frame(0));
    std::thread::sleep(before);
    assert_eq!(rig.session.take_messages(), [HostToGuest::Tick]);
    std::thread::sleep(after);
    rig.session.handle_message(frame(1));
    drop(rig.session);
    guest_row(&rig.errors.text())
}

#[test]
fn host_work_before_a_tick_is_handed_over_is_not_the_guests_time() {
    let (count, mean) = guest_wait(ms(200), ms(0));
    assert_eq!(count, 1);
    assert!(mean < 100.0, "the host's 200 ms were counted: {mean} ms");
}

#[test]
fn time_from_handing_a_tick_over_to_the_next_frame_is_the_guests() {
    let (count, mean) = guest_wait(ms(0), ms(60));
    assert_eq!(count, 1);
    assert!(
        mean >= 60.0,
        "the guest's 60 ms were not counted: {mean} ms"
    );
}

#[test]
fn guest_wait_is_measured_from_the_send_when_the_session_runs_against_its_link() {
    let (guest, host) = link(|| {});
    let mut rig = Rig::with_options(SessionOptions {
        frame_stats: true,
        ..SessionOptions::default()
    });
    guest.send(frame(0)).unwrap();
    rig.session
        .handle_message(host.try_recv().unwrap().unwrap());
    // The tick is queued; the host is busy before it sends it.
    std::thread::sleep(ms(200));
    rig.session.pump(&host);
    assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));
    std::thread::sleep(ms(60));
    guest.send(frame(1)).unwrap();
    rig.session.pump(&host);
    drop(rig.session);
    let (count, mean) = guest_row(&rig.errors.text());
    assert_eq!(count, 1);
    assert!((60.0..160.0).contains(&mean), "{mean} ms");
}

#[test]
fn frame_statistics_are_written_once_when_the_session_ends() {
    let mut rig = Rig::with_options(SessionOptions {
        frame_stats: true,
        ..SessionOptions::default()
    });
    run_frames(&mut rig, 5);
    assert_eq!(rig.errors.text(), "", "nothing before the end");
    drop(rig.session);
    let text = rig.errors.text();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "vkxp: frame times over 5 frame(s):");
    // The wait for the guest is from the answer to a frame to the next frame.
    assert!(
        lines[1].starts_with("vkxp:   guest: 4 frame(s), mean "),
        "{text}"
    );
    // The recording display measures nothing.
    assert_eq!(lines[2], "vkxp:   encode: 0 frame(s)");
    assert_eq!(lines.len(), 9, "{text}");
}

#[test]
fn frame_statistics_are_not_written_unless_asked_for() {
    let mut rig = Rig::new();
    run_frames(&mut rig, 5);
    drop(rig.session);
    assert_eq!(rig.errors.text(), "");
}

#[test]
fn frame_statistics_with_an_interval_are_written_after_each_and_start_anew() {
    let mut rig = Rig::with_options(SessionOptions {
        frame_stats: true,
        frame_stats_interval: Some(3),
        ..SessionOptions::default()
    });
    run_frames(&mut rig, 2);
    assert_eq!(rig.errors.text(), "");
    run_frames(&mut rig, 5);
    let headings = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|line| line.contains("frame times over"))
            .map(str::to_owned)
            .collect()
    };
    assert_eq!(
        headings(&rig.errors.text()),
        ["vkxp: frame times over 3 frame(s):"; 2]
    );
    // The frame since the last report is summed up at the end.
    drop(rig.session);
    let all = headings(&rig.errors.text());
    assert_eq!(all.len(), 3);
    assert_eq!(all[2], "vkxp: frame times over 1 frame(s):");
}

#[test]
fn frame_statistics_do_not_move_a_tick() {
    let due = |frame_stats| {
        let mut rig = Rig::with_options(SessionOptions {
            frame_stats,
            ..SessionOptions::default()
        });
        let mut due = Vec::new();
        for index in 0..4 {
            rig.session.handle_message(frame(index));
            due.push((rig.session.next_wake(), rig.session.take_messages()));
            rig.clock.advance(ms(30));
            rig.session.poll();
            due.push((rig.session.next_wake(), rig.session.take_messages()));
        }
        due
    };
    assert_eq!(due(true), due(false));
}
