//! The library's activity: the library screen ([`Screen`]) in a window of its own, drawn by
//! the compositor as the picture over an empty frame, with the imports running on a thread
//! of their own.
//!
//! A file to import comes from the activity's Java side (the picker's choice, or the extra
//! `import` of the intent), through [`request_import`], into a queue that the event loop
//! empties one import at a time: the file is copied into the cache directory through the
//! activity, a zip is unpacked into the library, a built-in scripts file is stored there
//! under the name of its runtime, and the copy removed. When the classes of built-in scripts
//! do not tell their runtime, the screen asks for it, and the next import waits for the
//! answer.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};
use vkrgss_proto::Size;
use vkrgss_vfs::Runtime;
use vkrgss_vk::{DeviceChoice, VkCompositor};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{NativeKeyCode, PhysicalKey};
use winit::platform::android::EventLoopBuilderExtAndroid;
use winit::platform::android::activity::AndroidApp;
use winit::window::{Window, WindowId};

use crate::java::{Activity, Pick};
use crate::library::{BUILTIN_LIMIT, Imported, Library, builtin_runtime};
use crate::screen::{Request, Screen};
use crate::unpack::Kind;

/// The files waiting to be imported, as URIs, with what they hold.
static PENDING: Mutex<Vec<(String, Pick)>> = Mutex::new(Vec::new());

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
    /// Built-in scripts whose classes do not tell their runtime, which the player is asked
    /// for.
    Unclear(Vec<u8>),
}

/// Puts the file `uri` into the queue of imports, as what `pick` says it holds.
pub(crate) fn request_import(uri: String, pick: Pick) {
    tracing::info!("library: asked to import {uri} as {pick:?}");
    PENDING.lock().expect("not poisoned").push((uri, pick));
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
        unclear: None,
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
    /// Whether an import runs, or waits for the runtime of its built-in scripts.
    importing: bool,
    /// The built-in scripts whose runtime the screen asks for.
    unclear: Option<Vec<u8>>,
    /// Why the screen could not be shown.
    failed: Option<anyhow::Error>,
}

impl LibraryApp {
    fn size(&self) -> Size {
        let size = self.window.as_ref().map(|window| window.inner_size());
        size.map_or(Size::new(0, 0), |size| Size::new(size.width, size.height))
    }

    /// Gives the screen what covers the window's edges, which can change sides without a
    /// change of the window's size.
    fn read_insets(&mut self) {
        match self.activity.insets() {
            Ok(insets) => self.screen.set_insets(insets),
            Err(error) => tracing::error!("cannot read the window's insets: {error:#}"),
        }
    }

    /// Shows the screen, drawn afresh when it has changed.
    fn draw(&mut self) {
        self.read_insets();
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
        self.read_insets();
        let size = self.size();
        for (index, game) in games.iter().enumerate() {
            let place = self
                .screen
                .row_centre(size, index)
                .map(|(x, y)| format!(", row {index} at ({x:.0}, {y:.0})"))
                .unwrap_or_default();
            tracing::info!("library: game {} is {}{place}", game.id, game.title);
        }
        self.screen.set_games(games);
        self.log_buttons();
    }

    /// Says where the buttons shown are, so that a test can tap them.
    fn log_buttons(&self) {
        let size = self.size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let buttons: Vec<String> = self
            .screen
            .button_centres(size)
            .iter()
            .map(|(label, (x, y))| format!("{label} at ({x:.0}, {y:.0})"))
            .collect();
        tracing::info!("library: buttons {}", buttons.join(", "));
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
        let Some((uri, pick)) = next else {
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
                let event = match import(&activity, &library, &uri, pick) {
                    Ok(Ok(imported)) => Event::Imported(Ok(imported)),
                    Ok(Err(text)) => Event::Unclear(text),
                    Err(error) => Event::Imported(Err(format!("{error:#}"))),
                };
                let _ = proxy.send_event(event);
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
            Ok(Imported::BuiltinScripts(runtime)) => {
                let file = self.library.builtin_scripts(runtime);
                tracing::info!(
                    "library: imported the built-in scripts of {} as {}",
                    runtime.name(),
                    file.display()
                );
                self.screen.set_status(format!(
                    "Imported the built-in scripts of {}.",
                    runtime_title(runtime)
                ));
            }
            Err(error) => {
                tracing::error!("library: the import failed: {error}");
                self.screen
                    .set_status(format!("The import failed: {error}"));
            }
        }
        self.next_import();
    }

