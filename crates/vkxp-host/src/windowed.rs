//! The windowed runner: a session in a `winit` window, shown by the Vulkan backend.
//!
//! The event loop sleeps until the session's next tick is due and is woken early by the
//! link's wake hook, through an event-loop proxy, whenever the guest has sent something.
//! [`Driver`] is the part between the window events and the session; it has no window in it,
//! so the tests drive it with synthetic events.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use thiserror::Error;
use vkxp_proto::link::{GuestEnd, HostEnd, link};
use vkxp_proto::{BitmapId, FramePacket, Size};
use vkxp_vk::{DeviceChoice, VkCompositor, VkError};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Fullscreen, Window, WindowId};

use crate::{
    AudioOptions, AudioSink, Clock, Display, KeyAction, KeyMap, NullSink, Session, SessionEnd,
    SessionOptions, SystemClock,
};

/// How a windowed session is set up.
#[derive(Debug, Clone)]
pub struct WindowedOptions {
    /// The device that composites and presents. There is deliberately no default: on a
    /// machine with several GPUs the caller says which one drives the window.
    pub device: DeviceChoice,
    /// The window title until the guest sets one.
    pub title: String,
    /// The logical screen size the window is first sized for.
    pub logical_size: Size,
    /// The physical keys that hold each button.
    pub keys: KeyMap,
    /// The audio output, or `None` for a run without sound, which opens no audio device.
    pub audio: Option<AudioOptions>,
}

impl WindowedOptions {
    /// The defaults for a game of 640 by 480 pixels on `device`, without sound.
    pub fn new(device: DeviceChoice) -> WindowedOptions {
        WindowedOptions {
            device,
            title: "vkxp".to_owned(),
            logical_size: Size::new(640, 480),
            keys: KeyMap::default(),
            audio: None,
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

/// The size in pixels a window for a `logical` screen starts with on a monitor of `monitor`
/// pixels: the logical size at the largest whole-number scale that fits, and at least once.
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
}

impl VulkanDisplay {
    fn present(&mut self) -> Result<(), VkError> {
        self.window.pre_present_notify();
        self.compositor.present().map(|_| ())
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
        self.present()
    }

    fn hold(&mut self) -> Result<(), VkError> {
        self.compositor.freeze()
    }

    fn set_mix(&mut self, weight: f32) {
        self.compositor.set_mix(weight);
    }

    fn set_mask(&mut self, mask: Option<(BitmapId, u32)>) {
        self.compositor.set_mask(mask);
    }

    fn has_bitmap(&self, id: BitmapId) -> bool {
        self.compositor.has_bitmap(id)
    }

    fn redraw(&mut self) -> Result<(), VkError> {
        self.present()
    }

    fn set_title(&mut self, title: &str) {
        self.window.set_title(title);
    }
}

/// The user event that the link's wake hook sends to the event loop.
#[derive(Debug, Clone, Copy)]
struct GuestSpoke;

/// A session that has its window.
struct Running {
    driver: Driver<VulkanDisplay, SystemClock>,
    clock: SystemClock,
    window: Arc<Window>,
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
        let attributes = Window::default_attributes()
            .with_title(self.options.title.clone())
            .with_inner_size(PhysicalSize::new(size.width, size.height));
        let window = event_loop
            .create_window(attributes)
            .map(Arc::new)
            .map_err(|error| WindowedError::Window(error.to_string()))?;

        let compositor =
            VkCompositor::for_window(self.options.device.clone(), Arc::clone(&window), None)
                .map_err(WindowedError::Compositor)?;
        let mut display = VulkanDisplay {
            compositor,
            window: Arc::clone(&window),
        };
        display.resized(window.inner_size());
        // Some window systems only show a window once it has content.
        if let Err(error) = display.redraw() {
            eprintln!("vkxp: cannot draw the window: {error}");
        }

        let clock = SystemClock::new();
        let audio: Box<dyn AudioSink> = match &self.options.audio {
            Some(audio) => audio.open(),
            None => Box::new(NullSink),
        };
        let options = SessionOptions {
            keys: self.options.keys.clone(),
            audio,
            ..SessionOptions::default()
        };
        let link = self.link.take().expect("the session starts once");
        Ok(Running {
            driver: Driver::new(Session::new(display, clock, options), link),
            clock,
            window,
        })
    }

    /// Runs the session and tells the event loop when to come back.
    fn advance(&mut self, event_loop: &ActiveEventLoop) {
        let Some(running) = &mut self.running else {
            return;
        };
        let next = running.driver.wake();
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
                if action == KeyAction::ToggleFullscreen {
                    let window = &running.window;
                    window.set_fullscreen(match window.fullscreen() {
                        Some(_) => None,
                        None => Some(Fullscreen::Borderless(None)),
                    });
                }
            }
            WindowEvent::Focused(focused) => running.driver.focus_changed(focused),
            WindowEvent::Resized(size) => {
                let display = running.driver.session_mut().display_mut();
                display.resized(size);
                if let Err(error) = display.redraw() {
                    eprintln!("vkxp: cannot redraw the window: {error}");
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = running.driver.session_mut().display_mut().redraw() {
                    eprintln!("vkxp: cannot redraw the window: {error}");
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
/// answered with its shutdown message. F12 asks the guest to reset, Alt+Enter switches to
/// borderless fullscreen and back, F2 shows the frame rate in the title, and Tab held runs
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
            bitmaps: Vec::new(),
            ops: vec![Op::Clear {
                clip: Rect::from_size(size),
                color: Color::new(0, 0, 0, 255),
            }],
        })
    }

    fn received(guest: &GuestEnd) -> Vec<HostToGuest> {
        std::iter::from_fn(|| guest.try_recv().unwrap()).collect()
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
        assert_eq!(received(&guest), []);

        let (key, state) = release(KeyCode::ArrowDown);
        driver.key(key, state);
        let (key, state) = release(KeyCode::ShiftLeft);
        driver.key(key, state);
        assert_eq!(
            received(&guest),
            [held(&[Button::A, Button::Shift]), held(&[])]
        );
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
        let (key, state) = release(KeyCode::AltRight);
        driver.key(key, state);
        assert_eq!(received(&guest), [held(&[])]);

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
    fn window_starts_at_the_largest_whole_scale_that_fits_the_monitor() {
        let logical = Size::new(640, 480);
        let cases = [
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
