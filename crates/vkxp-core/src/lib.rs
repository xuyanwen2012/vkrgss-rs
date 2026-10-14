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
//!   that clamp when set, are all 0 by default as in RGSS3, and convert into each other; [`Rect`] stores four integers as given; [`Table`] is a one- to
//!   three-dimensional array of 16-bit integers; [`Font`] holds the attributes text is drawn
//!   with and [`FontDefaults`] the values new fonts copy. They are plain values without
//!   identity; [`SharedTable`] is the one handle with identity, a reference-counted [`Table`]
//!   with a revision counter, so that a table assigned to a tilemap stays connected to the
//!   script's own. `to_proto` converts to the integer types of `vkxp-proto`, and `to_bytes` /
//!   `from_bytes` give the serialized layouts of the original runtime's data files
//!   ([`DecodeError`] for malformed input).
//! - [`Bitmaps`], the arena that owns every bitmap. A bitmap is named by its protocol
//!   `BitmapId` and holds its pixels in the protocol's layout. The arena creates and disposes
//!   bitmaps and draws into them: pixel access, `clear`, `fill_rect`, `blt`, `stretch_blt`,
//!   `hue_change`. Failures are a [`BitmapError`]: a disposed bitmap, or a creation that
//!   failed for a [`CreateFailure`].
//! - Text. `draw_text` and `text_size` lay text out and blend it; the measurements
//!   ([`TextMetrics`]) and the glyph coverage (a [`Mask`]) come from a [`TextRasterizer`]
//!   that the embedder supplies. [`test_support`] has a deterministic stand-in for tests.
//!   `draw_text_as` draws by the [`TextRules`] of a runtime: those of RGSS3 blend the whole
//!   rectangle given, squeeze in one step ([`squeeze_width_rgss3`]) and draw the outline of
//!   a font that has one.
//! - Deltas. Every drawing operation records the rectangle it may have changed, and
//!   [`Bitmaps::deltas`] turns that into the `BitmapDelta`s (create, update, destroy) a
//!   compositor needs for the bitmaps a frame refers to.
//! - The scene. [`Scene`] owns every [`Sprite`], [`Viewport`], [`Window`], [`Plane`] and
//!   [`Tilemap`] in arenas and names them by handles ([`SpriteId`], [`ViewportId`],
//!   [`WindowId`], [`PlaneId`], [`TilemapId`]) that carry a generation, like bitmap ids. An
//!   element is a property holder with its RGSS1 defaults, reached through the scene
//!   (`scene.sprite(id)`, `scene.sprite_mut(id)`); z is set through the scene
//!   (`scene.set_sprite_z`), because it decides the draw order: ascending z, and among equal z
//!   the element created more recently in front. Using a disposed
//!   element fails with [`SceneError::Disposed`], which names the [`ElementKind`]. A scene
//!   follows the [`Rules`] of the runtime it was made for: with those of RGSS2 the y of a
//!   sprite orders elements of equal z before their creation, a viewport lays its flash
//!   colour over its colour and does nothing while a flash without a colour lasts, and the
//!   tile of a plane is sized in `f64`. The rules of RGSS3 are those of RGSS2 except for a
//!   turned sprite in a viewport, which stands at its position plus the viewport's
//!   rectangle position less the viewport's origin, as it would on the screen.
//!   [`Scene::create_screen_viewport`] makes the viewport `Viewport.new` of RGSS3 makes
//!   without arguments.
//! - Extraction. [`Scene::extract`] is the only place a frame is assembled: it returns the
//!   protocol `FramePacket` with the ops of every visible element in draw order and the
//!   deltas for exactly the bitmaps those ops refer to, and counts the frame. A sprite that is
//!   not turned becomes a stretch quad: its source rectangle stretched into a rectangle of
//!   whole pixels, found in `f32` as the original runtime finds it, and in a scene with the
//!   rules of RGSS2 in `f64` with the bush rows and the wave of that runtime. A sprite with an angle
//!   other than exactly 0 is rendered by the rule measured on the original runtime, in `f64`
//!   with sines and cosines from the pure-Rust `libm`, into a bitmap of the scene's own that
//!   is drawn as an unscaled quad and rendered again only when what it shows changes. A
//!   viewport clips and offsets its
//!   elements and then applies its tone, colour and flash as a rectangle effect. A window is
//!   drawn from bitmaps of the scene's own in the bitmap arena (background, frame, cursor),
//!   redrawn from the window skin only when the skin, the size, the stretch flag or the
//!   cursor size changes, plus its contents, pause sign and scroll arrows. A window of RGSS2
//!   or RGSS3 ([`Scene::create_window_rgss2`], [`Scene::create_window_rgss3`]) is one element
//!   whose bitmaps are built at its calls ([`Window::call`]). A plane is one
//!   stretch quad per repeat of its tile, on a grid of whole pixels, and draws from a
//!   pre-tiled bitmap of the scene's own when its bitmap is small against the area and the
//!   zoom lets copies in a row stretch as one. A tilemap is drawn by the compositor, from
//!   tilemap layer ops that name its sheets and its two tables: for the tiles of priority 0
//!   one op for each layer of the map that shows such a tile, at z 0, and for every higher
//!   priority one op per row of tiles and layer, at the z that ranks the row among sprites,
//!   so that every tile meets the frame by itself. A tilemap of RGSS2
//!   ([`TilemapRgss2`]) is two tilemap plane ops, the lower at z 0 and the upper at z 200,
//!   which name its nine sheets and its map, passages and flash tables. A tilemap of RGSS3
//!   ([`Scene::create_tilemap_rgss3`]) is the same two ops with the rule set of RGSS3, its
//!   flags table in the place of the passages table, and none while it is not visible. The
//!   tables are sent when an op first reads them and again when they were written to. The
//!   scene also holds `Graphics`
//!   bookkeeping: frame count, frame rate and the logical size (640 by 480 unless given, at
//!   most [`MAX_LOGICAL_DIMENSION`] a side).
//! - Input. [`InputState`] takes the protocol's input snapshots as they arrive and latches
//!   the newest one per update, from which it answers pressed, triggered, repeated and the
//!   four- and eight-way direction queries. The same snapshots always give the same answers.
//!   The repeat timing is that of its [`Rules`]: after 16 updates and then every 4 in RGSS1,
//!   after 24 and then every 6 in RGSS2 and RGSS3.
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
//! | `table` | [`Table`], [`SharedTable`] |
//! | `font` | [`Font`], [`FontDefaults`] |
//! | `bitmap` | [`Bitmaps`], [`MAX_BITMAP_PIXELS`], the drawing operations |
//! | `text` | [`TextRasterizer`], [`TextMetrics`], [`Mask`], [`Align`], [`TextRules`], `draw_text`, `draw_text_as`, `text_size` |
//! | `delta` | changed rectangles and [`Bitmaps::deltas`] |
//! | `scene` | [`Scene`], [`Rules`], the element arenas and their handles, creation and disposal |
//! | `element` | [`Sprite`], [`Viewport`], [`Window`], [`Plane`], [`Tilemap`], flashes |
//! | `geometry` | the fixed-point transform of a quad, the rectangle of a stretch, the walk of a turned sprite |
//! | `extract` | [`Scene::extract`], the draw order, sprites and viewports as ops; the sprite of RGSS2 |
//! | `turned` | the bitmap a turned sprite is rendered into |
//! | `window` | the bitmaps cached from a window skin, windows as ops; the windows of RGSS2 and RGSS3 with [`WindowCall`] |
//! | `plane` | planes as tiled quads, the pre-tiled bitmap of a small pattern |
//! | `tilemap` | tilemaps as ops, the tables sent for them, the overlay of flashing cells; the tile numbering is `vkxp_proto::tiles` |
//! | `input` | [`InputState`] |
//! | `error` | [`BitmapError`], [`CreateFailure`], [`DecodeError`], [`TableError`], [`SceneError`], [`ElementKind`] |
//! | `test_support` | stand-ins for tests, such as a rasteriser with fixed-size cells; scenes of windows and of sprites of RGSS2 as text |

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
mod turned;
mod window;

pub use bitmap::{Bitmaps, MAX_BITMAP_PIXELS};
pub use color::{Color, Tone};
pub use element::{Plane, Sprite, Tilemap, TilemapRgss2, Viewport, Window};
pub use error::{BitmapError, CreateFailure, DecodeError, ElementKind, SceneError, TableError};
pub use font::{Font, FontDefaults};
pub use input::InputState;
pub use rect::Rect;
pub use scene::{
    DisposedElements, MAX_LOGICAL_DIMENSION, PlaneId, Rules, Scene, SpriteId, TilemapId,
    ViewportId, WindowId,
};
pub use table::{SharedTable, Table};
pub use text::{
    Align, Mask, SQUEEZE_STEPS, Squeeze, TextMetrics, TextRasterizer, TextRules,
    squeeze_width_rgss3,
};
pub use window::rgss2::WindowCall;
