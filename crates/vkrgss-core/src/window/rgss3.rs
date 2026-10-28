//! The window of RGSS3: what it keeps beyond the window of RGSS2, what its calls latch, and
//! the quads it is drawn with.
//!
//! It is the window of RGSS2 (`rgss2`) with these differences. The skin is laid out the
//! same, but the cursor has a border of 4 pixels around a centre of 24 by 24.
//!
//! - A new window has z 100, back opacity 192, a padding and a bottom padding of 8, may show
//!   scroll arrows and has a neutral tone. Neither padding is clamped.
//! - The background is drawn in the normal blend at every openness, after the window's tone
//!   has changed its colours. Nothing else of the window is toned.
//! - The cursor lies at `padding + |cursor_rect.x - ox|`, `padding + |cursor_rect.y - oy|`.
//!   Where it lies and which part of it shows is decided at certain calls and kept until
//!   the next of them.
//! - The contents lie at the padding and are cut by the paddings and by the window.
//! - The scroll arrows are decided at a list of calls of their own, which `update` is not on.
//! - An open window draws background, frame, cursor, arrows, pause sign and contents, in this
//!   order.
//! - A window whose width or height is 0 or less draws no background and no frame, and its
//!   other parts are cut to the one pixel at its origin.
//! - A skin smaller than the layout cuts the centre and the edges of the cursor as it cuts
//!   the background, and a part of the skin that the skin cuts counts its cut width in the
//!   rule of the odd column.
//! - A disposed skin or contents bitmap is drawn as none.
//!
//! What each call latches, in the order it does so:
//!
//! | Call | Contents shown | Arrows decided | Built | Cursor placed |
//! |---|---|---|---|---|
//! | `Update` | yes | | | |
//! | `X`, `Y`, `Active`, `Pause` | on a change | | | |
//! | `Width`, `Height` | yes | yes | background, frame | |
//! | `Ox`, `Oy` | yes | yes | | yes |
//! | `ContentsOpacity`, `Openness` | yes | | | |
//! | `Padding`, `PaddingBottom` | yes | | | yes |
//! | `ArrowsVisible` | | yes | | |
//! | `Windowskin` with another bitmap | yes | yes | background, frame, cursor | yes |
//! | `Contents` with another bitmap | yes | yes | | |
//! | `CursorRect` | on a change | | cursor, on another size | yes |
//!
//! The arrows are decided only while the contents are a live bitmap; otherwise they stay as
//! they were, with the skin they were bound to. `Move` is `X`, `Y`, `Width`, `Height`.
//!
//! Every rule here was measured on the original runtime of VX Ace (`RGSS300.dll` 3.0.0.1 and
//! `RGSS301.dll` 3.0.1.1, window measurement of 2026-10-13, 1185 captures without a wrong
//! pixel); the scenes of `tests/data/window-rgss3-cases.txt` and
//! `tests/data/window-rgss3-random.txt` are a selection of what was measured. The rule for a
//! window without area rests on 14 pixels. Where the original ends with a page fault (the
//! cursor placed in a window that never had a skin assigned; a window or a cursor too large
//! for a bitmap), the calls take effect by these rules and what cannot be built is not
//! drawn. Not measured: sizes above 600 and origins beyond 1000.

use vkrgss_proto::BitmapId;

use super::rgss2::{self, CURSOR, PAUSE, WindowCall, open_rows};
use super::{Kind, clip_to, cursor_pieces, layer};
use crate::extract::{Assembly, Level, Look};
use crate::geometry::Geometry;
use crate::{BitmapError, Bitmaps, Rect, Tone, Window};

/// Width of the cursor's border that keeps its size.
const CURSOR_BORDER: i32 = 4;

/// Where the cursor was placed, relative to the window: its corner, and the part of it that
/// shows.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Placed {
    x: i64,
    y: i64,
    left: i64,
    top: i64,
    width: i64,
    height: i64,
}

/// What a window of RGSS3 keeps beyond what one of RGSS2 keeps.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct State {
    padding: i32,
    padding_bottom: i32,
    arrows_visible: bool,
    tone: Tone,
    /// The cursor as it was placed last; `None` when nothing of it showed then, and before
    /// it was placed.
    cursor: Option<Placed>,
}

impl State {
    pub(crate) fn new() -> State {
        State {
            padding: 8,
            padding_bottom: 8,
            arrows_visible: true,
            tone: Tone::default(),
            cursor: None,
        }
    }
}

