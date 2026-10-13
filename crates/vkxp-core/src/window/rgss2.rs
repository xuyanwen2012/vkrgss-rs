//! The window of RGSS2: what it keeps, what its calls latch, and the quads it is drawn with.
//!
//! The skin is laid out in 128 by 128 pixels whatever its size: the background at
//! (0, 0, 64, 64), the pattern tiled over it at (0, 64, 64, 64), the frame at (64, 0, 64, 64)
//! with the four scroll arrows in its centre, the cursor at (64, 64, 32, 32) and the four
//! images of the pause sign at (96, 64, 32, 32). Pixels outside the skin count as
//! transparent.
//!
//! A window of RGSS2 differs from one of RGSS1 in more than the layout. It is one element at
//! its z. Background, frame and cursor are bitmaps built at certain calls from the skin as it
//! is then, and are not built again when the pixels of the skin change later. Whether the
//! contents are shown and which scroll arrows are drawn is decided at certain calls too, and
//! stays as decided until the next of them. That is why the properties of such a window are
//! assigned through [`Window::call`].
//!
//! Every rule here was measured on the original runtime of VX (`RGSS202E.dll` 2.0.2.1 and
//! 2.0.0.1, window measurement of 2026-10-12, 490 captures without a wrong pixel); the samples
//! of `tests/data/window-rgss2-samples.txt` and the cases of
//! `tests/data/window-rgss2-cases.txt` are a selection of what was measured. Not measured: a
//! skin narrower or lower than 64 pixels, of which the original draws less of the background
//! than is drawn here, and a disposed skin or contents bitmap.

use vkxp_proto::BitmapId;

use super::{
    CORNER, Key, PADDING, clip_to, cursor_pieces, frame_pieces, layer, stretch_bilinear,
    stretched_part,
};
use crate::extract::{Assembly, Level, Look, intersect};
use crate::geometry::Geometry;
use crate::{BitmapError, Bitmaps, FontDefaults, Rect, Window};

/// Side of the area of the skin that the layout names.
const LAYOUT: i32 = 128;
/// Side of the background block and of the pattern.
const BACK: i32 = 64;
/// Distance from the window's edge to its background.
const BACK_MARGIN: i32 = 2;
/// Left and top of the pattern.
const PATTERN: (usize, usize) = (0, 64);
/// Left and top of the frame area.
const FRAME: (i32, i32) = (64, 0);
/// Where a frame edge starts: where the corner ends.
const EDGE_START: i32 = 16;
/// The cursor image.
const CURSOR: Rect = Rect::new(64, 64, 32, 32);
/// Left and top of the pause sign's four images, each 16 by 16.
const PAUSE: (i32, i32) = (96, 64);
/// Number of updates one image of the pause sign stays.
const PAUSE_PERIOD: u32 = 16;
/// Number of updates of one blink of the cursor.
const BLINK: u8 = 40;
/// Where the blink stands in a window that was made inactive.
const BLINK_INACTIVE: u8 = 16;

/// One call of a script on a window that can change what the window latches: `update`, or
/// the assignment of a property. The properties that latch nothing in either runtime
/// (`visible`, `z`, `opacity`, `back_opacity`, `viewport`) are not among them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowCall {
    Update,
    X(i32),
    Y(i32),
    Width(i32),
    Height(i32),
    Ox(i32),
    Oy(i32),
    /// Clamped to 0 to 255.
    ContentsOpacity(i32),
    /// Clamped to 0 to 255. A window of RGSS1 has no openness and ignores it.
    Openness(i32),
    Active(bool),
    Pause(bool),
    Windowskin(Option<BitmapId>),
    Contents(Option<BitmapId>),
    CursorRect(Rect),
}

/// A scroll arrow as it was decided: where it is drawn, relative to the window, and from
/// which rectangle of the skin.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Arrow {
    x: i64,
    y: i64,
    from: Rect,
}

