//! The guest state: what the Ruby classes operate on.

use std::cell::{Cell, RefCell};

use vkxp_core::{Bitmaps, FontDefaults, InputState, Scene};
use vkxp_proto::BitmapId;
use vkxp_ruby_sys::ffi::VALUE;
use vkxp_ruby_sys::{Raised, Ruby};
use vkxp_vfs::GameFiles;

use crate::element::{self, SceneClasses};
use crate::error::{self, ErrorClasses};
use crate::runtime::{self, RunConfig};
use crate::text::TextBackend;
use crate::{bitmap, color, font, rect, table, win32api};

/// What a guest is created from.
pub struct Config {
    /// The game's files: its directory and its run-time package directories.
    pub files: GameFiles,
    /// The family names `Font.default_name` starts with, tried in order.
    pub font_names: Vec<String>,
    /// Where glyphs and the knowledge of installed fonts come from.
    pub text: Box<dyn TextBackend + Send>,
    /// What running a game needs: its configuration file, the built-in scripts and the modes.
    pub run: RunConfig,
}

/// The state the Ruby classes operate on. One exists per interpreter, on its thread.
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

thread_local! {
    static GUEST: RefCell<Option<Guest>> = const { RefCell::new(None) };
    /// Arena bitmaps whose Ruby objects were collected, waiting to be disposed.
    static FREED: RefCell<Vec<BitmapId>> = const { RefCell::new(Vec::new()) };
    static CLASSES: Cell<Option<Classes>> = const { Cell::new(None) };
}

impl Guest {
    fn new(config: Config) -> Guest {
        let font_names_listed = config.font_names.len() != 1;
        Guest {
            bitmaps: Bitmaps::new(),
            font_defaults: FontDefaults::new(config.font_names),
            font_names_listed,
            files: config.files,
            text: config.text,
            scene: Scene::new(),
            input: InputState::new(),
            runtime: runtime::State::new(config.run),
            win32api: win32api::State::default(),
        }
    }

    /// Defines the RGSS classes in the interpreter and makes a guest of `config` the state
    /// they operate on. Called once, on the interpreter thread, before any script runs.
    ///
    /// The classes are reachable from Ruby's constants, which is what keeps them alive.
    ///
    /// # Panics
    ///
    /// Panics when a guest is already installed on this thread.
    pub fn install(ruby: &Ruby, config: Config) -> Result<(), Raised> {
        assert!(
            CLASSES.get().is_none(),
            "a guest is already installed on this thread"
        );
        let classes = Classes {
            errors: error::define(ruby)?,
            color: color::define_color(ruby)?,
            tone: color::define_tone(ruby)?,
            rect: rect::define(ruby)?,
            table: table::define(ruby)?,
            font: font::define(ruby)?,
            bitmap: bitmap::define(ruby)?,
            scene: element::define(ruby)?,
        };
        CLASSES.set(Some(classes));
        enter(Guest::new(config));
        runtime::define(ruby)?;
        font::install_default_color(ruby)
    }

    /// Runs `access` on the guest of this thread.
    ///
    /// Bitmaps whose Ruby objects were collected since the last access are disposed first.
    /// `access` must not call into Ruby: a Ruby call can run script code that calls back
    /// into a method that needs the guest.
    ///
    /// # Panics
    ///
    /// Panics when called from inside `access` (re-entrant access), and when no guest is
    /// installed on this thread.
    pub fn with<R>(access: impl FnOnce(&mut Guest) -> R) -> R {
        GUEST.with(|cell| {
            let mut slot = cell
                .try_borrow_mut()
                .unwrap_or_else(|_| panic!("re-entrant access to the guest state"));
            let guest = slot
                .as_mut()
                .expect("a guest is installed on the interpreter thread");
            guest.dispose_freed();
            access(guest)
        })
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
        let freed = FREED.with(|queue| std::mem::take(&mut *queue.borrow_mut()));
        for id in freed {
            self.bitmaps.dispose(id);
        }
        element::dispose_freed(self);
    }
}

/// Makes `guest` the guest of this thread.
fn enter(guest: Guest) {
    GUEST.with(|cell| *cell.borrow_mut() = Some(guest));
}

/// The installed classes.
///
/// # Panics
///
/// Panics when no guest is installed on this thread.
pub(crate) fn classes() -> Classes {
    CLASSES
        .get()
        .expect("a guest is installed on the interpreter thread")
}

/// Queues the arena bitmap of a collected `Bitmap` object for disposal.
///
/// The collector calls the free functions of data objects, and a collection can start while
/// a method holds the guest. So a free function only pushes here, and the queue is emptied
/// the next time the guest is entered.
pub(crate) fn defer_free(id: BitmapId) {
    // After the thread's locals are gone there is no arena left to free from.
    let _ = FREED.try_with(|queue| queue.borrow_mut().push(id));
}

#[cfg(test)]
mod tests {
    use std::panic::{self, AssertUnwindSafe};

    use vkxp_core::test_support::CellRasterizer;
    use vkxp_core::{Font, Mask, TextMetrics, TextRasterizer};

    use super::*;

    struct NoFonts;

    impl TextRasterizer for NoFonts {
        fn metrics(&self, font: &Font, text: &str) -> TextMetrics {
            CellRasterizer.metrics(font, text)
        }

        fn rasterize(
            &self,
            font: &Font,
            text: &str,
            advances: &[u32],
            condense: Option<(u32, u32)>,
        ) -> Mask {
            CellRasterizer.rasterize(font, text, advances, condense)
        }
    }

    impl TextBackend for NoFonts {
        fn font_exists(&self, _name: &str) -> bool {
            false
        }
    }

    /// A guest on the calling thread, without an interpreter.
    fn enter_plain_guest() {
        enter(Guest::new(Config {
            files: GameFiles::new("/nonexistent", []),
            font_names: vec!["Test".to_owned()],
            text: Box::new(NoFonts),
            run: RunConfig::default(),
        }));
    }

    #[test]
    fn reentrant_access_is_detected() {
        enter_plain_guest();
        let nested = panic::catch_unwind(AssertUnwindSafe(|| {
            Guest::with(|_outer| Guest::with(|_inner| ()))
        }));
        let payload = nested.expect_err("the inner access panics");
        assert_eq!(
            payload.downcast_ref::<&str>().copied(),
            Some("re-entrant access to the guest state")
        );
        // The failed access left the guest usable.
        assert_eq!(Guest::with(|guest| guest.bitmaps().len()), 0);
    }

    #[test]
    fn deferred_frees_are_disposed_on_the_next_access() {
        enter_plain_guest();
        let (kept, freed) = Guest::with(|guest| {
            let defaults = guest.font_defaults.clone();
            let kept = guest.bitmaps.create(1, 1, &defaults).unwrap();
            let freed = guest.bitmaps.create(1, 1, &defaults).unwrap();
            // A free during an access only queues.
            defer_free(freed);
            assert!(!guest.bitmaps.is_disposed(freed));
            (kept, freed)
        });
        Guest::with(|guest| {
            assert!(guest.bitmaps.is_disposed(freed));
            assert!(!guest.bitmaps.is_disposed(kept));
            assert_eq!(guest.bitmaps.len(), 1);
        });
    }
}
