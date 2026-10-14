//! CPU reference compositor.
//!
//! Side: host and tests.
//!
//! Turns guest-to-host messages into frames with plain Rust and the arithmetic of `vkxp-pixel`.
//! It defines the output every other backend must reproduce byte for byte, so it favours being
//! obviously right over being fast: each output pixel is computed on its own by walking the op
//! list in order, the same structure the compute shader of the Vulkan backend has.
//!
//! - [`BitmapStore`] holds the bitmaps announced by `BitmapDelta`s and detects stale ids.
//! - [`TableStore`] holds the tables announced by `TableDelta`s.
//! - [`Compositor`] owns both stores and renders `FramePacket`s into [`Frame`]s.
//! - [`snapshot()`] gives the pixels of `Graphics.snap_to_bitmap` of a packet that stands alone,
//!   by a [`SnapshotRule`].
//! - [`encode_png`] writes a frame as PNG, for the headless example and for tests.
//! - [`fixtures`] generates the message streams whose expected frames are committed.

#![forbid(unsafe_code)]

pub mod fixtures;

mod compose;
mod error;
mod png_out;
mod snapshot;
mod store;
mod tables;
mod tilemap;
mod tilemap_rgss2;

pub use compose::{BACKGROUND, Compositor, Frame, render_stream};
pub use error::{BitmapError, ComposeError, SlotState, TableError};
pub use png_out::encode_png;
pub use snapshot::{SnapshotRule, snapshot};
pub use store::{Bitmap, BitmapStore, MAX_BITMAP_PIXELS};
pub use tables::{Table, TableStore};
