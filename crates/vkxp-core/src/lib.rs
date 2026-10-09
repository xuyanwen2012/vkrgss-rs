//! Value types, `Bitmap` pixels and operations, scene objects, frame extraction.
//!
//! This crate does no file I/O and depends on neither Ruby nor Vulkan.
//!
//! Side: guest.
//!
//! What is here is what the Ruby binding wraps: values, bitmaps, the scene and the input
//! state.
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
//! - The scene. [`Scene`] owns every [`Sprite`], [`Viewport`], [`Window`], [`Plane`] and
//!   [`Tilemap`] in arenas and names them by handles ([`SpriteId`], [`ViewportId`],
//!   [`WindowId`], [`PlaneId`], [`TilemapId`]) that carry a generation, like bitmap ids. An
//!   element is a property holder with its RGSS1 defaults, reached through the scene
//!   (`scene.sprite(id)`, `scene.sprite_mut(id)`); z is set through the scene
//!   (`scene.set_sprite_z`), because it decides the draw order: ascending z, and among equal z
//!   the element created or last given a different z more recently in front. Using a disposed
//!   element fails with [`SceneError::Disposed`], which names the [`ElementKind`]. Planes and
//!   tilemaps hold their properties and draw nothing yet.
//! - Extraction. [`Scene::extract`] is the only place a frame is assembled: it returns the
//!   protocol `FramePacket` with the ops of every visible element in draw order and the
//!   deltas for exactly the bitmaps those ops refer to, and counts the frame. A sprite becomes
//!   one quad; its position, origin, zoom, angle and mirror are resolved once, in `f64` with
//!   sines and cosines from the pure-Rust `libm`, and rounded to the protocol's fixed-point
//!   transform, exactly for multiples of 90 degrees. A viewport clips and offsets its
//!   elements and then applies its tone, colour and flash as a rectangle effect. A window is
//!   drawn from bitmaps of the scene's own in the bitmap arena (background, frame, cursor),
//!   redrawn from the window skin only when the skin, the size, the stretch flag or the
//!   cursor size changes, plus its contents, pause sign and scroll arrows. The scene also
//!   holds `Graphics` bookkeeping: frame count, frame rate and the logical size
//!   (640 by 480 unless given, at most [`MAX_LOGICAL_DIMENSION`] a side).
//! - Input. [`InputState`] takes the protocol's input snapshots as they arrive and latches
//!   the newest one per update, from which it answers pressed, triggered, repeated and the
//!   four- and eight-way direction queries. The same snapshots always give the same answers.
//!
//! Nothing here talks to the host: sending packets, waiting for frame ticks and receiving
//! snapshots is the binding's part.
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
//! | `scene` | [`Scene`], the element arenas and their handles, creation and disposal |
//! | `element` | [`Sprite`], [`Viewport`], [`Window`], [`Plane`], [`Tilemap`], flashes |
//! | `geometry` | the fixed-point transform of a quad |
//! | `extract` | [`Scene::extract`], the draw order, sprites and viewports as ops |
//! | `window` | the bitmaps cached from a window skin, windows as ops |
//! | `input` | [`InputState`] |
//! | `error` | [`BitmapError`], [`CreateFailure`], [`DecodeError`], [`TableError`], [`SceneError`], [`ElementKind`] |
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
mod input;
mod plane;
mod rect;
mod scene;
mod table;
pub mod test_support;
mod text;
mod tilemap;
mod tiles;
mod window;

pub use bitmap::{Bitmaps, MAX_BITMAP_PIXELS};
pub use color::{Color, Tone};
pub use element::{Plane, Sprite, Tilemap, Viewport, Window};
pub use error::{BitmapError, CreateFailure, DecodeError, ElementKind, SceneError, TableError};
pub use font::{Font, FontDefaults};
pub use input::InputState;
pub use rect::Rect;
pub use scene::{MAX_LOGICAL_DIMENSION, PlaneId, Scene, SpriteId, TilemapId, ViewportId, WindowId};
pub use table::{SharedTable, Table};
pub use text::{Align, Mask, TextRasterizer};
