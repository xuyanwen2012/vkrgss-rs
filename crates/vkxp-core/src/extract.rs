//! Frame extraction: the scene and the bitmap arena become a protocol frame packet.

use vkxp_proto::{
    BitmapId, BlendMode, FramePacket, FrameRate, Op, Quad, RectEffect, Sampling, Transform,
};

use crate::element::Order;
use crate::geometry::{self, Geometry};
use crate::scene::Arena;
use crate::scene::Key;
use crate::tilemap::Strip;
use crate::{Bitmaps, Rect, Rules, Scene, Sprite, Viewport, ViewportId};

/// The pixels both rectangles cover; a rectangle without pixels when they share none.
pub(crate) fn intersect(a: vkxp_proto::Rect, b: vkxp_proto::Rect) -> vkxp_proto::Rect {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (i64::from(a.x) + i64::from(a.width)).min(i64::from(b.x) + i64::from(b.width));
    let bottom = (i64::from(a.y) + i64::from(a.height)).min(i64::from(b.y) + i64::from(b.height));
    if i64::from(left) < right && i64::from(top) < bottom {
        // Both differences are at most the sides of `a`.
        vkxp_proto::Rect::new(
            left,
            top,
            (right - i64::from(left)) as u32,
            (bottom - i64::from(top)) as u32,
        )
    } else {
        vkxp_proto::Rect::new(left, top, 0, 0)
    }
}

/// Where the elements of one level are drawn: the screen itself, or a viewport.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Level {
    /// No element draws outside it. Never empty.
    pub(crate) clip: vkxp_proto::Rect,
    /// Added to the position of every element.
    pub(crate) dx: i64,
    pub(crate) dy: i64,
    /// The origin of the viewport, which `dx` and `dy` hold subtracted; 0 for the screen.
    pub(crate) ox: i64,
    pub(crate) oy: i64,
}

/// Where the elements of `viewport` are drawn, or of the screen for `None`. No level when
/// they are not drawn: the viewport is disposed, invisible or has no pixel on the screen.
pub(crate) fn level_of(
    viewports: &Arena<Viewport>,
    logical_size: vkxp_proto::Size,
    viewport: Option<ViewportId>,
) -> Option<Level> {
    let screen = vkxp_proto::Rect::from_size(logical_size);
    let Some(id) = viewport else {
        return Some(Level {
            clip: screen,
            dx: 0,
            dy: 0,
            ox: 0,
            oy: 0,
        });
    };
    let viewport = viewports.get(id.0)?;
    let clip = intersect(viewport.rect.to_proto(), screen);
    (viewport.visible && !clip.is_empty()).then(|| Level {
        clip,
        dx: i64::from(viewport.rect.x) - i64::from(viewport.ox),
        dy: i64::from(viewport.rect.y) - i64::from(viewport.oy),
        ox: i64::from(viewport.ox),
        oy: i64::from(viewport.oy),
    })
}

/// The rows of `clip` from `top` up to `bottom`; a rectangle without pixels when there are
/// none.
pub(crate) fn rows(clip: vkxp_proto::Rect, top: i64, bottom: i64) -> vkxp_proto::Rect {
    let first = top.max(i64::from(clip.y));
    let end = bottom.min(i64::from(clip.y) + i64::from(clip.height));
    if first < end {
        // Both lie inside `clip`.
        vkxp_proto::Rect::new(clip.x, first as i32, clip.width, (end - first) as u32)
    } else {
        vkxp_proto::Rect::new(clip.x, clip.y, clip.width, 0)
    }
}

/// A run of rows of a sprite that is not turned: `length` source rows from row `start` of
/// the bitmap on, stretched into `pixels` screen rows from row `first` of the sprite on.
/// Neither is 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rows {
    pub(crate) start: i64,
    pub(crate) length: u32,
    pub(crate) first: u32,
    pub(crate) pixels: u32,
}

/// The rows of a sprite that is not turned, as its bush depth splits them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Bush {
    /// The number of screen rows drawn with half the opacity.
    pub(crate) faint: u32,
    /// The rows above the faint ones; all rows when none is faint.
    pub(crate) upper: Option<Rows>,
    /// The faint rows, when they show anything.
    pub(crate) lower: Option<Rows>,
}