    /// Built-in scripts whose runtime the player is asked for.
    fn unclear(&mut self, text: Vec<u8>) {
        tracing::info!("library: the built-in scripts do not tell their runtime; asking");
        self.unclear = Some(text);
        self.screen.set_busy(false);
        self.screen.ask_runtime();
        self.log_buttons();
    }

    fn request(&mut self, request: Request) {
        let outcome = match request {
            Request::Import(Kind::Game) => self.activity.open_picker(Pick::Game),
            Request::Import(Kind::Rtp) => self.activity.open_picker(Pick::Rtp),
            Request::ImportBuiltinScripts => self.activity.open_picker(Pick::BuiltinScripts),
            Request::BuiltinScriptsOf(runtime) => {
                // Only asked with scripts that wait.
                let Some(text) = self.unclear.take() else {
                    return;
                };
                match runtime {
                    Some(runtime) => {
                        tracing::info!("library: the player says {}", runtime.name());
                        let stored = self
                            .library
                            .import_builtin_scripts(runtime, &text)
                            .map_err(|error| format!("{error:#}"));
                        self.imported(stored);
                    }
                    None => {
                        tracing::info!("library: the built-in scripts are not imported");
                        self.importing = false;
                        self.screen
                            .set_status("The built-in scripts were not imported.");
                        self.next_import();
                    }
                }
                Ok(())
            }
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

/// The name of a runtime as the player knows it.
fn runtime_title(runtime: Runtime) -> &'static str {
    match runtime {
        Runtime::Rgss1 => "XP",
        Runtime::Rgss2 => "VX",
        Runtime::Rgss3 => "VX Ace",
    }
}

/// Reads the copy of a file in the cache directory, at most [`BUILTIN_LIMIT`] bytes, and
/// removes it.
fn read_cached(file: PathBuf) -> Result<Vec<u8>> {
    let size = std::fs::metadata(&file)
        .with_context(|| format!("cannot read {}", file.display()))?
        .len();
    let text = match size > BUILTIN_LIMIT {
        true => Err(anyhow::anyhow!(
            "the file has {size} bytes; a built-in scripts file has at most {BUILTIN_LIMIT}"
        )),
        false => std::fs::read(&file).with_context(|| format!("cannot read {}", file.display())),
    };
    let _ = std::fs::remove_file(&file);
    text
}

/// Imports the file `uri` into `library` as what `pick` says it holds. `Ok(Err(text))`:
/// built-in scripts whose classes do not tell their runtime, which are not imported yet.
fn import(
    activity: &Activity,
    library: &Library,
    uri: &str,
    pick: Pick,
) -> Result<Result<Imported, Vec<u8>>> {
    let copy = activity
        .copy_to_cache(uri)
        .context("cannot read the file")?;
    let kind = match pick {
        Pick::Game => Kind::Game,
        Pick::Rtp => Kind::Rtp,
        Pick::BuiltinScripts => {
            let text = read_cached(copy)?;
            return match builtin_runtime(&text) {
                Some(runtime) => library.import_builtin_scripts(runtime, &text).map(Ok),
                None => Ok(Err(text)),
            };
        }
    };
    let imported = library.import(&copy, kind);
    let _ = std::fs::remove_file(&copy);
    imported.map(Ok)
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
            Event::Unclear(text) => self.unclear(text),
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
