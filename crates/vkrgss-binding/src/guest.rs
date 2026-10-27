//! The guest state: what the Ruby classes operate on.

use std::cell::{Cell, RefCell};
use std::sync::{Mutex, PoisonError};

use vkrgss_core::{Bitmaps, FontDefaults, InputState, Scene};
use vkrgss_proto::BitmapId;
use vkrgss_ruby_sys::ffi::VALUE;
use vkrgss_ruby_sys::{Raised, Ruby};
use vkrgss_vfs::{GameFiles, Runtime};

use crate::element::{self, SceneClasses};
use crate::error::{self, ErrorClasses};
use crate::runtime::{self, RunConfig};
use crate::text::TextBackend;
use crate::{audio, bitmap, color, font, rect, table, win32api};

/// What a guest is created from.
pub struct Config {
    /// The game's files: its directory and its run-time package directories.
    pub files: GameFiles,
    /// The family names `Font.default_name` starts with in RGSS1, tried in order. A game of
    /// RGSS2 starts with the font defaults of its runtime instead.
    pub font_names: Vec<String>,
    /// Where glyphs and the knowledge of installed fonts come from.
    pub text: Box<dyn TextBackend + Send>,
    /// What running a game needs: its configuration file, the built-in scripts and the modes.
    pub run: RunConfig,
}

/// The state the Ruby classes operate on. One exists per process, for its one interpreter.
pub struct Guest {
    pub(crate) bitmaps: Bitmaps,
    pub(crate) font_defaults: FontDefaults,
    /// Whether `Font.default_name` is an Array (or a single String).
    pub(crate) font_names_listed: bool,
    pub(crate) files: GameFiles,
    pub(crate) text: Box<dyn TextBackend + Send>,
    /// What is on the screen: the elements of the scene classes, from which frames are
    /// extracted.
    pub(crate) scene: Scene,
    /// What `Input` answers from.
    pub(crate) input: InputState,
    /// The link to the host and what running a game keeps.
    pub(crate) runtime: runtime::State,
    /// What the emulated `Win32API` functions keep between calls.
    pub(crate) win32api: win32api::State,
    /// The positions of the music and the background sound of RGSS3.
    pub(crate) audio: audio::State,
    /// Whether `owner` and `observer` are still to be interned: in a game of RGSS2 until the
    /// first of the calls at which the original runtime interns them (see
    /// [`wrap::intern_owner_names`](crate::wrap::intern_owner_names)).
    pub(crate) owner_names_pending: bool,
}

/// The Ruby classes this crate defines, and the ones it raises.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Classes {
    pub(crate) errors: ErrorClasses,
    pub(crate) color: VALUE,
    pub(crate) tone: VALUE,
    pub(crate) rect: VALUE,
    pub(crate) table: VALUE,
    pub(crate) font: VALUE,
    pub(crate) bitmap: VALUE,
    pub(crate) scene: SceneClasses,
}

/// A value of the process that only a thread holding the interpreter reaches: [`get`]
/// needs the [`Ruby`] token, which exists only on such a thread.
///
/// Ruby calls into the binding on any thread that holds the interpreter: over Ruby 1.8.7
/// every Ruby thread is the interpreter's one OS thread, over Ruby 1.9.2 a script thread is a
/// native thread of its own that runs under the interpreter lock, one at a time. The binding
/// never releases that lock, so accesses through two tokens never overlap, and the lock
/// orders them.
///
/// [`get`]: InterpreterCell::get
pub(crate) struct InterpreterCell<T>(T);

// SAFETY: the value is reached only through `get`, whose reference lives no longer than the
// `&Ruby` it was given. A `Ruby` is neither `Send` nor `Sync` and exists only on a thread that
// holds the interpreter, so at most one thread reaches the value at a time, and the
// interpreter lock passed between them orders their accesses. That the value may be used on
// another thread than the one that made it is the condition of `new`.
unsafe impl<T> Sync for InterpreterCell<T> {}