impl Bush {
    /// Splits the `h` rows of `src`, drawn `dh` screen rows high, by the bush depth, as the
    /// original runtime does:
    ///
    /// ```text
    /// bd = min(depth, dh)        the faint screen rows; none for a depth of 0 or less
    /// sb = bd * h / dh           the source rows they show
    /// upper: source rows [0, h - sb) stretched into screen rows [0, dh - bd)
    /// lower: source rows [h - sb, h) stretched into screen rows [dh - bd, dh)
    /// ```
    ///
    /// each stretched on its own. With `sb` of 0 the faint rows are not drawn at all.
    ///
    /// Provenance: oracle measurement 2026-10-10, RGSS103J, probes `bushzoom`, `geom2` and
    /// `bush` (36 zoomed sprites, without a differing pixel; opaque texels at opacity 255
    /// only).
    pub(crate) fn new(src: Rect, dh: u32, depth: i32) -> Bush {
        let h = src.height.max(0).unsigned_abs();
        let faint = depth.max(0).unsigned_abs().min(dh);
        // At most `h`.
        let shown = if dh == 0 {
            0
        } else {
            (u64::from(faint) * u64::from(h) / u64::from(dh)) as u32
        };
        let run = |start: u32, length: u32, first: u32, pixels: u32| {
            (length > 0 && pixels > 0).then(|| Rows {
                start: i64::from(src.y) + i64::from(start),
                length,
                first,
                pixels,
            })
        };
        Bush {
            faint,
            upper: run(0, h - shown, 0, dh - faint),
            lower: run(h - shown, shown, dh - faint, faint),
        }
    }
}

/// One axis of a stretch whose source is cut: `src` (start and length) is stretched into
/// `dest`, and only `inside`, a part of `src`, may be drawn. The pixels of `dest` that show a
/// texel of `inside`, as start and length, or `None` when none does. `mirror` counts the
/// texels from the far end of `src`.
fn cut_span(
    src: (i32, u32),
    inside: (i32, u32),
    dest: (i32, u32),
    mirror: bool,
) -> Option<(i32, u32)> {
    if inside == src {
        return Some(dest);
    }
    // The texels that may show, counted from the start of `src` in the order of the pixels.
    let low = u64::from(inside.0.abs_diff(src.0));
    let high = low + u64::from(inside.1);
    let length = u64::from(src.1);
    let (low, high) = if mirror {
        (length - high, length - low)
    } else {
        (low, high)
    };
    let step = vkxp_proto::stretch_step(src.1, dest.1);
    // The first pixel whose texel is at least `texel`: the texels never decrease.
    let first_at = |texel: u64| match (texel, step) {
        (0, _) => 0,
        (texel, _) if texel >= length => u64::from(dest.1),
        (_, 0) => u64::from(dest.1),
        (texel, step) => (texel << vkxp_proto::STRETCH_FRAC_BITS)
            .div_ceil(step)
            .min(u64::from(dest.1)),
    };
    let (first, end) = (first_at(low), first_at(high));
    let start = i32::try_from(i64::from(dest.0) + first as i64).ok()?;
    // At most the length of `dest`.
    (first < end).then_some((start, (end - first) as u32))
}

impl Sprite {
    /// The pixels a sprite that is not turned covers inside `level`, before the level clips
    /// it, or `None` when it covers none or begins beyond the coordinates of the protocol.
    pub(crate) fn upright_dest(&self, level: &Level) -> Option<vkxp_proto::Rect> {
        let src = self.src_rect;
        let width = geometry::stretch_length(src.width, self.zoom_x)?;
        let height = geometry::stretch_length(src.height, self.zoom_y)?;
        let x = geometry::stretch_start(self.x, self.ox, self.zoom_x)?;
        let y = geometry::stretch_start(self.y, self.oy, self.zoom_y)?;
        let x = i32::try_from(i64::from(x) + level.dx).ok()?;
        let y = i32::try_from(i64::from(y) + level.dy).ok()?;
        Some(vkxp_proto::Rect::new(x, y, width, height))
    }
}

/// The blend mode a blend type selects: 1 adds, 2 subtracts, every other value blends
/// normally.
pub(crate) fn blend_mode(blend_type: i32) -> BlendMode {
    match blend_type {
        1 => BlendMode::Add,
        2 => BlendMode::Subtract,
        _ => BlendMode::Normal,
    }
}

/// How a quad looks, apart from where it is.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Look {
    pub(crate) tone: vkxp_proto::Tone,
    pub(crate) color: vkxp_proto::Color,
    pub(crate) opacity: u8,
    pub(crate) blend: BlendMode,
}

