//! The windowed runner: a session in a `winit` window, shown by the Vulkan backend.
//!
//! The event loop sleeps until the session's next tick is due and is woken early by the
//! link's wake hook, through an event-loop proxy, whenever the guest has sent something.
//! [`Driver`] is the part between the window events and the session; it has no window in it,
//! so the tests drive it with synthetic events.

use std::io::Write;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use thiserror::Error;
use vkxp_proto::link::{GuestEnd, HostEnd, link};
use vkxp_proto::{BitmapId, FramePacket, FrameRate, Size};
use vkxp_vk::{
    DeviceChoice, Filter, FrameTiming, IntervalSummary, Mix, STATS_INTERVALS, VkCompositor, VkError,
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Fullscreen, Window, WindowId};

use crate::report::ErrorStream;
use crate::{
    AudioOptions, AudioSink, Clock, Display, GamepadMap, Gamepads, InputSource, KeyAction, KeyMap,
    NullSink, Reporter, Session, SessionEnd, SessionOptions, SystemClock, frame_pointer,
};

/// How a windowed session is set up.
pub struct WindowedOptions {
    /// The device that composites and presents. There is deliberately no default: on a
    /// machine with several GPUs the caller says which one drives the window.
    pub device: DeviceChoice,
    /// The window title until the guest sets one.
    pub title: String,
    /// The logical screen size the window is first sized for.
    pub logical_size: Size,
    /// The filter that scales the frame into the window.
    pub filter: Filter,
    /// Whether the window starts in borderless fullscreen.
    pub fullscreen: bool,
    /// The physical keys that hold each button.
    pub keys: KeyMap,
    /// The gamepad buttons that hold each button.
    pub pads: GamepadMap,
    /// The audio output, or `None` for a run without sound, which opens no audio device.
    pub audio: Option<AudioOptions>,
    /// A recorder or a replayer of the input, which the session takes when it starts.
    pub input: Option<Box<dyn InputSource>>,
    /// Whether a summary of the intervals between the frames shown is reported, every
    /// [`STATS_INTERVALS`] frames and when the session ends. It is a warning, so that it is
    /// shown at the default level.
    pub present_stats: bool,
    /// Whether the frame time statistics are collected and their summary written to
    /// `errors`, every [`STATS_INTERVALS`] frames and when the session ends.
    pub frame_stats: bool,
    /// Whether the guest can be saved and loaded with the state keys: it runs in a process
    /// of its own, behind something that keeps its states.
    pub states: bool,
    /// Where the session reports modal texts and errors, which it takes when it starts.
    pub errors: Box<dyn Reporter>,
    /// Where the audio side reports sounds that cannot be played, taken likewise.
    pub audio_errors: Box<dyn Write + Send>,
}

impl WindowedOptions {
    /// The defaults for a game of 640 by 480 pixels on `device`, without sound.
    pub fn new(device: DeviceChoice) -> WindowedOptions {
        WindowedOptions {
            device,
            title: "vkxp".to_owned(),
            logical_size: Size::new(640, 480),
            filter: Filter::Auto,
            fullscreen: false,
            keys: KeyMap::default(),
            pads: GamepadMap::default(),
            audio: None,
            input: None,
            present_stats: false,
            frame_stats: false,
            states: false,
            errors: Box::new(ErrorStream),
            audio_errors: Box::new(std::io::stderr()),
        }
    }
}

