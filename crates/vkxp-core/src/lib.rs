//! Value types, `Bitmap` pixels and operations, scene objects, frame extraction.
//!
//! This crate does no file I/O and depends on neither Ruby nor Vulkan.
//!
//! Side: guest.

#![forbid(unsafe_code)]

mod color;
mod error;
mod font;
mod rect;
mod table;

pub use color::{Color, Tone};
pub use error::{DecodeError, TableError};
pub use font::{Font, FontDefaults};
pub use rect::Rect;
pub use table::Table;