impl Look {
    /// Unchanged colours, normal blending.
    pub(crate) fn plain(opacity: u8) -> Look {
        Look {
            tone: vkxp_proto::Tone::NEUTRAL,
            color: vkxp_proto::Color::default(),
            opacity,
            blend: BlendMode::Normal,
        }
    }
}

/// The frame being assembled.
pub(crate) struct Assembly<'a> {
    pub(crate) bitmaps: &'a Bitmaps,
    /// The rules of the scene's runtime.
    pub(crate) rules: Rules,
    pub(crate) ops: Vec<Op>,
    /// The bitmap of every quad and the sheets of every tilemap layer in `ops`.
    pub(crate) referenced: Vec<BitmapId>,
}

impl Assembly<'_> {
    /// Draws `src` of `bitmap` through `geometry`, if anything of it can show.
    ///
    /// The protocol wants the source rectangle inside the bitmap, so the part of `src` outside
    /// is cut off here and the transform moved by what was cut at the left and top; the part
    /// that is left stays where it was. A disposed bitmap, an empty rectangle, an empty clip
    /// and an opacity of 0 draw nothing.
    pub(crate) fn quad(
        &mut self,
        bitmap: BitmapId,
        src: Rect,
        geometry: &Geometry,
        clip: vkxp_proto::Rect,
        look: Look,
    ) {
        self.part(bitmap, src, src, geometry, clip, look);
    }

    /// Draws the pixels of `part`, a rectangle inside `src`, where a quad of `src` through
    /// `geometry` shows them, and nothing of the rest of `src`.
    fn part(
        &mut self,
        bitmap: BitmapId,
        src: Rect,
        part: Rect,
        geometry: &Geometry,
        clip: vkxp_proto::Rect,
        look: Look,
    ) {
        self.transformed(bitmap, src, part, clip, look, |cut_left, cut_top| {
            geometry.transform(f64::from(src.width), cut_left, cut_top)
        });
    }

    /// Draws the pixels of `part`, a rectangle inside `src`, through the transform that
    /// `transform_of` gives for what was cut off `src` at its left and top.
    fn transformed(
        &mut self,
        bitmap: BitmapId,
        src: Rect,
        part: Rect,
        clip: vkxp_proto::Rect,
        look: Look,
        transform_of: impl Fn(f64, f64) -> Option<Transform>,
    ) {
        if look.opacity == 0 || clip.is_empty() {
            return;
        }
        let Ok(bounds) = self.bitmaps.rect(bitmap) else {
            return;
        };
        let inside = intersect(part.to_proto(), bounds.to_proto());
        if inside.is_empty() {
            return;
        }
        let cut_left = i64::from(inside.x) - i64::from(src.x);
        let cut_top = i64::from(inside.y) - i64::from(src.y);
        // Cuts are at most a rectangle side, which `f64` holds exactly.
        let Some(transform) = transform_of(cut_left as f64, cut_top as f64) else {
            return;
        };
        self.referenced.push(bitmap);
        self.ops.push(Op::Quad(Quad {
            clip,
            bitmap,
            src: inside,
            sampling: Sampling::Affine(transform),
            tone: look.tone,
            color: look.color,
            opacity: look.opacity,
            blend: look.blend,
        }));
    }

    /// Draws `src` of `bitmap` stretched into `dest`, mirrored or not, if anything of it can
    /// show inside `clip`.
    ///
    /// The protocol wants the source rectangle inside the bitmap, so the part of `src` outside
    /// is cut off and `dest` with it, down to the pixels that show what is left. The stretch
    /// of what is left starts anew at its corner, which is not what the original runtime
    /// draws of a sprite: a stretched sprite with such a rectangle does not come here but is
    /// drawn from a picture of its own ([`Assembly::cut_sprite`]).
    pub(crate) fn stretched(
        &mut self,
        bitmap: BitmapId,
        src: Rect,
        dest: vkxp_proto::Rect,
        mirror: bool,
        clip: vkxp_proto::Rect,
        look: Look,
    ) {
        let clip = intersect(clip, dest);
        if look.opacity == 0 || clip.is_empty() {
            return;
        }
        let Ok(bounds) = self.bitmaps.rect(bitmap) else {
            return;
        };
        let src = src.to_proto();
        let inside = intersect(src, bounds.to_proto());
        if inside.is_empty() {
            return;
        }
        let across = cut_span(
            (src.x, src.width),
            (inside.x, inside.width),
            (dest.x, dest.width),
            mirror,
        );
        let down = cut_span(
            (src.y, src.height),
            (inside.y, inside.height),
            (dest.y, dest.height),
            false,
        );
        let (Some((x, width)), Some((y, height))) = (across, down) else {
            return;
        };
        self.referenced.push(bitmap);
        self.ops.push(Op::Quad(Quad {
            clip,
            bitmap,
            src: inside,
            sampling: Sampling::Stretch {
                dest: vkxp_proto::Rect::new(x, y, width, height),
                mirror,
            },
            tone: look.tone,
            color: look.color,
            opacity: look.opacity,
            blend: look.blend,
        }));
    }

    fn sprite(&mut self, sprite: &Sprite, level: &Level) {
        let Some(bitmap) = sprite.bitmap else {
            return;
        };
        if !sprite.visible || sprite.flash.hides() {
            return;
        }
        let look = Look {
            tone: sprite.tone.to_proto(),
            color: sprite.flash.effective_color(sprite.color),
            opacity: sprite.opacity,
            blend: blend_mode(sprite.blend_type),
        };
        // Only an angle of exactly 0 is not turned: a full turn is.
        if sprite.angle == 0.0 {
            match sprite.rules {
                Rules::Rgss1 => self.upright_sprite(sprite, bitmap, level, look),
                Rules::Rgss2 | Rules::Rgss3 => {
                    self.upright_sprite_rgss2(sprite, bitmap, level, look)
                }
            }
        } else {
            self.turned_sprite(sprite, level, look);
        }
    }

    /// A sprite that is not turned: its source rectangle stretched into a rectangle of whole
    /// pixels, as measured on the original runtime. The lowest rows of that rectangle, as
    /// many as the bush depth whatever the zoom, are drawn with half the opacity, and are a
    /// stretch of their own ([`Bush::new`]).
    fn upright_sprite(&mut self, sprite: &Sprite, bitmap: BitmapId, level: &Level, look: Look) {
        let Ok(bounds) = self.bitmaps.rect(bitmap) else {
            return;
        };
        if sprite.is_cut(bounds) {
            self.cut_sprite(sprite, level, look);
            return;
        }
        let Some(dest) = sprite.upright_dest(level) else {
            return;
        };
        let src = sprite.src_rect;
        let bush = Bush::new(src, dest.height, sprite.bush_depth);
        let faint = Look {
            opacity: look.opacity >> 1,
            ..look
        };
        for (rows, look) in [(bush.upper, look), (bush.lower, faint)] {
            let Some(rows) = rows else {
                continue;
            };
            // Rows that begin beyond the coordinates of a rectangle are not drawn.
            let (Ok(top), Ok(y), Ok(height)) = (
                i32::try_from(rows.start),
                i32::try_from(i64::from(dest.y) + i64::from(rows.first)),
                i32::try_from(rows.length),
            ) else {
                continue;
            };
            let part = Rect::new(src.x, top, src.width, height);
            let into = vkxp_proto::Rect::new(dest.x, y, dest.width, rows.pixels);
            self.stretched(bitmap, part, into, sprite.mirror, level.clip, look);
        }
    }

    /// Applies the viewport's tone and colour to everything drawn so far inside `clip`.
    ///
    /// RGSS1: the flash colour stands in for the colour by the rule of a sprite
    /// ([`Flash::effective_color`](crate::element::Flash::effective_color)).
    ///
    /// RGSS2: the tone, then the colour, then the flash colour at its current alpha, each
    /// laid on by `s + (((c - s) * (a + 1)) >> 8)`; the flash colour is a rectangle effect
    /// of its own.
    ///
    /// Provenance of RGSS2: oracle measurement 2026-10-12, RGSS202E versions 2.0.0.1 and
    /// 2.0.2.1, `vx-v1-sprite-REPORT.md` section 4 (topics `viewport` and `flash`, without a
    /// differing pixel; the rule of RGSS1 leaves 107256 wrong on the 6 flash frames).
    fn viewport_effect(&mut self, viewport: &Viewport, clip: vkxp_proto::Rect) {
        let tone = viewport.tone.to_proto();
        let (color, flash) = match self.rules {
            Rules::Rgss1 => (viewport.flash.effective_color(viewport.color), None),
            Rules::Rgss2 | Rules::Rgss3 => (viewport.color.to_proto(), viewport.flash.color()),
        };
        if tone != vkxp_proto::Tone::NEUTRAL || color.a != 0 {
            self.ops
                .push(Op::RectEffect(RectEffect { clip, tone, color }));
        }
        if let Some(color) = flash.filter(|color| color.a != 0) {
            let tone = vkxp_proto::Tone::NEUTRAL;
            self.ops
                .push(Op::RectEffect(RectEffect { clip, tone, color }));
        }
    }
}

