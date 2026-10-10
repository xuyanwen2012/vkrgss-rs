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
//! - [`encode_png`] writes a frame as PNG, for the headless example and for tests.
//! - [`fixtures`] generates the message streams whose expected frames are committed.

#![forbid(unsafe_code)]

pub mod fixtures;

mod compose;
mod error;
mod png_out;
mod store;
mod tables;
mod tilemap;

pub use compose::{BACKGROUND, Compositor, Frame, MAX_LOGICAL_DIMENSION, render_stream};
pub use error::{BitmapError, ComposeError, SlotState, TableError};
pub use png_out::encode_png;
pub use store::{Bitmap, BitmapStore, MAX_BITMAP_PIXELS};
pub use tables::{Table, TableStore};
pub use tilemap::AUTOTILE_QUARTERS;
