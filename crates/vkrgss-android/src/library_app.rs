//! The library's activity: the library screen ([`Screen`]) in a window of its own, drawn by
//! the compositor as the picture over an empty frame, with the imports running on a thread
//! of their own.
//!
//! A zip to import comes from the activity's Java side (the picker's choice, or the extra
//! `import` of the intent), through [`request_import`], into a queue that the event loop
//! empties one import at a time: the zip is copied into the cache directory through the
//! activity, unpacked into the library, and the copy removed.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};
use vkrgss_proto::Size;
use vkrgss_vk::{DeviceChoice, VkCompositor};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{NativeKeyCode, PhysicalKey};
use winit::platform::android::EventLoopBuilderExtAndroid;
use winit::platform::android::activity::AndroidApp;
use winit::window::{Window, WindowId};

use crate::java::Activity;
use crate::library::{Imported, Library};
use crate::screen::{Request, Screen};
use crate::unpack::Kind;

/// The zips waiting to be imported, as URIs.
static PENDING: Mutex<Vec<(String, Kind)>> = Mutex::new(Vec::new());

/// What wakes the library's event loop while it runs.
static WAKER: Mutex<Option<EventLoopProxy<Event>>> = Mutex::new(None);

/// Android's `KEYCODE_BACK`.
const KEYCODE_BACK: u32 = 4;

/// What the event loop is told.
#[derive(Debug)]
enum Event {
    /// A zip waits in the queue.
    Pending,
    /// An import has ended with what it imported or why it failed.
    Imported(Result<Imported, String>),
}

/// Puts the zip `uri` into the queue of imports: the RTP with `rtp`, else a game.
pub(crate) fn request_import(uri: String, rtp: bool) {
    let kind = if rtp { Kind::Rtp } else { Kind::Game };
    tracing::info!("library: asked to import {uri} as {kind:?}");
    PENDING.lock().expect("not poisoned").push((uri, kind));
    if let Some(waker) = WAKER.lock().expect("not poisoned").as_ref() {
        let _ = waker.send_event(Event::Pending);
    }
}

