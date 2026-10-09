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
    /// Whether the flash shows: from its start until the update that ends it.
    shown: bool,
    color: Option<Color>,
    /// The alpha the colour is laid on with now.
    alpha: u8,
    /// The updates the flash still lasts.
    remaining: u32,
}

impl Flash {
    /// Starts a flash, replacing a running one. It shows until `duration` updates have
    /// passed, and one of duration 0 or less until the first update.
    fn start(&mut self, color: Option<Color>, duration: i32) {
        *self = Flash {
            shown: true,
            color,
            alpha: color.map_or(0, |color| color.to_proto().a),
            remaining: duration.max(0).unsigned_abs(),
        };
    }

    /// One update of the element. As measured on the original runtime, with `r` updates
    /// remaining the alpha becomes `alpha * (r - 1) / r`, rounded down, and the update that
    /// leaves none ends the flash.
    fn advance(&mut self) {
        if !self.shown {
            return;
        }
        if self.remaining <= 1 {
            *self = Flash::default();
        } else {
            // In 64 bits: the product of an alpha and any duration fits. The quotient is at
            // most the alpha it was.
            let (alpha, remaining) = (u64::from(self.alpha), u64::from(self.remaining));
            self.alpha = (alpha * (remaining - 1) / remaining) as u8;
            self.remaining -= 1;
        }
    }

    /// Whether a flash without a colour is running, which hides the element.
    pub(crate) fn hides(&self) -> bool {
        self.shown && self.color.is_none()
    }

    /// The colour of a running flash with its current alpha.
    fn color(&self) -> Option<vkxp_proto::Color> {
        if !self.shown {
            return None;
        }
        let mut color = self.color?.to_proto();
        color.a = self.alpha;
        Some(color)
    }