/// Something that takes one place in the draw order of its level.
#[derive(Debug, Clone, Copy)]
enum Item {
    Sprite(Key),
    Plane(Key),
    /// One piece of a tilemap: its ground, its flashing cells, or a row of tiles of one
    /// priority.
    Tiles(Key, Strip),
    /// The frame and background of a window.
    WindowBack(Key),
    /// What a window shows inside: cursor, contents, pause sign and arrows.
    WindowFront(Key),
    /// A window of RGSS2, which is one element.
    Window(Key),
    Viewport(Key),
}

/// An item with what it is ordered by.
#[derive(Debug, Clone, Copy)]
struct Entry {
    z: i64,
    /// What orders items of equal z before their stamps do: the y of a sprite of RGSS2, and
    /// 0 for every other item.
    y: i32,
    stamp: u64,
    item: Item,
}

impl Entry {
    fn new(order: Order, item: Item) -> Entry {
        Entry {
            z: i64::from(order.z),
            y: 0,
            stamp: order.stamp,
            item,
        }
    }
}

/// Sorts into draw order: ascending z, among equal z ascending y, and among those ascending
/// stamp.
///
/// The stamp is that of creation, so among equal z the element created later is in front.
/// The y is 0 for every item of a scene of RGSS1, whose order is (z, creation) alone.
///
/// Provenance: oracle measurement, RGSS103J, z order probe (`ph-scene-zorder-probe.rb`): the
/// key (z, creation) predicts all 109 observations; keys that follow z assignments do not.
///
/// Provenance of RGSS2: oracle measurement 2026-10-12, RGSS202E versions 2.0.0.1 and 2.0.2.1,
/// `vx-v1-sprite-REPORT.md` section 3 (topic `order`, 36 cases without a differing pixel; the
/// key of RGSS1 leaves 13472 wrong): the key is (z, y, creation), y being the property of a
/// sprite, not where it is drawn, and 0 for a plane, a viewport and a window.
fn sort(entries: &mut [Entry]) {
    entries.sort_by_key(|entry| (entry.z, entry.y, entry.stamp));
}

