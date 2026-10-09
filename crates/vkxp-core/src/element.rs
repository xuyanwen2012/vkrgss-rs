//! The elements of a scene and their properties.
//!
//! An element is a plain property holder owned by the [`Scene`](crate::Scene). Properties
//! without a constraint are public fields; the others have a getter and a setter that
//! enforces the constraint. `z` is the exception: it is read here and set through the scene,
//! because a change of z moves the element in the draw order of the whole scene.

use vkxp_proto::BitmapId;

use crate::scene::ViewportId;
use crate::tilemap::TileCache;
use crate::window::{Cache, Caches};
use crate::{Bitmaps, Color, Rect, SharedTable, Tone};

/// An opacity as scripts assign it, clamped to 0 to 255.
fn clamp_opacity(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

/// Where an element stands in the draw order: its z, and the stamp it got when it was created
/// or last given a different z.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Order {
    pub(crate) z: i32,
    pub(crate) stamp: u64,
}

/// A flash of a sprite or a viewport: a colour, or none, for a number of updates.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Flash {
    color: Option<Color>,
    duration: u32,
    elapsed: u32,
}

impl Flash {
    /// Starts a flash, replacing a running one. A duration of 0 or less is no flash.
    fn start(&mut self, color: Option<Color>, duration: i32) {
        *self = Flash {
            color,
            duration: duration.max(0).unsigned_abs(),
            elapsed: 0,
        };
    }

    /// One update of the element.
    fn advance(&mut self) {
        if self.running() {
            self.elapsed += 1;
        }
    }

    fn running(&self) -> bool {
        self.elapsed < self.duration
    }

    /// Whether a flash without a colour is running, which hides the element.
    pub(crate) fn hides(&self) -> bool {
        self.running() && self.color.is_none()
    }

    /// The colour of a running flash with its current alpha: after `k` updates of a flash of
    /// duration `d` and alpha `A` it is `A * (d - k) / d`, rounded down.
    ///
    /// PLACEHOLDER: the linear decay and its rounding are not yet checked against the original
    /// runtime.
    fn color(&self) -> Option<vkxp_proto::Color> {
        if !self.running() {
            return None;
        }
        let mut color = self.color?.to_proto();
        let remaining = u64::from(self.duration - self.elapsed);
        // The quotient is at most the alpha it started from.
        color.a = (u64::from(color.a) * remaining / u64::from(self.duration)) as u8;
        Some(color)
    }

    /// The colour an element with the colour `own` and this flash is drawn with: the flash
    /// colour while its current alpha exceeds the alpha of `own`, otherwise `own`.
    ///
    /// PLACEHOLDER: which of the two colours wins is not yet checked against the original
    /// runtime.
    pub(crate) fn effective_color(&self, own: Color) -> vkxp_proto::Color {
        let own = own.to_proto();
        match self.color() {
            Some(flash) if flash.a > own.a => flash,
            _ => own,
        }
    }
}

/// A sprite: a bitmap, or part of one, placed on the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    /// The point of the source rectangle that lands on (`x`, `y`).
    pub ox: i32,
    pub oy: i32,
    pub zoom_x: f64,
    pub zoom_y: f64,
    /// Rotation about the origin in degrees, counter-clockwise on screen.
    pub angle: f64,
    pub mirror: bool,
    /// Number of rows at the bottom of the source rectangle that are drawn with half the
    /// opacity; 0 or less for none.
    pub bush_depth: i32,
    /// 1 adds, 2 subtracts, every other value blends normally.
    pub blend_type: i32,
    pub color: Color,
    pub tone: Tone,
    /// The part of the bitmap that is drawn. It may reach outside the bitmap; that part draws
    /// nothing.
    pub src_rect: Rect,
    pub(crate) order: Order,
    pub(crate) viewport: Option<ViewportId>,
    pub(crate) bitmap: Option<BitmapId>,
    pub(crate) opacity: u8,
    pub(crate) flash: Flash,
}