/// The cursor: the pieces of RGSS1 from the cursor image of the layout of RGSS2, with a
/// border of 4. What the skin lacks of a piece is cut as the stretch of that piece cuts it.
fn draw_cursor(
    bitmaps: &mut Bitmaps,
    dst: BitmapId,
    skin: BitmapId,
    width: i32,
    height: i32,
    _stretch: bool,
) -> Result<(), BitmapError> {
    let size = (width, height);
    cursor_pieces(bitmaps, dst, skin, size, CURSOR, CURSOR_BORDER, true)
}

impl Window {
    pub(super) fn rgss3(&self) -> Option<&State> {
        match &self.kind {
            Kind::Rgss3(_, rules) => Some(rules),
            Kind::Rgss1 | Kind::Rgss2(_) => None,
        }
    }

    fn rgss3_mut(&mut self) -> Option<&mut State> {
        match &mut self.kind {
            Kind::Rgss3(_, rules) => Some(rules),
            Kind::Rgss1 | Kind::Rgss2(_) => None,
        }
    }

    /// The distance from the window's left, top and right edge to its contents area: 8 in a
    /// new window of RGSS3, and the fixed 16 of a window of RGSS1 or RGSS2.
    pub fn padding(&self) -> i32 {
        self.rgss3().map_or(super::PADDING, |rules| rules.padding)
    }

    /// The distance from the window's bottom edge to its contents area, as
    /// [`Window::padding`].
    pub fn padding_bottom(&self) -> i32 {
        self.rgss3()
            .map_or(super::PADDING, |rules| rules.padding_bottom)
    }

    /// Whether the window may show scroll arrows; always in a window of RGSS1 or RGSS2.
    pub fn arrows_visible(&self) -> bool {
        self.rgss3().is_none_or(|rules| rules.arrows_visible)
    }

    /// The tone of the background of a window of RGSS3; neutral in any other window.
    pub fn tone(&self) -> Tone {
        self.rgss3().map_or_else(Tone::default, |rules| rules.tone)
    }

    /// Sets the tone of the background of a window of RGSS3. It is read when the window is
    /// drawn and latches nothing. A window of RGSS1 or RGSS2 has no tone and ignores it.
    pub fn set_tone(&mut self, tone: Tone) {
        if let Some(rules) = self.rgss3_mut() {
            rules.tone = tone;
        }
    }

    /// One call on a window of RGSS3: the value is stored, then the contents follow
    /// `visible`, the arrows are decided, the bitmaps are built and the cursor is placed,
    /// each when the call is on its list (see the table of the module).
    pub(super) fn call_rgss3(&mut self, bitmaps: &mut Bitmaps, call: WindowCall) {
        match call {
            WindowCall::Update => {
                self.show_contents(true);
                self.advance();
            }
            WindowCall::X(x) => {
                let changed = self.x != x;
                self.x = x;
                self.show_contents(changed);
            }
            WindowCall::Y(y) => {
                let changed = self.y != y;
                self.y = y;
                self.show_contents(changed);
            }
            WindowCall::Width(width) => {
                self.width = width;
                self.show_contents(true);
                self.decide_arrows(bitmaps);
                self.build_back(bitmaps);
            }
            WindowCall::Height(height) => {
                self.height = height;
                self.show_contents(true);
                self.decide_arrows(bitmaps);
                self.build_back(bitmaps);
            }
            WindowCall::Ox(ox) => {
                self.ox = ox;
                self.show_contents(true);
                self.decide_arrows(bitmaps);
                self.place_cursor();
            }
            WindowCall::Oy(oy) => {
                self.oy = oy;
                self.show_contents(true);
                self.decide_arrows(bitmaps);
                self.place_cursor();
            }
            WindowCall::ContentsOpacity(opacity) => {
                self.set_contents_opacity(opacity);
                self.show_contents(true);
            }
            WindowCall::Openness(openness) => {
                if let Kind::Rgss2(state) | Kind::Rgss3(state, _) = &mut self.kind {
                    state.openness = openness.clamp(0, 255) as u8;
                }
                self.show_contents(true);
            }
            WindowCall::Active(active) => {
                let changed = self.store_active(active);
                self.show_contents(changed);
            }
            WindowCall::Pause(pause) => {
                let changed = self.store_pause(pause);
                self.show_contents(changed);
            }
            WindowCall::Windowskin(skin) => {
                if self.windowskin == skin {
                    return;
                }
                self.windowskin = skin;
                self.show_contents(true);
                self.decide_arrows(bitmaps);
                self.build_back(bitmaps);
                self.build_cursor_rgss3(bitmaps);
                self.place_cursor();
            }
            WindowCall::Contents(contents) => {
                if self.contents == contents {
                    return;
                }
                self.contents = contents;
                self.show_contents(true);
                self.decide_arrows(bitmaps);
            }
            WindowCall::CursorRect(rect) => {
                let old = self.cursor_rect;
                self.cursor_rect = rect;
                self.show_contents(old != rect);
                if (old.width, old.height) != (rect.width, rect.height) {
                    self.build_cursor_rgss3(bitmaps);
                }
                self.place_cursor();
            }
            WindowCall::Padding(padding) => {
                if let Some(rules) = self.rgss3_mut() {
                    (rules.padding, rules.padding_bottom) = (padding, padding);
                }
                self.show_contents(true);
                self.place_cursor();
            }
            WindowCall::PaddingBottom(padding) => {
                if let Some(rules) = self.rgss3_mut() {
                    rules.padding_bottom = padding;
                }
                self.show_contents(true);
                self.place_cursor();
            }
            WindowCall::ArrowsVisible(visible) => {
                if let Some(rules) = self.rgss3_mut() {
                    rules.arrows_visible = visible;
                }
                self.decide_arrows(bitmaps);
            }
            // `Window::call` has made the four assignments of it.
            WindowCall::Move(..) => {}
        }
    }