impl Scene {
    /// Number of frames, as `Graphics.frame_count`: it starts at 0 and every extracted frame
    /// adds 1.
    pub fn frame_count(&self) -> i64 {
        self.frame_count
    }

    pub fn set_frame_count(&mut self, frame_count: i64) {
        self.frame_count = frame_count;
    }

    /// The frame rate every extracted packet carries, 40 until it is set.
    pub fn frame_rate(&self) -> FrameRate {
        self.frame_rate
    }

    /// Sets the frame rate, clamped to 10 to 120.
    pub fn set_frame_rate(&mut self, fps: i64) {
        self.frame_rate = FrameRate::clamped(fps);
    }

    /// The brightness every extracted packet carries, 255 until it is set.
    pub fn brightness(&self) -> u8 {
        self.brightness
    }

    /// Sets the brightness, clamped to 0 to 255.
    pub fn set_brightness(&mut self, brightness: i64) {
        self.brightness = brightness.clamp(0, 255) as u8;
    }

    /// Takes the compositor to have none of the scene's tables, so that the next frame sends
    /// every table its tilemap ops read. For a compositor that starts anew.
    pub fn resend_tables(&mut self) {
        self.tables = Default::default();
    }

    /// Assembles the next frame: the ops of every visible element in draw order, and before
    /// them the bitmap deltas for exactly the bitmaps those ops refer to and the table deltas
    /// for the tables the tilemap ops read.
    ///
    /// The packets of one scene must reach a compositor in the order they were extracted,
    /// because the bitmap arena and the scene take every delta they hand out to be delivered.
    pub fn extract(&mut self, bitmaps: &mut Bitmaps) -> FramePacket {
        self.refresh(bitmaps);
        let tables = self.tables.finish();
        let (ops, referenced) = self.ops(bitmaps);
        // Every op was checked to name live bitmaps only, which is all `deltas` requires.
        let deltas = bitmaps.deltas(&referenced).unwrap_or_default();

        let frame_index = self.frame_index;
        self.frame_index += 1;
        self.frame_count = self.frame_count.wrapping_add(1);
        FramePacket {
            frame_index,
            logical_size: self.logical_size,
            frame_rate: self.frame_rate,
            brightness: self.brightness,
            tables,
            bitmaps: deltas,
            ops,
        }
    }