/// Why a windowed session could not run.
#[derive(Debug, Error)]
pub enum WindowedError {
    #[error("cannot run the event loop: {0}")]
    EventLoop(String),
    #[error("cannot create the window: {0}")]
    Window(String),
    #[error("cannot start the compositor for the window: {0}")]
    Compositor(#[source] VkError),
    #[error("cannot start the guest thread: {0}")]
    GuestThread(#[source] std::io::Error),
    #[error("the guest thread panicked")]
    GuestPanicked,
}

/// The size in pixels of the client area a window for a `logical` screen starts with when
/// the work area of its monitor is `monitor` pixels: the logical size at the largest
/// whole-number scale that fits, and at least once. An unknown monitor gives the logical size.
///
/// The windowing library does not report a work area, so the caller passes the monitor's
/// size; a window system may then shrink the window, which is a resize like any other.
pub fn initial_window_size(logical: Size, monitor: Option<Size>) -> Size {
    let scale = match monitor {
        Some(monitor) if logical.width > 0 && logical.height > 0 => {
            (monitor.width / logical.width).min(monitor.height / logical.height)
        }
        _ => 1,
    }
    .max(1);
    Size::new(
        logical.width.saturating_mul(scale),
        logical.height.saturating_mul(scale),
    )
}

/// What stands between the window's events and a session: it translates them, hands them to
/// the session and runs the session against its link after each one.
pub struct Driver<D: Display, C: Clock> {
    session: Session<D, C>,
    link: HostEnd,
}

impl<D: Display, C: Clock> Driver<D, C> {
    pub fn new(session: Session<D, C>, link: HostEnd) -> Driver<D, C> {
        Driver { session, link }
    }

    pub fn session(&self) -> &Session<D, C> {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut Session<D, C> {
        &mut self.session
    }

    /// How the session ended, or `None` while it runs.
    pub fn end(&self) -> Option<SessionEnd> {
        self.session.end()
    }

    /// A key went down or up. Keys the platform cannot identify are ignored.
    pub fn key(&mut self, key: PhysicalKey, state: ElementState) -> KeyAction {
        let action = match key {
            PhysicalKey::Code(code) => self.session.key(code, state.is_pressed()),
            PhysicalKey::Unidentified(_) => KeyAction::None,
        };
        self.session.pump(&self.link);
        action
    }

    /// What the keys the guest injected ask of the window, since the last call.
    pub fn take_actions(&mut self) -> Vec<KeyAction> {
        self.session.take_actions()
    }

    /// A mouse button was clicked.
    pub fn click(&mut self) {
        self.session.click();
        self.session.pump(&self.link);
    }

    /// A mouse button went down or up. A press also answers a modal message.
    pub fn mouse_button(&mut self, button: MouseButton, state: ElementState) {
        if state.is_pressed() {
            self.session.click();
        }
        self.session.mouse_button(button, state.is_pressed());
        self.session.pump(&self.link);
    }

    /// The pointer is at `position` in a window whose client area is `window` pixels, or has
    /// left the window.
    pub fn pointer_moved(&mut self, window: Size, position: Option<(f64, f64)>) {
        let pointer = self
            .session
            .frame_size()
            .zip(position)
            .and_then(|(frame, (x, y))| frame_pointer(frame, window, x, y));
        self.session.pointer_moved(pointer);
        self.session.pump(&self.link);
    }

    /// The window gained or lost keyboard focus.
    pub fn focus_changed(&mut self, focused: bool) {
        if !focused {
            self.session.focus_lost();
            self.session.pump(&self.link);
        }
    }

    /// The user asked to close the window.
    pub fn close_requested(&mut self) {
        self.session.close_requested();
        self.session.pump(&self.link);
    }

    /// Runs the session as far as it goes without waiting and returns when to call again, on
    /// the session's clock; `None` means only when something happens.
    pub fn wake(&mut self) -> Option<Duration> {
        self.session.pump(&self.link);
        self.session.next_wake()
    }
}

/// The Vulkan backend showing its frames in a window.
struct VulkanDisplay {
    // Before the window in declaration order: the surface goes before its window does. The
    // compositor also keeps the window alive itself.
    compositor: VkCompositor,
    window: Arc<Window>,
    /// Whether the intervals between the frames shown are reported.
    present_stats: bool,
    /// Frames shown since the last report.
    frames: usize,
}

/// The line that reports the intervals between the frames shown: their summary, if there
/// is one yet, and whether the times are those `VK_EXT_present_timing` reports.
fn present_stats_line(summary: Option<IntervalSummary>, timed: bool) -> String {
    let source = if timed {
        "as VK_EXT_present_timing reports them"
    } else {
        "by the host's clock, without VK_EXT_present_timing"
    };
    match summary {
        Some(summary) => format!("frames shown, {source}: {summary}"),
        None => format!("frames shown, {source}: no intervals yet"),
    }
}

impl VulkanDisplay {
    fn present(&mut self) -> Result<(), VkError> {
        self.window.pre_present_notify();
        let presented = self.compositor.present().map(|_| ());
        if let Some(report) = self.compositor.take_pacing_report() {
            tracing::warn!("{report}");
        }
        presented
    }

    fn report_present_stats(&mut self) {
        self.frames = 0;
        let line = present_stats_line(
            self.compositor.present_summary(),
            self.compositor.paces_with_present_timing(),
        );
        tracing::warn!("{line}");
    }

    fn resized(&mut self, size: PhysicalSize<u32>) {
        self.compositor
            .resize_surface(Size::new(size.width, size.height));
    }
}

impl Display for VulkanDisplay {
    type Error = VkError;

    fn render(&mut self, packet: &FramePacket) -> Result<(), VkError> {
        self.compositor.render(packet)?;
        self.present()?;
        self.frames += 1;
        if self.present_stats && self.frames >= STATS_INTERVALS {
            self.report_present_stats();
        }
        Ok(())
    }

    fn hold(&mut self) -> Result<(), VkError> {
        self.compositor.freeze()
    }

    fn set_frame_rate(&mut self, rate: FrameRate) {
        self.compositor.set_frame_rate(u32::from(rate.get()));
    }

    fn restart_pacing(&mut self) {
        self.compositor.restart_pacing();
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

    fn forget_resources(&mut self) -> Result<(), VkError> {
        self.compositor.forget_resources()
    }

    fn take_frame_timing(&mut self) -> Option<FrameTiming> {
        self.compositor.take_frame_timing()
    }

    fn redraw(&mut self) -> Result<(), VkError> {
        self.present()
    }

    fn set_title(&mut self, title: &str) {
        self.window.set_title(title);
    }

    fn set_pointer_visible(&mut self, visible: bool) {
        self.window.set_cursor_visible(visible);
    }
}

impl Drop for VulkanDisplay {
    fn drop(&mut self) {
        if self.present_stats {
            self.report_present_stats();
        }
    }
}

/// Takes a stream out of the options, which keep one that discards what it is given.
fn take_stream(stream: &mut Box<dyn Write + Send>) -> Box<dyn Write + Send> {
    std::mem::replace(stream, Box::new(std::io::sink()))
}

/// The user event that the link's wake hook sends to the event loop.
#[derive(Debug, Clone, Copy)]
struct GuestSpoke;

/// A session that has its window.
struct Running {
    driver: Driver<VulkanDisplay, SystemClock>,
    clock: SystemClock,
    window: Arc<Window>,
    /// The gamepads, unless the machine has no support for them.
    pads: Option<Gamepads>,
}

struct App {
    options: WindowedOptions,
    /// The host's end of the link until the window exists and the session takes it.
    link: Option<HostEnd>,
    running: Option<Running>,
    outcome: Option<Result<SessionEnd, WindowedError>>,
}

impl App {
    fn start(&mut self, event_loop: &ActiveEventLoop) -> Result<Running, WindowedError> {
        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
            .map(|monitor| monitor.size())
            .map(|size| Size::new(size.width, size.height));
        let size = initial_window_size(self.options.logical_size, monitor);
        // Leaving fullscreen later goes back to this size.
        let attributes = Window::default_attributes()
            .with_title(self.options.title.clone())
            .with_inner_size(PhysicalSize::new(size.width, size.height))
            .with_fullscreen(
                self.options
                    .fullscreen
                    .then_some(Fullscreen::Borderless(None)),
            );
        let window = event_loop
            .create_window(attributes)
            .map(Arc::new)
            .map_err(|error| WindowedError::Window(error.to_string()))?;

        let mut compositor =
            VkCompositor::for_window(self.options.device.clone(), Arc::clone(&window), None)
                .map_err(WindowedError::Compositor)?;
        compositor.set_filter(self.options.filter);
        compositor.set_frame_timing(self.options.frame_stats);
        let mut display = VulkanDisplay {
            compositor,
            window: Arc::clone(&window),
            present_stats: self.options.present_stats,
            frames: 0,
        };
        display.resized(window.inner_size());
        // Some window systems only show a window once it has content.
        if let Err(error) = display.redraw() {
            tracing::error!("cannot draw the window: {error}");
        }
        let chosen = display.compositor.device();
        tracing::info!("compositing on {} ({})", chosen.name, chosen.driver);
        let pacing = if display.compositor.paces_with_present_timing() {
            "with VK_EXT_present_timing"
        } else {
            "by the host's timer"
        };
        tracing::info!("frames are paced {pacing}");

        let clock = SystemClock::new();
        let audio: Box<dyn AudioSink> = match &self.options.audio {
            Some(audio) => audio.open(take_stream(&mut self.options.audio_errors)),
            None => Box::new(NullSink),
        };
        let options = SessionOptions {
            keys: self.options.keys.clone(),
            audio,
            input: self.options.input.take(),
            wait_for_modal: true,
            frame_stats: self.options.frame_stats,
            frame_stats_interval: Some(STATS_INTERVALS),
            states: self.options.states,
            // The options keep a reporter that nothing is given to.
            errors: std::mem::replace(&mut self.options.errors, Box::new(ErrorStream)),
            ..SessionOptions::default()
        };
        let link = self.link.take().expect("the session starts once");
        Ok(Running {
            driver: Driver::new(Session::new(display, clock, options), link),
            clock,
            window,
            pads: Gamepads::open(self.options.pads.clone()),
        })
    }

    /// Runs the session and tells the event loop when to come back.
    fn advance(&mut self, event_loop: &ActiveEventLoop) {
        let Some(running) = &mut self.running else {
            return;
        };
        // The pads have no events of their own in this loop: they are read whenever it wakes,
        // which is at least once per frame.
        if let Some(pads) = &mut running.pads {
            let held = pads.poll();
            running.driver.session_mut().pad_buttons(held);
        }
        let next = running.driver.wake();
        for action in running.driver.take_actions() {
            act(&running.window, action);
        }
        if let Some(end) = running.driver.end() {
            self.outcome = Some(Ok(end));
            event_loop.exit();
            return;
        }
        event_loop.set_control_flow(match next {
            Some(due) => ControlFlow::WaitUntil(running.clock.instant_at(due)),
            None => ControlFlow::Wait,
        });
    }
}

/// Does what a key asks of the window.
fn act(window: &Window, action: KeyAction) {
    match action {
        KeyAction::None => {}
        // Borderless on the monitor the window is on; the window system gives the window its
        // earlier size back when it leaves.
        KeyAction::ToggleFullscreen => window.set_fullscreen(match window.fullscreen() {
            Some(_) => None,
            None => Some(Fullscreen::Borderless(None)),
        }),
    }
}

impl ApplicationHandler<GuestSpoke> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.running.is_some() || self.link.is_none() {
            return;
        }
        match self.start(event_loop) {
            Ok(running) => self.running = Some(running),
            Err(error) => {
                self.outcome = Some(Err(error));
                event_loop.exit();
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: GuestSpoke) {
        self.advance(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(running) = &mut self.running else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => running.driver.close_requested(),
            WindowEvent::KeyboardInput { event, .. } => {
                let action = running.driver.key(event.physical_key, event.state);
                act(&running.window, action);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                running.driver.mouse_button(button, state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let size = running.window.inner_size();
                running.driver.pointer_moved(
                    Size::new(size.width, size.height),
                    Some((position.x, position.y)),
                );
            }
            WindowEvent::CursorLeft { .. } => {
                let size = running.window.inner_size();
                running
                    .driver
                    .pointer_moved(Size::new(size.width, size.height), None);
            }
            WindowEvent::Focused(focused) => running.driver.focus_changed(focused),
            WindowEvent::Resized(size) => {
                let display = running.driver.session_mut().display_mut();
                display.resized(size);
                if let Err(error) = display.redraw() {
                    tracing::error!("cannot redraw the window: {error}");
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = running.driver.session_mut().display_mut().redraw() {
                    tracing::error!("cannot redraw the window: {error}");
                }
            }
            _ => {}
        }
        self.advance(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.advance(event_loop);
    }
}

/// Runs `guest` on its own thread and a session for it in a window, until the guest shuts
/// down or goes away. Must be called on the main thread, as `winit` requires.
///
/// Closing the window sends the guest a quit request; the window stays until the guest has
/// answered with its shutdown message. F12 asks the guest to reset, Alt+Enter and F11 switch
/// to borderless fullscreen and back, F2 shows the frame rate in the title, and Tab held runs
/// the guest as fast as it goes.
pub fn run_windowed<G>(options: WindowedOptions, guest: G) -> Result<SessionEnd, WindowedError>
where
    G: FnOnce(GuestEnd) + Send + 'static,
{
    let event_loop = EventLoop::<GuestSpoke>::with_user_event()
        .build()
        .map_err(|error| WindowedError::EventLoop(error.to_string()))?;
    let proxy = event_loop.create_proxy();
    let (guest_end, host_end) = link(move || {
        // Fails only once the event loop has gone, when nobody is left to wake.
        let _ = proxy.send_event(GuestSpoke);
    });
    let guest_thread = thread::Builder::new()
        .name("vkxp-guest".to_owned())
        .spawn(move || guest(guest_end))
        .map_err(WindowedError::GuestThread)?;

    let mut app = App {
        options,
        link: Some(host_end),
        running: None,
        outcome: None,
    };
    let ran = event_loop.run_app(&mut app);
    let outcome = app.outcome.take();
    // Dropping the session and the link tells a guest that is still running that the host
    // has gone, so the join below does not wait for a guest that waits for the host.
    drop(app);
    let joined = guest_thread.join();

    ran.map_err(|error| WindowedError::EventLoop(error.to_string()))?;
    let end = outcome.unwrap_or(Ok(SessionEnd::Disconnected))?;
    joined.map_err(|_| WindowedError::GuestPanicked)?;
    Ok(end)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use vkxp_proto::{Button, Color, FrameRate, GuestToHost, HostToGuest, InputSnapshot, Op, Rect};

    use winit::keyboard::{KeyCode, NativeKeyCode};

    use super::*;
    use crate::ManualClock;
    use crate::testing::{RecordingDisplay, Shown};

    /// A driver over the recording display and a fake clock, with the guest's end of its link.
    /// No window exists anywhere in these tests.
    fn driver() -> (Driver<RecordingDisplay, ManualClock>, GuestEnd, ManualClock) {
        let (guest, host) = link(|| {});
        let clock = ManualClock::new();
        let session = Session::new(
            RecordingDisplay::new(),
            clock.clone(),
            SessionOptions::default(),
        );
        (Driver::new(session, host), guest, clock)
    }

    fn press(code: KeyCode) -> (PhysicalKey, ElementState) {
        (PhysicalKey::Code(code), ElementState::Pressed)
    }

    fn release(code: KeyCode) -> (PhysicalKey, ElementState) {
        (PhysicalKey::Code(code), ElementState::Released)
    }

    fn frame(frame_index: u64) -> GuestToHost {
        let size = Size::new(32, 24);
        GuestToHost::Frame(FramePacket {
            frame_index,
            logical_size: size,
            frame_rate: FrameRate::DEFAULT,
            brightness: 255,
            tables: vec![],
            bitmaps: Vec::new(),
            ops: vec![Op::Clear {
                clip: Rect::from_size(size),
                color: Color::new(0, 0, 0, 255),
            }],
        })
    }

    /// Everything the guest was sent, raw keys and pointer included.
    fn received_raw(guest: &GuestEnd) -> Vec<HostToGuest> {
        std::iter::from_fn(|| guest.try_recv().unwrap()).collect()
    }

    thread_local! {
        /// The last snapshot [`received`] saw in this test, as the session sent it.
        static LAST: Cell<InputSnapshot> = const { Cell::new(InputSnapshot::EMPTY) };
    }

    /// What the guest was sent, as far as the buttons go: a snapshot is reduced to its
    /// buttons, and left out when only its raw keys or its pointer differ from the snapshot
    /// before it. A snapshot that repeats the one before it stays.
    fn received(guest: &GuestEnd) -> Vec<HostToGuest> {
        let mut messages = received_raw(guest);
        messages.retain_mut(|message| {
            let HostToGuest::Input(snapshot) = message else {
                return true;
            };
            let last = LAST.replace(*snapshot);
            let only_raw = *snapshot != last && snapshot.buttons == last.buttons;
            *snapshot = InputSnapshot {
                buttons: snapshot.buttons,
                ..InputSnapshot::EMPTY
            };
            !only_raw
        });
        messages
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

    #[test]
    fn key_events_reach_the_guest_as_snapshots_at_once() {
        let (mut driver, guest, _) = driver();
        let (key, state) = press(KeyCode::ArrowDown);
        assert_eq!(driver.key(key, state), KeyAction::None);
        assert_eq!(received(&guest), [held(&[Button::Down])]);

        let (key, state) = press(KeyCode::ShiftLeft);
        driver.key(key, state);
        assert_eq!(
            received(&guest),
            [held(&[Button::Down, Button::A, Button::Shift])]
        );
        // The platform repeats a held key; the guest hears nothing new.
        let (key, state) = press(KeyCode::ArrowDown);
        driver.key(key, state);
        assert_eq!(received_raw(&guest), []);

        let (key, state) = release(KeyCode::ArrowDown);
        driver.key(key, state);
        let (key, state) = release(KeyCode::ShiftLeft);
        driver.key(key, state);
        assert_eq!(
            received(&guest),
            [held(&[Button::A, Button::Shift]), held(&[])]
        );
    }

    fn raw(keys: &[u8], pointer: Option<(i16, i16)>) -> HostToGuest {
        let snapshot = keys.iter().fold(InputSnapshot::EMPTY, |snapshot, key| {
            snapshot.with_key(*key)
        });
        HostToGuest::Input(snapshot.with_pointer(pointer))
    }

    /// Scenario "Letter and modifier", from the events of the window.
    #[test]
    fn held_keys_and_mouse_buttons_reach_the_guest_as_virtual_keys() {
        let (mut driver, guest, _) = driver();
        // Keys without a button in the default map.
        let (key, state) = press(KeyCode::KeyJ);
        driver.key(key, state);
        assert_eq!(received_raw(&guest), [raw(&[0x4A], None)]);
        let (key, state) = press(KeyCode::ControlLeft);
        driver.key(key, state);
        let HostToGuest::Input(snapshot) = received_raw(&guest)[0] else {
            panic!("a snapshot")
        };
        assert_eq!(snapshot.keys, raw_keys(&[0x4A, 0x11, 0xA2]));
        let (key, state) = release(KeyCode::ControlLeft);
        driver.key(key, state);
        assert_eq!(received_raw(&guest), [raw(&[0x4A], None)]);

        driver.mouse_button(MouseButton::Left, ElementState::Pressed);
        assert_eq!(received_raw(&guest), [raw(&[0x4A, 0x01], None)]);
        driver.mouse_button(MouseButton::Right, ElementState::Pressed);
        driver.mouse_button(MouseButton::Left, ElementState::Released);
        assert_eq!(
            received_raw(&guest),
            [raw(&[0x4A, 0x01, 0x02], None), raw(&[0x4A, 0x02], None)]
        );
        // Without focus nothing is held.
        driver.focus_changed(false);
        assert_eq!(received_raw(&guest), [raw(&[], None)]);
    }

    fn raw_keys(keys: &[u8]) -> [u64; 4] {
        let HostToGuest::Input(snapshot) = raw(keys, None) else {
            unreachable!()
        };
        snapshot.keys
    }

    #[test]
    fn a_letter_and_left_shift_hold_buttons_and_virtual_keys_together() {
        let (mut driver, guest, _) = driver();
        let (key, state) = press(KeyCode::KeyA);
        driver.key(key, state);
        let (key, state) = press(KeyCode::ShiftLeft);
        driver.key(key, state);
        let sent = received_raw(&guest);
        let Some(HostToGuest::Input(snapshot)) = sent.last() else {
            panic!("a snapshot")
        };
        assert_eq!(snapshot.keys, raw_keys(&[0x41, 0x10, 0xA0]));
        assert!(
            [Button::X, Button::A, Button::Shift]
                .iter()
                .all(|button| snapshot.is_down(*button))
        );
    }

    /// Scenario "Pointer in a letterboxed window", from the events of the window.
    #[test]
    fn the_pointer_reaches_the_guest_in_frame_pixels_while_it_is_over_the_picture() {
        let (mut driver, guest, _) = driver();
        // A frame of 32 by 24 at scale 10, with bars of 40 pixels left and right.
        let window = Size::new(400, 240);
        driver.pointer_moved(window, Some((100.0, 100.0)));
        assert_eq!(received_raw(&guest), [], "no frame yet, so no picture");

        guest.send(frame(0)).unwrap();
        driver.wake();
        assert_eq!(received_raw(&guest), [HostToGuest::Tick]);
        driver.pointer_moved(window, Some((140.0, 100.0)));
        assert_eq!(received_raw(&guest), [raw(&[], Some((10, 10)))]);
        driver.pointer_moved(window, Some((145.5, 109.0)));
        assert_eq!(received_raw(&guest), [], "the same pixel of the frame");
        driver.pointer_moved(window, Some((20.0, 100.0)));
        assert_eq!(received_raw(&guest), [raw(&[], None)], "on a bar");
        driver.pointer_moved(window, Some((359.0, 239.0)));
        assert_eq!(received_raw(&guest), [raw(&[], Some((31, 23)))]);
        driver.pointer_moved(window, None);
        assert_eq!(received_raw(&guest), [raw(&[], None)], "left the window");
    }

    #[test]
    fn a_mouse_press_answers_a_modal_message() {
        let (guest, host) = link(|| {});
        let options = SessionOptions {
            wait_for_modal: true,
            ..SessionOptions::default()
        };
        let session = Session::new(RecordingDisplay::new(), ManualClock::new(), options);
        let mut driver = Driver::new(session, host);
        guest
            .send(GuestToHost::Modal {
                text: "hello".to_owned(),
            })
            .unwrap();
        driver.wake();
        assert_eq!(received(&guest), []);
        driver.mouse_button(MouseButton::Left, ElementState::Pressed);
        assert_eq!(received(&guest), [HostToGuest::ModalDismissed]);
    }

    #[test]
    fn unidentified_keys_are_ignored() {
        let (mut driver, guest, _) = driver();
        let key = PhysicalKey::Unidentified(NativeKeyCode::Xkb(240));
        assert_eq!(driver.key(key, ElementState::Pressed), KeyAction::None);
        assert_eq!(driver.key(key, ElementState::Released), KeyAction::None);
        assert_eq!(received(&guest), []);
    }

    #[test]
    fn host_keys_reset_toggle_fullscreen_and_fast_forward() {
        let (mut driver, guest, _) = driver();
        let (key, state) = press(KeyCode::F12);
        assert_eq!(driver.key(key, state), KeyAction::None);
        driver.key(key, state);
        assert_eq!(received(&guest), [HostToGuest::Reset], "once per press");

        let (key, state) = press(KeyCode::AltRight);
        driver.key(key, state);
        let (key, state) = press(KeyCode::Enter);
        assert_eq!(driver.key(key, state), KeyAction::ToggleFullscreen);
        assert_eq!(
            received(&guest),
            [held(&[Button::Alt])],
            "Enter is not C here"
        );
        let (key, state) = release(KeyCode::Enter);
        driver.key(key, state);
        // The second press toggles back, again without C.
        let (key, state) = press(KeyCode::Enter);
        assert_eq!(driver.key(key, state), KeyAction::ToggleFullscreen);
        let (key, state) = release(KeyCode::Enter);
        driver.key(key, state);
        assert_eq!(received(&guest), [], "neither C nor ALT is newly held");
        let (key, state) = release(KeyCode::AltRight);
        driver.key(key, state);
        assert_eq!(received(&guest), [held(&[])]);

        for _ in 0..2 {
            let (key, state) = press(KeyCode::F11);
            assert_eq!(driver.key(key, state), KeyAction::ToggleFullscreen);
            let (key, state) = release(KeyCode::F11);
            assert_eq!(driver.key(key, state), KeyAction::None);
        }
        assert_eq!(received(&guest), [], "F11 is not a button");

        let (key, state) = press(KeyCode::Tab);
        driver.key(key, state);
        assert!(driver.session().fast_forward());
        for index in 0..3 {
            guest.send(frame(index)).unwrap();
            assert_eq!(driver.wake(), None, "no tick is left waiting");
            assert_eq!(received(&guest), [HostToGuest::Tick]);
        }
        let (key, state) = release(KeyCode::Tab);
        driver.key(key, state);
        assert!(!driver.session().fast_forward());
    }

    #[test]
    fn frame_rate_key_changes_the_title_only() {
        let (mut driver, guest, clock) = driver();
        guest
            .send(GuestToHost::SetTitle {
                title: "Game".to_owned(),
            })
            .unwrap();
        guest.send(frame(0)).unwrap();
        driver.wake();
        let (key, state) = press(KeyCode::F2);
        assert_eq!(driver.key(key, state), KeyAction::None);
        for index in 1..=40 {
            guest.send(frame(index)).unwrap();
            while let Some(due) = driver.wake() {
                clock.advance_to(due);
            }
        }
        assert_eq!(driver.session().display().title(), "Game - 40.0 FPS");
        assert_eq!(received(&guest), vec![HostToGuest::Tick; 41]);
    }

    #[test]
    fn losing_focus_releases_the_keys_and_gaining_it_sends_nothing() {
        let (mut driver, guest, _) = driver();
        let (key, state) = press(KeyCode::KeyZ);
        driver.key(key, state);
        received(&guest);
        driver.focus_changed(false);
        assert_eq!(received(&guest), [held(&[])]);
        driver.focus_changed(true);
        assert_eq!(received(&guest), []);
    }

    #[test]
    fn close_request_asks_the_guest_to_quit_and_shutdown_ends_the_session() {
        let (mut driver, guest, _) = driver();
        guest.send(frame(0)).unwrap();
        driver.wake();
        assert_eq!(received(&guest), [HostToGuest::Tick]);

        driver.close_requested();
        assert_eq!(received(&guest), [HostToGuest::Quit]);
        assert_eq!(
            driver.end(),
            None,
            "the window stays until the guest answers"
        );

        guest.send(GuestToHost::Shutdown).unwrap();
        driver.wake();
        assert_eq!(driver.end(), Some(SessionEnd::Shutdown));
    }

    #[test]
    fn frames_are_shown_and_paced_between_events() {
        let (mut driver, guest, clock) = driver();
        guest.send(frame(0)).unwrap();
        assert_eq!(driver.wake(), None);
        guest.send(frame(1)).unwrap();
        // The event loop is told to come back when the tick is due.
        assert_eq!(driver.wake(), Some(Duration::from_millis(25)));
        assert_eq!(received(&guest), [HostToGuest::Tick]);

        // An event in between does not release the tick early.
        clock.advance(Duration::from_millis(10));
        let (key, state) = press(KeyCode::KeyC);
        driver.key(key, state);
        assert_eq!(received(&guest), [held(&[Button::C])]);
        clock.advance(Duration::from_millis(15));
        assert_eq!(driver.wake(), None);
        assert_eq!(received(&guest), [HostToGuest::Tick]);
        assert_eq!(driver.session().display().shown(), Shown::Frame(1));
    }

    #[test]
    fn vanished_guest_ends_the_session() {
        let (mut driver, guest, _) = driver();
        drop(guest);
        driver.wake();
        assert_eq!(driver.end(), Some(SessionEnd::Disconnected));
    }

    #[test]
    fn present_statistics_line_names_the_source_of_its_times() {
        let summary = IntervalSummary {
            count: 600,
            mean: 25.0e6,
            smallest: 16_600_000,
            largest: 33_400_000,
        };
        assert_eq!(
            present_stats_line(Some(summary), true),
            "frames shown, as VK_EXT_present_timing reports them: 600 intervals, mean 25.00 ms, \
             smallest 16.60 ms, largest 33.40 ms"
        );
        assert_eq!(
            present_stats_line(None, false),
            "frames shown, by the host's clock, without VK_EXT_present_timing: no intervals yet"
        );
    }

    #[test]
    fn window_starts_at_the_largest_whole_scale_that_fits_the_monitor() {
        let logical = Size::new(640, 480);
        let cases = [
            (Some(Size::new(1920, 1040)), Size::new(1280, 960)),
            (Some(Size::new(600, 400)), Size::new(640, 480)),
            (Some(Size::new(1920, 1080)), Size::new(1280, 960)),
            (Some(Size::new(2560, 1440)), Size::new(1920, 1440)),
            (Some(Size::new(3840, 2160)), Size::new(2560, 1920)),
            (Some(Size::new(1280, 1024)), Size::new(1280, 960)),
            (Some(Size::new(640, 480)), Size::new(640, 480)),
            // Smaller than the game, or unknown: once.
            (Some(Size::new(320, 240)), Size::new(640, 480)),
            (Some(Size::new(0, 0)), Size::new(640, 480)),
            (None, Size::new(640, 480)),
        ];
        for (monitor, expected) in cases {
            assert_eq!(
                initial_window_size(logical, monitor),
                expected,
                "{monitor:?}"
            );
        }
        assert_eq!(
            initial_window_size(Size::new(0, 0), Some(Size::new(1920, 1080))),
            Size::new(0, 0)
        );
    }
}