impl Sprite {
    pub(crate) fn new(viewport: Option<ViewportId>, stamp: u64) -> Sprite {
        Sprite {
            visible: true,
            x: 0,
            y: 0,
            ox: 0,
            oy: 0,
            zoom_x: 1.0,
            zoom_y: 1.0,
            angle: 0.0,
            mirror: false,
            bush_depth: 0,
            blend_type: 0,
            color: Color::TRANSPARENT,
            tone: Tone::NEUTRAL,
            src_rect: Rect::default(),
            order: Order { z: 0, stamp },
            viewport,
            bitmap: None,
            opacity: 255,
            flash: Flash::default(),
        }
    }

    /// The viewport the sprite was created in.
    pub fn viewport(&self) -> Option<ViewportId> {
        self.viewport
    }

    /// Set with [`Scene::set_sprite_z`](crate::Scene::set_sprite_z).
    pub fn z(&self) -> i32 {
        self.order.z
    }

    pub fn bitmap(&self) -> Option<BitmapId> {
        self.bitmap
    }

    /// Assigns a bitmap, or none. Assigning a live bitmap sets the source rectangle to the
    /// whole bitmap; otherwise the source rectangle stays as it is.
    pub fn set_bitmap(&mut self, bitmap: Option<BitmapId>, bitmaps: &Bitmaps) {
        self.bitmap = bitmap;
        if let Some(Ok(rect)) = bitmap.map(|id| bitmaps.rect(id)) {
            self.src_rect = rect;
        }
    }

    pub fn opacity(&self) -> u8 {
        self.opacity
    }

    /// Sets the opacity, clamped to 0 to 255.
    pub fn set_opacity(&mut self, opacity: i32) {
        self.opacity = clamp_opacity(opacity);
    }

    /// Starts a flash that lasts `duration` updates: the sprite shows `color` with its alpha
    /// falling to 0, or is hidden when `color` is `None`. A duration of 0 or less ends a
    /// running flash.
    pub fn flash(&mut self, color: Option<Color>, duration: i32) {
        self.flash.start(color, duration);
    }

    /// Advances the flash by one step.
    pub fn update(&mut self) {
        self.flash.advance();
    }
}

/// A viewport: a rectangle of the screen that clips, scrolls and recolours its elements.
#[derive(Debug, Clone, PartialEq)]
pub struct Viewport {
    /// The rectangle on the screen. One without pixels shows nothing.
    pub rect: Rect,
    pub visible: bool,
    /// The point of the viewport's content that lands on the rectangle's top-left corner.
    pub ox: i32,
    pub oy: i32,
    pub color: Color,
    pub tone: Tone,
    pub(crate) order: Order,
    pub(crate) flash: Flash,
}

impl Viewport {
    pub(crate) fn new(rect: Rect, stamp: u64) -> Viewport {
        Viewport {
            rect,
            visible: true,
            ox: 0,
            oy: 0,
            color: Color::TRANSPARENT,
            tone: Tone::NEUTRAL,
            order: Order { z: 0, stamp },
            flash: Flash::default(),
        }
    }

    /// Set with [`Scene::set_viewport_z`](crate::Scene::set_viewport_z).
    pub fn z(&self) -> i32 {
        self.order.z
    }

    /// Starts a flash that lasts `duration` updates, with the rules of [`Sprite::flash`]:
    /// `color` is laid over the viewport's rectangle, and `None` hides its elements.
    pub fn flash(&mut self, color: Option<Color>, duration: i32) {
        self.flash.start(color, duration);
    }

    /// Advances the flash by one step.
    pub fn update(&mut self) {
        self.flash.advance();
    }
}