    /// The scene as it is now in a packet that stands alone: the ops and the brightness of
    /// the frame [`Scene::extract`] would assemble, with every bitmap and table they refer
    /// to created and sent whole, for a compositor that holds nothing and is not the one the
    /// extracted packets go to.
    ///
    /// Nothing that compositor is owed changes: the frame count and the index of the next
    /// packet stay, no bitmap or table is taken to be delivered, and the next extracted
    /// packet is the one it would have been. The packet carries the index of that next
    /// packet.
    pub fn snapshot(&mut self, bitmaps: &mut Bitmaps) -> FramePacket {
        // With no table taken to be sent, every table an op reads is sent whole. The ops
        // name the tables by the ids of this packet; the next refresh names them anew.
        let sent = std::mem::take(&mut self.tables);
        self.refresh(bitmaps);
        let tables = std::mem::replace(&mut self.tables, sent).finish();
        let (ops, referenced) = self.ops(bitmaps);
        FramePacket {
            frame_index: self.frame_index,
            logical_size: self.logical_size,
            frame_rate: self.frame_rate,
            brightness: self.brightness,
            tables,
            bitmaps: bitmaps.whole(&referenced),
            ops,
        }
    }

    /// Brings what the elements keep for drawing up to date: the pictures of windows, turned
    /// sprites and planes, and what each tilemap shows, with its tables noted in
    /// `self.tables`.
    fn refresh(&mut self, bitmaps: &mut Bitmaps) {
        for (_, window) in self.windows.iter_mut() {
            window.refresh_caches(bitmaps);
        }
        for id in self.released.drain(..) {
            bitmaps.dispose(id);
        }
        // An element without a level is not drawn, but still gives its cache back when it is
        // invisible itself.
        for (_, sprite) in self.sprites.iter_mut() {
            let level = level_of(&self.viewports, self.logical_size, sprite.viewport);
            sprite.refresh_turned(bitmaps, level.as_ref());
        }
        self.notice_viewport_origins();
        for (_, tilemap) in self.tilemaps.iter_mut() {
            let level = level_of(&self.viewports, self.logical_size, tilemap.viewport);
            tilemap.refresh(bitmaps, level.as_ref(), &mut self.tables);
        }
        for (_, plane) in self.planes.iter_mut() {
            let level = level_of(&self.viewports, self.logical_size, plane.viewport);
            plane.refresh_cache(bitmaps, self.rules, level.as_ref());
        }
    }
}

impl Scene {
    /// The ops of the frame in draw order, and the bitmaps they refer to.
    fn ops(&self, bitmaps: &Bitmaps) -> (Vec<Op>, Vec<BitmapId>) {
        // The items of the screen, and of every viewport by its slot.
        let mut top: Vec<Entry> = Vec::new();
        let mut inside: Vec<Vec<Entry>> = vec![Vec::new(); self.viewports.slots()];
        for (key, viewport) in self.viewports.iter() {
            top.push(Entry::new(viewport.order, Item::Viewport(key)));
        }
        let mut place = |viewport: Option<ViewportId>, entry: Entry| match viewport {
            None => top.push(entry),
            // The elements of a disposed viewport are not drawn.
            Some(id) if self.viewports.get(id.0).is_some() => {
                inside[id.0.index as usize].push(entry);
            }
            Some(_) => {}
        };
        for (key, sprite) in self.sprites.iter() {
            let y = match self.rules() {
                Rules::Rgss1 => 0,
                Rules::Rgss2 | Rules::Rgss3 => sprite.y,
            };
            let entry = Entry {
                y,
                ..Entry::new(sprite.order, Item::Sprite(key))
            };
            place(sprite.viewport, entry);
        }
        for (key, plane) in self.planes.iter() {
            place(plane.viewport, Entry::new(plane.order, Item::Plane(key)));
        }
        for (key, tilemap) in self.tilemaps.iter() {
            for (z, strip) in tilemap.strips() {
                let entry = Entry {
                    z,
                    y: 0,
                    stamp: tilemap.order.stamp,
                    item: Item::Tiles(key, strip),
                };
                place(tilemap.viewport, entry);
            }
        }
        for (key, window) in self.windows.iter() {
            if window.is_rgss2() {
                let whole = Entry::new(window.order, Item::Window(key));
                place(window.viewport, whole);
                continue;
            }
            let back = Entry::new(window.order, Item::WindowBack(key));
            let front = Entry {
                z: back.z + 2,
                item: Item::WindowFront(key),
                ..back
            };
            place(window.viewport, back);
            place(window.viewport, front);
        }
        sort(&mut top);

        let level = |viewport| level_of(&self.viewports, self.logical_size, viewport);
        let Some(screen) = level(None) else {
            return (Vec::new(), Vec::new());
        };
        let mut assembly = Assembly {
            bitmaps,
            rules: self.rules(),
            ops: Vec::new(),
            referenced: Vec::new(),
        };
        for entry in top {
            match entry.item {
                Item::Viewport(key) => {
                    let Some(viewport) = self.viewports.get(key) else {
                        continue;
                    };
                    let Some(level) = level(Some(ViewportId(key))) else {
                        continue;
                    };
                    if viewport.flash.hides() {
                        // RGSS2: for as long as a flash without a colour lasts, nothing of
                        // the viewport is done, its tone and colour included (measured,
                        // `vx-v1-sprite-REPORT.md` section 4).
                        if self.rules().as_rgss2() {
                            continue;
                        }
                    } else {
                        let entries = &mut inside[key.index as usize];
                        sort(entries);
                        for entry in entries.iter() {
                            self.item(&mut assembly, entry.item, &level);
                        }
                    }
                    assembly.viewport_effect(viewport, level.clip);
                }
                item => self.item(&mut assembly, item, &screen),
            }
        }
        (assembly.ops, assembly.referenced)
    }