/// What a window of RGSS2 keeps beside its properties.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct State {
    openness: u8,
    /// Whether the contents are drawn when the window is: the value `visible` had at the
    /// last refresh.
    contents_shown: bool,
    /// The scroll arrows decided at the last refresh that found contents, in the order they
    /// are drawn in, and the skin of that moment, from which they are drawn.
    arrows: Vec<Arrow>,
    arrow_skin: Option<BitmapId>,
}

impl State {
    pub(crate) fn new() -> State {
        State {
            openness: 255,
            contents_shown: true,
            arrows: Vec::new(),
            arrow_skin: None,
        }
    }
}

/// Runs `draw` with a bitmap that holds the layout of `skin`: the skin itself, or for a skin
/// smaller than the layout a copy with transparent pixels where the skin has none, so that
/// every piece is taken as the layout names it.
fn with_layout(
    bitmaps: &mut Bitmaps,
    skin: BitmapId,
    draw: impl FnOnce(&mut Bitmaps, BitmapId) -> Result<(), BitmapError>,
) -> Result<(), BitmapError> {
    let size = bitmaps.rect(skin)?;
    if size.width >= LAYOUT && size.height >= LAYOUT {
        return draw(bitmaps, skin);
    }
    let side = LAYOUT.unsigned_abs();
    let whole = bitmaps.create(side, side, &FontDefaults::new(Vec::new()))?;
    let drawn = bitmaps
        .blt(whole, 0, 0, skin, Rect::new(0, 0, LAYOUT, LAYOUT), 255)
        .and_then(|()| draw(bitmaps, whole));
    bitmaps.dispose(whole);
    drawn
}

/// The background, in a bitmap of the window's size, of which it leaves a margin of 2 pixels
/// transparent: the 64 by 64 block of the skin stretched, and the pattern tiled over it from
/// its top-left corner.
///
/// The pattern changes the colour where both it and the background have an alpha above 0,
/// by the window blend with its own alpha, and leaves the alpha of the background. In the
/// last column of a background of odd width it is blended by the normal formula instead.
///
/// The margin is what lets one bitmap serve the open window and the opening one: the open
/// window draws the background with the transparent column at its right, which makes its
/// rows count as the original counts them (see `draw_background` of RGSS1), and the opening
/// window samples rows of the whole window.
fn draw_background(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    skin: BitmapId,
    width: i32,
    height: i32,
    _stretch: bool,
) -> Result<(), BitmapError> {
    let to = Rect::new(
        BACK_MARGIN,
        BACK_MARGIN,
        width - 2 * BACK_MARGIN,
        height - 2 * BACK_MARGIN,
    );
    with_layout(bitmaps, skin, |bitmaps, skin| {
        if stretched_part(BACK, to.width, to.height) < BACK {
            stretch_bilinear(bitmaps, dst, to, skin, (0, 0), BACK)?;
        } else {
            bitmaps.stretch_blt(dst, to, skin, Rect::new(0, 0, BACK, BACK), 255)?;
        }
        lay_pattern(bitmaps, dst, to, skin)
    })
}