impl<T> InterpreterCell<T> {
    /// # Safety
    ///
    /// Nothing in `value`, now or later, may be tied to the thread that uses it: a value that
    /// is not `Send` only because its accesses are not synchronized, such as an `Rc` or a
    /// `RefCell`, is allowed, because the interpreter lock orders them.
    pub(crate) const unsafe fn new(value: T) -> Self {
        InterpreterCell(value)
    }

    /// The value, for a thread that holds the interpreter.
    pub(crate) fn get<'a>(&'a self, _ruby: &'a Ruby) -> &'a T {
        &self.0
    }
}

// SAFETY: what is not `Send` in a guest are the `Rc`s of the tables that tilemaps share with
// their `Table` objects, and its cells; no part of it is tied to a thread.
static GUEST: InterpreterCell<RefCell<Option<Guest>>> =
    unsafe { InterpreterCell::new(RefCell::new(None)) };
// SAFETY: the classes are plain values.
static CLASSES: InterpreterCell<Cell<Option<Classes>>> =
    unsafe { InterpreterCell::new(Cell::new(None)) };
/// Arena bitmaps whose Ruby objects were collected, waiting to be disposed. The collector
/// frees objects on whichever Ruby thread it runs, without a `Ruby` token, so the queue is
/// behind a lock of its own.
static FREED: Mutex<Vec<BitmapId>> = Mutex::new(Vec::new());

impl Guest {
    fn new(config: Config) -> Guest {
        let font_defaults = font::initial_defaults(config.run.runtime, config.font_names);
        let font_names_listed = font_defaults.names.len() != 1;
        Guest {
            bitmaps: Bitmaps::new(),
            font_defaults,
            font_names_listed,
            files: config.files,
            text: config.text,
            scene: runtime::new_scene(config.run.runtime),
            input: InputState::with_rules(runtime::rules(config.run.runtime)),
            owner_names_pending: config.run.runtime == Runtime::Rgss2,
            runtime: runtime::State::new(config.run),
            win32api: win32api::State::default(),
            audio: audio::State::default(),
        }
    }

    /// Defines the RGSS classes in the interpreter and makes a guest of `config` the state
    /// they operate on. Called once per process, on the interpreter thread, before any
    /// script runs.
    ///
    /// The classes are reachable from Ruby's constants, which is what keeps them alive.
    ///
    /// # Panics
    ///
    /// Panics when a guest is already installed.
    pub fn install(ruby: &Ruby, config: Config) -> Result<(), Raised> {
        assert!(
            CLASSES.get(ruby).get().is_none(),
            "a guest is already installed"
        );
        let classes = Classes {
            errors: error::define(ruby)?,
            color: color::define_color(ruby, config.run.runtime)?,
            tone: color::define_tone(ruby, config.run.runtime)?,
            rect: rect::define(ruby, config.run.runtime)?,
            table: table::define(ruby, config.run.runtime)?,
            font: font::define(ruby, config.run.runtime)?,
            bitmap: bitmap::define(ruby, config.run.runtime)?,
            scene: element::define(ruby, config.run.runtime)?,
        };
        CLASSES.get(ruby).set(Some(classes));
        enter(ruby, Guest::new(config));
        runtime::define(ruby)?;
        font::install_defaults(ruby)
    }

    /// Runs `access` on the guest, from any Ruby thread: the main script's or a script
    /// thread's.
    ///
    /// Bitmaps and scene elements whose Ruby objects were collected since the last access,
    /// on any thread, are disposed first. `access` must not call into Ruby: a Ruby call can
    /// run script code that calls back into a method that needs the guest, and over Ruby
    /// 1.9.2 it can let another script thread run.
    ///
    /// # Panics
    ///
    /// Panics when called from inside `access` (re-entrant access), and when no guest is
    /// installed.
    pub fn with<R>(ruby: &Ruby, access: impl FnOnce(&mut Guest) -> R) -> R {
        let mut slot = GUEST
            .get(ruby)
            .try_borrow_mut()
            .unwrap_or_else(|_| panic!("re-entrant access to the guest state"));
        let guest = slot.as_mut().expect("a guest is installed");
        guest.dispose_freed();
        access(guest)
    }