    /// Draws one element that is not a viewport.
    fn item(&self, assembly: &mut Assembly<'_>, item: Item, level: &Level) {
        match item {
            Item::Sprite(key) => {
                if let Some(sprite) = self.sprites.get(key) {
                    assembly.sprite(sprite, level);
                }
            }
            Item::Plane(key) => {
                if let Some(plane) = self.planes.get(key) {
                    assembly.plane(plane, level);
                }
            }
            Item::Tiles(key, strip) => {
                if let Some(tilemap) = self.tilemaps.get(key) {
                    assembly.tiles(tilemap, strip, level);
                }
            }
            Item::WindowBack(key) => {
                if let Some(window) = self.windows.get(key) {
                    assembly.window_back(window, level);
                }
            }
            Item::WindowFront(key) => {
                if let Some(window) = self.windows.get(key) {
                    assembly.window_front(window, level);
                }
            }
            Item::Window(key) => {
                if let Some(window) = self.windows.get(key) {
                    assembly.window_rgss2(window, level);
                }
            }
            Item::Viewport(_) => {}
        }
    }
}

pub(crate) mod rgss2;

#[cfg(test)]
pub(crate) mod testing {
    use vkxp_compose_ref::{ComposeError, Compositor, Frame};
    use vkxp_pixel::Rgba;
    use vkxp_proto::{BitmapId, FramePacket, Size};

    use crate::bitmap::testing::defaults;
    use crate::{Bitmaps, Scene};

    pub(crate) const BLACK: Rgba = [0, 0, 0, 255];
    pub(crate) const WHITE: Rgba = [255, 255, 255, 255];
    pub(crate) const RED: Rgba = [255, 0, 0, 255];
    pub(crate) const GREEN: Rgba = [0, 255, 0, 255];
    pub(crate) const BLUE: Rgba = [0, 0, 255, 255];
    pub(crate) const CLEAR: Rgba = [0, 0, 0, 0];

    /// The committed digests of the frames of the tilemap tests: a test's name, then one
    /// digest for each frame [`Stage::frame`] drew of a scene with a tilemap, in order. They
    /// were recorded while each of these frames was also compared with the frame the former
    /// composition of tilemaps into cached bitmaps drew, whose pictures were those of the
    /// original runtime on real games. To accept an intended change, run
    /// `VKXP_BLESS=1 cargo test -p vkxp-core`, which rewrites the lines of the tests that ran.
    const PINNED: &str = include_str!("../tests/data/tilemap-frames.txt");
    const PINNED_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/tilemap-frames.txt");

    /// One writer of the file at a time when the digests are recorded.
    static BLESSING: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// The digest of a frame: FNV-1a, 64 bits, over its size and its pixels.
    fn digest(frame: &Frame) -> u64 {
        let size = [frame.size.width, frame.size.height].map(u32::to_le_bytes);
        let bytes = size.as_flattened().iter().chain(&frame.rgba);
        bytes.fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }

