//! Value types, `Bitmap` pixels and operations, scene objects, frame extraction.
//!
//! This crate does no file I/O and depends on neither Ruby nor Vulkan.
//!
//! Side: guest.

#![forbid(unsafe_code)]

mod bitmap;
mod color;
mod delta;
mod error;
mod font;
mod rect;
mod table;
pub mod test_support;
mod text;

pub use bitmap::{Bitmaps, MAX_BITMAP_PIXELS};
pub use color::{Color, Tone};
pub use error::{BitmapError, CreateFailure, DecodeError, TableError};
pub use font::{Font, FontDefaults};
pub use rect::Rect;
pub use table::Table;
pub use text::{Align, Mask, TextRasterizer};
