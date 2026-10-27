//! A global allocator that counts, per thread, the bytes allocated and held, for the tests and
//! fuzz targets that bound the memory of a decode. It hands every call to the system
//! allocator; implementing `GlobalAlloc` is the only `unsafe` it has, and it is test code.
//!
//! A test binary installs it with
//!
//! ```ignore
//! #[global_allocator]
//! static ALLOCATOR: counting::Counting = counting::Counting;
//! ```
//!
//! and measures a closure with [`measure`]. Counting per thread keeps the tests of one binary,
//! which run in parallel, out of each other's numbers.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// The allocator.
pub(crate) struct Counting;

thread_local! {
    /// The bytes this thread holds, relative to an arbitrary start: memory freed here that
    /// another thread allocated makes it smaller.
    static HELD: Cell<isize> = const { Cell::new(0) };
    /// The largest value of `HELD` since the last [`measure`] began.
    static PEAK: Cell<isize> = const { Cell::new(0) };
    /// The bytes this thread allocated, freed or not.
    static TOTAL: Cell<usize> = const { Cell::new(0) };
}

fn count(change: isize) {
    // A thread that is being torn down has no counters any more; its calls are not counted.
    let _ = HELD.try_with(|held| {
        let now = held.get() + change;
        held.set(now);
        let _ = PEAK.try_with(|peak| peak.set(peak.get().max(now)));
    });
    if change > 0 {
        let _ = TOTAL.try_with(|total| total.set(total.get() + change.unsigned_abs()));
    }
}

// SAFETY: every call is passed to `System` unchanged; the counters do not allocate.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller keeps the contract of `GlobalAlloc::alloc`, which is that of `System`.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            count(layout.size() as isize);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as for `alloc`.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            count(layout.size() as isize);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the caller passes memory allocated by this allocator, so by `System`.
        unsafe { System.dealloc(pointer, layout) };
        count(-(layout.size() as isize));
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: as for `dealloc`, with the contract of `GlobalAlloc::realloc`.
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        if !moved.is_null() {
            count(new_size as isize - layout.size() as isize);
        }
        moved
    }
}

/// What a closure allocated on its thread.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Usage {
    /// The most it held at once beyond what the thread held before.
    pub(crate) peak: usize,
    /// All it allocated, freed or not.
    pub(crate) total: usize,
}

/// Runs `work` and says what it allocated on this thread.
pub(crate) fn measure<R>(work: impl FnOnce() -> R) -> (R, Usage) {
    let start = HELD.with(Cell::get);
    PEAK.with(|peak| peak.set(start));
    let total = TOTAL.with(Cell::get);
    let result = work();
    let usage = Usage {
        peak: (PEAK.with(Cell::get) - start).max(0) as usize,
        total: TOTAL.with(Cell::get) - total,
    };
    (result, usage)
}
