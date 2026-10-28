//! Windows: the bitmaps cached from a window skin, and the quads a window is drawn with.
//!
//! A window skin is laid out as the original runtime's: the background pattern at
//! (0, 0, 128, 128), the frame at (128, 0, 64, 64) with the four scroll arrows in its centre,
//! the cursor at (128, 64, 32, 32) and the four images of the pause sign at (160, 64, 32, 32).
//!
//! Background, frame and cursor are not drawn piece by piece every frame. Each is drawn once
//! into a bitmap of the scene's own, with the operations of the bitmap arena, and drawn again
//! only when what it depends on changes.
//!
//! Every rule here was measured on the original runtime (RGSS103J, window probes,
//! 2026-10-09); the samples of `tests/data/window-samples.txt` are a selection of what was
//! measured. All parts of a window are drawn in the window blend mode.
//!
//! This is the window of RGSS1. The windows of RGSS2 and RGSS3 are in the modules `rgss2`
//! and `rgss3`; they share the pieces of frame and cursor and the stretch of a block with it.

use vkrgss_proto::{BitmapId, BlendMode};

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
/// How far a frame edge runs under the corners at its ends.
const EDGE_START: i32 = 8;
/// Left and top of the frame area.
const FRAME: (i32, i32) = (128, 0);
/// The cursor image.
const CURSOR: Rect = Rect::new(128, 64, 32, 32);
/// Width of the cursor's border that keeps its size.
const CURSOR_BORDER: i32 = 2;
/// Left and top of the pause sign's four images, each 16 by 16.
const PAUSE: (i32, i32) = (160, 64);
/// Number of updates one image of the pause sign stays.
const PAUSE_PERIOD: u32 = 8;

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

    /// Draws the bitmap anew from `wanted`, also when it was drawn from the same before.
    fn rebuild(&mut self, bitmaps: &mut Bitmaps, wanted: Option<Key>, draw: Draw) {
        if wanted.is_none() {
            self.dispose(bitmaps);
            return;
        }
        self.key = None;
        self.refresh(bitmaps, wanted, draw);
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

/// The numerator with which a `side` by `side` block of the skin is stretched to `width` by
/// `height`: `side`, or one less in both directions as soon as the target is more than half
/// the block in both directions and not a copy of it. With `side` the stretch is the stretch
/// rule, which picks texels; with the reduced numerator it is [`stretch_bilinear`].
fn stretched_part(side: i32, width: i32, height: i32) -> i32 {
    let over_half = width > side / 2 && height > side / 2;
    if over_half && (width, height) != (side, side) {
        side - 1
    } else {
        side
    }
}

/// A colour and an alpha; no colour where no texel with an alpha above 0 lent one.
type Mixed = (Option<[u8; 3]>, u8);

/// `p + (((q - p) * f) >> 8)`, the shift being an arithmetic one.
fn lerp(p: u8, q: u8, f: i32) -> u8 {
    // Between `p` and `q`, as `f` is below 256.
    (i32::from(p) + (((i32::from(q) - i32::from(p)) * f) >> 8)) as u8
}

/// `p` and `q` weighted by `f` of 256: the alpha always, the colour where both have one;
/// where one has none, the colour is the other's, unweighted.
fn mix(p: Mixed, q: Mixed, f: i32) -> Mixed {
    let colour = match (p.0, q.0) {
        (Some(p), Some(q)) => Some([0, 1, 2].map(|c| lerp(p[c], q[c], f))),
        (p, q) => p.or(q),
    };
    (colour, lerp(p.1, q.1, f))
}

/// Where pixel `i` of `length` reads a block stretched with the reduced numerator `part`:
/// the texel, and the weight of the next one in 256ths.
fn bilinear_position(i: usize, part: i32, length: i32) -> (i32, i32) {
    let position = i as i64 * ((i64::from(part) << 32) / i64::from(length));
    // The texel is below `part`, as `i` is below `length`.
    ((position >> 32) as i32, ((position >> 24) & 255) as i32)
}

/// How many of the `length` columns or rows of its target the bilinear stretch of a block
/// of `side` draws when the skin holds only the first `held` of the block's: those whose
/// next one still reads a texel with a neighbour inside the skin,
/// `(i + 1) * (side - 1) / length < held - 1` in exact arithmetic.
///
/// Measured on the originals of VX and VX Ace (window measurement of RGSS3, 2026-10-13).
/// The stretch that picks texels follows the same rule with `side` for `side - 1` and
/// `held` for `held - 1`, which is how [`Bitmaps::stretch_blt`] cuts.
fn bilinear_kept(side: i32, held: i64, length: i32) -> i32 {
    if held >= i64::from(side) {
        return length;
    }
    let part = i64::from(side) - 1;
    let reach = (held - 1).max(0) * i64::from(length);
    // Between 0 and `length`.
    ((reach + part - 1) / part - 1).clamp(0, i64::from(length)) as i32
}

/// Stretches the `side` by `side` block of `skin` at `corner` over the rectangle `to` of the
/// cleared bitmap `dst` as the original does with the reduced numerator `side - 1`: each
/// pixel is interpolated between four texels, along the row first and then between the two
/// rows, with 8-bit weights and truncation, the alpha like a colour channel. A texel with an
/// alpha of 0 lends no colour.
///
/// Texels outside the skin count as transparent. With `cut`, a block that leaves the skin
/// shortens the target ([`bilinear_kept`]); without it the whole target is drawn (RGSS1,
/// not measured there).
fn stretch_bilinear(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    to: Rect,
    skin: BitmapId,
    corner: (i32, i32),
    side: i32,
    cut: bool,
) -> Result<(), BitmapError> {
    let mut drawn = to;
    if cut {
        let size = bitmaps.rect(skin)?;
        let held = |length: i32, start: i32| i64::from(length) - i64::from(start);
        drawn.width = bilinear_kept(side, held(size.width, corner.0), to.width);
        drawn.height = bilinear_kept(side, held(size.height, corner.1), to.height);
    }
    let Some(area) = bitmaps.get(dst)?.clip(drawn) else {
        return Ok(());
    };
    let source = bitmaps.get(skin)?;
    let texel = |x: i32, y: i32| -> Mixed {
        let inside = (0..source.width as i64).contains(&i64::from(x))
            && (0..source.height as i64).contains(&i64::from(y));
        if !inside {
            return (None, 0);
        }
        let [r, g, b, a] = source.pixel(x as usize, y as usize);
        ((a != 0).then_some([r, g, b]), a)
    };
    let mut pixels = Vec::with_capacity((area.right - area.left) * (area.bottom - area.top));
    for y in area.top..area.bottom {
        let (y0, fy) = bilinear_position(y - to.y as usize, side - 1, to.height);
        for x in area.left..area.right {
            let (x0, fx) = bilinear_position(x - to.x as usize, side - 1, to.width);
            let row = |y: i32| {
                let (x, y) = (corner.0 + x0, corner.1 + y);
                mix(texel(x, y), texel(x + 1, y), fx)
            };
            let (colour, alpha) = mix(row(y0), row(y0 + 1), fy);
            let [r, g, b] = colour.unwrap_or_default();
            pixels.push([r, g, b, alpha]);
        }
    }
    let target = bitmaps.get_mut(dst)?;
    let mut pixels = pixels.into_iter();
    for y in area.top..area.bottom {
        for (x, pixel) in (area.left..area.right).zip(&mut pixels) {
            target.put(x, y, pixel);
        }
    }
    target.mark(area);
    Ok(())
}

/// The background: the skin's pattern stretched to the inner size of the window, or tiled
/// over it from its corner.
///
/// The bitmap is one column wider than the background, and that column stays transparent. A
/// row of a window layer whose visible part is odd blends its last pixel with another
/// formula; for the background the original does so only where the screen or a viewport cuts
/// the row, and the spare column makes the quad's rows behave that way.
fn draw_background(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    skin: BitmapId,
    width: i32,
    height: i32,
    stretch: bool,
) -> Result<(), BitmapError> {
    let width = width - 1;
    if stretch {
        let to = Rect::new(0, 0, width, height);
        if stretched_part(PATTERN, width, height) < PATTERN {
            return stretch_bilinear(bitmaps, dst, to, skin, (0, 0), PATTERN, false);
        }
        let from = Rect::new(0, 0, PATTERN, PATTERN);
        return bitmaps.stretch_blt(dst, to, skin, from, 255);
    }
    for y in (0..height).step_by(PATTERN as usize) {
        for x in (0..width).step_by(PATTERN as usize) {
            let piece = Rect::new(0, 0, PATTERN.min(width - x), PATTERN.min(height - y));
            bitmaps.blt(dst, x, y, skin, piece, 255)?;
        }
    }
    Ok(())
}

/// The frame, composed as the original composes it: the top, bottom, left and right edges,
/// each tiled from its 32-pixel piece of the skin from 8 pixels inside the window to 8 pixels
/// before its far side, then the four corners over their ends. A window too small for a
/// piece shows what of it lies inside, later pieces over earlier ones.
fn draw_frame(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    skin: BitmapId,
    width: i32,
    height: i32,
    _stretch: bool,
) -> Result<(), BitmapError> {
    frame_pieces(bitmaps, dst, skin, (width, height), FRAME, EDGE_START)
}

/// The frame of `size` from the frame area at `corner` of the skin, its edges running from
/// `edge_start` pixels inside the window to as many before its far side.
fn frame_pieces(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    skin: BitmapId,
    (width, height): (i32, i32),
    (left, top): (i32, i32),
    edge_start: i32,
) -> Result<(), BitmapError> {
    let (far_x, far_y) = (left + CORNER + EDGE, top + CORNER + EDGE);
    for (y, skin_y) in [(0, top), (height - CORNER, far_y)] {
        for x in (edge_start..width - edge_start).step_by(EDGE as usize) {
            let length = EDGE.min(width - edge_start - x);
            let piece = Rect::new(left + CORNER, skin_y, length, CORNER);
            bitmaps.blt(dst, x, y, skin, piece, 255)?;
        }
    }
    for (x, skin_x) in [(0, left), (width - CORNER, far_x)] {
        for y in (edge_start..height - edge_start).step_by(EDGE as usize) {
            let length = EDGE.min(height - edge_start - y);
            let piece = Rect::new(skin_x, top + CORNER, CORNER, length);
            bitmaps.blt(dst, x, y, skin, piece, 255)?;
        }
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

/// The cursor, composed as the original composes it: the centre of the skin's cursor image
/// stretched to all but a border of 2 pixels, then the top, bottom, left and right edges
/// stretched along their length, then the four corners. A cursor too small for a piece shows
/// what of it lies inside, later pieces over earlier ones.
fn draw_cursor(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    skin: BitmapId,
    width: i32,
    height: i32,
    _stretch: bool,
) -> Result<(), BitmapError> {
    cursor_pieces(
        bitmaps,
        dst,
        skin,
        (width, height),
        CURSOR,
        CURSOR_BORDER,
        false,
    )
}

/// The cursor of `size` from the cursor image `cursor` of the skin, whose border of `border`
/// pixels keeps its size. `cut` is that of [`stretch_bilinear`], for the centre.
fn cursor_pieces(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    skin: BitmapId,
    (width, height): (i32, i32),
    cursor: Rect,
    border: i32,
    cut: bool,
) -> Result<(), BitmapError> {
    let (inner_width, inner_height) = (width - 2 * border, height - 2 * border);
    let image = cursor.width - 2 * border;
    let (near_x, near_y) = (cursor.x, cursor.y);
    let (far_x, far_y) = (near_x + border + image, near_y + border + image);
    if inner_width > 0 && inner_height > 0 {
        let corner = (near_x + border, near_y + border);
        let to = Rect::new(border, border, inner_width, inner_height);
        if stretched_part(image, inner_width, inner_height) < image {
            stretch_bilinear(bitmaps, dst, to, skin, corner, image, cut)?;
        } else {
            let from = Rect::new(corner.0, corner.1, image, image);
            bitmaps.stretch_blt(dst, to, skin, from, 255)?;
        }
    }
    if inner_width > 0 {
        for (y, skin_y) in [(0, near_y), (height - border, far_y)] {
            let from = Rect::new(near_x + border, skin_y, image, border);
            let to = Rect::new(border, y, inner_width, border);
            bitmaps.stretch_blt(dst, to, skin, from, 255)?;
        }
    }
    if inner_height > 0 {
        for (x, skin_x) in [(0, near_x), (width - border, far_x)] {
            let from = Rect::new(skin_x, near_y + border, border, image);
            let to = Rect::new(x, border, border, inner_height);
            bitmaps.stretch_blt(dst, to, skin, from, 255)?;
        }
    }
    for (x, y, skin_x, skin_y) in [
        (0, 0, near_x, near_y),
        (width - border, 0, far_x, near_y),
        (0, height - border, near_x, far_y),
        (width - border, height - border, far_x, far_y),
    ] {
        let corner = Rect::new(skin_x, skin_y, border, border);
        bitmaps.blt(dst, x, y, skin, corner, 255)?;
    }
    Ok(())
}

/// The pixels of the rectangle at (`x`, `y`) of `width` by `height` that lie inside `clip`.
fn clip_to(
    clip: vkrgss_proto::Rect,
    x: i64,
    y: i64,
    width: i64,
    height: i64,
) -> vkrgss_proto::Rect {
    let left = x.max(i64::from(clip.x));
    let top = y.max(i64::from(clip.y));
    let right = (x + width).min(i64::from(clip.x) + i64::from(clip.width));
    let bottom = (y + height).min(i64::from(clip.y) + i64::from(clip.height));
    if left < right && top < bottom {
        // All four bounds lie inside `clip`.
        vkrgss_proto::Rect::new(
            left as i32,
            top as i32,
            (right - left) as u32,
            (bottom - top) as u32,
        )
    } else {
        vkrgss_proto::Rect::new(clip.x, clip.y, 0, 0)
    }
}

/// How a part of a window looks: unchanged colours, the window blend.
fn layer(opacity: u8) -> Look {
    Look {
        blend: BlendMode::Window,
        ..Look::plain(opacity)
    }
}

impl Window {
    /// The opacity of the background: the window's opacity times its back opacity over 255,
    /// rounded down.
    fn background_opacity(&self) -> u8 {
        (u32::from(self.opacity) * u32::from(self.back_opacity) / 255) as u8
    }

    /// The opacity of the cursor: the contents opacity less 8 for each step the blink is
    /// away from its start, at most 16 steps either way. The contents opacity is subtracted
    /// from, not scaled.
    pub(crate) fn cursor_opacity(&self) -> u8 {
        let step = match self.cursor_step {
            step @ 0..=16 => step,
            step => 32 - step,
        };
        self.contents_opacity.saturating_sub(8 * step)
    }

    /// The opacity and the image of the pause sign, or `None` while it is not shown: it
    /// fades in by 32 per update from the update after the flag was set, and shows each of
    /// its four images for 8 updates.
    pub(crate) fn pause_sign(&self) -> Option<(u8, u32)> {
        if !self.pause || self.pause_step == 0 {
            return None;
        }
        let opacity = self.pause_step.saturating_mul(32).min(255) as u8;
        Some((opacity, self.pause_step / PAUSE_PERIOD % 4))
    }

    /// Brings the cached bitmaps this frame draws up to date.
    pub(crate) fn refresh_caches(&mut self, bitmaps: &mut Bitmaps) {
        match self.kind {
            Kind::Rgss1 => self.refresh_caches_rgss1(bitmaps),
            // A window of RGSS2 or RGSS3 builds its bitmaps at its calls, not for a frame.
            Kind::Rgss2(_) | Kind::Rgss3(..) => {}
        }
    }

    /// Brings the cached bitmaps of a window of RGSS1 that this frame draws up to date.
    /// What is not drawn this frame is left as it is.
    fn refresh_caches_rgss1(&mut self, bitmaps: &mut Bitmaps) {
        self.follow_flags();
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
            let width = self.width.saturating_sub(2 * BACK_MARGIN);
            let height = self.height.saturating_sub(2 * BACK_MARGIN);
            // One spare column: see `draw_background`.
            let wanted = key(width.saturating_add(1), height, self.stretch).filter(|_| width > 0);
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
    /// A window that is one element at its z: one of RGSS2 or RGSS3, drawn by the rules of
    /// its runtime. A window of RGSS1 is two elements, [`Assembly::window_back`] and
    /// [`Assembly::window_front`], and draws nothing here.
    pub(crate) fn window(&mut self, window: &Window, level: &Level) {
        match &window.kind {
            Kind::Rgss1 => {}
            Kind::Rgss2(state) => self.window_rgss2(window, state, level),
            Kind::Rgss3(state, rules) => self.window_rgss3(window, state, rules, level),
        }
    }

    /// Draws the whole of `bitmap` unscaled with its top-left corner at (`x`, `y`), looking
    /// as `look` says.
    fn whole_as(&mut self, bitmap: BitmapId, x: i64, y: i64, clip: vkrgss_proto::Rect, look: Look) {
        if let Ok(rect) = self.bitmaps.rect(bitmap) {
            self.quad(bitmap, rect, &Geometry::at(x, y), clip, look);
        }
    }

    /// Background and frame of a window, at the window's z. The background takes the window's
    /// opacity and its back opacity, the frame the window's opacity alone.
    pub(crate) fn window_back(&mut self, window: &Window, level: &Level) {
        if !window.visible {
            return;
        }
        let x = i64::from(window.x) + level.dx;
        let y = i64::from(window.y) + level.dy;
        if let Some(bitmap) = window.caches.background.bitmap {
            let margin = i64::from(BACK_MARGIN);
            let look = layer(window.background_opacity());
            self.whole_as(bitmap, x + margin, y + margin, level.clip, look);
        }
        if let Some(bitmap) = window.caches.frame.bitmap {
            self.whole_as(bitmap, x, y, level.clip, layer(window.opacity));
        }
    }

    /// The upper part of a window, at the window's z plus 2, from bottom to top: cursor,
    /// contents, the scroll arrows in the order down, left, right, up, and the pause sign.
    ///
    /// The cursor does not move with the window's origin and is cut off by the window, not by
    /// the contents area; the contents opacity acts on the contents and the cursor only, and
    /// no opacity on the arrows and the pause sign. Arrows need a contents bitmap.
    pub(crate) fn window_front(&mut self, window: &Window, level: &Level) {
        if !window.visible {
            return;
        }
        let x = i64::from(window.x) + level.dx;
        let y = i64::from(window.y) + level.dy;
        let (width, height) = (i64::from(window.width), i64::from(window.height));
        let padding = i64::from(PADDING);
        let (inner_width, inner_height) = (width - 2 * padding, height - 2 * padding);
        let inside = clip_to(level.clip, x, y, width, height);

        let cursor = window.cursor_rect;
        if let Some(bitmap) = window.caches.cursor.bitmap
            && cursor.width > 0
            && cursor.height > 0
        {
            self.whole_as(
                bitmap,
                x + padding + i64::from(cursor.x),
                y + padding + i64::from(cursor.y),
                inside,
                layer(window.cursor_opacity()),
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
            self.whole_as(
                bitmap,
                x + padding - i64::from(window.ox),
                y + padding - i64::from(window.oy),
                clip,
                layer(window.contents_opacity),
            );
        }

        let Some(skin) = window.windowskin else {
            return;
        };
        let centre_x = x + width / 2 - 8;
        let centre_y = y + height / 2 - 8;
        let mut piece = |from: Rect, x: i64, y: i64, opacity: u8| {
            self.quad(skin, from, &Geometry::at(x, y), inside, layer(opacity));
        };
        if let Some((_, size)) = contents {
            let (ox, oy) = (i64::from(window.ox), i64::from(window.oy));
            // The arrows sit around the 32 by 32 centre of the frame area.
            let (left, top) = (FRAME.0 + CORNER, FRAME.1 + CORNER);
            if i64::from(size.height) - oy > inner_height {
                let from = Rect::new(left + 8, top + 24, 16, 8);
                piece(from, centre_x, y + height - 12, 255);
            }
            if ox > 0 {
                piece(Rect::new(left, top + 8, 8, 16), x + 4, centre_y, 255);
            }
            if i64::from(size.width) - ox > inner_width {
                let from = Rect::new(left + 24, top + 8, 8, 16);
                piece(from, x + width - 12, centre_y, 255);
            }
            if oy > 0 {
                piece(Rect::new(left + 8, top, 16, 8), centre_x, y + 4, 255);
            }
        }
        if let Some((opacity, image)) = window.pause_sign() {
            let image = image as i32;
            let from = Rect::new(
                PAUSE.0 + 16 * (image % 2),
                PAUSE.1 + 16 * (image / 2),
                16,
                16,
            );
            piece(from, centre_x, y + height - 16, opacity);
        }
    }
}

/// The runtime a window belongs to, with what a window of that runtime keeps beside its
/// properties. A window of RGSS3 keeps what one of RGSS2 keeps, and its own beyond that.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Kind {
    Rgss1,
    Rgss2(rgss2::State),
    Rgss3(rgss2::State, rgss3::State),
}

pub(crate) mod rgss2;
pub(crate) mod rgss3;

#[cfg(test)]
mod tests;
