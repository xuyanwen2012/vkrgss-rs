//! Windows: the bitmaps cached from a window skin, and the quads a window is drawn with.
//!
//! A window skin is laid out as the original runtime's: the background pattern at
//! (0, 0, 128, 128), the frame at (128, 0, 64, 64) with the four scroll arrows in its centre,
//! the cursor at (128, 64, 32, 32) and the four images of the pause sign at (160, 64, 32, 32).
//!
//! Background, frame and cursor are not drawn piece by piece every frame. Each is drawn once
//! into a bitmap of the scene's own, with the operations of the bitmap arena, and drawn again
//! only when what it depends on changes.

use vkxp_proto::BitmapId;

use crate::extract::{Assembly, Level, Look};
use crate::geometry::Geometry;
use crate::{BitmapError, Bitmaps, FontDefaults, Rect, Window};

/// Distance from the window's edge to its contents area.
const PADDING: i32 = 16;
/// Distance from the window's edge to its background.
const BACK_MARGIN: i32 = 2;
/// Side of the background pattern.
const PATTERN: i32 = 128;
/// Side of a frame corner, and thickness of a frame edge.
const CORNER: i32 = 16;
/// Length of the piece of the skin that a frame edge repeats.
const EDGE: i32 = 32;
/// Left and top of the frame area.
const FRAME: (i32, i32) = (128, 0);
/// The cursor image.
const CURSOR: Rect = Rect::new(128, 64, 32, 32);
/// Width of the cursor's border that keeps its size.
const CURSOR_BORDER: i32 = 2;
/// Left and top of the pause sign's four images, each 16 by 16.
const PAUSE: (i32, i32) = (160, 64);
/// Number of updates one image of the pause sign stays.
const PAUSE_PERIOD: u32 = 16;

/// What a cached bitmap was drawn from: the skin, the skin's revision, the width and height
/// of the result, and the stretch flag where it matters.
pub(crate) type Key = (BitmapId, u64, i32, i32, bool);

/// Draws into the cleared bitmap `dst` from the skin `skin`.
type Draw = fn(&mut Bitmaps, BitmapId, BitmapId, i32, i32, bool) -> Result<(), BitmapError>;

/// One bitmap derived from a window skin, or from another bitmap in the same way: planes
/// keep their pre-tiled bitmap in one.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Cache {
    /// What `bitmap` was drawn from, or would have been had it been possible to create.
    key: Option<Key>,
    pub(crate) bitmap: Option<BitmapId>,
}

impl Cache {
    /// Brings the bitmap up to date with `wanted`; `None` means there is nothing to show.
    pub(crate) fn refresh(&mut self, bitmaps: &mut Bitmaps, wanted: Option<Key>, draw: Draw) {
        let lost = self.bitmap.is_some_and(|id| bitmaps.is_disposed(id));
        if wanted == self.key && !lost {
            return;
        }
        let Some(key) = wanted else {
            self.dispose(bitmaps);
            return;
        };
        let (skin, _, width, height, stretch) = key;
        let size = Rect::new(0, 0, width, height);
        let reusable = self.bitmap.filter(|&id| bitmaps.rect(id) == Ok(size));
        let target = match reusable {
            Some(id) => bitmaps.clear(id).ok().map(|()| id),
            None => {
                self.dispose(bitmaps);
                // What is too large for a bitmap goes without; the key is kept so that the
                // attempt is not repeated every frame.
                let defaults = FontDefaults::new(Vec::new());
                bitmaps
                    .create(width.unsigned_abs(), height.unsigned_abs(), &defaults)
                    .ok()
            }
        };
        self.key = Some(key);
        self.bitmap = target.filter(|&id| {
            let drawn = draw(bitmaps, id, skin, width, height, stretch).is_ok();
            if !drawn {
                bitmaps.dispose(id);
            }
            drawn
        });
    }

    pub(crate) fn dispose(&mut self, bitmaps: &mut Bitmaps) {
        if let Some(id) = self.bitmap.take() {
            bitmaps.dispose(id);
        }
        self.key = None;
    }
}

/// The bitmaps a window keeps in the bitmap arena.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Caches {
    background: Cache,
    frame: Cache,
    cursor: Cache,
}

impl Caches {
    pub(crate) fn dispose(&mut self, bitmaps: &mut Bitmaps) {
        self.background.dispose(bitmaps);
        self.frame.dispose(bitmaps);
        self.cursor.dispose(bitmaps);
    }
}

/// The background: the skin's pattern stretched to `width` by `height`, or tiled over it.
///
/// PLACEHOLDER: the sampling of the stretched pattern is that of `stretch_blt`, not yet
/// checked against the original runtime.
fn draw_background(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    skin: BitmapId,
    width: i32,
    height: i32,
    stretch: bool,
) -> Result<(), BitmapError> {
    let pattern = Rect::new(0, 0, PATTERN, PATTERN);
    if stretch {
        return bitmaps.stretch_blt(dst, Rect::new(0, 0, width, height), skin, pattern, 255);
    }
    for y in (0..height).step_by(PATTERN as usize) {
        for x in (0..width).step_by(PATTERN as usize) {
            bitmaps.blt(dst, x, y, skin, pattern, 255)?;
        }
    }
    Ok(())
}

/// The frame: the four corners of the skin's frame area, and between them its edges, each
/// repeated from the middle piece of the matching side and cut off at the far corner.
///
/// PLACEHOLDER: how the original runtime fills the edges (repeating or stretching, and from
/// where) and what a window smaller than two corners shows are not yet checked.
fn draw_frame(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    skin: BitmapId,
    width: i32,
    height: i32,
    _stretch: bool,
) -> Result<(), BitmapError> {
    let (left, top) = FRAME;
    let (far_x, far_y) = (left + CORNER + EDGE, top + CORNER + EDGE);
    for x in (CORNER..width - CORNER).step_by(EDGE as usize) {
        let length = EDGE.min(width - CORNER - x);
        let piece = Rect::new(left + CORNER, top, length, CORNER);
        bitmaps.blt(dst, x, 0, skin, piece, 255)?;
        let piece = Rect::new(left + CORNER, far_y, length, CORNER);
        bitmaps.blt(dst, x, height - CORNER, skin, piece, 255)?;
    }
    for y in (CORNER..height - CORNER).step_by(EDGE as usize) {
        let length = EDGE.min(height - CORNER - y);
        let piece = Rect::new(left, top + CORNER, CORNER, length);
        bitmaps.blt(dst, 0, y, skin, piece, 255)?;
        let piece = Rect::new(far_x, top + CORNER, CORNER, length);
        bitmaps.blt(dst, width - CORNER, y, skin, piece, 255)?;
    }
    for (x, y, skin_x, skin_y) in [
        (0, 0, left, top),
        (width - CORNER, 0, far_x, top),
        (0, height - CORNER, left, far_y),
        (width - CORNER, height - CORNER, far_x, far_y),
    ] {
        let corner = Rect::new(skin_x, skin_y, CORNER, CORNER);
        bitmaps.blt(dst, x, y, skin, corner, 255)?;
    }
    Ok(())
}