    /// A scene, its bitmaps and the reference compositor that draws its frames.
    pub(crate) struct Stage {
        pub(crate) scene: Scene,
        pub(crate) bitmaps: Bitmaps,
        compositor: Compositor,
        /// The digests of the frames drawn of the scene while it had a tilemap.
        tilemap_frames: Vec<u64>,
    }

    impl Stage {
        /// A stage of 640 by 480 pixels.
        pub(crate) fn new() -> Stage {
            Stage::with(Scene::new())
        }

        /// A stage with a small screen, for tests that look at few pixels.
        pub(crate) fn sized(width: u32, height: u32) -> Stage {
            Stage::with(Scene::with_logical_size(Size::new(width, height)))
        }

        /// A stage whose scene follows `rules`.
        pub(crate) fn with_rules(rules: crate::Rules, width: u32, height: u32) -> Stage {
            Stage::with(Scene::with_rules(rules, Size::new(width, height)))
        }

        /// A stage with a small screen and the rules of RGSS2.
        pub(crate) fn rgss2(width: u32, height: u32) -> Stage {
            let size = Size::new(width, height);
            Stage::with(Scene::with_rules(crate::Rules::Rgss2, size))
        }

        fn with(scene: Scene) -> Stage {
            Stage {
                scene,
                bitmaps: Bitmaps::new(),
                compositor: Compositor::new(),
                tilemap_frames: Vec::new(),
            }
        }

        /// Extracts the next frame and renders it. The digest of a frame of a scene with
        /// tilemaps is kept for [`Stage::assert_pinned`].
        pub(crate) fn frame(&mut self) -> Frame {
            let tilemaps = self.scene.tilemaps.iter().next().is_some();
            let packet = self.scene.extract(&mut self.bitmaps);
            let frame = self
                .render(&packet)
                .expect("the reference compositor accepts every extracted packet");
            if tilemaps {
                self.tilemap_frames.push(digest(&frame));
            }
            frame
        }

        /// Asserts that the frames drawn so far of the scene with its tilemaps are those
        /// committed under `name`.
        pub(crate) fn assert_pinned(&self, name: &str) {
            let digests: Vec<String> = self
                .tilemap_frames
                .iter()
                .map(|d| format!("{d:016x}"))
                .collect();
            assert!(!digests.is_empty(), "`{name}` drew no frame with a tilemap");
            let line = format!("{name} {}", digests.join(" "));
            if std::env::var_os("VKXP_BLESS").is_some() {
                let _guard = BLESSING.lock().unwrap();
                let old = std::fs::read_to_string(PINNED_PATH).unwrap();
                let (comments, lines): (Vec<&str>, Vec<&str>) =
                    old.lines().partition(|line| line.starts_with('#'));
                let kept = lines
                    .into_iter()
                    .filter(|line| line.split(' ').next() != Some(name));
                let mut lines: Vec<&str> = kept.chain([line.as_str()]).collect();
                lines.sort_unstable();
                let new = comments
                    .into_iter()
                    .chain(lines)
                    .fold(String::new(), |text, line| text + line + "\n");
                std::fs::write(PINNED_PATH, new).unwrap();
                return;
            }
            let expected = PINNED
                .lines()
                .find(|line| line.split(' ').next() == Some(name));
            let expected =
                expected.unwrap_or_else(|| panic!("no frames are committed for `{name}`"));
            assert!(
                expected == line,
                "the frames of `{name}` are not the committed ones"
            );
        }

        /// Renders a packet extracted from the scene.
        pub(crate) fn render(&mut self, packet: &FramePacket) -> Result<Frame, ComposeError> {
            self.compositor.render(packet)
        }

        /// A bitmap with the given pixels.
        pub(crate) fn bitmap(&mut self, width: u32, height: u32, rgba: Vec<u8>) -> BitmapId {
            self.bitmaps
                .create_from_rgba(width, height, rgba, &defaults())
                .unwrap()
        }

        /// A bitmap of one colour.
        pub(crate) fn solid(&mut self, width: u32, height: u32, pixel: Rgba) -> BitmapId {
            self.bitmap(width, height, pixel.repeat((width * height) as usize))
        }

        /// A bitmap whose pixels are given one by one, row by row.
        pub(crate) fn pixels(&mut self, width: u32, height: u32, pixels: &[Rgba]) -> BitmapId {
            self.bitmap(width, height, pixels.concat())
        }
    }

    /// Whether every pixel of the frame is `pixel`.
    pub(crate) fn all(frame: &Frame, pixel: Rgba) -> bool {
        frame
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|found| *found == pixel)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod samples;

#[cfg(test)]
mod scene_cases;