/// A window: a frame and background from a window skin, and a contents bitmap inside.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub windowskin: Option<BitmapId>,
    pub contents: Option<BitmapId>,
    /// Whether the background pattern is stretched (otherwise tiled).
    pub stretch: bool,
    /// The cursor, relative to the contents area. One without pixels shows no cursor.
    pub cursor_rect: Rect,
    /// Whether the cursor blinks.
    pub active: bool,
    pub visible: bool,
    /// Whether the pause sign is shown.
    pub pause: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// The point of the contents that lands on the top-left corner of the contents area.
    pub ox: i32,
    pub oy: i32,
    pub(crate) order: Order,
    pub(crate) viewport: Option<ViewportId>,
    pub(crate) opacity: u8,
    pub(crate) back_opacity: u8,
    pub(crate) contents_opacity: u8,
    /// Position in the 32 steps of the cursor blink.
    pub(crate) cursor_step: u8,
    /// Number of updates since the pause sign appeared.
    pub(crate) pause_step: u32,
    /// The bitmaps drawn from the window skin.
    pub(crate) caches: Caches,
}

impl Window {
    pub(crate) fn new(viewport: Option<ViewportId>, stamp: u64) -> Window {
        Window {
            windowskin: None,
            contents: None,
            stretch: true,
            cursor_rect: Rect::default(),
            active: true,
            visible: true,
            pause: false,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            ox: 0,
            oy: 0,
            order: Order { z: 0, stamp },
            viewport,
            opacity: 255,
            back_opacity: 255,
            contents_opacity: 255,
            cursor_step: 0,
            pause_step: 0,
            caches: Caches::default(),
        }
    }

    /// The viewport the window was created in.
    pub fn viewport(&self) -> Option<ViewportId> {
        self.viewport
    }

    /// Set with [`Scene::set_window_z`](crate::Scene::set_window_z).
    pub fn z(&self) -> i32 {
        self.order.z
    }

    /// The opacity of frame and background.
    pub fn opacity(&self) -> u8 {
        self.opacity
    }

    /// Sets the opacity, clamped to 0 to 255.
    pub fn set_opacity(&mut self, opacity: i32) {
        self.opacity = clamp_opacity(opacity);
    }

    /// The opacity of the background alone, on top of [`Window::opacity`].
    pub fn back_opacity(&self) -> u8 {
        self.back_opacity
    }

    /// Sets the background opacity, clamped to 0 to 255.
    pub fn set_back_opacity(&mut self, opacity: i32) {
        self.back_opacity = clamp_opacity(opacity);
    }

    pub fn contents_opacity(&self) -> u8 {
        self.contents_opacity
    }

    /// Sets the contents opacity, clamped to 0 to 255.
    pub fn set_contents_opacity(&mut self, opacity: i32) {
        self.contents_opacity = clamp_opacity(opacity);
    }

    /// Advances the cursor blink while the window is active, and the pause sign while the
    /// pause flag is set.
    ///
    /// PLACEHOLDER: that an inactive window keeps the cursor at its current alpha, and the
    /// pause sign's speed of 16 updates per image, are not yet checked against the original
    /// runtime.
    pub fn update(&mut self) {
        if self.active {
            self.cursor_step = (self.cursor_step + 1) % 32;
        }
        // Wrapping keeps the animation seamless: 2^32 is a multiple of its period.
        self.pause_step = if self.pause {
            self.pause_step.wrapping_add(1)
        } else {
            0
        };
    }

    /// The alpha of the cursor: from 255 it falls by 8 per step for 16 steps, then rises by
    /// 8 per step back to 255.
    ///
    /// PLACEHOLDER: the blink values are not yet checked against the original runtime.
    pub(crate) fn cursor_alpha(&self) -> u8 {
        match self.cursor_step {
            step @ 0..=16 => 255 - 8 * step,
            step => 127 + 8 * (step - 16),
        }
    }
}