/// The cursor: the skin's cursor image stretched to `width` by `height`, except for a border
/// of 2 pixels that keeps its size.
///
/// PLACEHOLDER: the width of the unscaled border, and what a cursor too small for it shows
/// (here: the whole image stretched), are not yet checked against the original runtime.
fn draw_cursor(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    skin: BitmapId,
    width: i32,
    height: i32,
    _stretch: bool,
) -> Result<(), BitmapError> {
    if width < 2 * CURSOR_BORDER || height < 2 * CURSOR_BORDER {
        return bitmaps.stretch_blt(dst, Rect::new(0, 0, width, height), skin, CURSOR, 255);
    }
    // Along one axis: where each of the three strips starts and how long it is, in the
    // destination and in the image.
    let strips = |length: i32, image: i32| {
        [
            (0, CURSOR_BORDER, 0, CURSOR_BORDER),
            (
                CURSOR_BORDER,
                length - 2 * CURSOR_BORDER,
                CURSOR_BORDER,
                image - 2 * CURSOR_BORDER,
            ),
            (
                length - CURSOR_BORDER,
                CURSOR_BORDER,
                image - CURSOR_BORDER,
                CURSOR_BORDER,
            ),
        ]
    };
    for (y, rows, image_y, image_rows) in strips(height, CURSOR.height) {
        for (x, columns, image_x, image_columns) in strips(width, CURSOR.width) {
            let from = Rect::new(
                CURSOR.x + image_x,
                CURSOR.y + image_y,
                image_columns,
                image_rows,
            );
            bitmaps.stretch_blt(dst, Rect::new(x, y, columns, rows), skin, from, 255)?;
        }
    }
    Ok(())
}

/// The pixels of the rectangle at (`x`, `y`) of `width` by `height` that lie inside `clip`.
fn clip_to(clip: vkxp_proto::Rect, x: i64, y: i64, width: i64, height: i64) -> vkxp_proto::Rect {
    let left = x.max(i64::from(clip.x));
    let top = y.max(i64::from(clip.y));
    let right = (x + width).min(i64::from(clip.x) + i64::from(clip.width));
    let bottom = (y + height).min(i64::from(clip.y) + i64::from(clip.height));
    if left < right && top < bottom {
        // All four bounds lie inside `clip`.
        vkxp_proto::Rect::new(
            left as i32,
            top as i32,
            (right - left) as u32,
            (bottom - top) as u32,
        )
    } else {
        vkxp_proto::Rect::new(clip.x, clip.y, 0, 0)
    }
}

impl Window {
    /// The opacity of the background: the window's opacity times its back opacity over 255.
    fn background_opacity(&self) -> u8 {
        (u32::from(self.opacity) * u32::from(self.back_opacity) / 255) as u8
    }

    /// The opacity of the cursor: its blink alpha times the contents opacity over 255.
    fn cursor_opacity(&self) -> u8 {
        (u32::from(self.cursor_alpha()) * u32::from(self.contents_opacity) / 255) as u8
    }

    /// Brings the cached bitmaps this frame draws up to date. What is not drawn this frame
    /// is left as it is.
    pub(crate) fn refresh_caches(&mut self, bitmaps: &mut Bitmaps) {
        if !self.visible {
            return;
        }
        // A skin that is missing or disposed leaves nothing to show.
        let skin = self
            .windowskin
            .and_then(|id| Some((id, bitmaps.revision(id).ok()?)));
        let key = |width: i32, height: i32, stretch: bool| {
            skin.filter(|_| width > 0 && height > 0)
                .map(|(id, revision)| (id, revision, width, height, stretch))
        };
        if self.background_opacity() > 0 {
            let wanted = key(
                self.width.saturating_sub(2 * BACK_MARGIN),
                self.height.saturating_sub(2 * BACK_MARGIN),
                self.stretch,
            );
            self.caches
                .background
                .refresh(bitmaps, wanted, draw_background);
        }
        if self.opacity > 0 {
            let wanted = key(self.width, self.height, false);
            self.caches.frame.refresh(bitmaps, wanted, draw_frame);
        }
        if self.cursor_opacity() > 0 {
            let wanted = key(self.cursor_rect.width, self.cursor_rect.height, false);
            self.caches.cursor.refresh(bitmaps, wanted, draw_cursor);
        }
    }
}

