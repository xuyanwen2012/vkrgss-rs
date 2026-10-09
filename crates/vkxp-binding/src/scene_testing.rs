//! Test access to the guest's scene, next to the harness of [`testing`](crate::testing).
//!
//! The functions here run on the interpreter thread, inside the closure given to
//! [`testing::run`](crate::testing::run).

use std::cell::Cell;

use vkxp_core::{Bitmaps, ElementKind, Scene};

use crate::Guest;

thread_local! {
    /// The number of live scene elements of each kind that the element classes created.
    static LIVE: Cell<[isize; 5]> = const { Cell::new([0; 5]) };
}

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
    let mut live = LIVE.get();
    live[slot(kind)] += change;
    LIVE.set(live);
}

/// The number of live scene elements of `kind` that Ruby objects created. The elements of
/// objects the collector has freed are disposed first.
pub fn scene_count(kind: ElementKind) -> usize {
    Guest::with(|_guest| LIVE.get()[slot(kind)].max(0).unsigned_abs())
}

/// Runs `work` on the guest's scene and bitmap arena, for tests that build or change a
/// scene from Rust. `work` must not call into Ruby.
pub fn with_scene<R>(work: impl FnOnce(&mut Scene, &mut Bitmaps) -> R) -> R {
    Guest::with(|guest| work(&mut guest.scene, &mut guest.bitmaps))
}