    /// Lets the contents follow `visible` when the call `latches`.
    fn show_contents(&mut self, latches: bool) {
        if let Kind::Rgss2(state) | Kind::Rgss3(state, _) = &mut self.kind
            && latches
        {
            state.contents_shown = self.visible;
        }
    }

    /// The size of the rectangle that cuts cursor, arrows, pause sign and contents: the
    /// window's, or one pixel for a window without area.
    fn cut_size(&self) -> (i64, i64) {
        if self.width > 0 && self.height > 0 {
            (i64::from(self.width), i64::from(self.height))
        } else {
            (1, 1)
        }
    }

    /// Decides the scroll arrows from the values of this moment and binds them to the skin
    /// of this moment. Without live contents they stay as they were, with the skin they had.
    fn decide_arrows(&mut self, bitmaps: &Bitmaps) {
        let Some(contents) = self.contents.and_then(|id| bitmaps.rect(id).ok()) else {
            return;
        };
        let (width, height) = (i64::from(self.width), i64::from(self.height));
        let (ox, oy) = (i64::from(self.ox), i64::from(self.oy));
        let padding = i64::from(self.padding());
        let padding_bottom = i64::from(self.padding_bottom());
        let shown = [
            i64::from(contents.height) - oy > height - padding - padding_bottom,
            ox > 0,
            i64::from(contents.width) - ox > width - 2 * padding,
            oy > 0,
        ];
        let allowed = self.arrows_visible();
        // Halves are rounded towards zero, which matters for a negative size.
        let centre = (width / 2 - 8, height / 2 - 8);
        let skin = self.windowskin;
        if let Kind::Rgss2(state) | Kind::Rgss3(state, _) = &mut self.kind {
            state.arrows =
                rgss2::arrows(shown.map(|shown| shown && allowed), (width, height), centre);
            state.arrow_skin = skin;
        }
    }

    /// Builds the cursor from the skin as it is now.
    fn build_cursor_rgss3(&mut self, bitmaps: &mut Bitmaps) {
        let size = self.cursor_rect;
        let wanted = self.built_from(bitmaps, size.width, size.height);
        self.caches.cursor.rebuild(bitmaps, wanted, draw_cursor);
    }

    /// Places the cursor from the values of this moment: its corner, and the part of it
    /// inside the paddings and the window.
    fn place_cursor(&mut self) {
        let (width, height) = (i64::from(self.width), i64::from(self.height));
        let (cut_width, cut_height) = self.cut_size();
        let padding = i64::from(self.padding());
        let padding_bottom = i64::from(self.padding_bottom());
        let rect = self.cursor_rect;
        let x = padding + (i64::from(rect.x) - i64::from(self.ox)).abs();
        let y = padding + (i64::from(rect.y) - i64::from(self.oy)).abs();
        let left = x.max(padding).max(0);
        let top = y.max(padding).max(0);
        let right = (x + i64::from(rect.width))
            .min(width - padding)
            .min(cut_width);
        let bottom = (y + i64::from(rect.height))
            .min(height - padding_bottom)
            .min(cut_height);
        let placed = (left < right && top < bottom).then_some(Placed {
            x,
            y,
            left,
            top,
            width: right - left,
            height: bottom - top,
        });
        if let Some(rules) = self.rgss3_mut() {
            rules.cursor = placed;
        }
    }
}