impl Assembly<'_> {
    /// Draws the whole of `bitmap` unscaled with its top-left corner at (`x`, `y`).
    fn whole(&mut self, bitmap: BitmapId, x: i64, y: i64, clip: vkxp_proto::Rect, opacity: u8) {
        if let Ok(rect) = self.bitmaps.rect(bitmap) {
            self.quad(
                bitmap,
                rect,
                &Geometry::at(x, y),
                clip,
                Look::plain(opacity),
            );
        }
    }

    /// Background and frame of a window, at the window's z.
    pub(crate) fn window_back(&mut self, window: &Window, level: &Level) {
        if !window.visible {
            return;
        }
        let x = i64::from(window.x) + level.dx;
        let y = i64::from(window.y) + level.dy;
        if let Some(bitmap) = window.caches.background.bitmap {
            let margin = i64::from(BACK_MARGIN);
            let opacity = window.background_opacity();
            self.whole(bitmap, x + margin, y + margin, level.clip, opacity);
        }
        if let Some(bitmap) = window.caches.frame.bitmap {
            self.whole(bitmap, x, y, level.clip, window.opacity);
        }
    }

    /// Cursor, contents, pause sign and scroll arrows of a window, at the window's z plus 2.
    ///
    /// PLACEHOLDER: where the arrows sit (centred on their edge, 4 pixels in), when they
    /// appear (whenever contents lie beyond the visible area in that direction) and that
    /// pause sign and arrows ignore the contents opacity are not yet checked against the
    /// original runtime.
    pub(crate) fn window_front(&mut self, window: &Window, level: &Level) {
        if !window.visible {
            return;
        }
        let x = i64::from(window.x) + level.dx;
        let y = i64::from(window.y) + level.dy;
        let (width, height) = (i64::from(window.width), i64::from(window.height));
        let padding = i64::from(PADDING);
        let (inner_width, inner_height) = (width - 2 * padding, height - 2 * padding);

        let cursor = window.cursor_rect;
        if let Some(bitmap) = window.caches.cursor.bitmap
            && cursor.width > 0
            && cursor.height > 0
        {
            let clip = clip_to(level.clip, x, y, width, height);
            self.whole(
                bitmap,
                x + padding + i64::from(cursor.x),
                y + padding + i64::from(cursor.y),
                clip,
                window.cursor_opacity(),
            );
        }

        let contents = window
            .contents
            .and_then(|id| Some((id, self.bitmaps.rect(id).ok()?)));
        if let Some((bitmap, _)) = contents {
            let clip = clip_to(
                level.clip,
                x + padding,
                y + padding,
                inner_width,
                inner_height,
            );
            self.whole(
                bitmap,
                x + padding - i64::from(window.ox),
                y + padding - i64::from(window.oy),
                clip,
                window.contents_opacity,
            );
        }

        let Some(skin) = window.windowskin else {
            return;
        };
        let centre_x = x + (width - 16) / 2;
        let centre_y = y + (height - 16) / 2;
        let mut piece = |from: Rect, x: i64, y: i64| {
            self.quad(
                skin,
                from,
                &Geometry::at(x, y),
                level.clip,
                Look::plain(255),
            );
        };
        if window.pause {
            let image = (window.pause_step / PAUSE_PERIOD % 4) as i32;
            let from = Rect::new(
                PAUSE.0 + 16 * (image % 2),
                PAUSE.1 + 16 * (image / 2),
                16,
                16,
            );
            piece(from, centre_x, y + height - 16);
        }
        if let Some((_, size)) = contents {
            let (ox, oy) = (i64::from(window.ox), i64::from(window.oy));
            // The arrows sit around the 32 by 32 centre of the frame area.
            let (left, top) = (FRAME.0 + CORNER, FRAME.1 + CORNER);
            if oy > 0 {
                piece(Rect::new(left + 8, top, 16, 8), centre_x, y + 4);
            }
            if i64::from(size.height) - oy > inner_height {
                piece(
                    Rect::new(left + 8, top + 24, 16, 8),
                    centre_x,
                    y + height - 12,
                );
            }
            if ox > 0 {
                piece(Rect::new(left, top + 8, 8, 16), x + 4, centre_y);
            }
            if i64::from(size.width) - ox > inner_width {
                piece(
                    Rect::new(left + 24, top + 8, 8, 16),
                    x + width - 12,
                    centre_y,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use vkxp_compose_ref::Frame;
    use vkxp_pixel::Rgba;

    use super::*;
    use crate::bitmap::testing::{color, defaults};
    use crate::extract::testing::*;
    use crate::{Color, WindowId};

    const BACK: Rgba = [0, 0, 160, 255];
    const TOP_LEFT: Rgba = [255, 0, 0, 255];
    const TOP_RIGHT: Rgba = [0, 255, 0, 255];
    const BOTTOM_LEFT: Rgba = [255, 255, 0, 255];
    const BOTTOM_RIGHT: Rgba = [255, 0, 255, 255];
    const TOP: Rgba = [90, 10, 10, 255];
    const BOTTOM: Rgba = [10, 90, 10, 255];
    const LEFT: Rgba = [90, 90, 10, 255];
    const RIGHT: Rgba = [90, 10, 90, 255];
    const ARROW_UP: Rgba = [1, 200, 200, 255];
    const ARROW_DOWN: Rgba = [2, 200, 200, 255];
    const ARROW_LEFT: Rgba = [3, 200, 200, 255];
    const ARROW_RIGHT: Rgba = [4, 200, 200, 255];
    const CURSOR_EDGE: Rgba = [250, 250, 250, 255];
    const CURSOR_FILL: Rgba = [120, 120, 120, 255];
    const PAUSE_IMAGES: [Rgba; 4] = [
        [11, 50, 50, 255],
        [22, 50, 50, 255],
        [33, 50, 50, 255],
        [44, 50, 50, 255],
    ];

    /// A window skin of 192 by 128 pixels in which every part has a colour of its own.
    fn skin(stage: &mut Stage) -> BitmapId {
        let id = stage.bitmaps.create(192, 128, &defaults()).unwrap();
        let mut fill = |x, y, width, height, pixel| {
            stage
                .bitmaps
                .fill_rect(id, Rect::new(x, y, width, height), color(pixel))
                .unwrap();
        };
        fill(0, 0, 128, 128, BACK);
        fill(144, 0, 32, 16, TOP);
        fill(144, 48, 32, 16, BOTTOM);
        fill(128, 16, 16, 32, LEFT);
        fill(176, 16, 16, 32, RIGHT);
        fill(128, 0, 16, 16, TOP_LEFT);
        fill(176, 0, 16, 16, TOP_RIGHT);
        fill(128, 48, 16, 16, BOTTOM_LEFT);
        fill(176, 48, 16, 16, BOTTOM_RIGHT);
        fill(152, 16, 16, 8, ARROW_UP);
        fill(152, 40, 16, 8, ARROW_DOWN);
        fill(144, 24, 8, 16, ARROW_LEFT);
        fill(168, 24, 8, 16, ARROW_RIGHT);
        fill(128, 64, 32, 32, CURSOR_EDGE);
        fill(130, 66, 28, 28, CURSOR_FILL);
        fill(160, 64, 16, 16, PAUSE_IMAGES[0]);
        fill(176, 64, 16, 16, PAUSE_IMAGES[1]);
        fill(160, 80, 16, 16, PAUSE_IMAGES[2]);
        fill(176, 80, 16, 16, PAUSE_IMAGES[3]);
        id
    }

    /// A skin that has nothing but a background, each pixel of which differs from its
    /// neighbours.
    fn patterned_skin(stage: &mut Stage) -> BitmapId {
        let mut rgba = vec![0; 192 * 128 * 4];
        for y in 0..128 {
            for x in 0..128 {
                let offset = (y * 192 + x) * 4;
                rgba[offset..offset + 4].copy_from_slice(&pattern(x as u32, y as u32));
            }
        }
        stage.bitmap(192, 128, rgba)
    }

    /// The pixel at (`x`, `y`) of the background of [`patterned_skin`].
    fn pattern(x: u32, y: u32) -> Rgba {
        [x as u8 + 1, y as u8 + 1, 99, 255]
    }

    /// The window of the scenarios: 160 by 96 at 0, 0, with `skin`.
    fn window(stage: &mut Stage, skin: Option<BitmapId>) -> WindowId {
        let id = stage.scene.create_window(None).unwrap();
        let window = stage.scene.window_mut(id).unwrap();
        window.windowskin = skin;
        window.width = 160;
        window.height = 96;
        id
    }

    /// An opaque bitmap whose every pixel differs from its neighbours.
    fn contents(stage: &mut Stage, width: u32, height: u32) -> BitmapId {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&content(x, y));
            }
        }
        stage.bitmap(width, height, rgba)
    }

    /// The pixel at (`x`, `y`) of a [`contents`] bitmap.
    fn content(x: u32, y: u32) -> Rgba {
        [(x % 251) as u8 + 1, (y % 241) as u8 + 1, 7, 255]
    }

    /// Number of pixels of the frame that are not black.
    fn drawn(frame: &Frame) -> usize {
        let pixels = frame.rgba.as_chunks::<4>().0;
        pixels.iter().filter(|pixel| **pixel != BLACK).count()
    }

    #[test]
    fn the_background_fills_the_window_inside_a_margin_of_two() {
        let mut stage = Stage::new();
        // Only a background: the frame area of the skin is transparent.
        let skin = stage.bitmaps.create(192, 128, &defaults()).unwrap();
        stage
            .bitmaps
            .fill_rect(skin, Rect::new(0, 0, 128, 128), color(BACK))
            .unwrap();
        window(&mut stage, Some(skin));
        let frame = stage.frame();
        for y in 0..100 {
            for x in 0..164 {
                let inside = (2..=157).contains(&x) && (2..=93).contains(&y);
                let expected = if inside { BACK } else { BLACK };
                assert_eq!(frame.pixel(x, y), expected, "({x}, {y})");
            }
        }
    }

    #[test]
    fn the_background_shows_between_the_frame() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        window(&mut stage, Some(skin));
        let frame = stage.frame();
        for y in 16..80 {
            for x in 16..144 {
                assert_eq!(frame.pixel(x, y), BACK, "({x}, {y})");
            }
        }
    }

    #[test]
    fn the_frame_is_built_from_the_corners_and_edges_of_the_skin() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let id = window(&mut stage, Some(skin));
        let window = stage.scene.window_mut(id).unwrap();
        window.x = 40;
        window.y = 30;
        let frame = stage.frame();
        let at = |x: u32, y: u32| frame.pixel(40 + x, 30 + y);
        for (x, y, expected) in [
            (0, 0, TOP_LEFT),
            (15, 15, TOP_LEFT),
            (159, 0, TOP_RIGHT),
            (144, 15, TOP_RIGHT),
            (0, 95, BOTTOM_LEFT),
            (15, 80, BOTTOM_LEFT),
            (159, 95, BOTTOM_RIGHT),
            (144, 80, BOTTOM_RIGHT),
            (16, 0, TOP),
            (143, 15, TOP),
            (80, 7, TOP),
            (16, 80, BOTTOM),
            (143, 95, BOTTOM),
            (0, 16, LEFT),
            (15, 79, LEFT),
            (144, 16, RIGHT),
            (159, 79, RIGHT),
            (16, 16, BACK),
            (143, 79, BACK),
        ] {
            assert_eq!(at(x, y), expected, "({x}, {y})");
        }
        assert_eq!(frame.pixel(39, 30), BLACK);
        assert_eq!(frame.pixel(200, 30), BLACK);
        assert_eq!(frame.pixel(40, 126), BLACK);
        assert_eq!(drawn(&frame), 160 * 96);
    }

    #[test]
    fn frame_edges_repeat_their_piece_of_the_skin() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        // Mark the first column of the top piece and the first row of the left piece.
        stage
            .bitmaps
            .fill_rect(skin, Rect::new(144, 0, 1, 16), color(WHITE))
            .unwrap();
        stage
            .bitmaps
            .fill_rect(skin, Rect::new(128, 16, 16, 1), color(WHITE))
            .unwrap();
        let id = window(&mut stage, Some(skin));
        stage.scene.window_mut(id).unwrap().width = 100;
        let frame = stage.frame();
        for x in 16..84 {
            let expected = if (x - 16) % 32 == 0 { WHITE } else { TOP };
            assert_eq!(frame.pixel(x, 3), expected, "x {x}");
        }
        assert_eq!(frame.pixel(84, 3), TOP_RIGHT);
        for y in 16..80 {
            let expected = if (y - 16) % 32 == 0 { WHITE } else { LEFT };
            assert_eq!(frame.pixel(3, y), expected, "y {y}");
        }
        assert_eq!(frame.pixel(3, 80), BOTTOM_LEFT);
    }

    #[test]
    fn back_opacity_fades_the_background_alone() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let id = window(&mut stage, Some(skin));
        stage.scene.window_mut(id).unwrap().set_back_opacity(0);
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 0), TOP_LEFT);
        assert_eq!(frame.pixel(80, 0), TOP);
        assert_eq!(frame.pixel(80, 48), BLACK);
        assert_eq!(frame.pixel(16, 16), BLACK);

        stage.scene.window_mut(id).unwrap().set_back_opacity(100);
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 0), TOP_LEFT);
        assert_eq!(
            frame.pixel(80, 48),
            vkxp_pixel::blend_normal(BLACK, BACK, 100)
        );
    }

    #[test]
    fn opacity_fades_frame_and_background_but_not_the_contents() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let id = window(&mut stage, Some(skin));
        let bitmap = contents(&mut stage, 128, 64);
        let window = stage.scene.window_mut(id).unwrap();
        window.contents = Some(bitmap);
        window.set_opacity(0);
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 0), BLACK);
        assert_eq!(frame.pixel(80, 0), BLACK);
        assert_eq!(frame.pixel(2, 2), BLACK);
        assert_eq!(frame.pixel(16, 16), content(0, 0));
        assert_eq!(drawn(&frame), 128 * 64);

        // The background takes opacity times back opacity over 255.
        let window = stage.scene.window_mut(id).unwrap();
        window.contents = None;
        window.set_opacity(200);
        window.set_back_opacity(100);
        let frame = stage.frame();
        assert_eq!(
            frame.pixel(0, 0),
            vkxp_pixel::blend_normal(BLACK, TOP_LEFT, 200)
        );
        assert_eq!(
            frame.pixel(80, 48),
            vkxp_pixel::blend_normal(BLACK, BACK, 78)
        );
    }

    #[test]
    fn a_window_without_a_live_skin_draws_neither_frame_nor_background() {
        let mut stage = Stage::new();
        let id = window(&mut stage, None);
        let bitmap = contents(&mut stage, 128, 64);
        stage.scene.window_mut(id).unwrap().contents = Some(bitmap);
        let frame = stage.frame();
        assert_eq!(drawn(&frame), 128 * 64, "only the contents");

        let skin = skin(&mut stage);
        stage.scene.window_mut(id).unwrap().windowskin = Some(skin);
        assert_eq!(drawn(&stage.frame()), 160 * 96);
        stage.bitmaps.dispose(skin);
        assert_eq!(drawn(&stage.frame()), 128 * 64, "disposed skin");

        let skin = self::skin(&mut stage);
        stage.scene.window_mut(id).unwrap().windowskin = Some(skin);
        assert_eq!(drawn(&stage.frame()), 160 * 96);
        stage.scene.window_mut(id).unwrap().windowskin = None;
        assert_eq!(drawn(&stage.frame()), 128 * 64, "skin taken away");
    }

    #[test]
    fn an_invisible_or_disposed_window_draws_nothing() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let id = window(&mut stage, Some(skin));
        let bitmap = contents(&mut stage, 128, 64);
        let window = stage.scene.window_mut(id).unwrap();
        window.contents = Some(bitmap);
        window.cursor_rect = Rect::new(0, 0, 32, 32);
        window.pause = true;
        assert_eq!(drawn(&stage.frame()), 160 * 96);
        stage.scene.window_mut(id).unwrap().visible = false;
        assert!(all(&stage.frame(), BLACK));
        stage.scene.window_mut(id).unwrap().visible = true;
        assert_eq!(drawn(&stage.frame()), 160 * 96);
        stage.scene.dispose_window(id, &mut stage.bitmaps);
        assert!(all(&stage.frame(), BLACK));
    }

    #[test]
    fn the_stretch_flag_stretches_or_tiles_the_background() {
        let mut stage = Stage::new();
        let skin = patterned_skin(&mut stage);
        let id = stage.scene.create_window(None).unwrap();
        let window = stage.scene.window_mut(id).unwrap();
        window.windowskin = Some(skin);
        window.width = 204;
        window.height = 164;
        let stretched = stage.frame();
        for (x, y) in [(0, 0), (1, 0), (57, 3), (199, 159), (100, 80)] {
            let from_x = vkxp_pixel::stretch_index(x, 128, 200);
            let from_y = vkxp_pixel::stretch_index(y, 128, 160);
            assert_eq!(
                stretched.pixel(2 + x, 2 + y),
                pattern(from_x, from_y),
                "({x}, {y})"
            );
        }

        stage.scene.window_mut(id).unwrap().stretch = false;
        let tiled = stage.frame();
        assert_ne!(tiled, stretched);
        for (x, y) in [(0, 0), (127, 127), (128, 0), (199, 159), (130, 140)] {
            assert_eq!(
                tiled.pixel(2 + x, 2 + y),
                pattern(x % 128, y % 128),
                "({x}, {y})"
            );
        }
        assert_eq!(tiled.pixel(202, 2), BLACK);
        assert_eq!(tiled.pixel(2, 162), BLACK);

        stage.scene.window_mut(id).unwrap().stretch = true;
        assert_eq!(stage.frame(), stretched);
    }

    #[test]
    fn changing_the_size_redraws_frame_and_background() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let id = window(&mut stage, Some(skin));
        let before = stage.frame();
        assert_eq!(before.pixel(159, 95), BOTTOM_RIGHT);

        let window = stage.scene.window_mut(id).unwrap();
        window.width = 200;
        window.height = 64;
        let after = stage.frame();
        assert_ne!(after, before);
        assert_eq!(after.pixel(199, 63), BOTTOM_RIGHT);
        assert_eq!(after.pixel(159, 63), BOTTOM);
        assert_eq!(after.pixel(100, 30), BACK);
        assert_eq!(after.pixel(100, 64), BLACK);
        assert_eq!(drawn(&after), 200 * 64);

        // Back at the first size, the first picture.
        let window = stage.scene.window_mut(id).unwrap();
        window.width = 160;
        window.height = 96;
        assert_eq!(stage.frame(), before);

        // Sizes without pixels show nothing and do not fail.
        for (width, height) in [(0, 96), (160, 0), (-5, 96), (160, -1), (4, 4), (1, 1)] {
            let window = stage.scene.window_mut(id).unwrap();
            window.width = width;
            window.height = height;
            let frame = stage.frame();
            let expected = (width.max(0) * height.max(0)) as usize;
            assert_eq!(drawn(&frame), expected, "{width} by {height}");
        }
    }

    #[test]
    fn drawing_into_the_skin_redraws_the_window() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let id = window(&mut stage, Some(skin));
        stage.scene.window_mut(id).unwrap().stretch = false;
        let before = stage.frame();
        assert_eq!(before.pixel(16, 16), BACK);
        assert_eq!(before.pixel(0, 0), TOP_LEFT);

        // One pixel of the background pattern, and one of a frame corner.
        stage.bitmaps.set_pixel(skin, 14, 14, color(WHITE)).unwrap();
        let after = stage.frame();
        assert_eq!(after.pixel(16, 16), WHITE);
        assert_eq!(after.pixel(17, 16), BACK);
        stage.bitmaps.set_pixel(skin, 128, 0, color(WHITE)).unwrap();
        let after = stage.frame();
        assert_eq!(after.pixel(0, 0), WHITE);
        assert_eq!(after.pixel(1, 0), TOP_LEFT);

        // Another skin altogether.
        let other = stage.solid(192, 128, GREEN);
        stage.scene.window_mut(id).unwrap().windowskin = Some(other);
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 0), GREEN);
        assert_eq!(frame.pixel(80, 48), GREEN);
    }

    #[test]
    fn two_windows_share_a_skin() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let first = window(&mut stage, Some(skin));
        let second = window(&mut stage, Some(skin));
        stage.scene.window_mut(second).unwrap().x = 200;
        // The second window is hidden while the skin changes.
        stage.frame();
        stage.scene.window_mut(second).unwrap().visible = false;
        stage.bitmaps.set_pixel(skin, 128, 0, color(WHITE)).unwrap();
        assert_eq!(stage.frame().pixel(0, 0), WHITE);
        stage.scene.window_mut(second).unwrap().visible = true;
        let frame = stage.frame();
        assert_eq!(frame.pixel(200, 0), WHITE);
        assert!(!stage.scene.is_window_disposed(first));
    }

    #[test]
    fn the_contents_are_clipped_to_the_area_inside_the_padding() {
        let mut stage = Stage::new();
        let id = window(&mut stage, None);
        let bitmap = contents(&mut stage, 300, 300);
        stage.scene.window_mut(id).unwrap().contents = Some(bitmap);
        let frame = stage.frame();
        assert_eq!(frame.pixel(16, 16), content(0, 0));
        assert_eq!(frame.pixel(143, 79), content(127, 63));
        assert_eq!(frame.pixel(15, 16), BLACK);
        assert_eq!(frame.pixel(16, 15), BLACK);
        assert_eq!(frame.pixel(144, 16), BLACK);
        assert_eq!(frame.pixel(16, 80), BLACK);
        assert_eq!(drawn(&frame), 128 * 64);

        // Smaller contents are not stretched.
        let small = contents(&mut stage, 10, 5);
        stage.scene.window_mut(id).unwrap().contents = Some(small);
        let frame = stage.frame();
        assert_eq!(frame.pixel(25, 20), content(9, 4));
        assert_eq!(drawn(&frame), 50);

        // Disposed contents draw nothing.
        stage.bitmaps.dispose(small);
        assert!(all(&stage.frame(), BLACK));
    }

    #[test]
    fn the_origin_of_a_window_scrolls_its_contents() {
        let mut stage = Stage::new();
        let id = window(&mut stage, None);
        let bitmap = contents(&mut stage, 300, 300);
        let window = stage.scene.window_mut(id).unwrap();
        window.contents = Some(bitmap);
        window.oy = 32;
        let frame = stage.frame();
        assert_eq!(frame.pixel(16, 16), content(0, 32));
        assert_eq!(frame.pixel(143, 79), content(127, 95));
        assert_eq!(drawn(&frame), 128 * 64);

        let window = stage.scene.window_mut(id).unwrap();
        window.ox = 100;
        window.oy = -8;
        window.x = 50;
        window.y = 60;
        let frame = stage.frame();
        assert_eq!(frame.pixel(66, 84), content(100, 0));
        assert_eq!(frame.pixel(66, 83), BLACK);
        assert_eq!(frame.pixel(193, 139), content(227, 55));
        assert_eq!(drawn(&frame), 128 * 56);
    }

    #[test]
    fn the_contents_opacity_fades_the_contents() {
        let mut stage = Stage::new();
        let id = window(&mut stage, None);
        let bitmap = stage.solid(128, 64, WHITE);
        let window = stage.scene.window_mut(id).unwrap();
        window.contents = Some(bitmap);
        window.set_contents_opacity(90);
        assert_eq!(
            stage.frame().pixel(16, 16),
            vkxp_pixel::blend_normal(BLACK, WHITE, 90)
        );
        stage.scene.window_mut(id).unwrap().set_contents_opacity(0);
        assert!(all(&stage.frame(), BLACK));
    }

    #[test]
    fn a_sprite_can_stand_between_frame_and_contents() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let id = window(&mut stage, Some(skin));
        // Contents that are opaque in their left half only.
        let bitmap = stage.bitmaps.create(128, 64, &defaults()).unwrap();
        stage
            .bitmaps
            .fill_rect(bitmap, Rect::new(0, 0, 64, 64), color(WHITE))
            .unwrap();
        stage.scene.window_mut(id).unwrap().contents = Some(bitmap);
        stage.scene.set_window_z(id, 100).unwrap();

        let green = stage.solid(160, 96, GREEN);
        let sprite = stage.scene.create_sprite(None).unwrap();
        stage
            .scene
            .sprite_mut(sprite)
            .unwrap()
            .set_bitmap(Some(green), &stage.bitmaps);

        stage.scene.set_sprite_z(sprite, 101).unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 0), GREEN, "in front of the frame");
        assert_eq!(frame.pixel(100, 40), GREEN, "in front of the background");
        assert_eq!(frame.pixel(20, 40), WHITE, "behind the contents");

        stage.scene.set_sprite_z(sprite, 99).unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 0), TOP_LEFT);
        assert_eq!(frame.pixel(100, 40), BACK);
        assert_eq!(frame.pixel(20, 40), WHITE);

        stage.scene.set_sprite_z(sprite, 103).unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 0), GREEN);
        assert_eq!(frame.pixel(20, 40), GREEN);

        // At the z of the contents, the one given its z later is in front.
        stage.scene.set_sprite_z(sprite, 102).unwrap();
        assert_eq!(stage.frame().pixel(20, 40), GREEN);
        stage.scene.set_window_z(id, 0).unwrap();
        stage.scene.set_window_z(id, 100).unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(20, 40), WHITE);
        assert_eq!(frame.pixel(100, 40), GREEN);
    }

    #[test]
    fn a_window_in_a_viewport_is_offset_and_clipped() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let viewport = stage
            .scene
            .create_viewport(Rect::new(100, 50, 100, 60))
            .unwrap();
        stage.scene.viewport_mut(viewport).unwrap().ox = 10;
        let id = stage.scene.create_window(Some(viewport)).unwrap();
        let bitmap = contents(&mut stage, 128, 64);
        let window = stage.scene.window_mut(id).unwrap();
        window.windowskin = Some(skin);
        window.contents = Some(bitmap);
        window.width = 160;
        window.height = 96;
        window.cursor_rect = Rect::new(0, 0, 128, 64);
        window.pause = true;
        let frame = stage.frame();
        // The window's corner is at 90, 50: ten pixels of it are cut off at the left.
        assert_eq!(frame.pixel(100, 50), TOP_LEFT);
        assert_eq!(frame.pixel(105, 50), TOP_LEFT);
        assert_eq!(frame.pixel(106, 50), TOP);
        assert_eq!(frame.pixel(106, 66), content(0, 0));
        assert_eq!(frame.pixel(199, 109), content(93, 43));
        assert_eq!(frame.pixel(99, 50), BLACK);
        assert_eq!(drawn(&frame), 100 * 60, "nothing outside the viewport");
    }

    /// A window with the test skin and a cursor of 128 by 32 at 0, 32.
    fn with_cursor(stage: &mut Stage) -> WindowId {
        let skin = skin(stage);
        let id = window(stage, Some(skin));
        stage.scene.window_mut(id).unwrap().cursor_rect = Rect::new(0, 32, 128, 32);
        id
    }

    #[test]
    fn the_cursor_covers_its_rectangle_inside_the_contents_area() {
        let mut stage = Stage::new();
        with_cursor(&mut stage);
        let frame = stage.frame();
        for y in 16..80 {
            for x in 16..144 {
                let inside = (48..80).contains(&y);
                let edge = inside && (!(18..142).contains(&x) || !(50..78).contains(&y));
                let expected = match (inside, edge) {
                    (false, _) => BACK,
                    (true, true) => CURSOR_EDGE,
                    (true, false) => CURSOR_FILL,
                };
                assert_eq!(frame.pixel(x, y), expected, "({x}, {y})");
            }
        }
        // The frame around the contents area is untouched.
        assert_eq!(frame.pixel(15, 48), LEFT);
        assert_eq!(frame.pixel(144, 79), RIGHT);
    }

    #[test]
    fn the_cursor_follows_its_rectangle_and_the_window() {
        let mut stage = Stage::new();
        let id = with_cursor(&mut stage);
        let window = stage.scene.window_mut(id).unwrap();
        window.x = 100;
        window.y = 200;
        window.cursor_rect = Rect::new(8, 4, 40, 10);
        // The window's origin scrolls the contents, not the cursor.
        window.ox = 5;
        window.oy = 5;
        let frame = stage.frame();
        assert_eq!(frame.pixel(124, 220), CURSOR_EDGE);
        assert_eq!(frame.pixel(126, 222), CURSOR_FILL);
        assert_eq!(frame.pixel(161, 227), CURSOR_FILL);
        assert_eq!(frame.pixel(163, 229), CURSOR_EDGE);
        assert_eq!(frame.pixel(123, 220), BACK);
        assert_eq!(frame.pixel(164, 229), BACK);
        assert_eq!(frame.pixel(124, 230), BACK);

        // Too small for its border, the whole image is stretched.
        stage.scene.window_mut(id).unwrap().cursor_rect = Rect::new(0, 0, 3, 32);
        let frame = stage.frame();
        assert_eq!(frame.pixel(116, 216), CURSOR_EDGE);
        assert_eq!(frame.pixel(117, 220), CURSOR_FILL);
        assert_eq!(frame.pixel(119, 220), BACK);
    }

    #[test]
    fn the_cursor_is_behind_the_contents_and_in_front_of_the_background() {
        let mut stage = Stage::new();
        let id = with_cursor(&mut stage);
        // Contents that are opaque in their left half only.
        let bitmap = stage.bitmaps.create(128, 64, &defaults()).unwrap();
        stage
            .bitmaps
            .fill_rect(bitmap, Rect::new(0, 0, 64, 64), color(GREEN))
            .unwrap();
        stage.scene.window_mut(id).unwrap().contents = Some(bitmap);
        let frame = stage.frame();
        assert_eq!(frame.pixel(40, 60), GREEN);
        assert_eq!(frame.pixel(100, 60), CURSOR_FILL);
        assert_eq!(frame.pixel(100, 30), BACK);

        // It belongs to the upper part of the window: a sprite at z + 1 is behind it.
        let red = stage.solid(160, 96, RED);
        let sprite = stage.scene.create_sprite(None).unwrap();
        stage
            .scene
            .sprite_mut(sprite)
            .unwrap()
            .set_bitmap(Some(red), &stage.bitmaps);
        stage.scene.set_sprite_z(sprite, 1).unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(100, 60), CURSOR_FILL);
        assert_eq!(frame.pixel(100, 30), RED);
    }

    /// The colour of the cursor's inside over the background at `alpha`.
    fn blinking(alpha: u8) -> Rgba {
        vkxp_pixel::blend_normal(BACK, CURSOR_FILL, alpha)
    }

    #[test]
    fn the_cursor_of_an_active_window_blinks_over_32_updates() {
        let mut stage = Stage::new();
        let id = with_cursor(&mut stage);
        assert_eq!(stage.frame().pixel(80, 60), blinking(255));
        assert_eq!(
            stage.frame().pixel(80, 60),
            blinking(255),
            "no update, no blink"
        );

        let mut alphas = Vec::new();
        for _ in 0..32 {
            stage.scene.window_mut(id).unwrap().update();
            alphas.push(stage.scene.window(id).unwrap().cursor_alpha());
        }
        let falling: Vec<u8> = (1..=16).map(|step| 255 - 8 * step).collect();
        let rising: Vec<u8> = (1..=16).map(|step| 127 + 8 * step).collect();
        assert_eq!(alphas[..16], falling);
        assert_eq!(alphas[16..], rising);

        let mut stage = Stage::new();
        let id = with_cursor(&mut stage);
        for _ in 0..16 {
            stage.scene.window_mut(id).unwrap().update();
        }
        assert_eq!(stage.frame().pixel(80, 60), blinking(127));
        stage.scene.window_mut(id).unwrap().update();
        assert_eq!(stage.frame().pixel(80, 60), blinking(135));
        for _ in 0..15 {
            stage.scene.window_mut(id).unwrap().update();
        }
        assert_eq!(stage.frame().pixel(80, 60), blinking(255));
        assert_eq!(stage.frame().pixel(80, 60), CURSOR_FILL);
    }

    #[test]
    fn the_cursor_of_an_inactive_window_keeps_its_alpha() {
        let mut stage = Stage::new();
        let id = with_cursor(&mut stage);
        for _ in 0..5 {
            stage.scene.window_mut(id).unwrap().update();
        }
        assert_eq!(stage.frame().pixel(80, 60), blinking(215));
        stage.scene.window_mut(id).unwrap().active = false;
        for _ in 0..7 {
            stage.scene.window_mut(id).unwrap().update();
            assert_eq!(stage.frame().pixel(80, 60), blinking(215));
        }
        stage.scene.window_mut(id).unwrap().active = true;
        stage.scene.window_mut(id).unwrap().update();
        assert_eq!(stage.frame().pixel(80, 60), blinking(207));
    }

    #[test]
    fn the_contents_opacity_fades_the_cursor_too() {
        let mut stage = Stage::new();
        let id = with_cursor(&mut stage);
        stage
            .scene
            .window_mut(id)
            .unwrap()
            .set_contents_opacity(100);
        assert_eq!(stage.frame().pixel(80, 60), blinking(100));
        for _ in 0..16 {
            stage.scene.window_mut(id).unwrap().update();
        }
        assert_eq!(stage.frame().pixel(80, 60), blinking(49));
    }

    #[test]
    fn an_empty_cursor_rectangle_draws_no_cursor() {
        let mut stage = Stage::new();
        let id = with_cursor(&mut stage);
        let with = stage.frame();
        stage.scene.window_mut(id).unwrap().cursor_rect = Rect::new(0, 32, 0, 32);
        let without = stage.frame();
        assert_ne!(without, with);
        for y in 16..80 {
            for x in 16..144 {
                assert_eq!(without.pixel(x, y), BACK, "({x}, {y})");
            }
        }
        for rect in [
            Rect::new(0, 32, 128, 0),
            Rect::new(0, 0, -4, 32),
            Rect::default(),
        ] {
            stage.scene.window_mut(id).unwrap().cursor_rect = rect;
            assert_eq!(stage.frame(), without, "{rect:?}");
        }
        stage.scene.window_mut(id).unwrap().cursor_rect = Rect::new(0, 32, 128, 32);
        assert_eq!(stage.frame(), with);
    }

    #[test]
    fn the_pause_sign_steps_through_its_four_images() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let id = window(&mut stage, Some(skin));
        let without = stage.frame();
        assert_eq!(without.pixel(72, 80), BOTTOM);

        stage.scene.window_mut(id).unwrap().pause = true;
        let frame = stage.frame();
        for y in 78..98 {
            for x in 70..90 {
                let inside = (72..88).contains(&x) && (80..96).contains(&y);
                let expected = if inside {
                    PAUSE_IMAGES[0]
                } else {
                    without.pixel(x, y)
                };
                assert_eq!(frame.pixel(x, y), expected, "({x}, {y})");
            }
        }

        for round in 0..2 {
            for image in [1, 2, 3, 0] {
                for step in 0..16 {
                    let previous = PAUSE_IMAGES[(image + 3) % 4];
                    assert_eq!(
                        stage.frame().pixel(72, 80),
                        previous,
                        "{round} {image} {step}"
                    );
                    stage.scene.window_mut(id).unwrap().update();
                }
                assert_eq!(stage.frame().pixel(72, 80), PAUSE_IMAGES[image]);
                assert_eq!(stage.frame().pixel(87, 95), PAUSE_IMAGES[image]);
            }
        }

        // Without the flag there is no sign, and it starts over when the flag returns.
        for _ in 0..20 {
            stage.scene.window_mut(id).unwrap().update();
        }
        assert_eq!(stage.frame().pixel(72, 80), PAUSE_IMAGES[1]);
        stage.scene.window_mut(id).unwrap().pause = false;
        assert_eq!(stage.frame(), without);
        stage.scene.window_mut(id).unwrap().update();
        stage.scene.window_mut(id).unwrap().pause = true;
        assert_eq!(stage.frame().pixel(72, 80), PAUSE_IMAGES[0]);
    }

    #[test]
    fn the_pause_sign_is_in_front_of_the_contents() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let id = stage.scene.create_window(None).unwrap();
        let bitmap = contents(&mut stage, 128, 64);
        let window = stage.scene.window_mut(id).unwrap();
        window.windowskin = Some(skin);
        window.contents = Some(bitmap);
        window.width = 160;
        window.height = 96;
        window.oy = -20;
        window.pause = true;
        stage.scene.set_window_z(id, 10).unwrap();
        // A sprite between the two parts of the window is behind the sign.
        let green = stage.solid(160, 96, GREEN);
        let sprite = stage.scene.create_sprite(None).unwrap();
        stage
            .scene
            .sprite_mut(sprite)
            .unwrap()
            .set_bitmap(Some(green), &stage.bitmaps);
        stage.scene.set_sprite_z(sprite, 11).unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(72, 80), PAUSE_IMAGES[0]);
        assert_eq!(frame.pixel(60, 80), GREEN);
        assert_eq!(frame.pixel(60, 70), content(44, 34));
    }

    /// Which arrows the frame shows: up, down, left, right.
    fn arrows(frame: &Frame) -> [bool; 4] {
        let shows = |x: u32, y: u32, width: u32, height: u32, pixel: Rgba| {
            let first = frame.pixel(x, y) == pixel;
            let last = frame.pixel(x + width - 1, y + height - 1) == pixel;
            assert_eq!(first, last, "an arrow is drawn whole or not at all");
            first
        };
        [
            shows(72, 4, 16, 8, ARROW_UP),
            shows(72, 84, 16, 8, ARROW_DOWN),
            shows(4, 40, 8, 16, ARROW_LEFT),
            shows(148, 40, 8, 16, ARROW_RIGHT),
        ]
    }

    #[test]
    fn arrows_show_where_the_contents_go_on() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let id = window(&mut stage, Some(skin));
        assert_eq!(arrows(&stage.frame()), [false; 4], "no contents");

        // Exactly the visible area: nothing is hidden.
        let fitting = contents(&mut stage, 128, 64);
        stage.scene.window_mut(id).unwrap().contents = Some(fitting);
        assert_eq!(arrows(&stage.frame()), [false; 4]);

        // Taller than the visible area.
        let tall = contents(&mut stage, 128, 200);
        stage.scene.window_mut(id).unwrap().contents = Some(tall);
        let frame = stage.frame();
        assert_eq!(arrows(&frame), [false, true, false, false]);
        assert_eq!(frame.pixel(71, 84), BOTTOM);
        assert_eq!(frame.pixel(88, 91), BOTTOM);
        assert_eq!(frame.pixel(72, 83), BOTTOM);
        assert_eq!(frame.pixel(72, 92), BOTTOM);

        stage.scene.window_mut(id).unwrap().oy = 32;
        assert_eq!(arrows(&stage.frame()), [true, true, false, false]);
        stage.scene.window_mut(id).unwrap().oy = 136;
        assert_eq!(arrows(&stage.frame()), [true, false, false, false]);
        stage.scene.window_mut(id).unwrap().oy = 135;
        assert_eq!(arrows(&stage.frame()), [true, true, false, false]);

        // Wider than the visible area.
        let wide = contents(&mut stage, 300, 64);
        let window = stage.scene.window_mut(id).unwrap();
        window.contents = Some(wide);
        window.oy = 0;
        assert_eq!(arrows(&stage.frame()), [false, false, false, true]);
        stage.scene.window_mut(id).unwrap().ox = 1;
        assert_eq!(arrows(&stage.frame()), [false, false, true, true]);
        stage.scene.window_mut(id).unwrap().ox = 172;
        assert_eq!(arrows(&stage.frame()), [false, false, true, false]);

        // Without a skin there is nothing to draw them from.
        stage.scene.window_mut(id).unwrap().windowskin = None;
        assert_eq!(arrows(&stage.frame()), [false; 4]);
    }

    #[test]
    fn arrows_are_in_front_of_what_stands_between_the_parts_of_the_window() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let id = window(&mut stage, Some(skin));
        let tall = contents(&mut stage, 128, 200);
        stage.scene.window_mut(id).unwrap().contents = Some(tall);
        let green = stage.solid(160, 96, GREEN);
        let sprite = stage.scene.create_sprite(None).unwrap();
        stage
            .scene
            .sprite_mut(sprite)
            .unwrap()
            .set_bitmap(Some(green), &stage.bitmaps);
        stage.scene.set_sprite_z(sprite, 1).unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(72, 84), ARROW_DOWN);
        assert_eq!(frame.pixel(71, 84), GREEN);
    }

    #[test]
    fn a_skin_smaller_than_the_layout_draws_what_it_has() {
        let mut stage = Stage::new();
        let small = stage.solid(40, 40, BACK);
        let id = window(&mut stage, Some(small));
        let tall = contents(&mut stage, 128, 200);
        let window = stage.scene.window_mut(id).unwrap();
        window.contents = Some(tall);
        window.cursor_rect = Rect::new(0, 0, 32, 32);
        window.pause = true;
        window.stretch = false;
        let frame = stage.frame();
        assert_eq!(frame.pixel(2, 2), BACK);
        assert_eq!(frame.pixel(0, 0), BLACK);
        assert_eq!(frame.pixel(41, 10), BACK);
        assert_eq!(frame.pixel(42, 10), BLACK);
        assert_eq!(frame.pixel(130, 10), BACK, "the next tile");
        assert_eq!(frame.pixel(72, 84), BLACK);
    }

    #[test]
    fn cached_bitmaps_are_disposed_with_the_window() {
        let mut stage = Stage::new();
        let skin = skin(&mut stage);
        let bitmap = contents(&mut stage, 128, 200);
        let before = stage.bitmaps.len();
        assert_eq!(before, 2);

        let id = window(&mut stage, Some(skin));
        let window = stage.scene.window_mut(id).unwrap();
        window.contents = Some(bitmap);
        window.cursor_rect = Rect::new(0, 0, 128, 32);
        window.pause = true;
        stage.frame();
        assert_eq!(
            stage.bitmaps.len(),
            before + 3,
            "background, frame and cursor"
        );

        // Changes that redraw them do not let them pile up.
        let window = stage.scene.window_mut(id).unwrap();
        window.width = 200;
        window.stretch = false;
        window.cursor_rect = Rect::new(0, 0, 64, 32);
        stage.frame();
        stage
            .bitmaps
            .set_pixel(skin, 0, 0, Color::new(1.0, 2.0, 3.0, 255.0))
            .unwrap();
        stage.frame();
        assert_eq!(stage.bitmaps.len(), before + 3);

        // A cursor rectangle without pixels gives its bitmap back.
        stage.scene.window_mut(id).unwrap().cursor_rect = Rect::default();
        stage.frame();
        assert_eq!(stage.bitmaps.len(), before + 2);

        stage.scene.dispose_window(id, &mut stage.bitmaps);
        assert_eq!(stage.bitmaps.len(), before);
        assert!(!stage.bitmaps.is_disposed(skin));
        assert!(!stage.bitmaps.is_disposed(bitmap));
        // The compositor is told, and accepts the frame that follows.
        assert!(all(&stage.frame(), BLACK));

        // Losing the skin gives them back as well.
        let id = self::window(&mut stage, Some(skin));
        stage.frame();
        assert_eq!(stage.bitmaps.len(), before + 2);
        stage.scene.window_mut(id).unwrap().windowskin = None;
        stage.frame();
        assert_eq!(stage.bitmaps.len(), before);
    }
}
