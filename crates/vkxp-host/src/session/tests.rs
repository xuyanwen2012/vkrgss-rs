//! The scenarios of `specs/host-session` that need no window or GPU, and the tick handshake of
//! `specs/guest-host-protocol`, on a fake clock with the recording display.

use std::time::Duration;

use vkxp_proto::link::{GuestEnd, HostEnd, link};
use vkxp_proto::{
    AudioChannel, AudioCommand, BitmapId, Button, Color, FramePacket, FrameRate, GuestToHost,
    HostToGuest, InputSnapshot, Op, Rect, Size,
};
use winit::keyboard::KeyCode;

use super::*;
use crate::ManualClock;
use crate::testing::{DisplayEvent, ErrorBuffer, RecordingDisplay, Shown};

const SIZE: Size = Size::new(32, 24);

fn ms(milliseconds: u64) -> Duration {
    Duration::from_millis(milliseconds)
}

fn packet(frame_index: u64, rate: u8) -> FramePacket {
    FramePacket {
        frame_index,
        logical_size: SIZE,
        frame_rate: FrameRate::new(rate).unwrap(),
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
    packet.ops.push(Op::Plane {
        clip: Rect::from_size(SIZE),
    });
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
}

impl Rig {
    fn new() -> Rig {
        Rig::with(false)
    }

    fn with(fail_on_rejected_frame: bool) -> Rig {
        let clock = ManualClock::new();
        let errors = ErrorBuffer::new();
        let options = SessionOptions {
            fail_on_rejected_frame,
            errors: Box::new(errors.clone()),
            ..SessionOptions::default()
        };
        Rig {
            session: Session::new(RecordingDisplay::new(), clock.clone(), options),
            clock,
            errors,
        }
    }

    fn send(&mut self, message: GuestToHost) -> Vec<HostToGuest> {
        self.session.handle_message(message);
        self.session.take_messages()
    }

    /// Moves the clock to the pending tick and returns the time it was sent at.
    fn wait_for_tick(&mut self) -> Duration {
        let due = self.session.next_wake().expect("a tick is pending");
        self.clock.advance_to(due);
        self.session.poll();
        assert_eq!(self.session.take_messages(), [HostToGuest::Tick]);
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
        self.session.take_messages()
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
    assert_eq!(rig.session.take_messages(), []);
    rig.clock.advance(ms(1));
    rig.session.poll();
    assert_eq!(rig.session.take_messages(), [HostToGuest::Tick]);
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
    assert_eq!(rig.session.take_messages(), [HostToGuest::Tick]);

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
    assert_eq!(rig.session.take_messages(), [HostToGuest::Tick]);
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
    assert_eq!(rig.session.take_messages(), [HostToGuest::Tick]);
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
    rig.session.take_messages();

    rig.session.focus_lost();
    assert_eq!(rig.session.take_messages(), [held(&[])]);
    assert!(!rig.session.fast_forward());
    rig.session.focus_lost();
    assert_eq!(rig.session.take_messages(), [], "nothing changed");
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
    rig.session.take_messages();
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
    assert_eq!(rig.session.take_messages(), []);
    // Still swallowed after Alt goes up first.
    assert_eq!(rig.key(KeyCode::AltLeft, false), [held(&[])]);
    assert_eq!(rig.key(KeyCode::Enter, false), []);
    assert_eq!(rig.session.key(KeyCode::Enter, true), KeyAction::None);
    assert_eq!(rig.session.take_messages(), [held(&[Button::C])]);
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
    assert_eq!(rig.session.take_messages(), [HostToGuest::Quit]);
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
    assert_eq!(rig.session.take_messages(), []);
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
fn transition_cross_fades_in_equal_steps_then_shows_frames_normally() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(transition(4));
    assert_eq!(
        rig.shown(),
        Shown::Frame(0),
        "the fade starts with the next frame"
    );

    let mix = |newest, weight| Shown::mix(Shown::Frame(0), newest, weight);
    rig.frame(frame(1));
    assert_eq!(rig.shown(), mix(1, 0.25));
    rig.frame(frame(2));
    assert_eq!(rig.shown(), mix(2, 0.5), "the even mix");
    rig.frame(frame(3));
    assert_eq!(rig.shown(), mix(3, 0.75));
    rig.frame(frame(4));
    assert_eq!(rig.shown(), Shown::Frame(4), "the newest frame alone");
    rig.frame(frame(5));
    assert_eq!(rig.shown(), Shown::Frame(5));

    let mixes: Vec<f32> = rig
        .session
        .display()
        .events
        .iter()
        .filter_map(|event| match event {
            DisplayEvent::Mix(weight) => Some(*weight),
            _ => None,
        })
        .collect();
    assert_eq!(mixes, [0.0, 0.25, 0.5, 0.75, 1.0]);
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
    assert_eq!(rig.shown(), Shown::mix(Shown::Frame(0), 3, 0.5));
    rig.frame(frame(4));
    assert_eq!(rig.shown(), Shown::Frame(4));
}

#[test]
fn transition_of_duration_zero_shows_the_newest_frame_at_once() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.frame(frame(1));
    assert_eq!(rig.shown(), Shown::Frame(0));
    rig.send(transition(0));
    assert_eq!(rig.shown(), Shown::Frame(1), "without waiting for a frame");
    rig.frame(frame(2));
    assert_eq!(rig.shown(), Shown::Frame(2));
}

#[test]
fn transition_without_freeze_changes_nothing() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(transition(4));
    for index in 1..=5 {
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

#[test]
fn masked_transition_is_a_plain_cross_fade_of_the_same_duration() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(GuestToHost::Transition {
        duration: 2,
        mask: Some(BitmapId::new(3, 0)),
        vague: 40,
    });
    rig.frame(frame(1));
    assert_eq!(rig.shown(), Shown::mix(Shown::Frame(0), 1, 0.5));
    rig.frame(frame(2));
    assert_eq!(rig.shown(), Shown::Frame(2));
    assert_eq!(rig.errors.text().matches("mask").count(), 1);

    // The note is written once, not per transition.
    rig.send(GuestToHost::Freeze);
    rig.send(GuestToHost::Transition {
        duration: 1,
        mask: Some(BitmapId::new(3, 0)),
        vague: 40,
    });
    assert_eq!(rig.errors.text().matches("mask").count(), 1);
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

/// A rig one step into a cross-fade of four from frame 0 to frame 1.
fn fading() -> (Rig, Shown) {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(transition(4));
    rig.frame(frame(1));
    let shown = Shown::mix(Shown::Frame(0), 1, 0.25);
    assert_eq!(rig.shown(), shown);
    (rig, shown)
}

#[test]
fn freeze_during_a_transition_holds_the_mixed_image_on_screen() {
    let (mut rig, on_screen) = fading();
    rig.send(GuestToHost::Freeze);
    for index in 2..=4 {
        rig.frame(frame(index));
        assert_eq!(rig.shown(), on_screen, "after frame {index}");
    }

    // The next transition fades from that image, not from either frame behind it.
    rig.send(transition(2));
    rig.frame(frame(5));
    assert_eq!(rig.shown(), Shown::mix(on_screen, 5, 0.5));
    rig.frame(frame(6));
    assert_eq!(rig.shown(), Shown::Frame(6));
}

#[test]
fn frame_rejected_during_a_transition_keeps_the_image_and_its_weight() {
    let (mut rig, on_screen) = fading();
    assert_eq!(rig.send(invalid_frame(2)), []);
    rig.wait_for_tick();
    assert_eq!(rig.shown(), on_screen);

    // Whatever makes the window draw again, a resize or an expose, shows the same image.
    rig.session.display_mut().redraw().unwrap();
    assert_eq!(rig.shown(), on_screen, "after a redraw");
    assert_eq!(rig.session.end(), None);

    // The fade goes on with the next frame that is shown, at the step the rejected one missed.
    rig.frame(frame(3));
    assert_eq!(rig.shown(), Shown::mix(Shown::Frame(0), 3, 0.5));
    rig.frame(frame(4));
    rig.frame(frame(5));
    assert_eq!(rig.shown(), Shown::Frame(5));
    rig.session.display_mut().redraw().unwrap();
    assert_eq!(rig.shown(), Shown::Frame(5));
}

#[test]
fn frame_rejected_on_the_last_step_of_a_transition_does_not_end_it() {
    let mut rig = Rig::new();
    rig.frame(frame(0));
    rig.send(GuestToHost::Freeze);
    rig.send(transition(1));
    rig.send(invalid_frame(1));
    rig.wait_for_tick();
    rig.session.display_mut().redraw().unwrap();
    assert_eq!(rig.shown(), Shown::Frame(0), "still the held frame");
    rig.frame(frame(2));
    assert_eq!(rig.shown(), Shown::Frame(2));
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
    assert_eq!(rig.shown(), Shown::Frame(1));
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

#[test]
fn audio_commands_are_accepted_and_noted_once() {
    let mut rig = Rig::new();
    for _ in 0..3 {
        let answer = rig.send(GuestToHost::Audio(AudioCommand::Stop {
            channel: AudioChannel::Bgm,
        }));
        assert_eq!(answer, []);
    }
    assert_eq!(
        rig.errors.text().matches("audio").count(),
        2,
        "{}",
        rig.errors.text()
    );
    assert_eq!(rig.errors.text().lines().count(), 1);
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
    assert!(errors.contains("op 1 is of a reserved kind"), "{errors}");

    assert_eq!(rig.frame(frame(2)), ms(50));
    assert_eq!(rig.shown(), Shown::Frame(2));
}

#[test]
fn rejected_frame_ends_a_session_that_does_not_continue_with_failure() {
    let mut rig = Rig::with(true);
    rig.frame(frame(0));
    rig.send(invalid_frame(1));

    let end = rig.session.end().expect("the session has ended");
    assert_eq!(end, SessionEnd::Failed);
    assert!(!end.is_success());
    let errors = rig.errors.text();
    assert!(errors.contains("frame 1 was rejected"), "{errors}");
    assert!(errors.contains("reserved kind"), "{errors}");
}