impl Assembly<'_> {
    /// A window of RGSS3, all of it at the window's z, from bottom to top: background,
    /// frame, cursor, the scroll arrows in the order down, left, right, up, the pause sign,
    /// and the contents.
    ///
    /// The background is drawn in the normal blend with the window's tone, the frame as in
    /// RGSS2. A window that is not fully open shows these two alone, as RGSS2 shows them.
    /// With a disposed skin nothing that comes from the skin is drawn.
    pub(super) fn window_rgss3(
        &mut self,
        window: &Window,
        state: &rgss2::State,
        rules: &State,
        level: &Level,
    ) {
        if !window.visible {
            return;
        }
        let live = |id: BitmapId| !self.bitmaps.is_disposed(id);
        let skin = window.windowskin.filter(|&id| live(id));
        let arrow_skin = state.arrow_skin.filter(|&id| live(id));
        let x = i64::from(window.x) + level.dx;
        let y = i64::from(window.y) + level.dy;
        let (width, height) = (i64::from(window.width), i64::from(window.height));

        if skin.is_some() && window.width > 0 && window.height > 0 {
            let whole = Rect::new(0, 0, window.width, window.height);
            let back = Look {
                tone: rules.tone.to_proto(),
                ..Look::plain(window.background_opacity())
            };
            let (background, frame) = (window.caches.background.bitmap, window.caches.frame.bitmap);
            if state.openness < 255 {
                let (rows, top) = open_rows(window.height, state.openness);
                if let (Ok(left), Ok(top)) = (i32::try_from(x), i32::try_from(y + i64::from(top))) {
                    let dest =
                        vkrgss_proto::Rect::new(left, top, whole.width.unsigned_abs(), rows as u32);
                    for (bitmap, look) in [(background, back), (frame, Look::plain(window.opacity))]
                    {
                        if let Some(bitmap) = bitmap {
                            self.stretched(bitmap, whole, dest, false, level.clip, look);
                        }
                    }
                }
            } else {
                let at = Geometry::at(x, y);
                if let Some(bitmap) = background {
                    self.quad(bitmap, whole, &at, level.clip, back);
                }
                if let Some(bitmap) = frame {
                    self.quad(bitmap, whole, &at, level.clip, layer(window.opacity));
                }
            }
        }
        if state.openness < 255 {
            return;
        }

        let (cut_width, cut_height) = window.cut_size();
        let inside = clip_to(level.clip, x, y, cut_width, cut_height);
        if let Some(bitmap) = window.caches.cursor.bitmap
            && skin.is_some()
            && let Some(placed) = rules.cursor
            && let Ok(size) = self.bitmaps.rect(bitmap)
        {
            let shown = clip_to(
                inside,
                x + placed.left,
                y + placed.top,
                placed.width,
                placed.height,
            );
            let at = Geometry::at(x + placed.x, y + placed.y);
            let look = layer(window.cursor_opacity_rgss2());
            self.quad(bitmap, size, &at, shown, look);
        }

        // A plain quad cuts a part to what the skin has of it, and the rule of the odd
        // column then counts the cut width, as the original does in RGSS3.
        if let Some(skin) = arrow_skin {
            for arrow in &state.arrows {
                let at = Geometry::at(x + arrow.x, y + arrow.y);
                self.quad(skin, arrow.from, &at, inside, layer(255));
            }
        }
        if let Some(skin) = skin
            && let Some((opacity, image)) = window.pause_sign_rgss2()
        {
            let image = image as i32;
            let from = Rect::new(
                PAUSE.0 + 16 * (image % 2),
                PAUSE.1 + 16 * (image / 2),
                16,
                16,
            );
            // The half is rounded towards zero.
            let at = Geometry::at(x + width / 2 - 8, y + height - 16);
            self.quad(skin, from, &at, inside, layer(opacity));
        }

        if let Some(contents) = window.contents
            && state.contents_shown
            && let Ok(size) = self.bitmaps.rect(contents)
        {
            let padding = i64::from(rules.padding);
            let clip = clip_to(
                inside,
                x + padding,
                y + padding,
                width - 2 * padding,
                height - padding - i64::from(rules.padding_bottom),
            );
            let at = Geometry::at(
                x + padding - i64::from(window.ox),
                y + padding - i64::from(window.oy),
            );
            self.quad(contents, size, &at, clip, layer(window.contents_opacity));
        }
    }
}

#[cfg(test)]
mod tests;