/// The library screen until the player leaves it with the back key.
pub(crate) fn run(
    app: AndroidApp,
    activity: Activity,
    private: &Path,
    fonts: PathBuf,
) -> Result<()> {
    let mut builder = EventLoop::<Event>::with_user_event();
    builder.with_android_app(app);
    let event_loop = builder.build().context("cannot run the event loop")?;
    let proxy = event_loop.create_proxy();
    *WAKER.lock().expect("not poisoned") = Some(proxy.clone());
    let library = Library::new(private);
    let games = library.games();
    for game in &games {
        tracing::info!(
            "library: game {} is {} in {}",
            game.id,
            game.title,
            game.dir.display()
        );
    }
    let mut state = LibraryApp {
        screen: Screen::new(games, vec![fonts]),
        library,
        activity,
        proxy,
        compositor: None,
        window: None,
        suspended: true,
        importing: false,
        failed: None,
    };
    let ran = event_loop.run_app(&mut state);
    *WAKER.lock().expect("not poisoned") = None;
    ran.context("the event loop failed")?;
    match state.failed {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct LibraryApp {
    screen: Screen,
    library: Library,
    activity: Activity,
    proxy: EventLoopProxy<Event>,
    // Before the window in declaration order: the surface goes before its window does. The
    // compositor also keeps the window alive itself.
    compositor: Option<VkCompositor>,
    window: Option<Arc<Window>>,
    /// Whether the window system has taken the window away.
    suspended: bool,
    /// Whether an import runs.
    importing: bool,
    /// Why the screen could not be shown.
    failed: Option<anyhow::Error>,
}

impl LibraryApp {
    fn size(&self) -> Size {
        let size = self.window.as_ref().map(|window| window.inner_size());
        size.map_or(Size::new(0, 0), |size| Size::new(size.width, size.height))
    }

    /// Shows the screen, drawn afresh when it has changed.
    fn draw(&mut self) {
        let size = self.size();
        let Some(compositor) = self.compositor.as_mut().filter(|_| !self.suspended) else {
            return;
        };
        if size.width == 0 || size.height == 0 {
            return;
        }
        compositor.resize_surface(size);
        if self.screen.changed(size) {
            let picture = self.screen.picture(size);
            if let Err(error) = compositor.set_overlay(Some(&picture)) {
                tracing::error!("cannot draw the library: {error}");
            }
        }
        if let Err(error) = compositor.present() {
            tracing::error!("cannot show the library: {error}");
        }
    }

    /// Shows the games of the library afresh, and says where their rows are for the tests.
    fn reload(&mut self) {
        let games = self.library.games();
        let size = self.size();
        for (index, game) in games.iter().enumerate() {
            let place = Screen::row_centre(size, index)
                .map(|(x, y)| format!(", row {index} at ({x:.0}, {y:.0})"))
                .unwrap_or_default();
            tracing::info!("library: game {} is {}{place}", game.id, game.title);
        }
        self.screen.set_games(games);
    }

    /// Starts the next import of the queue, unless one runs.
    fn next_import(&mut self) {
        if self.importing {
            return;
        }
        let next = {
            let mut pending = PENDING.lock().expect("not poisoned");
            (!pending.is_empty()).then(|| pending.remove(0))
        };
        let Some((uri, kind)) = next else {
            return;
        };
        self.importing = true;
        self.screen.set_busy(true);
        let name = uri.rsplit(['/', ':']).next().unwrap_or(&uri).to_owned();
        self.screen.set_status(format!("Importing {name}…"));
        let (activity, library, proxy) = (
            self.activity.clone(),
            self.library.clone(),
            self.proxy.clone(),
        );
        let spawned = thread::Builder::new()
            .name("vkrgss-import".to_owned())
            .spawn(move || {
                let outcome =
                    import(&activity, &library, &uri, kind).map_err(|error| format!("{error:#}"));
                let _ = proxy.send_event(Event::Imported(outcome));
            });
        if let Err(error) = spawned {
            self.imported(Err(format!("cannot start the import: {error}")));
        }
    }

    /// An import has ended.
    fn imported(&mut self, outcome: Result<Imported, String>) {
        self.importing = false;
        self.screen.set_busy(false);
        self.reload();
        match outcome {
            Ok(Imported::Game(game)) => {
                tracing::info!("library: imported game {} as {}", game.id, game.title);
                self.screen.set_status(format!("Imported {}.", game.title));
            }
            Ok(Imported::Rtp(name)) => {
                tracing::info!("library: imported the RTP {name}");
                self.screen.set_status(format!("Imported the RTP {name}."));
            }
            Err(error) => {
                tracing::error!("library: the import failed: {error}");
                self.screen
                    .set_status(format!("The import failed: {error}"));
            }
        }
        self.next_import();
    }

    fn request(&mut self, request: Request) {
        let outcome = match request {
            Request::Import(kind) => self.activity.open_picker(kind == Kind::Rtp),
            Request::Play(game) => {
                tracing::info!("library: starting game {}, {}", game.id, game.title);
                let _ = self.library.play(game.id);
                self.reload();
                self.activity.start_game(&game.dir)
            }
            Request::Delete(game) => {
                tracing::info!("library: deleting game {}, {}", game.id, game.title);
                let deleted = self.library.delete(game.id);
                self.reload();
                if deleted.is_ok() {
                    self.screen.set_status(format!("Deleted {}.", game.title));
                }
                deleted
            }
        };
        if let Err(error) = outcome {
            tracing::error!("library: {error:#}");
            self.screen.set_status(format!("{error:#}"));
        }
    }
}

/// Imports the zip `uri` into `library`.
fn import(activity: &Activity, library: &Library, uri: &str, kind: Kind) -> Result<Imported> {
    let zip = activity.copy_to_cache(uri).context("cannot read the zip")?;
    let imported = library.import(&zip, kind);
    let _ = std::fs::remove_file(&zip);
    imported
}

impl ApplicationHandler<Event> for LibraryApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let started = match (&self.window, &mut self.compositor) {
            (Some(window), Some(compositor)) => compositor
                .attach_window(Arc::clone(window))
                .context("cannot show the library in the window that came back"),
            _ => event_loop
                .create_window(Window::default_attributes().with_title("vkrgss"))
                .context("cannot create the window")
                .and_then(|window| {
                    let window = Arc::new(window);
                    let compositor = VkCompositor::for_window(
                        DeviceChoice::default(),
                        Arc::clone(&window),
                        None,
                    )
                    .context("cannot start the compositor for the window")?;
                    self.window = Some(window);
                    self.compositor = Some(compositor);
                    Ok(())
                }),
        };
        if let Err(error) = started {
            self.failed = Some(error);
            event_loop.exit();
            return;
        }
        self.suspended = false;
        // The picture is drawn afresh for the new surface.
        self.reload();
        self.draw();
        self.next_import();
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(compositor) = &mut self.compositor
            && let Err(error) = compositor.release_surface()
        {
            tracing::error!("cannot release the surface of the window: {error}");
        }
        self.suspended = true;
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: Event) {
        match event {
            Event::Pending => self.next_import(),
            Event::Imported(outcome) => self.imported(outcome),
        }
        self.draw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. }
                if event.state.is_pressed()
                    && event.physical_key
                        == PhysicalKey::Unidentified(NativeKeyCode::Android(KEYCODE_BACK)) =>
            {
                event_loop.exit();
            }
            WindowEvent::Touch(touch) => {
                let position = (touch.location.x as f32, touch.location.y as f32);
                let request = self
                    .screen
                    .touch(self.size(), touch.id, touch.phase, position);
                if let Some(request) = request {
                    self.request(request);
                }
                self.draw();
            }
            WindowEvent::Resized(_) | WindowEvent::RedrawRequested => self.draw(),
            _ => {}
        }
    }
}