/// A plane: a bitmap repeated without end over its viewport's rectangle, or over the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Plane {
    pub bitmap: Option<BitmapId>,
    pub visible: bool,
    /// The bitmap pixel at the top-left corner of the area.
    pub ox: i32,
    pub oy: i32,
    /// Scales the repeated image about the top-left corner of the area.
    pub zoom_x: f64,
    pub zoom_y: f64,
    /// 1 adds, 2 subtracts, every other value blends normally.
    pub blend_type: i32,
    pub color: Color,
    pub tone: Tone,
    pub(crate) order: Order,
    pub(crate) viewport: Option<ViewportId>,
    pub(crate) opacity: u8,
    /// The bitmap repeated a few times, for when it is small against the area.
    pub(crate) pretiled: Cache,
}

impl Plane {
    pub(crate) fn new(viewport: Option<ViewportId>, stamp: u64) -> Plane {
        Plane {
            bitmap: None,
            visible: true,
            ox: 0,
            oy: 0,
            zoom_x: 1.0,
            zoom_y: 1.0,
            blend_type: 0,
            color: Color::TRANSPARENT,
            tone: Tone::NEUTRAL,
            order: Order { z: 0, stamp },
            viewport,
            opacity: 255,
            pretiled: Cache::default(),
        }
    }

    /// The viewport the plane was created in.
    pub fn viewport(&self) -> Option<ViewportId> {
        self.viewport
    }

    /// Set with [`Scene::set_plane_z`](crate::Scene::set_plane_z).
    pub fn z(&self) -> i32 {
        self.order.z
    }

    pub fn opacity(&self) -> u8 {
        self.opacity
    }

    /// Sets the opacity, clamped to 0 to 255.
    pub fn set_opacity(&mut self, opacity: i32) {
        self.opacity = clamp_opacity(opacity);
    }
}

/// A tilemap: a tileset, autotiles and the tables that say which tile goes where, drawn as
/// three layers of 32 by 32 tiles over its viewport's rectangle, or over the screen.
///
/// It has no z of its own: tiles of priority 0 are drawn at z 0 and the others at a z that
/// follows from their row, all with the stamp the tilemap got when it was created.
#[derive(Debug, Clone, PartialEq)]
pub struct Tilemap {
    pub tileset: Option<BitmapId>,
    pub autotiles: [Option<BitmapId>; 7],
    /// Which tile each cell of each of the three layers holds. The table is shared: the
    /// tilemap shows what it holds when a frame is extracted.
    pub map_data: Option<SharedTable>,
    /// Stored only: flash data is not drawn.
    pub flash_data: Option<SharedTable>,
    /// The priority of every tile identifier, shared like the map data.
    pub priorities: Option<SharedTable>,
    pub visible: bool,
    /// The pixel of the map at the top-left corner of the area.
    pub ox: i32,
    pub oy: i32,
    pub(crate) viewport: Option<ViewportId>,
    /// The stamp of every piece of the tilemap; `z` is not used.
    pub(crate) order: Order,
    /// Number of updates so far, which the autotile animation follows.
    pub(crate) updates: u64,
    /// The bitmaps the visible tiles are composed into.
    pub(crate) cache: TileCache,
}

impl Tilemap {
    pub(crate) fn new(viewport: Option<ViewportId>, stamp: u64) -> Tilemap {
        Tilemap {
            tileset: None,
            autotiles: [None; 7],
            map_data: None,
            flash_data: None,
            priorities: None,
            visible: true,
            ox: 0,
            oy: 0,
            viewport,
            order: Order { z: 0, stamp },
            updates: 0,
            cache: TileCache::default(),
        }
    }

    /// The viewport the tilemap was created in.
    pub fn viewport(&self) -> Option<ViewportId> {
        self.viewport
    }

    /// One step of the autotile animation: every 16 updates each animated autotile shows its
    /// next frame.
    ///
    /// PLACEHOLDER: the speed of 16 updates per frame is not yet checked against the original
    /// runtime.
    pub fn update(&mut self) {
        // The count does not wrap within the lifetime of a process.
        self.updates += 1;
    }
}