/// Lays the pattern of `skin` over the background in the rectangle `to` of `dst`.
fn lay_pattern(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    to: Rect,
    skin: BitmapId,
) -> Result<(), BitmapError> {
    let target = bitmaps.get(dst)?;
    let Some(area) = target.clip(to) else {
        return Ok(());
    };
    let source = bitmaps.get(skin)?;
    let side = BACK as usize;
    let (left, top) = PATTERN;
    let odd_tail = (to.width & 1 == 1).then_some(area.right - 1);
    let mut pixels = Vec::with_capacity((area.right - area.left) * (area.bottom - area.top));
    for y in area.top..area.bottom {
        for x in area.left..area.right {
            let back = target.pixel(x, y);
            let tile = source.pixel(left + (x - area.left) % side, top + (y - area.top) % side);
            if back[3] == 0 || tile[3] == 0 {
                pixels.push(back);
                continue;
            }
            let [r, g, b, _] = if odd_tail == Some(x) {
                vkxp_pixel::blend_normal([back[0], back[1], back[2], 255], tile, 255)
            } else {
                vkxp_pixel::blend_window(back, tile, 255)
            };
            pixels.push([r, g, b, back[3]]);
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

/// The frame: the pieces of RGSS1 from the frame area of this layout, the edges running
/// from the corners.
fn draw_frame(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    skin: BitmapId,
    width: i32,
    height: i32,
    _stretch: bool,
) -> Result<(), BitmapError> {
    with_layout(bitmaps, skin, |bitmaps, skin| {
        frame_pieces(bitmaps, dst, skin, (width, height), FRAME, EDGE_START)
    })
}

/// The cursor: the pieces of RGSS1 from the cursor image of this layout.
fn draw_cursor(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    skin: BitmapId,
    width: i32,
    height: i32,
    _stretch: bool,
) -> Result<(), BitmapError> {
    with_layout(bitmaps, skin, |bitmaps, skin| {
        cursor_pieces(bitmaps, dst, skin, (width, height), CURSOR)
    })
}

/// The rows an opening window of `height` rows shows at `openness`: how many, and the first
/// of them relative to the window's top. In double precision, as the original computes
/// them: the integer forms are wrong for some values.
pub(crate) fn open_rows(height: i32, openness: u8) -> (i32, i32) {
    let share = f64::from(openness) / 255.0;
    let height = f64::from(height);
    // Both lie between 0 and the height.
    (
        (height * share) as i32,
        (height * (1.0 - share) / 2.0) as i32,
    )
}

impl Window {
    /// Whether this is a window of RGSS2.
    pub fn is_rgss2(&self) -> bool {
        self.rgss2.is_some()
    }

    /// How far the window is open, from 0 to 255; 255 in a window of RGSS1, which is always
    /// open.
    pub fn openness(&self) -> u8 {
        self.rgss2.as_ref().map_or(255, |state| state.openness)
    }

    /// One call of a script on the window.
    ///
    /// On a window of RGSS1 an assignment stores the value and `Update` is
    /// [`Window::update`]. On a window of RGSS2 the call also latches what the original
    /// latches there:
    ///
    /// - A refresh happens in `Update`, in every assignment of `width`, `height`, `ox`, `oy`,
    ///   `contents_opacity` and `openness`, and in an assignment of `x`, `y`, `active`,
    ///   `pause`, `windowskin`, `contents` and `cursor_rect` that changes the value. It
    ///   makes the contents shown when the window is visible at that moment, and with
    ///   contents it decides the scroll arrows anew and binds them to the skin of that
    ///   moment.
    /// - Background and frame are built from the pixels the skin has at that moment when
    ///   another skin is assigned and in every assignment of `width` and `height`; the
    ///   cursor when another skin is assigned and when the size of the cursor changes.
    /// - `Update` advances the blink of the cursor in an active window and the pause sign
    ///   in a pausing one, whether the window is visible, open or has a skin. Making the
    ///   window inactive puts the blink at its sixteenth step, also when it was inactive.
    pub fn call(&mut self, bitmaps: &mut Bitmaps, call: WindowCall) {
        if self.rgss2.is_none() {
            return self.call_rgss1(call);
        }
        match call {
            WindowCall::Update => {
                self.latch(bitmaps);
                if self.active {
                    self.cursor_step = (self.cursor_step + 1) % BLINK;
                }
                if self.pause {
                    // 2^32 and 64 are the same step of the sign: both multiples of its
                    // four images, and both beyond its fading in.
                    self.pause_step = self.pause_step.checked_add(1).unwrap_or(64);
                }
            }
            WindowCall::X(x) => {
                let changed = self.x != x;
                self.x = x;
                self.latch_if(changed, bitmaps);
            }
            WindowCall::Y(y) => {
                let changed = self.y != y;
                self.y = y;
                self.latch_if(changed, bitmaps);
            }
            WindowCall::Width(width) => {
                self.width = width;
                self.latch(bitmaps);
                self.build_back(bitmaps);
            }
            WindowCall::Height(height) => {
                self.height = height;
                self.latch(bitmaps);
                self.build_back(bitmaps);
            }
            WindowCall::Ox(ox) => {
                self.ox = ox;
                self.latch(bitmaps);
            }
            WindowCall::Oy(oy) => {
                self.oy = oy;
                self.latch(bitmaps);
            }
            WindowCall::ContentsOpacity(opacity) => {
                self.set_contents_opacity(opacity);
                self.latch(bitmaps);
            }
            WindowCall::Openness(openness) => {
                if let Some(state) = &mut self.rgss2 {
                    state.openness = openness.clamp(0, 255) as u8;
                }
                self.latch(bitmaps);
            }
            WindowCall::Active(active) => {
                let changed = self.active != active;
                if !active {
                    self.cursor_step = BLINK_INACTIVE;
                }
                self.active = active;
                self.latch_if(changed, bitmaps);
            }
            WindowCall::Pause(pause) => {
                let changed = self.pause != pause;
                if changed && pause {
                    self.pause_step = 0;
                }
                self.pause = pause;
                self.latch_if(changed, bitmaps);
            }
            WindowCall::Windowskin(skin) => {
                let changed = self.windowskin != skin;
                self.windowskin = skin;
                if changed {
                    self.latch(bitmaps);
                    self.build_back(bitmaps);
                    self.build_cursor(bitmaps);
                }
            }
            WindowCall::Contents(contents) => {
                let changed = self.contents != contents;
                self.contents = contents;
                self.latch_if(changed, bitmaps);
            }
            WindowCall::CursorRect(rect) => {
                let old = self.cursor_rect;
                self.cursor_rect = rect;
                self.latch_if(old != rect, bitmaps);
                if (old.width, old.height) != (rect.width, rect.height) {
                    self.build_cursor(bitmaps);
                }
            }
        }
    }

    /// A call on a window of RGSS1, which latches nothing.
    fn call_rgss1(&mut self, call: WindowCall) {
        match call {
            WindowCall::Update => self.update(),
            WindowCall::X(x) => self.x = x,
            WindowCall::Y(y) => self.y = y,
            WindowCall::Width(width) => self.width = width,
            WindowCall::Height(height) => self.height = height,
            WindowCall::Ox(ox) => self.ox = ox,
            WindowCall::Oy(oy) => self.oy = oy,
            WindowCall::ContentsOpacity(opacity) => self.set_contents_opacity(opacity),
            WindowCall::Openness(_) => {}
            WindowCall::Active(active) => self.set_active(active),
            WindowCall::Pause(pause) => self.set_pause(pause),
            WindowCall::Windowskin(skin) => self.windowskin = skin,
            WindowCall::Contents(contents) => self.contents = contents,
            WindowCall::CursorRect(rect) => self.cursor_rect = rect,
        }
    }

    fn latch_if(&mut self, changed: bool, bitmaps: &Bitmaps) {
        if changed {
            self.latch(bitmaps);
        }
    }

    /// The refresh: the contents follow `visible`, and with contents the scroll arrows are
    /// decided from the values of this moment and bound to the skin of this moment. Without
    /// contents the arrows stay as they were, with the skin they had.
    fn latch(&mut self, bitmaps: &Bitmaps) {
        let contents = self.contents.and_then(|id| bitmaps.rect(id).ok());
        let Some(state) = &mut self.rgss2 else {
            return;
        };
        state.contents_shown = self.visible;
        let Some(contents) = contents else {
            return;
        };
        let (width, height) = (i64::from(self.width), i64::from(self.height));
        let (ox, oy) = (i64::from(self.ox), i64::from(self.oy));
        let padding = 2 * i64::from(PADDING);
        // Halves are rounded down, also of a negative size.
        let centre_x = width.div_euclid(2) - 8;
        let centre_y = height.div_euclid(2) - 8;
        // The arrows sit around the 32 by 32 centre of the frame area.
        let (left, top) = (FRAME.0 + CORNER, FRAME.1 + CORNER);
        let arrow = |shown: bool, x: i64, y: i64, from: Rect| shown.then_some(Arrow { x, y, from });
        state.arrows = [
            arrow(
                i64::from(contents.height) - oy > height - padding,
                centre_x,
                height - 12,
                Rect::new(left + 8, top + 24, 16, 8),
            ),
            arrow(ox > 0, 4, centre_y, Rect::new(left, top + 8, 8, 16)),
            arrow(
                i64::from(contents.width) - ox > width - padding,
                width - 12,
                centre_y,
                Rect::new(left + 24, top + 8, 8, 16),
            ),
            arrow(oy > 0, centre_x, 4, Rect::new(left + 8, top, 16, 8)),
        ]
        .into_iter()
        .flatten()
        .collect();
        state.arrow_skin = self.windowskin;
    }

    /// What a bitmap of `width` by `height` is built from: nothing without a live skin or
    /// without pixels.
    fn built_from(&self, bitmaps: &Bitmaps, width: i32, height: i32) -> Option<Key> {
        let skin = self.windowskin?;
        let revision = bitmaps.revision(skin).ok()?;
        (width > 0 && height > 0).then_some((skin, revision, width, height, false))
    }

    /// Builds background and frame from the skin as it is now.
    fn build_back(&mut self, bitmaps: &mut Bitmaps) {
        let whole = self.built_from(bitmaps, self.width, self.height);
        let inside =
            whole.filter(|_| self.width > 2 * BACK_MARGIN && self.height > 2 * BACK_MARGIN);
        self.caches
            .background
            .rebuild(bitmaps, inside, draw_background);
        self.caches.frame.rebuild(bitmaps, whole, draw_frame);
    }

    /// Builds the cursor from the skin as it is now.
    fn build_cursor(&mut self, bitmaps: &mut Bitmaps) {
        let size = self.cursor_rect;
        let wanted = self.built_from(bitmaps, size.width, size.height);
        self.caches.cursor.rebuild(bitmaps, wanted, draw_cursor);
    }

    /// The opacity of the cursor of a window of RGSS2: the contents opacity less 8 for each
    /// step the blink is away from its start, at most 20 steps either way.
    pub(super) fn cursor_opacity_rgss2(&self) -> u8 {
        let step = match self.cursor_step {
            step @ 0..=20 => step,
            step => BLINK.saturating_sub(step),
        };
        self.contents_opacity.saturating_sub(8 * step)
    }

    /// The opacity and the image of the pause sign of a window of RGSS2, or `None` while it
    /// is not shown: it fades in by 32 per update from the update after the flag was set,
    /// and shows each of its four images for 16 updates.
    pub(super) fn pause_sign_rgss2(&self) -> Option<(u8, u32)> {
        if !self.pause || self.pause_step == 0 {
            return None;
        }
        let opacity = self.pause_step.saturating_mul(32).min(255) as u8;
        Some((opacity, self.pause_step / PAUSE_PERIOD % 4))
    }
}

impl Assembly<'_> {
    /// Draws the rectangle `from` of a skin, which the skin may hold only in part, at
    /// (`x`, `y`) in the window blend.
    ///
    /// The original blends the last column of a row by another formula when the row shows
    /// an odd number of columns of `from`, whether the skin has them or not. A quad of the
    /// part the skin has counts its own columns, so where the two disagree, which is where
    /// the row goes on beyond what the skin has, the part is drawn as a stretch of its own
    /// size, which takes the window blend in every column.
    fn skin_part(
        &mut self,
        skin: BitmapId,
        from: Rect,
        x: i64,
        y: i64,
        clip: vkxp_proto::Rect,
        opacity: u8,
    ) {
        let Ok(bounds) = self.bitmaps.rect(skin) else {
            return;
        };
        let held = intersect(from.to_proto(), bounds.to_proto());
        let end = x + i64::from(held.width);
        let shown_end = (x + i64::from(from.width)).min(i64::from(clip.x) + i64::from(clip.width));
        if held.is_empty() || shown_end <= end {
            self.quad(skin, from, &Geometry::at(x, y), clip, layer(opacity));
            return;
        }
        let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) else {
            return;
        };
        let part = Rect::new(
            held.x,
            held.y,
            held.width.cast_signed(),
            held.height.cast_signed(),
        );
        let dest = vkxp_proto::Rect::new(x, y, held.width, held.height);
        self.stretched(skin, part, dest, false, clip, layer(opacity));
    }

    /// A window of RGSS2, all of it at the window's z, from bottom to top: background,
    /// frame, cursor, contents, the scroll arrows in the order down, left, right, up, and
    /// the pause sign.
    ///
    /// A window that is not fully open shows background and frame alone, in the normal
    /// blend: `n` rows from row `top` of its place ([`open_rows`]), screen row `top + i`
    /// showing the window's row `stretch_index(i, height, n)`, which is a quad stretched
    /// from all rows of the window into those `n`.
    pub(crate) fn window_rgss2(&mut self, window: &Window, level: &Level) {
        let Some(state) = &window.rgss2 else {
            return;
        };
        if !window.visible || window.width <= 0 || window.height <= 0 {
            return;
        }
        let x = i64::from(window.x) + level.dx;
        let y = i64::from(window.y) + level.dy;
        let (width, height) = (i64::from(window.width), i64::from(window.height));
        let whole = Rect::new(0, 0, window.width, window.height);
        let back_opacity = window.background_opacity();

        if state.openness < 255 {
            let (rows, top) = open_rows(window.height, state.openness);
            let (Ok(left), Ok(top)) = (i32::try_from(x), i32::try_from(y + i64::from(top))) else {
                return;
            };
            let dest = vkxp_proto::Rect::new(left, top, whole.width.unsigned_abs(), rows as u32);
            for (bitmap, opacity) in [
                (window.caches.background.bitmap, back_opacity),
                (window.caches.frame.bitmap, window.opacity),
            ] {
                if let Some(bitmap) = bitmap {
                    self.stretched(bitmap, whole, dest, false, level.clip, Look::plain(opacity));
                }
            }
            return;
        }

        if let Some(bitmap) = window.caches.background.bitmap {
            let margin = BACK_MARGIN;
            // One transparent column at the right: see `draw_background`.
            let inside = Rect::new(
                margin,
                margin,
                window.width - 2 * margin + 1,
                window.height - 2 * margin,
            );
            let at = Geometry::at(x + i64::from(margin), y + i64::from(margin));
            self.quad(bitmap, inside, &at, level.clip, layer(back_opacity));
        }
        if let Some(bitmap) = window.caches.frame.bitmap {
            self.quad(
                bitmap,
                whole,
                &Geometry::at(x, y),
                level.clip,
                layer(window.opacity),
            );
        }

        let padding = i64::from(PADDING);
        let inside = clip_to(level.clip, x, y, width, height);
        let cursor = window.cursor_rect;
        if let Some(bitmap) = window.caches.cursor.bitmap
            && window.windowskin.is_some()
        {
            let at = Geometry::at(
                x + padding + i64::from(cursor.x),
                y + padding + i64::from(cursor.y),
            );
            let size = Rect::new(0, 0, cursor.width, cursor.height);
            let look = layer(window.cursor_opacity_rgss2());
            self.quad(bitmap, size, &at, inside, look);
        }

        if let Some(contents) = window.contents
            && state.contents_shown
            && let Ok(size) = self.bitmaps.rect(contents)
        {
            let clip = clip_to(
                level.clip,
                x + padding,
                y + padding,
                width - 2 * padding,
                height - 2 * padding,
            );
            let at = Geometry::at(
                x + padding - i64::from(window.ox),
                y + padding - i64::from(window.oy),
            );
            let look = layer(window.contents_opacity);
            self.quad(contents, size, &at, clip, look);
        }

        if let Some(skin) = state.arrow_skin {
            for arrow in &state.arrows {
                self.skin_part(skin, arrow.from, x + arrow.x, y + arrow.y, inside, 255);
            }
        }
        if let Some(skin) = window.windowskin
            && let Some((opacity, image)) = window.pause_sign_rgss2()
        {
            let image = image as i32;
            let from = Rect::new(
                PAUSE.0 + 16 * (image % 2),
                PAUSE.1 + 16 * (image / 2),
                16,
                16,
            );
            let (x, y) = (x + width / 2 - 8, y + height - 16);
            self.skin_part(skin, from, x, y, inside, opacity);
        }
    }
}

#[cfg(test)]
mod tests;