    /// The colour an element with the colour `own` and this flash is drawn with: the flash
    /// colour while its current alpha is at least the alpha of `own`, otherwise `own`.
    pub(crate) fn effective_color(&self, own: Color) -> vkxp_proto::Color {
        let own = own.to_proto();
        match self.color() {
            Some(flash) if flash.a >= own.a => flash,
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
    /// falling to 0, or is hidden when `color` is `None`. A duration of 0 or less lasts until
    /// the next update.
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
    /// Whether the cursor blinks. Assign it with [`Window::set_active`]; an assignment to the
    /// field is noticed at the next update or frame.
    pub active: bool,
    pub visible: bool,
    /// Whether the pause sign is shown. Assign it with [`Window::set_pause`]; an assignment
    /// to the field is noticed at the next update or frame.
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
    /// Number of updates since the pause flag was assigned.
    pub(crate) pause_step: u32,
    /// The values of `active` and `pause` that the two counters have followed.
    counted: (bool, bool),
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
            counted: (true, false),
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

    /// Makes the window active or inactive. Making it inactive puts the cursor blink at its
    /// twelfth step at once, where it stays until the window is active and updated again.
    ///
    /// PLACEHOLDER: that `true` for an active window leaves the blink alone is not measured.
    pub fn set_active(&mut self, active: bool) {
        self.follow_flags();
        self.active = active;
        self.follow_flags();
    }

    /// Shows or hides the pause sign. A change starts its animation over.
    ///
    /// PLACEHOLDER: that `true` for a pausing window leaves the animation alone is not
    /// measured.
    pub fn set_pause(&mut self, pause: bool) {
        self.follow_flags();
        self.pause = pause;
        self.follow_flags();
    }

    /// Lets the counters follow `active` and `pause` when their fields were assigned.
    pub(crate) fn follow_flags(&mut self) {
        let (active, pause) = self.counted;
        if self.active != active && !self.active {
            self.cursor_step = 12;
        }
        if self.pause != pause {
            self.pause_step = 0;
        }
        self.counted = (self.active, self.pause);
    }

    /// Advances the cursor blink while the window is active, and the pause sign while the
    /// pause flag is set, as measured on the original runtime. Both run whether the window
    /// is visible or not.
    pub fn update(&mut self) {
        self.follow_flags();
        if self.active {
            self.cursor_step = (self.cursor_step + 1) % 32;
        }
        if self.pause {
            // 2^32 and 32 are the same step of the sign: both multiples of its period.
            self.pause_step = self.pause_step.checked_add(1).unwrap_or(32);
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
    /// The cells that flash: a cell whose entry is not 0 is overlaid with the colour its
    /// lowest twelve bits hold, four each for red, green and blue. Shared like the map data.
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

    /// One step of the autotile animation and of the pulse of the flash data: every 16
    /// updates each animated autotile shows its next frame.
    ///
    /// PLACEHOLDER: the speed of 16 updates per frame is not yet checked against the original
    /// runtime.
    pub fn update(&mut self) {
        // The count does not wrap within the lifetime of a process.
        self.updates += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The alphas a flash of `alpha` for `duration` updates shows: before any update and
    /// after each of `updates`. `None` is the flash being over.
    fn alphas(alpha: f64, duration: i32, updates: usize) -> Vec<Option<u8>> {
        let mut flash = Flash::default();
        flash.start(Some(Color::new(255.0, 0.0, 0.0, alpha)), duration);
        let mut shown = vec![flash.color().map(|color| color.a)];
        for _ in 0..updates {
            flash.advance();
            shown.push(flash.color().map(|color| color.a));
        }
        shown
    }

    /// The alpha series the measured pictures imply (`flash-bush-samples.txt`).
    #[test]
    fn the_alpha_decays_by_the_measured_integer_formula() {
        let series: [(f64, i32, &[u8]); 7] = [
            (255.0, 8, &[255, 223, 191, 159, 127, 95, 63, 31]),
            (128.0, 5, &[128, 102, 76, 50, 25]),
            (200.0, 10, &[200, 180, 160, 140, 120, 100, 80, 60, 40, 20]),
            (255.0, 6, &[255, 212, 169, 126, 84, 42]),
            (100.0, 3, &[100, 66, 33]),
            (
                255.0,
                16,
                &[
                    255, 239, 223, 207, 191, 175, 159, 143, 127, 111, 95, 79, 63, 47, 31, 15,
                ],
            ),
            (255.0, 1, &[255]),
        ];
        for (alpha, duration, expected) in series {
            let mut expected: Vec<Option<u8>> = expected.iter().copied().map(Some).collect();
            expected.extend([None, None]);
            assert_eq!(
                alphas(alpha, duration, duration as usize + 1),
                expected,
                "alpha {alpha} for {duration}"
            );
        }
    }

    /// The longest duration there is: the product of alpha and duration needs more than 32
    /// bits.
    #[test]
    fn a_flash_of_the_longest_duration_decays_without_overflow() {
        assert_eq!(
            alphas(255.0, i32::MAX, 2),
            [Some(255), Some(254), Some(253)]
        );
        // The same state serves sprites and viewports.
        let red = Color::new(255.0, 0.0, 0.0, 255.0);
        let mut sprite = Sprite::new(None, 0);
        sprite.flash(Some(red), i32::MAX);
        sprite.update();
        let mut viewport = Viewport::new(Rect::new(0, 0, 8, 8), 0);
        viewport.flash(Some(red), i32::MAX);
        viewport.update();
        let expected = vkxp_proto::Color::new(255, 0, 0, 254);
        assert_eq!(sprite.flash.effective_color(Color::TRANSPARENT), expected);
        assert_eq!(viewport.flash.effective_color(Color::TRANSPARENT), expected);
    }

    #[test]
    fn a_flash_of_duration_zero_or_less_shows_until_the_first_update() {
        for duration in [0, -1, i32::MIN] {
            assert_eq!(alphas(255.0, duration, 2), [Some(255), None, None]);
        }
        let mut flash = Flash::default();
        flash.start(None, 0);
        assert!(flash.hides());
        flash.advance();
        assert!(!flash.hides());
    }
}