    /// The bitmap arena.
    pub fn bitmaps(&self) -> &Bitmaps {
        &self.bitmaps
    }

    /// The scene.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// The input state.
    pub fn input(&self) -> &InputState {
        &self.input
    }

    /// Disposes the arena bitmaps of collected `Bitmap` objects.
    fn dispose_freed(&mut self) {
        let freed = std::mem::take(&mut *FREED.lock().unwrap_or_else(PoisonError::into_inner));
        for id in freed {
            self.bitmaps.dispose(id);
        }
        element::dispose_freed(self);
    }
}

/// Makes `guest` the guest of the process.
fn enter(ruby: &Ruby, guest: Guest) {
    *GUEST.get(ruby).borrow_mut() = Some(guest);
}

/// The installed classes.
///
/// # Panics
///
/// Panics when no guest is installed.
pub(crate) fn classes(ruby: &Ruby) -> Classes {
    CLASSES.get(ruby).get().expect("a guest is installed")
}

/// Queues the arena bitmap of a collected `Bitmap` object for disposal.
///
/// The collector calls the free functions of data objects, and a collection can start while
/// a method holds the guest, on any Ruby thread. So a free function only pushes here, and
/// the queue is emptied the next time the guest is entered, on whichever thread that is.
pub(crate) fn defer_free(id: BitmapId) {
    FREED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(id);
}

#[cfg(test)]
mod tests {
    use std::panic::{self, AssertUnwindSafe};

    use super::*;
    use crate::testing;

    fn harness() -> &'static testing::Harness {
        testing::start_as(Runtime::Rgss1)
    }

    #[test]
    fn reentrant_access_is_detected() {
        harness().run(|ruby| {
            let nested = panic::catch_unwind(AssertUnwindSafe(|| {
                Guest::with(ruby, |_outer| Guest::with(ruby, |_inner| ()))
            }));
            let payload = nested.expect_err("the inner access panics");
            assert_eq!(
                payload.downcast_ref::<&str>().copied(),
                Some("re-entrant access to the guest state")
            );
            // The failed access left the guest usable.
            Guest::with(ruby, |guest| guest.bitmaps().len());
        });
    }

    #[test]
    fn deferred_frees_are_disposed_on_the_next_access() {
        harness().run(|ruby| {
            let (kept, freed, count) = Guest::with(ruby, |guest| {
                let defaults = guest.font_defaults.clone();
                let kept = guest.bitmaps.create(1, 1, &defaults).unwrap();
                let freed = guest.bitmaps.create(1, 1, &defaults).unwrap();
                // A free during an access only queues.
                defer_free(freed);
                assert!(!guest.bitmaps.is_disposed(freed));
                (kept, freed, guest.bitmaps.len())
            });
            Guest::with(ruby, |guest| {
                assert!(guest.bitmaps.is_disposed(freed));
                assert!(!guest.bitmaps.is_disposed(kept));
                assert_eq!(guest.bitmaps.len(), count - 1);
                guest.bitmaps.dispose(kept);
            });
        });
    }

    /// A free that runs on another thread than the one of the next access, as a collection
    /// on a script thread of Ruby 1.9.2 does, is disposed by that access.
    #[test]
    fn a_free_queued_on_another_thread_is_disposed_on_the_next_access() {
        let freed = harness().run(|ruby| {
            Guest::with(ruby, |guest| {
                let defaults = guest.font_defaults.clone();
                guest.bitmaps.create(1, 1, &defaults).unwrap()
            })
        });
        std::thread::spawn(move || defer_free(freed))
            .join()
            .unwrap();
        harness().run(move |ruby| {
            Guest::with(ruby, |guest| assert!(guest.bitmaps.is_disposed(freed)));
        });
    }
}
