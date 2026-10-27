//! Test access to the guest's scene, next to the harness of [`testing`](crate::testing).
//!
//! The functions here run on a thread that holds the interpreter: inside the closure given to
//! [`Harness::run`](crate::testing::Harness::run), or in a host function.

use std::sync::atomic::{AtomicIsize, Ordering};

use vkrgss_core::{Bitmaps, ElementKind, Scene};
use vkrgss_ruby_sys::Ruby;

use crate::Guest;

/// The number of live scene elements of each kind that the element classes created, on any
/// Ruby thread.
static LIVE: [AtomicIsize; 5] = [const { AtomicIsize::new(0) }; 5];

fn slot(kind: ElementKind) -> usize {
    match kind {
        ElementKind::Sprite => 0,
        ElementKind::Viewport => 1,
        ElementKind::Window => 2,
        ElementKind::Plane => 3,
        ElementKind::Tilemap => 4,
    }
}

/// Records that the element classes created (`change` 1) or disposed (-1) an element.
pub(crate) fn count(kind: ElementKind, change: isize) {
    LIVE[slot(kind)].fetch_add(change, Ordering::Relaxed);
}

/// The number of live scene elements of `kind` that Ruby objects created. The elements of
/// objects the collector has freed are disposed first.
pub fn scene_count(ruby: &Ruby, kind: ElementKind) -> usize {
    Guest::with(ruby, |_guest| {
        LIVE[slot(kind)]
            .load(Ordering::Relaxed)
            .max(0)
            .unsigned_abs()
    })
}

/// Runs `work` on the guest's scene and bitmap arena, for tests that build or change a
/// scene from Rust. `work` must not call into Ruby.
pub fn with_scene<R>(ruby: &Ruby, work: impl FnOnce(&mut Scene, &mut Bitmaps) -> R) -> R {
    Guest::with(ruby, |guest| work(&mut guest.scene, &mut guest.bitmaps))
}
