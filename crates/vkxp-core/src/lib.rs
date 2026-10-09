//! Value types, `Bitmap` pixels and operations, scene objects, frame extraction.
//!
//! This crate does no file I/O and depends on neither Ruby nor Vulkan.
//!
//! Side: guest.
//!
//! What is here today is what the Ruby binding wraps and the scene objects build on:
//!
//! - Value types with RGSS1 semantics. [`Color`] and [`Tone`] hold floating-point components
//!   that clamp when set; [`Rect`] stores four integers as given; [`Table`] is a one- to
//!   three-dimensional array of 16-bit integers; [`Font`] holds the attributes text is drawn
//!   with and [`FontDefaults`] the values new fonts copy. They are plain values without
//!   identity. `to_proto` converts to the integer types of `vkxp-proto`, and `to_bytes` /
//!   `from_bytes` give the serialized layouts of the original runtime's data files
//!   ([`DecodeError`] for malformed input).
//! - [`Bitmaps`], the arena that owns every bitmap. A bitmap is named by its protocol
//!   `BitmapId` and holds its pixels in the protocol's layout. The arena creates and disposes
//!   bitmaps and draws into them: pixel access, `clear`, `fill_rect`, `blt`, `stretch_blt`,
//!   `hue_change`. Failures are a [`BitmapError`]: a disposed bitmap, or a creation that
//!   failed for a [`CreateFailure`].
//! - Text. `draw_text` and `text_size` lay text out and blend it; the glyph coverage comes
//!   from a [`TextRasterizer`] that the embedder supplies as a [`Mask`]. [`test_support`] has
//!   a deterministic stand-in for tests.
//! - Deltas. Every drawing operation records the rectangle it may have changed, and
//!   [`Bitmaps::deltas`] turns that into the `BitmapDelta`s (create, update, destroy) a
//!   compositor needs for the bitmaps a frame refers to.
//!
//! Pixel arithmetic is not defined here: blending, hue rotation and stretch sampling are the
//! functions of `vkxp-pixel`. Behaviour that is a placeholder until it is checked against the
//! original runtime is marked `PLACEHOLDER` where it is implemented.
//!
//! Modules, all private except [`test_support`]; their items are re-exported from the crate
//! root:
//!
//! | Module | Contents |
//! |---|---|
//! | `color` | [`Color`], [`Tone`] |
//! | `rect` | [`Rect`] |
//! | `table` | [`Table`] |
//! | `font` | [`Font`], [`FontDefaults`] |
//! | `bitmap` | [`Bitmaps`], [`MAX_BITMAP_PIXELS`], the drawing operations |
//! | `text` | [`TextRasterizer`], [`Mask`], [`Align`], `draw_text`, `text_size` |
//! | `delta` | changed rectangles and [`Bitmaps::deltas`] |
//! | `error` | [`BitmapError`], [`CreateFailure`], [`DecodeError`], [`TableError`] |
//! | `test_support` | stand-ins for tests, such as a rasteriser with fixed-size cells |

#![forbid(unsafe_code)]

mod bitmap;
mod color;
mod delta;
mod element;
mod error;
mod extract;
mod font;
mod geometry;
mod rect;
mod scene;
mod table;
pub mod test_support;
mod text;
mod window;

pub use bitmap::{Bitmaps, MAX_BITMAP_PIXELS};
pub use color::{Color, Tone};
pub use element::{Plane, Sprite, Tilemap, Viewport, Window};
pub use error::{BitmapError, CreateFailure, DecodeError, ElementKind, SceneError, TableError};
pub use font::{Font, FontDefaults};
pub use rect::Rect;
pub use scene::{MAX_LOGICAL_DIMENSION, PlaneId, Scene, SpriteId, TilemapId, ViewportId, WindowId};
pub use table::Table;
pub use text::{Align, Mask, TextRasterizer};
