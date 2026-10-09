//! Frame extraction: the scene and the bitmap arena become a protocol frame packet.

use vkxp_proto::{
    BitmapId, BlendMode, FramePacket, FrameRate, Op, Quad, RectEffect, Sampling, Transform,
};

use crate::element::Order;
use crate::geometry::{self, Geometry};
use crate::scene::Arena;
use crate::scene::Key;
use crate::tilemap::Strip;
use crate::{Bitmaps, Rect, Scene, Sprite, Viewport, ViewportId};

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
        });
    };
    let viewport = viewports.get(id.0)?;
    let clip = intersect(viewport.rect.to_proto(), screen);
    (viewport.visible && !clip.is_empty()).then(|| Level {
        clip,
        dx: i64::from(viewport.rect.x) - i64::from(viewport.ox),
        dy: i64::from(viewport.rect.y) - i64::from(viewport.oy),
    })
}

/// The rows of `clip` from `top` up to `bottom`; a rectangle without pixels when there are
/// none.
fn rows(clip: vkxp_proto::Rect, top: i64, bottom: i64) -> vkxp_proto::Rect {
    let first = top.max(i64::from(clip.y));
    let end = bottom.min(i64::from(clip.y) + i64::from(clip.height));
    if first < end {
        // Both lie inside `clip`.
        vkxp_proto::Rect::new(clip.x, first as i32, clip.width, (end - first) as u32)
    } else {
        vkxp_proto::Rect::new(clip.x, clip.y, clip.width, 0)
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
    pub(crate) ops: Vec<Op>,
    /// The bitmap of every quad in `ops`.
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
    /// is cut off and `dest` with it, down to the pixels that show what is left.
    ///
    /// PLACEHOLDER: the stretch of a cut rectangle starts anew at its corner, so a zoomed
    /// sprite whose source rectangle reaches outside its bitmap may show texels one off. What
    /// the original runtime draws there is not measured.
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
            self.upright_sprite(sprite, bitmap, level, look);
        } else {
            self.turned_sprite(sprite, level, look);
        }
    }

    /// A sprite that is not turned: its source rectangle stretched into a rectangle of whole
    /// pixels, as measured on the original runtime. The lowest rows of that rectangle, as
    /// many as the bush depth whatever the zoom, are drawn with half the opacity.
    fn upright_sprite(&mut self, sprite: &Sprite, bitmap: BitmapId, level: &Level, look: Look) {
        let src = sprite.src_rect;
        let (Some(width), Some(height)) = (
            geometry::stretch_length(src.width, sprite.zoom_x),
            geometry::stretch_length(src.height, sprite.zoom_y),
        ) else {
            return;
        };
        let (Some(x), Some(y)) = (
            geometry::stretch_start(sprite.x, sprite.ox, sprite.zoom_x),
            geometry::stretch_start(sprite.y, sprite.oy, sprite.zoom_y),
        ) else {
            return;
        };
        // A rectangle that begins beyond the coordinates of the protocol is not drawn.
        let (Ok(x), Ok(y)) = (
            i32::try_from(i64::from(x) + level.dx),
            i32::try_from(i64::from(y) + level.dy),
        ) else {
            return;
        };
        let dest = vkxp_proto::Rect::new(x, y, width, height);
        let bush = sprite.bush_depth.max(0).unsigned_abs().min(height);
        // Where the faint rows begin.
        let split = i64::from(y) + i64::from(height - bush);
        let faint = Look {
            opacity: look.opacity >> 1,
            ..look
        };
        let above = rows(level.clip, i64::MIN, split);
        let below = rows(level.clip, split, i64::MAX);
        self.stretched(bitmap, src, dest, sprite.mirror, above, look);
        self.stretched(bitmap, src, dest, sprite.mirror, below, faint);
    }

    /// Applies the viewport's tone and colour to everything drawn so far inside `clip`.
    fn viewport_effect(&mut self, viewport: &Viewport, clip: vkxp_proto::Rect) {
        let tone = viewport.tone.to_proto();
        let color = viewport.flash.effective_color(viewport.color);
        if tone != vkxp_proto::Tone::NEUTRAL || color.a != 0 {
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
    /// One piece of a tilemap: its ground, or a row of tiles of one priority.
    Tiles(Key, Strip),
    /// The frame and background of a window.
    WindowBack(Key),
    /// What a window shows inside: cursor, contents, pause sign and arrows.
    WindowFront(Key),
    Viewport(Key),
}

/// An item with what it is ordered by.
#[derive(Debug, Clone, Copy)]
struct Entry {
    z: i64,
    stamp: u64,
    item: Item,
}

impl Entry {
    fn new(order: Order, item: Item) -> Entry {
        Entry {
            z: i64::from(order.z),
            stamp: order.stamp,
            item,
        }
    }
}

/// Sorts into draw order: ascending z, and among equal z ascending stamp.
///
/// PLACEHOLDER: the order among elements with equal z is not yet checked against the original
/// runtime.
fn sort(entries: &mut [Entry]) {
    entries.sort_by_key(|entry| (entry.z, entry.stamp));
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

    /// Assembles the next frame: the ops of every visible element in draw order, and before
    /// them the bitmap deltas for exactly the bitmaps those ops refer to.
    ///
    /// The packets of one scene must reach a compositor in the order they were extracted,
    /// because the bitmap arena takes every delta it hands out to be delivered.
    pub fn extract(&mut self, bitmaps: &mut Bitmaps) -> FramePacket {
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
        for (_, tilemap) in self.tilemaps.iter_mut() {
            let level = level_of(&self.viewports, self.logical_size, tilemap.viewport);
            tilemap.refresh_cache(bitmaps, level.as_ref());
        }
        for (_, plane) in self.planes.iter_mut() {
            let level = level_of(&self.viewports, self.logical_size, plane.viewport);
            plane.refresh_cache(bitmaps, level.as_ref());
        }
        let (ops, referenced) = self.ops(bitmaps);
        // Every quad was checked to name a live bitmap, which is all `deltas` requires.
        let deltas = bitmaps.deltas(&referenced).unwrap_or_default();

        let frame_index = self.frame_index;
        self.frame_index += 1;
        self.frame_count = self.frame_count.wrapping_add(1);
        FramePacket {
            frame_index,
            logical_size: self.logical_size,
            frame_rate: self.frame_rate,
            tables: vec![],
            bitmaps: deltas,
            ops,
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
            place(sprite.viewport, Entry::new(sprite.order, Item::Sprite(key)));
        }
        for (key, plane) in self.planes.iter() {
            place(plane.viewport, Entry::new(plane.order, Item::Plane(key)));
        }
        for (key, tilemap) in self.tilemaps.iter() {
            // Rows rank by where they are in the viewport, which its origin scrolls.
            let viewport = tilemap.viewport.and_then(|id| self.viewports.get(id.0));
            let scroll = viewport.map_or(0, |viewport| i64::from(viewport.oy));
            for (z, strip) in tilemap.strips(scroll) {
                let entry = Entry {
                    z,
                    stamp: tilemap.order.stamp,
                    item: Item::Tiles(key, strip),
                };
                place(tilemap.viewport, entry);
            }
        }
        for (key, window) in self.windows.iter() {
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
                    if !viewport.flash.hides() {
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
            Item::Viewport(_) => {}
        }
    }
}

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

    /// A scene, its bitmaps and the reference compositor that draws its frames.
    pub(crate) struct Stage {
        pub(crate) scene: Scene,
        pub(crate) bitmaps: Bitmaps,
        compositor: Compositor,
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

        fn with(scene: Scene) -> Stage {
            Stage {
                scene,
                bitmaps: Bitmaps::new(),
                compositor: Compositor::new(),
            }
        }

        /// Extracts the next frame and renders it.
        pub(crate) fn frame(&mut self) -> Frame {
            let packet = self.scene.extract(&mut self.bitmaps);
            self.render(&packet)
                .expect("the reference compositor accepts every extracted packet")
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
mod tests {
    use vkxp_proto::BitmapId;

    use proptest::prelude::*;
    use vkxp_pixel::Rgba;
    use vkxp_proto::{BitmapDelta, FrameRate, Size};

    use super::testing::*;
    use crate::bitmap::testing::{color, defaults};
    use crate::{
        Bitmaps, Color, PlaneId, Rect, Scene, SharedTable, SpriteId, Table, TilemapId, Tone,
        ViewportId, WindowId,
    };

    /// A sprite showing `bitmap` at (`x`, `y`).
    fn sprite(stage: &mut Stage, bitmap: BitmapId, x: i32, y: i32) -> SpriteId {
        sprite_in(stage, None, bitmap, x, y)
    }

    fn sprite_in(
        stage: &mut Stage,
        viewport: Option<ViewportId>,
        bitmap: BitmapId,
        x: i32,
        y: i32,
    ) -> SpriteId {
        let id = stage.scene.create_sprite(viewport).unwrap();
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.set_bitmap(Some(bitmap), &stage.bitmaps);
        sprite.x = x;
        sprite.y = y;
        id
    }

    /// Two overlapping opaque sprites, the red one created first. They share (4, 4).
    fn red_then_green(stage: &mut Stage) -> (SpriteId, SpriteId) {
        let red = stage.solid(4, 4, RED);
        let green = stage.solid(4, 4, GREEN);
        (sprite(stage, red, 2, 2), sprite(stage, green, 4, 4))
    }

    #[test]
    fn the_larger_z_is_in_front_whichever_was_created_first() {
        let mut stage = Stage::sized(16, 16);
        let (red, green) = red_then_green(&mut stage);
        stage.scene.set_sprite_z(red, 1).unwrap();
        stage.scene.set_sprite_z(green, 2).unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(4, 4), GREEN);
        assert_eq!(frame.pixel(2, 2), RED);
        assert_eq!(frame.pixel(7, 7), GREEN);

        let mut stage = Stage::sized(16, 16);
        let (red, green) = red_then_green(&mut stage);
        stage.scene.set_sprite_z(red, 2).unwrap();
        stage.scene.set_sprite_z(green, 1).unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(4, 4), RED);
        assert_eq!(frame.pixel(7, 7), GREEN);

        // Negative z is behind z 0.
        stage.scene.set_sprite_z(red, -1).unwrap();
        stage.scene.set_sprite_z(green, 0).unwrap();
        assert_eq!(stage.frame().pixel(4, 4), GREEN);
    }

    #[test]
    fn with_equal_z_the_later_created_is_in_front() {
        let mut stage = Stage::sized(16, 16);
        red_then_green(&mut stage);
        assert_eq!(stage.frame().pixel(4, 4), GREEN);

        let mut stage = Stage::sized(16, 16);
        let (red, green) = red_then_green(&mut stage);
        stage.scene.set_sprite_z(green, 5).unwrap();
        stage.scene.set_sprite_z(red, 5).unwrap();
        // Red was the last to be given a different z.
        assert_eq!(stage.frame().pixel(4, 4), RED);
    }

    #[test]
    fn moving_to_an_occupied_z_puts_the_element_in_front() {
        let mut stage = Stage::sized(16, 16);
        let (red, _) = red_then_green(&mut stage);
        assert_eq!(stage.frame().pixel(4, 4), GREEN);
        stage.scene.set_sprite_z(red, 7).unwrap();
        stage.scene.set_sprite_z(red, 0).unwrap();
        assert_eq!(stage.frame().pixel(4, 4), RED);
    }

    #[test]
    fn setting_the_same_z_again_changes_nothing() {
        let mut stage = Stage::sized(16, 16);
        let (red, _) = red_then_green(&mut stage);
        stage.scene.set_sprite_z(red, 0).unwrap();
        assert_eq!(stage.frame().pixel(4, 4), GREEN);
        stage.scene.set_sprite_z(red, 0).unwrap();
        assert_eq!(stage.frame().pixel(4, 4), GREEN);
    }

    #[test]
    fn viewports_take_part_in_the_order_of_the_screen() {
        // A red viewport sprite and a green screen sprite share (4, 4).
        let mut stage = Stage::sized(16, 16);
        let red = stage.solid(4, 4, RED);
        let green = stage.solid(4, 4, GREEN);
        let viewport = stage
            .scene
            .create_viewport(Rect::new(0, 0, 16, 16))
            .unwrap();
        sprite_in(&mut stage, Some(viewport), red, 2, 2);
        sprite(&mut stage, green, 4, 4);
        assert_eq!(stage.frame().pixel(4, 4), GREEN, "equal z: created later");

        stage.scene.set_viewport_z(viewport, 1).unwrap();
        assert_eq!(stage.frame().pixel(4, 4), RED, "larger z");

        stage.scene.set_viewport_z(viewport, 0).unwrap();
        assert_eq!(stage.frame().pixel(4, 4), RED, "moved to an occupied z");
    }

    #[test]
    fn elements_inside_a_viewport_are_ordered_among_themselves() {
        let mut stage = Stage::sized(16, 16);
        let viewport = stage
            .scene
            .create_viewport(Rect::new(0, 0, 16, 16))
            .unwrap();
        let red = stage.solid(4, 4, RED);
        let green = stage.solid(4, 4, GREEN);
        let red = sprite_in(&mut stage, Some(viewport), red, 2, 2);
        let green = sprite_in(&mut stage, Some(viewport), green, 4, 4);
        assert_eq!(stage.frame().pixel(4, 4), GREEN);
        stage.scene.set_sprite_z(red, 3).unwrap();
        assert_eq!(stage.frame().pixel(4, 4), RED);
        stage.scene.set_sprite_z(green, 3).unwrap();
        assert_eq!(stage.frame().pixel(4, 4), GREEN);
        stage.scene.set_sprite_z(green, 3).unwrap();
        stage.scene.set_sprite_z(red, 3).unwrap();
        assert_eq!(
            stage.frame().pixel(4, 4),
            GREEN,
            "same z again changes nothing"
        );
    }

    /// The opaque pixel a [`grid`] bitmap has at (`x`, `y`).
    fn cell(x: u32, y: u32) -> Rgba {
        [10 + 40 * x as u8, 10 + 40 * y as u8, 77, 255]
    }

    /// An opaque bitmap whose every pixel differs.
    fn grid(stage: &mut Stage, width: u32, height: u32) -> BitmapId {
        let pixels: Vec<Rgba> = (0..height)
            .flat_map(|y| (0..width).map(move |x| cell(x, y)))
            .collect();
        stage.pixels(width, height, &pixels)
    }

    const A: Rgba = [200, 10, 10, 255];
    const B: Rgba = [10, 10, 200, 255];

    /// The sprite of the zoom, rotation and mirror scenarios: a 2 by 1 bitmap holding A and
    /// B, at (10, 10).
    fn a_and_b(stage: &mut Stage) -> SpriteId {
        let bitmap = stage.pixels(2, 1, &[A, B]);
        sprite(stage, bitmap, 10, 10)
    }

    /// The pixels of the frame that are not black, as (x, y, pixel).
    fn drawn(frame: &vkxp_compose_ref::Frame) -> Vec<(u32, u32, Rgba)> {
        let mut found = Vec::new();
        for y in 0..frame.size.height {
            for x in 0..frame.size.width {
                if frame.pixel(x, y) != BLACK {
                    found.push((x, y, frame.pixel(x, y)));
                }
            }
        }
        found
    }

    #[test]
    fn the_origin_of_a_sprite_lands_on_its_position() {
        let mut stage = Stage::sized(32, 32);
        let bitmap = grid(&mut stage, 4, 4);
        let id = sprite(&mut stage, bitmap, 10, 20);
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.ox = 1;
        sprite.oy = 1;
        let frame = stage.frame();
        assert_eq!(frame.pixel(10, 20), cell(1, 1));
        assert_eq!(frame.pixel(9, 19), cell(0, 0));
        assert_eq!(frame.pixel(12, 22), cell(3, 3));
        assert_eq!(frame.pixel(8, 19), BLACK);
        assert_eq!(frame.pixel(13, 22), BLACK);
        assert_eq!(drawn(&frame).len(), 16, "one bitmap pixel per screen pixel");
    }

    #[test]
    fn assigning_a_bitmap_sets_the_source_rectangle_to_all_of_it() {
        let mut stage = Stage::sized(16, 16);
        let small = stage.solid(4, 3, RED);
        let large = stage.solid(9, 7, GREEN);
        let id = sprite(&mut stage, small, 0, 0);
        assert_eq!(stage.scene.sprite(id).unwrap().bitmap(), Some(small));
        assert_eq!(
            stage.scene.sprite(id).unwrap().src_rect,
            Rect::new(0, 0, 4, 3)
        );
        stage.scene.sprite_mut(id).unwrap().src_rect = Rect::new(1, 1, 2, 2);
        stage
            .scene
            .sprite_mut(id)
            .unwrap()
            .set_bitmap(Some(large), &stage.bitmaps);
        assert_eq!(
            stage.scene.sprite(id).unwrap().src_rect,
            Rect::new(0, 0, 9, 7)
        );
        assert_eq!(drawn(&stage.frame()).len(), 63);
        // Taking the bitmap away keeps the rectangle.
        stage
            .scene
            .sprite_mut(id)
            .unwrap()
            .set_bitmap(None, &stage.bitmaps);
        assert_eq!(stage.scene.sprite(id).unwrap().bitmap(), None);
        assert_eq!(
            stage.scene.sprite(id).unwrap().src_rect,
            Rect::new(0, 0, 9, 7)
        );
    }

    #[test]
    fn the_source_rectangle_selects_what_is_drawn() {
        let mut stage = Stage::sized(32, 32);
        let bitmap = grid(&mut stage, 4, 4);
        let id = sprite(&mut stage, bitmap, 10, 20);
        stage.scene.sprite_mut(id).unwrap().src_rect = Rect::new(2, 0, 2, 4);
        let frame = stage.frame();
        assert_eq!(frame.pixel(10, 20), cell(2, 0));
        assert_eq!(frame.pixel(11, 23), cell(3, 3));
        assert_eq!(drawn(&frame).len(), 8, "columns 0 and 1 are not drawn");
        assert_eq!(frame.pixel(9, 20), BLACK);
        assert_eq!(frame.pixel(12, 20), BLACK);
    }

    #[test]
    fn the_part_of_the_source_rectangle_outside_the_bitmap_draws_nothing() {
        let mut stage = Stage::sized(32, 32);
        let bitmap = grid(&mut stage, 4, 4);
        let id = sprite(&mut stage, bitmap, 10, 20);
        stage.scene.sprite_mut(id).unwrap().src_rect = Rect::new(-2, 0, 4, 4);
        let frame = stage.frame();
        assert_eq!(frame.pixel(12, 20), cell(0, 0));
        assert_eq!(frame.pixel(13, 23), cell(1, 3));
        assert_eq!(frame.pixel(10, 20), BLACK);
        assert_eq!(frame.pixel(11, 20), BLACK);
        assert_eq!(frame.pixel(14, 20), BLACK);
        assert_eq!(drawn(&frame).len(), 8);

        // Beyond the right and bottom edges, and with an origin.
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.src_rect = Rect::new(3, 2, 10, 10);
        sprite.ox = 1;
        let frame = stage.frame();
        assert_eq!(frame.pixel(9, 20), cell(3, 2));
        assert_eq!(frame.pixel(9, 21), cell(3, 3));
        assert_eq!(drawn(&frame).len(), 2);

        // Mirrored, the rectangle flips in place: its empty left half ends up on the right.
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.src_rect = Rect::new(-2, 0, 4, 4);
        sprite.ox = 0;
        sprite.mirror = true;
        let frame = stage.frame();
        assert_eq!(frame.pixel(10, 20), cell(1, 0));
        assert_eq!(frame.pixel(11, 20), cell(0, 0));
        assert_eq!(frame.pixel(12, 20), BLACK);
        assert_eq!(drawn(&frame).len(), 8);

        // Entirely outside, and without pixels.
        for rect in [
            Rect::new(4, 0, 4, 4),
            Rect::new(0, -4, 4, 4),
            Rect::new(0, 0, 0, 4),
            Rect::new(0, 0, 4, -1),
        ] {
            stage.scene.sprite_mut(id).unwrap().src_rect = rect;
            assert!(all(&stage.frame(), BLACK), "{rect:?}");
        }
    }

    #[test]
    fn a_sprite_without_a_live_bitmap_draws_nothing() {
        let mut stage = Stage::sized(16, 16);
        let id = stage.scene.create_sprite(None).unwrap();
        assert!(all(&stage.frame(), BLACK), "no bitmap");

        let bitmap = stage.solid(4, 4, RED);
        stage
            .scene
            .sprite_mut(id)
            .unwrap()
            .set_bitmap(Some(bitmap), &stage.bitmaps);
        assert_eq!(stage.frame().pixel(0, 0), RED);
        stage.bitmaps.dispose(bitmap);
        assert!(all(&stage.frame(), BLACK), "disposed bitmap");

        // Assigning a disposed bitmap is no error either.
        stage
            .scene
            .sprite_mut(id)
            .unwrap()
            .set_bitmap(Some(bitmap), &stage.bitmaps);
        assert!(all(&stage.frame(), BLACK));
    }

    #[test]
    fn an_invisible_or_disposed_sprite_is_not_drawn() {
        let mut stage = Stage::sized(16, 16);
        let bitmap = stage.solid(4, 4, RED);
        let id = sprite(&mut stage, bitmap, 0, 0);
        assert_eq!(stage.frame().pixel(0, 0), RED);
        stage.scene.sprite_mut(id).unwrap().visible = false;
        assert!(all(&stage.frame(), BLACK));
        stage.scene.sprite_mut(id).unwrap().visible = true;
        assert_eq!(stage.frame().pixel(0, 0), RED);
        stage.scene.dispose_sprite(id);
        assert!(all(&stage.frame(), BLACK));
    }

    #[test]
    fn zoom_scales_a_sprite_about_its_origin() {
        let mut stage = Stage::sized(32, 32);
        let id = a_and_b(&mut stage);
        stage.scene.sprite_mut(id).unwrap().zoom_x = 2.0;
        let frame = stage.frame();
        assert_eq!(
            drawn(&frame),
            [(10, 10, A), (11, 10, A), (12, 10, B), (13, 10, B)]
        );

        // Independently in y, and about an origin that is not the corner.
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.zoom_x = 1.0;
        sprite.zoom_y = 3.0;
        sprite.ox = 1;
        let frame = stage.frame();
        assert_eq!(
            drawn(&frame),
            [
                (9, 10, A),
                (10, 10, B),
                (9, 11, A),
                (10, 11, B),
                (9, 12, A),
                (10, 12, B)
            ]
        );
    }

    #[test]
    fn a_zoom_of_zero_draws_nothing() {
        let mut stage = Stage::sized(32, 32);
        let id = a_and_b(&mut stage);
        stage.scene.sprite_mut(id).unwrap().zoom_x = 0.0;
        assert!(all(&stage.frame(), BLACK));
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.zoom_x = 1.0;
        sprite.zoom_y = 0.0;
        assert!(all(&stage.frame(), BLACK));
        stage.scene.sprite_mut(id).unwrap().zoom_y = f64::NAN;
        assert!(all(&stage.frame(), BLACK));
    }

    #[test]
    fn a_negative_zoom_draws_nothing_of_an_unrotated_sprite() {
        let mut stage = Stage::sized(32, 32);
        let id = a_and_b(&mut stage);
        for (zoom_x, zoom_y) in [(-1.0, 1.0), (1.0, -2.0), (-1.0, -1.0), (0.4, 1.0)] {
            let sprite = stage.scene.sprite_mut(id).unwrap();
            (sprite.zoom_x, sprite.zoom_y) = (zoom_x, zoom_y);
            // The last: two columns at 0.4 are less than one pixel.
            assert!(all(&stage.frame(), BLACK), "{zoom_x} by {zoom_y}");
        }
    }

    /// The pixel at a whole offset from the position shows the texel that offset turns to.
    /// At right angles every position lies on a texel boundary, where the original runtime's
    /// sine of pi and cosine of pi / 2, which are not quite 0, decide between two texels:
    /// some texels are lost and others shown twice. The lists are what the measured rule
    /// gives, each pixel with the texel it shows.
    #[test]
    fn the_angle_turns_a_sprite_counter_clockwise_about_its_origin() {
        let mut stage = Stage::sized(32, 32);
        let bitmap = grid(&mut stage, 4, 3);
        let id = sprite(&mut stage, bitmap, 10, 10);
        let mut shown = |angle: f64, zoom_x: f64| -> Vec<(u32, u32, (u32, u32))> {
            let sprite = stage.scene.sprite_mut(id).unwrap();
            (sprite.angle, sprite.zoom_x) = (angle, zoom_x);
            let texel = |pixel: Rgba| {
                let found = (0..4).flat_map(|x| (0..3).map(move |y| (x, y)));
                found
                    .into_iter()
                    .find(|&(x, y)| cell(x, y) == pixel)
                    .unwrap()
            };
            let pixels = drawn(&stage.frame());
            pixels
                .iter()
                .map(|&(x, y, pixel)| (x, y, texel(pixel)))
                .collect()
        };
        // The columns of the bitmap run up the screen.
        assert_eq!(
            shown(90.0, 1.0),
            [
                (11, 7, (3, 0)),
                (12, 7, (3, 1)),
                (11, 8, (2, 0)),
                (12, 8, (2, 2)),
                (11, 9, (1, 0)),
                (12, 9, (1, 2))
            ]
        );
        // Upside down, left of and above the position.
        assert_eq!(
            shown(180.0, 1.0),
            [
                (7, 7, (3, 2)),
                (8, 7, (2, 2)),
                (9, 7, (1, 2)),
                (7, 8, (3, 1)),
                (8, 8, (2, 2)),
                (9, 8, (1, 2)),
                (7, 9, (3, 0)),
                (8, 9, (2, 0)),
                (9, 9, (1, 0))
            ]
        );
        // The columns of the bitmap run down the screen.
        assert_eq!(
            shown(270.0, 1.0),
            [
                (8, 10, (0, 2)),
                (9, 10, (0, 1)),
                (8, 11, (1, 2)),
                (9, 11, (1, 1)),
                (7, 12, (2, 2)),
                (8, 12, (2, 1)),
                (9, 12, (2, 0)),
                (7, 13, (3, 2)),
                (8, 13, (3, 1)),
                (9, 13, (3, 0))
            ]
        );
        // Turned and zoomed: twice as far up the screen.
        let zoomed = shown(90.0, 2.0);
        assert_eq!(zoomed.len(), 14);
        assert_eq!(zoomed[0], (11, 3, (3, 0)));
        assert_eq!(zoomed[13], (12, 9, (0, 2)));
    }

    /// One line of `sprite-zoom-samples.txt`: a sprite 16 pixels wide, and the source column
    /// the original runtime showed at each screen pixel it covered.
    #[test]
    fn unrotated_sprites_show_the_measured_columns() {
        let column = |x: u32| [x as u8 * 10 + 5, 200, 30, 255];
        let samples = include_str!("../../vkxp-compose-ref/tests/data/sprite-zoom-samples.txt");
        let mut checked = 0;
        for line in samples.lines().filter(|line| !line.starts_with('#')) {
            let parts: Vec<&str> = line.split('|').collect();
            let given: Vec<&str> = parts[0].split_whitespace().collect();
            let mut stage = Stage::sized(640, 3);
            let bitmap = stage.pixels(16, 1, &(0..16).map(column).collect::<Vec<_>>());
            let id = sprite(&mut stage, bitmap, given[1].parse().unwrap(), 1);
            let sprite = stage.scene.sprite_mut(id).unwrap();
            sprite.zoom_x = given[0].parse().unwrap();
            sprite.ox = given[2].parse().unwrap();
            sprite.mirror = given[3] == "1";
            let frame = stage.frame();

            let mut expected = Vec::new();
            if parts[1].trim() != "none" {
                let first: u32 = parts[1].split_whitespace().next().unwrap().parse().unwrap();
                for (i, shown) in parts[2].split_whitespace().enumerate() {
                    expected.push((first + i as u32, 1, column(shown.parse().unwrap())));
                }
            }
            assert_eq!(drawn(&frame), expected, "{line}");
            checked += 1;
        }
        assert!(checked > 60);
    }

    /// The rows of a sprite follow the rule of its columns, and both count from the corner
    /// of the source rectangle.
    #[test]
    fn unrotated_sprites_stretch_their_source_rectangle_along_both_axes() {
        let mut stage = Stage::sized(64, 64);
        let bitmap = grid(&mut stage, 6, 6);
        let id = sprite(&mut stage, bitmap, 20, 30);
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.src_rect = Rect::new(1, 2, 5, 4);
        (sprite.ox, sprite.oy) = (3, 1);
        (sprite.zoom_x, sprite.zoom_y) = (2.3, 3.7);
        sprite.mirror = true;
        // 5 * 2.3 = 11.5 and 4 * 3.7 = 14.8; 20 - 3 * 2.3 = 13.1 and 30 - 3.7 = 26.3.
        let (width, height, left, top) = (11u32, 14u32, 13u32, 26u32);
        let (step_x, step_y) = ((5u64 << 20) / 11, (4u64 << 20) / 14);
        let frame = stage.frame();
        let mut expected = Vec::new();
        for j in 0..height {
            for i in 0..width {
                let u = ((u64::from(i) * step_x) >> 20) as u32;
                let v = ((u64::from(j) * step_y) >> 20) as u32;
                expected.push((left + i, top + j, cell(1 + 4 - u, 2 + v)));
            }
        }
        assert_eq!(drawn(&frame), expected);
    }

    #[test]
    fn a_viewport_moves_an_unrotated_sprite_by_whole_pixels() {
        let mut stage = Stage::sized(64, 32);
        let bitmap = grid(&mut stage, 6, 2);
        let viewport = stage
            .scene
            .create_viewport(Rect::new(5, 4, 40, 20))
            .unwrap();
        stage.scene.viewport_mut(viewport).unwrap().ox = -3;
        let id = sprite_in(&mut stage, Some(viewport), bitmap, 2, 1);
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.zoom_x = 1.5;
        sprite.ox = 1;
        // floor(2 - 1.5) = 0 inside the viewport, which begins at 5 and is scrolled by 3.
        let step = (6u64 << 20) / 9;
        let expected: Vec<_> = (0..2)
            .flat_map(|j| {
                (0..9u32).map(move |i| {
                    let u = ((u64::from(i) * step) >> 20) as u32;
                    (8 + i, 5 + j, cell(u, j))
                })
            })
            .collect();
        assert_eq!(drawn(&stage.frame()), expected);
    }

    /// A zoomed sprite whose source rectangle reaches outside its bitmap keeps its place and
    /// its size, and shows only texels of the bitmap.
    #[test]
    fn a_zoomed_sprite_keeps_its_place_when_its_source_rectangle_is_cut() {
        for mirror in [false, true] {
            let mut stage = Stage::sized(64, 16);
            let bitmap = grid(&mut stage, 4, 2);
            let id = sprite(&mut stage, bitmap, 10, 3);
            let sprite = stage.scene.sprite_mut(id).unwrap();
            // Two columns left of the bitmap, its four columns, two columns right of it.
            sprite.src_rect = Rect::new(-2, 0, 8, 2);
            sprite.zoom_x = 3.0;
            sprite.mirror = mirror;
            let pixels = drawn(&stage.frame());
            // 24 pixels for 8 columns, a step of just under a third: column 2, the first of
            // the bitmap, begins at the 8th pixel, and column 6 at the 20th.
            let row: Vec<(u32, Rgba)> = pixels
                .iter()
                .filter(|pixel| pixel.1 == 3)
                .map(|pixel| (pixel.0, pixel.2))
                .collect();
            let columns: Vec<u32> = row.iter().map(|pixel| pixel.0).collect();
            assert_eq!(columns, (17..29).collect::<Vec<_>>(), "mirror {mirror}");
            let (first, last) = if mirror { (3, 0) } else { (0, 3) };
            assert_eq!(row[0].1, cell(first, 0));
            assert_eq!(row[11].1, cell(last, 0));
            assert_eq!(pixels.len(), 24);
        }
    }

    /// The lines of `sprite-rotation-samples.txt`: a turned sprite of 200 by 150 at
    /// (320, 240), and the texel the original runtime showed at offsets from there.
    #[test]
    fn rotated_sprites_show_the_measured_texels() {
        let samples = include_str!("../tests/data/sprite-rotation-samples.txt");
        let lines: Vec<&str> = samples
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect();
        let mut stage = Stage::new();
        let mut rgba = Vec::new();
        for y in 0..150u8 {
            for x in 0..200u8 {
                rgba.extend_from_slice(&[x, y, 100, 255]);
            }
        }
        let bitmap = stage.bitmap(200, 150, rgba);
        let id = sprite(&mut stage, bitmap, 320, 240);
        let mut checked = (0, 0);
        // Lines of one placement follow each other.
        for group in lines.chunk_by(|a, b| a.split('|').next() == b.split('|').next()) {
            let given: Vec<f64> = group[0]
                .split('|')
                .next()
                .unwrap()
                .split_whitespace()
                .map(|word| word.parse().unwrap())
                .collect();
            let sprite = stage.scene.sprite_mut(id).unwrap();
            sprite.angle = given[0];
            (sprite.ox, sprite.oy) = (given[1] as i32, given[2] as i32);
            (sprite.zoom_x, sprite.zoom_y) = (given[3], given[4]);
            sprite.mirror = given[5] == 1.0;
            let frame = stage.frame();
            for line in group {
                let (offset, shown) = line.split('|').nth(1).unwrap().split_once("->").unwrap();
                let offset: Vec<i32> = offset
                    .split_whitespace()
                    .map(|word| word.parse().unwrap())
                    .collect();
                let texel: Vec<u8> = shown
                    .split_whitespace()
                    .map_while(|word| word.parse().ok())
                    .collect();
                let expected = match texel.as_slice() {
                    [u, v] => {
                        checked.0 += 1;
                        [*u, *v, 100, 255]
                    }
                    _ => {
                        checked.1 += 1;
                        BLACK
                    }
                };
                let pixel = frame.pixel((320 + offset[0]) as u32, (240 + offset[1]) as u32);
                assert_eq!(pixel, expected, "{line}");
            }
        }
        // Covered pixels and pixels beside the sprite.
        assert!(checked.0 > 30 && checked.1 > 200, "{checked:?}");
    }

    /// The bitmaps of the sprite angle probe: texel (x, y) holds its own coordinates.
    fn probe_bitmap(stage: &mut Stage, width: u32, height: u32) -> BitmapId {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let high = 100 + (x >> 8) + 2 * (y >> 8);
                rgba.extend_from_slice(&[x as u8, y as u8, high as u8, 255]);
            }
        }
        stage.bitmap(width, height, rgba)
    }

    fn fields(text: &str) -> Vec<f64> {
        text.split_whitespace()
            .map(|word| word.parse().unwrap())
            .collect()
    }

    /// The frames of `sprite-angle-frames.txt`: right angles, full turns, generic angles,
    /// zoom, mirror, a source rectangle and sprites partly off screen. Every frame is
    /// compared as a whole, by the number of pixels the sprite covers and by a hash, and
    /// pixel by pixel in the committed crops.
    #[test]
    fn rotated_sprites_equal_the_measured_frames() {
        let data = include_str!("../tests/data/sprite-angle-frames.txt");
        let mut lines = data
            .lines()
            .filter(|line| !line.starts_with('#'))
            .peekable();
        let mut stage = Stage::new();
        let bitmaps = [
            probe_bitmap(&mut stage, 200, 150),
            probe_bitmap(&mut stage, 329, 329),
        ];
        let id = stage.scene.create_sprite(None).unwrap();
        let mut cases = 0;
        while let Some(case) = lines.next() {
            let parts: Vec<&str> = case.strip_prefix("case ").unwrap().split('|').collect();
            let (size, given, src) = (fields(parts[0]), fields(parts[1]), fields(parts[2]));
            let covered: usize = parts[3].trim().parse().unwrap();
            let hash = u64::from_str_radix(parts[4].trim(), 16).unwrap();
            let bitmap = bitmaps[usize::from(size[0] != 200.0)];
            let sprite = stage.scene.sprite_mut(id).unwrap();
            sprite.set_bitmap(Some(bitmap), &stage.bitmaps);
            sprite.src_rect = Rect::new(src[0] as i32, src[1] as i32, src[2] as i32, src[3] as i32);
            sprite.angle = given[0];
            (sprite.x, sprite.y) = (given[1] as i32, given[2] as i32);
            (sprite.ox, sprite.oy) = (given[3] as i32, given[4] as i32);
            (sprite.zoom_x, sprite.zoom_y) = (given[5], given[6]);
            sprite.mirror = given[7] == 1.0;
            let frame = stage.frame();

            while let Some(crop) = lines.next_if(|line| line.starts_with("crop ")) {
                let at = fields(crop.strip_prefix("crop ").unwrap());
                for row in 0..at[3] as u32 {
                    let line = lines.next().unwrap();
                    for column in 0..at[2] as u32 {
                        let hex = &line[column as usize * 6..][..6];
                        let rgb = u32::from_str_radix(hex, 16).unwrap().to_be_bytes();
                        let (x, y) = (at[0] as u32 + column, at[1] as u32 + row);
                        assert_eq!(
                            frame.pixel(x, y),
                            [rgb[1], rgb[2], rgb[3], 255],
                            "({x}, {y}) of {case}"
                        );
                    }
                }
            }
            assert_eq!(drawn(&frame).len(), covered, "{case}");
            let mut found = 0xcbf2_9ce4_8422_2325u64;
            for pixel in frame.rgba.as_chunks::<4>().0 {
                for byte in &pixel[..3] {
                    found = (found ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3);
                }
            }
            assert_eq!(found, hash, "{case}");
            cases += 1;
        }
        assert_eq!(cases, 36);
    }

    /// A half turn about (100, 75) at (320, 240): the position shows the texel above the
    /// origin, and the last column and row of the box are missing.
    #[test]
    fn a_half_turn_shows_what_was_measured() {
        let mut stage = Stage::new();
        let bitmap = probe_bitmap(&mut stage, 200, 150);
        let id = sprite(&mut stage, bitmap, 320, 240);
        let sprite = stage.scene.sprite_mut(id).unwrap();
        (sprite.ox, sprite.oy) = (100, 75);
        sprite.angle = 180.0;
        let frame = stage.frame();
        assert_eq!(frame.pixel(320, 240), [100, 74, 100, 255]);
        assert_eq!(drawn(&frame).len(), 29651);
    }

    /// A full turn is drawn by the rule of turned sprites, not as a sprite that is not
    /// turned: the last column and the last row of its box are missing.
    #[test]
    fn a_full_turn_is_drawn_as_a_turned_sprite() {
        let mut stage = Stage::sized(32, 32);
        let bitmap = grid(&mut stage, 6, 4);
        let id = sprite(&mut stage, bitmap, 10, 10);
        let upright = drawn(&stage.frame());
        assert_eq!(upright.len(), 24);
        for angle in [360.0, -360.0, 720.0] {
            stage.scene.sprite_mut(id).unwrap().angle = angle;
            let packet = stage.scene.extract(&mut stage.bitmaps);
            let [vkxp_proto::Op::Quad(quad)] = packet.ops.as_slice() else {
                panic!("one quad");
            };
            assert_ne!(quad.bitmap, bitmap, "angle {angle}");
            let pixels = drawn(&stage.render(&packet).unwrap());
            assert!(!pixels.is_empty() && pixels != upright, "angle {angle}");
            // The box ends before column 16 and row 14.
            assert!(
                pixels
                    .iter()
                    .all(|pixel| (10..16).contains(&pixel.0) && (10..14).contains(&pixel.1)),
                "angle {angle}: {pixels:?}"
            );
        }
    }

    /// The bitmap of a turned sprite is rendered again only when what it shows changes, and
    /// is given back when the sprite is no longer turned or is disposed.
    #[test]
    fn the_bitmap_of_a_turned_sprite_is_kept_while_nothing_changes() {
        let mut stage = Stage::sized(64, 64);
        let bitmap = grid(&mut stage, 6, 4);
        let id = sprite(&mut stage, bitmap, 30, 30);
        assert_eq!(stage.bitmaps.len(), 1);
        stage.scene.sprite_mut(id).unwrap().angle = 30.0;
        let first = stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(stage.bitmaps.len(), 2);
        // Only the rendered bitmap is uploaded.
        assert!(!delta_ids(&first.bitmaps).is_empty());
        assert!(
            delta_ids(&first.bitmaps)
                .iter()
                .all(|delta| delta.1 != bitmap)
        );

        // Unchanged, and moved: nothing is uploaded.
        assert!(stage.scene.extract(&mut stage.bitmaps).bitmaps.is_empty());
        let sprite = stage.scene.sprite_mut(id).unwrap();
        (sprite.x, sprite.y) = (20, 25);
        sprite.set_opacity(100);
        assert!(stage.scene.extract(&mut stage.bitmaps).bitmaps.is_empty());

        // A changed source and a changed angle are rendered again.
        stage
            .bitmaps
            .set_pixel(bitmap, 0, 0, Color::new(1.0, 2.0, 3.0, 255.0))
            .unwrap();
        assert!(!stage.scene.extract(&mut stage.bitmaps).bitmaps.is_empty());
        stage.scene.sprite_mut(id).unwrap().angle = 31.0;
        assert!(!stage.scene.extract(&mut stage.bitmaps).bitmaps.is_empty());
        assert_eq!(stage.bitmaps.len(), 2);

        stage.scene.sprite_mut(id).unwrap().angle = 0.0;
        stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(stage.bitmaps.len(), 1);

        stage.scene.sprite_mut(id).unwrap().angle = 30.0;
        stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(stage.bitmaps.len(), 2);
        stage.scene.dispose_sprite(id);
        stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(stage.bitmaps.len(), 1);
    }

    #[test]
    fn mirroring_flips_a_sprite_in_place() {
        let mut stage = Stage::sized(32, 32);
        let id = a_and_b(&mut stage);
        stage.scene.sprite_mut(id).unwrap().mirror = true;
        assert_eq!(drawn(&stage.frame()), [(10, 10, B), (11, 10, A)]);
    }

    #[test]
    fn a_sprite_draws_wherever_it_is_turned_at_any_angle() {
        // Not exact, but a turned square still covers about its area around its centre.
        let mut stage = Stage::sized(64, 64);
        let bitmap = stage.solid(16, 16, RED);
        let id = sprite(&mut stage, bitmap, 32, 32);
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.ox = 8;
        sprite.oy = 8;
        for angle in [1.0, 33.3, 45.0, 123.0, -77.5, 359.0] {
            stage.scene.sprite_mut(id).unwrap().angle = angle;
            let frame = stage.frame();
            let count = drawn(&frame).len();
            assert!(
                (236..=276).contains(&count),
                "angle {angle}: {count} pixels"
            );
            assert_eq!(frame.pixel(32, 32), RED);
            assert_eq!(frame.pixel(31, 31), RED);
        }
    }

    /// A white 32 by 32 sprite at the top-left corner of a black stage, and its faint white.
    fn bush(stage: &mut Stage, depth: i32) -> (SpriteId, Rgba) {
        let bitmap = stage.solid(32, 32, WHITE);
        let id = sprite(stage, bitmap, 0, 0);
        stage.scene.sprite_mut(id).unwrap().bush_depth = depth;
        (id, vkxp_pixel::blend_normal(BLACK, WHITE, 127))
    }

    /// Whether the rows of the frame from `top` up to `bottom` are `pixel` in their first 32
    /// columns.
    fn rows(frame: &vkxp_compose_ref::Frame, top: u32, bottom: u32, pixel: Rgba) -> bool {
        (top..bottom).all(|y| (0..32).all(|x| frame.pixel(x, y) == pixel))
    }

    #[test]
    fn the_rows_of_the_bush_depth_are_drawn_with_half_the_opacity() {
        let mut stage = Stage::sized(48, 48);
        let (id, faint) = bush(&mut stage, 8);
        let frame = stage.frame();
        assert!(rows(&frame, 0, 24, WHITE));
        assert!(rows(&frame, 24, 32, faint));
        assert!(rows(&frame, 32, 48, BLACK));
        assert!((120..=135).contains(&faint[0]), "mid grey");

        // Half of the sprite's own opacity, rounded down.
        stage.scene.sprite_mut(id).unwrap().set_opacity(101);
        let frame = stage.frame();
        assert_eq!(
            frame.pixel(0, 0),
            vkxp_pixel::blend_normal(BLACK, WHITE, 101)
        );
        assert_eq!(
            frame.pixel(0, 31),
            vkxp_pixel::blend_normal(BLACK, WHITE, 50)
        );
    }

    #[test]
    fn the_bush_depth_counts_screen_rows_whatever_the_zoom() {
        let mut stage = Stage::sized(48, 80);
        let (id, faint) = bush(&mut stage, 8);
        stage.scene.sprite_mut(id).unwrap().zoom_y = 2.0;
        let frame = stage.frame();
        assert!(rows(&frame, 0, 56, WHITE));
        assert!(rows(&frame, 56, 64, faint));
        assert!(rows(&frame, 64, 80, BLACK));

        // Zoomed out below the bush depth: every row is faint.
        stage.scene.sprite_mut(id).unwrap().zoom_y = 0.2;
        let frame = stage.frame();
        assert!(rows(&frame, 0, 6, faint));
        assert!(rows(&frame, 6, 80, BLACK));
    }

    #[test]
    fn a_bush_depth_of_zero_or_less_changes_nothing() {
        let mut stage = Stage::sized(48, 48);
        let bitmap = stage.solid(32, 32, WHITE);
        sprite(&mut stage, bitmap, 0, 0);
        let plain = stage.scene.extract(&mut stage.bitmaps);
        let plain_frame = stage.render(&plain).unwrap();
        for depth in [0, -8, i32::MIN] {
            let mut stage = Stage::sized(48, 48);
            bush(&mut stage, depth);
            let packet = stage.scene.extract(&mut stage.bitmaps);
            assert_eq!(packet.ops, plain.ops, "depth {depth}");
            assert_eq!(stage.render(&packet).unwrap().rgba, plain_frame.rgba);
        }
    }

    #[test]
    fn a_bush_depth_beyond_the_source_height_covers_the_whole_sprite() {
        for depth in [32, 33, i32::MAX] {
            let mut stage = Stage::sized(48, 48);
            let (_, faint) = bush(&mut stage, depth);
            let frame = stage.frame();
            assert!(rows(&frame, 0, 32, faint), "depth {depth}");
            assert!(rows(&frame, 32, 48, BLACK));
        }
    }

    /// PLACEHOLDER: not measured; the lowest rows of the source rectangle of a turned sprite
    /// are faint, wherever they are on the screen.
    #[test]
    fn the_bush_depth_of_a_turned_sprite_counts_rows_of_its_source() {
        let mut stage = Stage::sized(64, 64);
        let bitmap = grid(&mut stage, 6, 6);
        let id = sprite(&mut stage, bitmap, 30, 30);
        let sprite = stage.scene.sprite_mut(id).unwrap();
        (sprite.ox, sprite.oy) = (2, 5);
        (sprite.zoom_x, sprite.zoom_y) = (3.0, 2.5);
        sprite.angle = 30.0;
        sprite.mirror = true;
        let plain = drawn(&stage.frame());
        stage.scene.sprite_mut(id).unwrap().bush_depth = 2;
        let bushy = drawn(&stage.frame());
        assert!(plain.len() > 200);
        assert_eq!(plain.len(), bushy.len());
        let mut faint = 0;
        for (full, found) in plain.iter().zip(&bushy) {
            let pixel = full.2;
            // The rows 4 and 5 of the grid.
            let expected = if pixel[1] >= cell(0, 4)[1] {
                faint += 1;
                vkxp_pixel::blend_normal(BLACK, [pixel[0], pixel[1], pixel[2], 127], 255)
            } else {
                pixel
            };
            assert_eq!(*found, (full.0, full.1, expected));
        }
        assert!(faint > 60, "{faint}");
    }

    #[test]
    fn opacity_scales_the_alpha_of_a_sprite() {
        let mut stage = Stage::sized(16, 16);
        let bitmap = stage.solid(4, 4, WHITE);
        let id = sprite(&mut stage, bitmap, 0, 0);
        for opacity in [255, 128, 1] {
            stage.scene.sprite_mut(id).unwrap().set_opacity(opacity);
            let expected = vkxp_pixel::blend_normal(BLACK, WHITE, opacity as u8);
            assert_eq!(stage.frame().pixel(1, 1), expected, "opacity {opacity}");
        }
        stage.scene.sprite_mut(id).unwrap().set_opacity(0);
        assert!(all(&stage.frame(), BLACK));
        stage.scene.sprite_mut(id).unwrap().set_opacity(300);
        assert_eq!(stage.frame().pixel(1, 1), WHITE);
    }

    #[test]
    fn blend_types_add_and_subtract() {
        const GREY: Rgba = [128, 128, 128, 255];
        let mut stage = Stage::sized(16, 16);
        let grey = stage.solid(8, 8, GREY);
        let white = stage.solid(4, 4, WHITE);
        let dim = stage.solid(4, 4, [100, 50, 200, 255]);
        sprite(&mut stage, grey, 0, 0);
        let over = sprite(&mut stage, white, 2, 2);

        stage.scene.sprite_mut(over).unwrap().blend_type = 2;
        let frame = stage.frame();
        assert_eq!(frame.pixel(3, 3), BLACK);
        assert_eq!(frame.pixel(0, 0), GREY);

        stage.scene.sprite_mut(over).unwrap().blend_type = 1;
        assert_eq!(stage.frame().pixel(3, 3), WHITE);

        stage
            .scene
            .sprite_mut(over)
            .unwrap()
            .set_bitmap(Some(dim), &stage.bitmaps);
        assert_eq!(stage.frame().pixel(3, 3), [228, 178, 255, 255]);
        stage.scene.sprite_mut(over).unwrap().blend_type = 2;
        assert_eq!(stage.frame().pixel(3, 3), [28, 78, 0, 255]);

        // Every other value blends normally.
        for blend_type in [0, 3, -1, i32::MAX] {
            stage.scene.sprite_mut(over).unwrap().blend_type = blend_type;
            assert_eq!(stage.frame().pixel(3, 3), [100, 50, 200, 255]);
        }
    }

    #[test]
    fn the_colour_of_a_sprite_recolours_its_own_pixels_only() {
        let mut stage = Stage::sized(16, 16);
        let blue = stage.solid(16, 16, BLUE);
        // Opaque on the left, transparent on the right.
        let shape = stage.pixels(2, 1, &[GREEN, CLEAR]);
        sprite(&mut stage, blue, 0, 0);
        let id = sprite(&mut stage, shape, 4, 4);
        stage.scene.sprite_mut(id).unwrap().color = Color::new(255.0, 0.0, 0.0, 255.0);
        let frame = stage.frame();
        assert_eq!(frame.pixel(4, 4), RED);
        assert_eq!(
            frame.pixel(5, 4),
            BLUE,
            "transparent pixels stay transparent"
        );
        assert_eq!(frame.pixel(3, 4), BLUE);
        assert_eq!(frame.pixel(4, 5), BLUE);

        // A partial alpha mixes the colour into the sprite's own pixel.
        stage.scene.sprite_mut(id).unwrap().color = Color::new(255.0, 0.0, 0.0, 100.0);
        assert_eq!(
            stage.frame().pixel(4, 4),
            vkxp_pixel::blend_color(GREEN, [255, 0, 0, 100])
        );
    }

    #[test]
    fn the_tone_of_a_sprite_comes_before_its_colour_and_stays_inside_it() {
        let mut stage = Stage::sized(16, 16);
        let blue = stage.solid(16, 16, BLUE);
        let patch = stage.solid(2, 2, [200, 100, 0, 255]);
        sprite(&mut stage, blue, 0, 0);
        let id = sprite(&mut stage, patch, 4, 4);
        stage.scene.sprite_mut(id).unwrap().tone = Tone::new(-50.0, 50.0, 30.0, 0.0);
        let frame = stage.frame();
        assert_eq!(frame.pixel(4, 4), [150, 150, 30, 255]);
        assert_eq!(frame.pixel(3, 4), BLUE);
        assert_eq!(frame.pixel(6, 6), BLUE);

        stage.scene.sprite_mut(id).unwrap().tone = Tone::new(0.0, 0.0, 0.0, 255.0);
        let grey = vkxp_pixel::apply_tone([200, 100, 0, 255], 0, 0, 0, 255);
        assert_eq!((grey[0], grey[1]), (grey[1], grey[2]), "full grey");
        assert_eq!(stage.frame().pixel(4, 4), grey);

        stage.scene.sprite_mut(id).unwrap().color = Color::new(0.0, 255.0, 0.0, 128.0);
        assert_eq!(
            stage.frame().pixel(4, 4),
            vkxp_pixel::blend_color(grey, [0, 255, 0, 128])
        );
    }

    /// A sprite with one opaque black pixel at (0, 0).
    fn dot(stage: &mut Stage) -> SpriteId {
        let bitmap = stage.solid(1, 1, [0, 0, 0, 255]);
        sprite(stage, bitmap, 0, 0)
    }

    /// What the black pixel of [`dot`] looks like under a white colour of `alpha`.
    fn whitened(alpha: u8) -> Rgba {
        vkxp_pixel::blend_color([0, 0, 0, 255], [255, 255, 255, alpha])
    }

    #[test]
    fn a_flash_decays_with_each_update() {
        let mut stage = Stage::sized(4, 4);
        let id = dot(&mut stage);
        let white = Color::new(255.0, 255.0, 255.0, 200.0);
        stage.scene.sprite_mut(id).unwrap().flash(Some(white), 4);
        for alpha in [200, 150, 100, 50] {
            assert_eq!(stage.frame().pixel(0, 0), whitened(alpha), "alpha {alpha}");
            stage.scene.sprite_mut(id).unwrap().update();
        }
        assert_eq!(
            stage.frame().pixel(0, 0),
            BLACK,
            "over after the fourth update"
        );
        stage.scene.sprite_mut(id).unwrap().update();
        assert_eq!(stage.frame().pixel(0, 0), BLACK);

        // Each step is rounded down from the one before: 128 * 4 / 5, 102 * 3 / 4, ...
        let white = Color::new(255.0, 255.0, 255.0, 128.0);
        stage.scene.sprite_mut(id).unwrap().flash(Some(white), 5);
        for alpha in [128, 102, 76, 50, 25] {
            assert_eq!(stage.frame().pixel(0, 0), whitened(alpha), "alpha {alpha}");
            stage.scene.sprite_mut(id).unwrap().update();
        }
        assert_eq!(stage.frame().pixel(0, 0), BLACK);
    }

    #[test]
    fn the_larger_alpha_of_flash_and_own_colour_wins_and_a_tie_goes_to_the_flash() {
        let mut stage = Stage::sized(4, 4);
        let id = dot(&mut stage);
        let own = vkxp_pixel::blend_color([0, 0, 0, 255], [255, 0, 0, 120]);
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.color = Color::new(255.0, 0.0, 0.0, 120.0);
        sprite.flash(Some(Color::new(255.0, 255.0, 255.0, 200.0)), 4);
        // Flash alphas 200 and 150 exceed 120; 100 and 50 do not.
        for expected in [whitened(200), whitened(150), own, own, own] {
            assert_eq!(stage.frame().pixel(0, 0), expected);
            stage.scene.sprite_mut(id).unwrap().update();
        }

        // 240, 180 and 120: the last equals the colour's alpha and still shows.
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.flash(Some(Color::new(255.0, 255.0, 255.0, 240.0)), 4);
        for expected in [whitened(240), whitened(180), whitened(120), own] {
            assert_eq!(stage.frame().pixel(0, 0), expected);
            stage.scene.sprite_mut(id).unwrap().update();
        }
    }

    #[test]
    fn a_flash_without_a_colour_hides_the_sprite() {
        let mut stage = Stage::sized(4, 4);
        let bitmap = stage.solid(1, 1, RED);
        let id = sprite(&mut stage, bitmap, 0, 0);
        stage.scene.sprite_mut(id).unwrap().flash(None, 2);
        assert!(all(&stage.frame(), BLACK));
        stage.scene.sprite_mut(id).unwrap().update();
        assert!(all(&stage.frame(), BLACK));
        stage.scene.sprite_mut(id).unwrap().update();
        assert_eq!(stage.frame().pixel(0, 0), RED);
    }

    #[test]
    fn a_flash_advances_only_when_the_sprite_is_updated() {
        let mut stage = Stage::sized(4, 4);
        let id = dot(&mut stage);
        let white = Color::new(255.0, 255.0, 255.0, 200.0);
        stage.scene.sprite_mut(id).unwrap().flash(Some(white), 4);
        let first = stage.frame();
        assert_eq!(first.pixel(0, 0), whitened(200));
        assert_eq!(stage.frame(), first);
        assert_eq!(stage.frame(), first);

        // A new flash replaces the running one, and one of duration 0 shows until the next
        // update.
        stage.scene.sprite_mut(id).unwrap().update();
        stage.scene.sprite_mut(id).unwrap().flash(Some(white), 2);
        assert_eq!(stage.frame().pixel(0, 0), whitened(200));
        stage.scene.sprite_mut(id).unwrap().flash(Some(white), 0);
        assert_eq!(stage.frame().pixel(0, 0), whitened(200));
        assert_eq!(stage.frame().pixel(0, 0), whitened(200));
        stage.scene.sprite_mut(id).unwrap().update();
        assert_eq!(stage.frame().pixel(0, 0), BLACK);
    }

    const MEASURED: &str = include_str!("../tests/data/flash-bush-samples.txt");
    /// The background of the measured frames.
    const BACKDROP: Rgba = [88, 120, 200, 255];

    /// The lines of [`MEASURED`] that begin with `kind`, as what follows it up to the colon
    /// and what follows the colon.
    fn measured(kind: &'static str) -> impl Iterator<Item = (&'static str, &'static str)> {
        MEASURED
            .lines()
            .filter_map(move |line| line.strip_prefix(kind)?.split_once(':'))
    }

    fn rgb(text: &str) -> Rgba {
        let channels: Vec<u8> = text.split(',').map(|c| c.parse().unwrap()).collect();
        [channels[0], channels[1], channels[2], 255]
    }

    /// A stage with the measured background behind everything.
    fn measured_stage() -> Stage {
        let mut stage = Stage::sized(8, 80);
        let backdrop = stage.solid(8, 80, BACKDROP);
        sprite(&mut stage, backdrop, 0, 0);
        stage
    }

    /// The "flash" lines of `flash-bush-samples.txt`: an opaque grey pixel, flashed as the
    /// row number says, before any update and after each of twelve.
    #[test]
    fn flashes_show_the_measured_pixels() {
        let red = Some(Color::new(255.0, 0.0, 0.0, 255.0));
        let mut checked = 0;
        for (row, pixels) in measured("flash ") {
            let row: u32 = row.parse().unwrap();
            let mut stage = measured_stage();
            let grey = stage.solid(1, 1, [100, 100, 100, 255]);
            let viewport = stage.scene.create_viewport(Rect::new(0, 0, 8, 8)).unwrap();
            stage.scene.set_viewport_z(viewport, 1).unwrap();
            let in_viewport = matches!(row, 6 | 7).then_some(viewport);
            let id = sprite_in(&mut stage, in_viewport, grey, 2, 2);
            stage.scene.set_sprite_z(id, 1).unwrap();
            let subject = stage.scene.sprite_mut(id).unwrap();
            match row {
                0 | 13 => subject.flash(red, 8),
                1 => subject.flash(Some(Color::new(0.0, 255.0, 0.0, 128.0)), 5),
                2 => subject.flash(None, 4),
                3 => {
                    subject.color = Color::new(0.0, 0.0, 255.0, 100.0);
                    subject.flash(Some(Color::new(255.0, 255.0, 255.0, 200.0)), 10);
                }
                4 => subject.flash(red, 1),
                6 => stage.scene.viewport_mut(viewport).unwrap().flash(red, 8),
                7 => stage.scene.viewport_mut(viewport).unwrap().flash(None, 4),
                9 => {
                    subject.tone = Tone::new(50.0, -50.0, 0.0, 128.0);
                    subject.flash(red, 8);
                }
                10 => {
                    subject.set_opacity(128);
                    subject.flash(red, 8);
                }
                11 => subject.flash(Some(Color::new(255.0, 255.0, 255.0, 255.0)), 16),
                12 => subject.flash(red, 8),
                14 => {
                    subject.blend_type = 1;
                    subject.flash(red, 8);
                }
                15 => subject.flash(red, 0),
                _ => panic!("no setup for flash row {row}"),
            }
            for (updates, expected) in pixels.split_whitespace().enumerate() {
                if row == 12 && updates == 4 {
                    let blue = Some(Color::new(0.0, 0.0, 255.0, 255.0));
                    stage.scene.sprite_mut(id).unwrap().flash(blue, 4);
                }
                assert_eq!(
                    stage.frame().pixel(2, 2),
                    rgb(expected),
                    "flash row {row} after {updates} updates"
                );
                if row != 13 {
                    stage.scene.sprite_mut(id).unwrap().update();
                    stage.scene.viewport_mut(viewport).unwrap().update();
                }
            }
            checked += 1;
        }
        assert!(checked >= 12);
    }

    /// The "bush" lines of `flash-bush-samples.txt`: a sprite 32 rows high of one colour,
    /// set up as the line says, and the colour of its screen rows as runs.
    #[test]
    fn bush_depths_show_the_measured_rows() {
        let mut checked = 0;
        for (setup, runs) in measured("bush ") {
            // The number that follows `name` in the setup, if the setup names it.
            let value = |name: &str| -> Option<f64> {
                let rest = &setup[setup.find(name)? + name.len()..];
                rest.split_whitespace()
                    .next()?
                    .trim_end_matches(',')
                    .parse()
                    .ok()
            };
            let mut stage = measured_stage();
            let alpha = value("pixel alpha").unwrap_or(255.0) as u8;
            let bitmap = stage.solid(4, 32, [100, 155, 128, alpha]);
            let id = sprite(&mut stage, bitmap, 2, 0);
            let subject = stage.scene.sprite_mut(id).unwrap();
            subject.bush_depth = value("depth").unwrap() as i32;
            subject.set_opacity(value("opacity").unwrap_or(255.0) as i32);
            subject.zoom_y = value("zoom_y").unwrap_or(1.0);
            subject.src_rect.height = value("src_rect height").unwrap_or(32.0) as i32;
            let frame = stage.frame();

            let runs: Vec<(u32, Rgba)> = runs
                .split_whitespace()
                .map(|run| {
                    let (first, colour) = run.split_once(':').unwrap();
                    (first.parse().unwrap(), rgb(colour))
                })
                .collect();
            for y in 0..80 {
                let run = runs.iter().rev().find(|run| run.0 <= y).unwrap();
                assert_eq!(frame.pixel(3, y), run.1, "row {y} of bush {setup}");
            }
            assert_eq!(runs.last().unwrap().1, BACKDROP);
            checked += 1;
        }
        assert!(checked >= 8);
    }

    /// The viewport of the viewport scenarios, 100, 50, 200, 100, holding a sprite at 0, 0
    /// with a 300 by 10 bitmap whose columns all differ.
    fn wide_viewport(stage: &mut Stage) -> (ViewportId, SpriteId) {
        let pixels: Vec<Rgba> = (0..10).flat_map(|_| (0..300u32).map(column)).collect();
        let bitmap = stage.pixels(300, 10, &pixels);
        let viewport = stage
            .scene
            .create_viewport(Rect::new(100, 50, 200, 100))
            .unwrap();
        let sprite = sprite_in(stage, Some(viewport), bitmap, 0, 0);
        (viewport, sprite)
    }

    /// The pixel of column `x` of the bitmap of [`wide_viewport`].
    fn column(x: u32) -> Rgba {
        [(x % 256) as u8, (x / 256) as u8 + 1, 200, 255]
    }

    #[test]
    fn a_viewport_offsets_and_clips_its_elements() {
        let mut stage = Stage::new();
        wide_viewport(&mut stage);
        let frame = stage.frame();
        assert_eq!(frame.pixel(100, 50), column(0));
        assert_eq!(frame.pixel(299, 59), column(199));
        assert_eq!(frame.pixel(99, 50), BLACK);
        assert_eq!(frame.pixel(100, 49), BLACK);
        assert_eq!(frame.pixel(100, 60), BLACK);
        for x in 300..640 {
            assert_eq!(frame.pixel(x, 50), BLACK, "x {x}");
        }
        assert_eq!(drawn(&frame).len(), 200 * 10);
    }

    #[test]
    fn the_origin_of_a_viewport_scrolls_its_elements() {
        let mut stage = Stage::new();
        let (viewport, sprite) = wide_viewport(&mut stage);
        stage.scene.viewport_mut(viewport).unwrap().ox = 10;
        let frame = stage.frame();
        assert_eq!(frame.pixel(100, 50), column(10));
        assert_eq!(frame.pixel(299, 50), column(209));
        assert_eq!(frame.pixel(99, 50), BLACK);
        assert_eq!(frame.pixel(300, 50), BLACK);

        let viewport = stage.scene.viewport_mut(viewport).unwrap();
        viewport.ox = -5;
        viewport.oy = 4;
        stage.scene.sprite_mut(sprite).unwrap().y = 6;
        let frame = stage.frame();
        assert_eq!(frame.pixel(105, 52), column(0));
        assert_eq!(frame.pixel(104, 52), BLACK);
        assert_eq!(frame.pixel(105, 51), BLACK);
    }

    #[test]
    fn the_z_of_a_viewport_outranks_the_z_of_its_elements() {
        let mut stage = Stage::sized(16, 16);
        let red = stage.solid(4, 4, RED);
        let green = stage.solid(4, 4, GREEN);
        let viewport = stage
            .scene
            .create_viewport(Rect::new(0, 0, 16, 16))
            .unwrap();
        stage.scene.set_viewport_z(viewport, 100).unwrap();
        let inside = sprite_in(&mut stage, Some(viewport), red, 2, 2);
        let outside = sprite(&mut stage, green, 4, 4);
        stage.scene.set_sprite_z(inside, 0).unwrap();
        stage.scene.set_sprite_z(outside, 50).unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(4, 4), RED);
        assert_eq!(frame.pixel(7, 7), GREEN);

        stage.scene.set_sprite_z(outside, 101).unwrap();
        assert_eq!(stage.frame().pixel(4, 4), GREEN);
    }

    #[test]
    fn an_invisible_or_disposed_viewport_hides_its_elements() {
        let mut stage = Stage::sized(16, 16);
        let red = stage.solid(4, 4, RED);
        let green = stage.solid(2, 2, GREEN);
        let viewport = stage
            .scene
            .create_viewport(Rect::new(0, 0, 16, 16))
            .unwrap();
        let inside = sprite_in(&mut stage, Some(viewport), red, 2, 2);
        sprite(&mut stage, green, 10, 10);
        assert_eq!(stage.frame().pixel(2, 2), RED);

        stage.scene.viewport_mut(viewport).unwrap().visible = false;
        let frame = stage.frame();
        assert_eq!(frame.pixel(2, 2), BLACK);
        assert_eq!(frame.pixel(10, 10), GREEN);
        assert_eq!(drawn(&frame).len(), 4);

        stage.scene.viewport_mut(viewport).unwrap().visible = true;
        assert_eq!(stage.frame().pixel(2, 2), RED);

        stage.scene.dispose_viewport(viewport, &mut stage.bitmaps);
        assert_eq!(drawn(&stage.frame()).len(), 4);
        // The sprite went with its viewport.
        assert!(stage.scene.is_sprite_disposed(inside));
        // One created in the disposed viewport is alive and draws nothing, also when the
        // slot is reused.
        let late = sprite_in(&mut stage, Some(viewport), red, 2, 2);
        let frame = stage.frame();
        assert_eq!(frame.pixel(2, 2), BLACK);
        assert_eq!(drawn(&frame).len(), 4);
        stage
            .scene
            .create_viewport(Rect::new(0, 0, 16, 16))
            .unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(2, 2), BLACK);
        assert_eq!(drawn(&frame).len(), 4);
        assert!(!stage.scene.is_sprite_disposed(late));
    }

    #[test]
    fn a_viewport_without_pixels_on_the_screen_shows_nothing() {
        let mut stage = Stage::sized(16, 16);
        let red = stage.solid(4, 4, RED);
        for rect in [
            Rect::new(0, 0, 0, 16),
            Rect::new(0, 0, 16, -3),
            Rect::new(16, 0, 8, 8),
            Rect::new(-8, -8, 8, 8),
        ] {
            let viewport = stage.scene.create_viewport(rect).unwrap();
            stage.scene.viewport_mut(viewport).unwrap().tone = Tone::new(255.0, 255.0, 255.0, 0.0);
            sprite_in(&mut stage, Some(viewport), red, 0, 0);
            assert!(all(&stage.frame(), BLACK), "{rect:?}");
        }
        // Partly on the screen, only that part shows.
        let viewport = stage
            .scene
            .create_viewport(Rect::new(-2, 14, 8, 8))
            .unwrap();
        sprite_in(&mut stage, Some(viewport), red, 0, 0);
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 14), RED);
        assert_eq!(frame.pixel(1, 15), RED);
        assert_eq!(drawn(&frame).len(), 4);
    }

    /// A scene with a striped sprite behind a viewport that covers the left half of it.
    fn tinted_half(stage: &mut Stage) -> ViewportId {
        let stripes = stage.pixels(4, 1, &[RED, GREEN, BLUE, [200, 100, 0, 255]]);
        let behind = sprite(stage, stripes, 0, 0);
        stage.scene.sprite_mut(behind).unwrap().zoom_y = 4.0;
        let viewport = stage.scene.create_viewport(Rect::new(0, 0, 2, 4)).unwrap();
        stage.scene.set_viewport_z(viewport, 10).unwrap();
        viewport
    }

    #[test]
    fn the_tone_of_a_viewport_reaches_what_is_behind_it() {
        let mut stage = Stage::sized(8, 8);
        let viewport = tinted_half(&mut stage);
        stage.scene.viewport_mut(viewport).unwrap().tone = Tone::new(0.0, 0.0, 0.0, 255.0);
        let frame = stage.frame();
        for (x, original) in [(0, RED), (1, GREEN)] {
            let grey = vkxp_pixel::apply_tone(original, 0, 0, 0, 255);
            assert_eq!((grey[0], grey[1]), (grey[1], grey[2]));
            assert_eq!(frame.pixel(x, 0), grey);
            assert_eq!(frame.pixel(x, 3), grey);
        }
        assert_eq!(frame.pixel(2, 0), BLUE);
        assert_eq!(frame.pixel(3, 3), [200, 100, 0, 255]);
        assert_eq!(frame.pixel(0, 4), BLACK, "outside the rectangle");
    }

    #[test]
    fn the_effect_of_a_viewport_covers_its_own_elements_too() {
        let mut stage = Stage::sized(8, 8);
        let viewport = tinted_half(&mut stage);
        let white = stage.solid(1, 1, WHITE);
        sprite_in(&mut stage, Some(viewport), white, 1, 1);
        stage.scene.viewport_mut(viewport).unwrap().tone = Tone::new(-100.0, -100.0, 0.0, 0.0);
        let frame = stage.frame();
        assert_eq!(frame.pixel(1, 1), [155, 155, 255, 255]);
        assert_eq!(frame.pixel(0, 0), [155, 0, 0, 255]);
    }

    #[test]
    fn elements_in_front_of_a_viewport_keep_their_colours() {
        let mut stage = Stage::sized(8, 8);
        let viewport = tinted_half(&mut stage);
        stage.scene.viewport_mut(viewport).unwrap().tone = Tone::new(0.0, 0.0, 0.0, 255.0);
        let white = stage.solid(1, 1, [10, 200, 30, 255]);
        let front = sprite(&mut stage, white, 0, 0);
        stage.scene.set_sprite_z(front, 11).unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 0), [10, 200, 30, 255]);
        assert_ne!(frame.pixel(0, 1), RED);

        // Behind the viewport, the same sprite is toned with the rest.
        stage.scene.set_sprite_z(front, 9).unwrap();
        assert_eq!(
            stage.frame().pixel(0, 0),
            vkxp_pixel::apply_tone([10, 200, 30, 255], 0, 0, 0, 255)
        );
    }

    #[test]
    fn the_colour_and_flash_of_a_viewport_fill_its_rectangle() {
        let mut stage = Stage::sized(8, 8);
        let viewport = tinted_half(&mut stage);
        stage.scene.viewport_mut(viewport).unwrap().color = Color::new(0.0, 0.0, 255.0, 100.0);
        let own = |pixel| vkxp_pixel::blend_color(pixel, [0, 0, 255, 100]);
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 0), own(RED));
        assert_eq!(frame.pixel(1, 2), own(GREEN));
        assert_eq!(frame.pixel(2, 0), BLUE);

        // A flash of alpha 200 for 4 updates: 200 and 150 exceed the colour's 100, and the
        // third step equals it, which still shows the flash.
        let white = Color::new(255.0, 255.0, 255.0, 200.0);
        stage
            .scene
            .viewport_mut(viewport)
            .unwrap()
            .flash(Some(white), 4);
        let flashed = |alpha| vkxp_pixel::blend_color(RED, [255, 255, 255, alpha]);
        assert_eq!(stage.frame().pixel(0, 0), flashed(200));
        assert_eq!(
            stage.frame().pixel(0, 0),
            flashed(200),
            "no update, no progress"
        );
        for expected in [flashed(150), flashed(100), own(RED), own(RED)] {
            stage.scene.viewport_mut(viewport).unwrap().update();
            assert_eq!(stage.frame().pixel(0, 0), expected);
        }
    }

    #[test]
    fn a_viewport_flash_without_a_colour_hides_its_elements() {
        let mut stage = Stage::sized(8, 8);
        let viewport = tinted_half(&mut stage);
        let white = stage.solid(1, 1, WHITE);
        sprite_in(&mut stage, Some(viewport), white, 1, 1);
        stage.scene.viewport_mut(viewport).unwrap().flash(None, 1);
        let frame = stage.frame();
        assert_eq!(frame.pixel(1, 1), GREEN, "what is behind still shows");
        stage.scene.viewport_mut(viewport).unwrap().update();
        assert_eq!(stage.frame().pixel(1, 1), WHITE);
    }

    #[test]
    fn a_neutral_viewport_changes_nothing() {
        let mut stage = Stage::sized(8, 8);
        let stripes = stage.pixels(4, 1, &[RED, GREEN, BLUE, [200, 100, 0, 255]]);
        sprite(&mut stage, stripes, 0, 0);
        let without = stage.frame();

        let viewport = stage.scene.create_viewport(Rect::new(0, 0, 8, 8)).unwrap();
        stage.scene.set_viewport_z(viewport, 10).unwrap();
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert!(
            packet
                .ops
                .iter()
                .all(|op| op.kind() != vkxp_proto::OpKind::RectEffect),
            "a neutral viewport emits no rectangle effect"
        );
        assert_eq!(stage.frame(), without);

        // A colour without alpha and a finished flash are neutral too.
        let viewport = stage.scene.viewport_mut(viewport).unwrap();
        viewport.color = Color::new(255.0, 255.0, 255.0, 0.0);
        viewport.flash(Some(Color::new(255.0, 255.0, 255.0, 255.0)), 1);
        viewport.update();
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert!(
            packet
                .ops
                .iter()
                .all(|op| op.kind() != vkxp_proto::OpKind::RectEffect)
        );
        assert_eq!(stage.frame(), without);
    }

    #[test]
    fn the_frame_count_follows_the_extracted_frames_and_can_be_set() {
        let mut scene = Scene::new();
        let mut bitmaps = Bitmaps::new();
        assert_eq!(scene.frame_count(), 0);
        for index in 0..3 {
            assert_eq!(scene.extract(&mut bitmaps).frame_index, index);
        }
        assert_eq!(scene.frame_count(), 3);
        scene.set_frame_count(100);
        assert_eq!(scene.frame_count(), 100);
        // The packet's own index is not the settable count.
        assert_eq!(scene.extract(&mut bitmaps).frame_index, 3);
        assert_eq!(scene.frame_count(), 101);

        scene.set_frame_count(-5);
        scene.extract(&mut bitmaps);
        assert_eq!(scene.frame_count(), -4);
        scene.set_frame_count(i64::MAX);
        assert_eq!(scene.extract(&mut bitmaps).frame_index, 5);
        assert_eq!(scene.frame_count(), i64::MIN);
    }

    #[test]
    fn the_frame_rate_is_clamped_and_carried_by_every_packet() {
        let mut scene = Scene::new();
        let mut bitmaps = Bitmaps::new();
        assert_eq!(scene.frame_rate().get(), 40);
        assert_eq!(scene.extract(&mut bitmaps).frame_rate, FrameRate::DEFAULT);
        scene.set_frame_rate(5);
        assert_eq!(scene.frame_rate().get(), 10);
        assert_eq!(scene.extract(&mut bitmaps).frame_rate.get(), 10);
        scene.set_frame_rate(500);
        assert_eq!(scene.frame_rate().get(), 120);
        assert_eq!(scene.extract(&mut bitmaps).frame_rate.get(), 120);
        scene.set_frame_rate(60);
        assert_eq!(scene.extract(&mut bitmaps).frame_rate.get(), 60);
        scene.set_frame_rate(-1);
        assert_eq!(scene.frame_rate().get(), 10);
    }

    #[test]
    fn packets_carry_the_logical_size_of_the_scene() {
        let mut bitmaps = Bitmaps::new();
        let packet = Scene::new().extract(&mut bitmaps);
        assert_eq!(packet.logical_size, Size::new(640, 480));
        let packet = Scene::with_logical_size(Size::new(320, 240)).extract(&mut bitmaps);
        assert_eq!(packet.logical_size, Size::new(320, 240));

        // A sprite is clipped to that size.
        let mut stage = Stage::sized(8, 6);
        let red = stage.solid(20, 20, RED);
        sprite(&mut stage, red, 0, 0);
        let frame = stage.frame();
        assert_eq!(frame.size, Size::new(8, 6));
        assert!(all(&frame, RED));
    }

    /// The ids the deltas create, update and destroy, in order.
    fn delta_ids(deltas: &[BitmapDelta]) -> Vec<(&'static str, BitmapId)> {
        deltas
            .iter()
            .map(|delta| match delta {
                BitmapDelta::Create { id, .. } => ("create", *id),
                BitmapDelta::Update { id, .. } => ("update", *id),
                BitmapDelta::Destroy { id } => ("destroy", *id),
            })
            .collect()
    }

    #[test]
    fn deltas_cover_exactly_the_bitmaps_the_ops_refer_to() {
        let mut stage = Stage::sized(16, 16);
        let used = stage.solid(4, 4, RED);
        let unused = stage.solid(4, 4, GREEN);
        let hidden = stage.solid(4, 4, BLUE);
        sprite(&mut stage, used, 0, 0);
        let invisible = sprite(&mut stage, hidden, 0, 0);
        stage.scene.sprite_mut(invisible).unwrap().visible = false;

        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(
            delta_ids(&packet.bitmaps),
            [("create", used), ("update", used)]
        );
        assert_eq!(packet.ops.len(), 1);

        // Nothing changed: nothing is sent, also not for the bitmaps nobody shows.
        stage.bitmaps.set_pixel(unused, 0, 0, color(WHITE)).unwrap();
        stage.bitmaps.set_pixel(hidden, 0, 0, color(WHITE)).unwrap();
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(delta_ids(&packet.bitmaps), []);

        // A change to a shown bitmap is sent, and a bitmap is uploaded once it is shown.
        stage.bitmaps.set_pixel(used, 1, 1, color(WHITE)).unwrap();
        stage.scene.sprite_mut(invisible).unwrap().visible = true;
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(
            delta_ids(&packet.bitmaps),
            [("create", hidden), ("update", hidden), ("update", used)]
        );

        // Two sprites with one bitmap upload it once.
        let shared = stage.solid(2, 2, WHITE);
        sprite(&mut stage, shared, 8, 8);
        sprite(&mut stage, shared, 10, 10);
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(
            delta_ids(&packet.bitmaps),
            [("create", shared), ("update", shared)]
        );

        // A disposed bitmap the compositor knows is destroyed there, the unused one is not.
        stage.bitmaps.dispose(used);
        stage.bitmaps.dispose(unused);
        let packet = stage.scene.extract(&mut stage.bitmaps);
        assert_eq!(delta_ids(&packet.bitmaps), [("destroy", used)]);
    }

    #[test]
    fn a_sprite_that_draws_nothing_does_not_upload_its_bitmap() {
        let mut stage = Stage::sized(16, 16);
        let bitmap = stage.solid(4, 4, RED);
        let id = sprite(&mut stage, bitmap, 0, 0);
        stage.scene.sprite_mut(id).unwrap().set_opacity(0);
        assert_eq!(stage.scene.extract(&mut stage.bitmaps).bitmaps, []);
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.set_opacity(255);
        sprite.zoom_x = 0.0;
        assert_eq!(stage.scene.extract(&mut stage.bitmaps).bitmaps, []);
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.zoom_x = 1.0;
        sprite.src_rect = Rect::new(4, 4, 4, 4);
        assert_eq!(stage.scene.extract(&mut stage.bitmaps).bitmaps, []);
    }

    /// One operation of a random script.
    #[derive(Debug, Clone)]
    enum Step {
        CreateBitmap(u32, u32, u8),
        CreateSkin(u8),
        DisposeBitmap(usize),
        Fill(usize, Rect, [u8; 4]),
        CreateViewport(Rect),
        DisposeViewport(usize),
        Viewport(usize, ViewportChange),
        CreateSprite(Option<usize>),
        DisposeSprite(usize),
        Sprite(usize, SpriteChange),
        CreateWindow(Option<usize>),
        DisposeWindow(usize),
        Window(usize, WindowChange),
        CreateSheet(u32, u32, u8),
        CreatePlane(Option<usize>),
        DisposePlane(usize),
        Plane(usize, PlaneChange),
        CreateMap(i32, i32, u32),
        CreatePriorities(u32),
        SetCell(usize, i32, i32, i32, i16),
        CreateTilemap(Option<usize>),
        DisposeTilemap(usize),
        Tilemap(usize, TilemapChange),
        Update,
    }

    #[derive(Debug, Clone)]
    enum ViewportChange {
        Rect(Rect),
        Origin(i32, i32),
        Z(i32),
        Visible(bool),
        Color([u8; 4]),
        Tone([i16; 4]),
        Flash(Option<[u8; 4]>, i32),
    }

    #[derive(Debug, Clone)]
    enum SpriteChange {
        Bitmap(Option<usize>),
        SrcRect(Rect),
        Position(i32, i32),
        Origin(i32, i32),
        Z(i32),
        Zoom(f64, f64),
        Angle(f64),
        Mirror(bool),
        Visible(bool),
        Opacity(i32),
        BlendType(i32),
        Color([u8; 4]),
        Tone([i16; 4]),
        Flash(Option<[u8; 4]>, i32),
    }

    #[derive(Debug, Clone)]
    enum PlaneChange {
        Bitmap(Option<usize>),
        Origin(i32, i32),
        Z(i32),
        Zoom(f64, f64),
        Visible(bool),
        Opacity(i32),
        BlendType(i32),
        Color([u8; 4]),
        Tone([i16; 4]),
    }

    #[derive(Debug, Clone)]
    enum TilemapChange {
        Tileset(Option<usize>),
        Autotile(usize, Option<usize>),
        MapData(Option<usize>),
        Priorities(Option<usize>),
        FlashData(Option<usize>),
        Origin(i32, i32),
        Visible(bool),
    }

    #[derive(Debug, Clone)]
    enum WindowChange {
        Skin(Option<usize>),
        Contents(Option<usize>),
        Position(i32, i32),
        Size(i32, i32),
        Origin(i32, i32),
        Z(i32),
        Stretch(bool),
        Cursor(Rect),
        Active(bool),
        Visible(bool),
        Pause(bool),
        Opacities(i32, i32, i32),
    }

    /// A coordinate around a small screen, now and then one at the limits of its type.
    fn coordinate() -> impl Strategy<Value = i32> {
        prop_oneof![
            12 => -40..80i32,
            1 => prop_oneof![
                Just(i32::MIN),
                Just(i32::MAX),
                Just(-70_000),
                Just(70_000),
                Just(32_767),
                Just(-32_768)
            ],
        ]
    }

    /// A length that may be empty or negative, and now and then one too large for a bitmap.
    fn length() -> impl Strategy<Value = i32> {
        prop_oneof![
            12 => -4..70i32,
            1 => prop_oneof![Just(i32::MIN), Just(i32::MAX), Just(100_000)],
        ]
    }

    fn rect() -> impl Strategy<Value = Rect> {
        (coordinate(), coordinate(), length(), length())
            .prop_map(|(x, y, width, height)| Rect::new(x, y, width, height))
    }

    fn z() -> impl Strategy<Value = i32> {
        prop_oneof![8 => -3..6i32, 1 => prop_oneof![Just(i32::MIN), Just(i32::MAX)]]
    }

    fn zoom() -> impl Strategy<Value = f64> {
        prop_oneof![
            4 => Just(1.0),
            4 => -4.0..8.0f64,
            1 => prop_oneof![
                Just(0.0),
                Just(-0.0),
                Just(f64::NAN),
                Just(f64::INFINITY),
                Just(f64::NEG_INFINITY),
                Just(1e-300),
                Just(1e300),
                Just(1.0 / 65536.0),
                Just(40_000.0)
            ],
        ]
    }

    fn angle() -> impl Strategy<Value = f64> {
        prop_oneof![
            3 => prop_oneof![Just(0.0), Just(90.0), Just(180.0), Just(270.0), Just(-90.0)],
            6 => -800.0..800.0f64,
            1 => prop_oneof![Just(f64::NAN), Just(f64::INFINITY), Just(1e300), Just(-1e-300)],
        ]
    }

    fn opacity() -> impl Strategy<Value = i32> {
        prop_oneof![Just(255), Just(0), -50..320i32]
    }

    fn tone() -> impl Strategy<Value = [i16; 4]> {
        prop_oneof![Just([0; 4]), any::<[i16; 4]>()]
    }

    fn flash() -> impl Strategy<Value = (Option<[u8; 4]>, i32)> {
        (proptest::option::of(any::<[u8; 4]>()), -2..6i32)
    }

    fn viewport_change() -> impl Strategy<Value = ViewportChange> {
        prop_oneof![
            rect().prop_map(ViewportChange::Rect),
            (coordinate(), coordinate()).prop_map(|(x, y)| ViewportChange::Origin(x, y)),
            z().prop_map(ViewportChange::Z),
            any::<bool>().prop_map(ViewportChange::Visible),
            any::<[u8; 4]>().prop_map(ViewportChange::Color),
            tone().prop_map(ViewportChange::Tone),
            flash().prop_map(|(color, duration)| ViewportChange::Flash(color, duration)),
        ]
    }

    fn sprite_change() -> impl Strategy<Value = SpriteChange> {
        prop_oneof![
            3 => proptest::option::weighted(0.9, any::<usize>()).prop_map(SpriteChange::Bitmap),
            2 => rect().prop_map(SpriteChange::SrcRect),
            3 => (coordinate(), coordinate()).prop_map(|(x, y)| SpriteChange::Position(x, y)),
            1 => (coordinate(), coordinate()).prop_map(|(x, y)| SpriteChange::Origin(x, y)),
            1 => z().prop_map(SpriteChange::Z),
            2 => (zoom(), zoom()).prop_map(|(x, y)| SpriteChange::Zoom(x, y)),
            2 => angle().prop_map(SpriteChange::Angle),
            1 => any::<bool>().prop_map(SpriteChange::Mirror),
            1 => any::<bool>().prop_map(SpriteChange::Visible),
            1 => opacity().prop_map(SpriteChange::Opacity),
            1 => (-1..4i32).prop_map(SpriteChange::BlendType),
            1 => any::<[u8; 4]>().prop_map(SpriteChange::Color),
            1 => tone().prop_map(SpriteChange::Tone),
            1 => flash().prop_map(|(color, duration)| SpriteChange::Flash(color, duration)),
        ]
    }

    fn window_change() -> impl Strategy<Value = WindowChange> {
        prop_oneof![
            3 => proptest::option::weighted(0.9, any::<usize>()).prop_map(WindowChange::Skin),
            2 => proptest::option::weighted(0.9, any::<usize>())
                .prop_map(WindowChange::Contents),
            2 => (coordinate(), coordinate()).prop_map(|(x, y)| WindowChange::Position(x, y)),
            3 => (length(), length()).prop_map(|(w, h)| WindowChange::Size(w, h)),
            1 => (coordinate(), coordinate()).prop_map(|(x, y)| WindowChange::Origin(x, y)),
            1 => z().prop_map(WindowChange::Z),
            1 => any::<bool>().prop_map(WindowChange::Stretch),
            2 => rect().prop_map(WindowChange::Cursor),
            1 => any::<bool>().prop_map(WindowChange::Active),
            1 => any::<bool>().prop_map(WindowChange::Visible),
            1 => any::<bool>().prop_map(WindowChange::Pause),
            1 => (opacity(), opacity(), opacity())
                .prop_map(|(a, b, c)| WindowChange::Opacities(a, b, c)),
        ]
    }

    fn plane_change() -> impl Strategy<Value = PlaneChange> {
        prop_oneof![
            3 => proptest::option::weighted(0.9, any::<usize>()).prop_map(PlaneChange::Bitmap),
            3 => (coordinate(), coordinate()).prop_map(|(x, y)| PlaneChange::Origin(x, y)),
            1 => z().prop_map(PlaneChange::Z),
            3 => (zoom(), zoom()).prop_map(|(x, y)| PlaneChange::Zoom(x, y)),
            1 => any::<bool>().prop_map(PlaneChange::Visible),
            1 => opacity().prop_map(PlaneChange::Opacity),
            1 => (-1..4i32).prop_map(PlaneChange::BlendType),
            1 => any::<[u8; 4]>().prop_map(PlaneChange::Color),
            1 => tone().prop_map(PlaneChange::Tone),
        ]
    }

    fn tilemap_change() -> impl Strategy<Value = TilemapChange> {
        let bitmap = || proptest::option::weighted(0.9, any::<usize>());
        let table = || proptest::option::weighted(0.9, any::<usize>());
        prop_oneof![
            2 => bitmap().prop_map(TilemapChange::Tileset),
            3 => (0..7usize, bitmap())
                .prop_map(|(slot, bitmap)| TilemapChange::Autotile(slot, bitmap)),
            2 => table().prop_map(TilemapChange::MapData),
            2 => table().prop_map(TilemapChange::Priorities),
            1 => table().prop_map(TilemapChange::FlashData),
            4 => (coordinate(), coordinate()).prop_map(|(x, y)| TilemapChange::Origin(x, y)),
            1 => any::<bool>().prop_map(TilemapChange::Visible),
        ]
    }

    /// The size of a tileset or of an autotile sheet, whole or undersized.
    fn sheet_size() -> impl Strategy<Value = (u32, u32)> {
        prop_oneof![
            Just((256, 96)),
            Just((96, 128)),
            Just((288, 128)),
            Just((32, 32)),
            Just((96, 32)),
            Just((70, 50)),
            Just((200, 16)),
        ]
    }

    fn step() -> impl Strategy<Value = Step> {
        let viewport = || proptest::option::of(any::<usize>());
        prop_oneof![
            3 => (1..40u32, 1..40u32, any::<u8>())
                .prop_map(|(width, height, seed)| Step::CreateBitmap(width, height, seed)),
            1 => any::<u8>().prop_map(Step::CreateSkin),
            1 => any::<usize>().prop_map(Step::DisposeBitmap),
            2 => (any::<usize>(), rect(), any::<[u8; 4]>())
                .prop_map(|(bitmap, rect, pixel)| Step::Fill(bitmap, rect, pixel)),
            2 => rect().prop_map(Step::CreateViewport),
            1 => any::<usize>().prop_map(Step::DisposeViewport),
            4 => (any::<usize>(), viewport_change())
                .prop_map(|(index, change)| Step::Viewport(index, change)),
            3 => viewport().prop_map(Step::CreateSprite),
            1 => any::<usize>().prop_map(Step::DisposeSprite),
            10 => (any::<usize>(), sprite_change())
                .prop_map(|(index, change)| Step::Sprite(index, change)),
            2 => viewport().prop_map(Step::CreateWindow),
            1 => any::<usize>().prop_map(Step::DisposeWindow),
            10 => (any::<usize>(), window_change())
                .prop_map(|(index, change)| Step::Window(index, change)),
            2 => (sheet_size(), any::<u8>())
                .prop_map(|((width, height), seed)| Step::CreateSheet(width, height, seed)),
            2 => viewport().prop_map(Step::CreatePlane),
            1 => any::<usize>().prop_map(Step::DisposePlane),
            8 => (any::<usize>(), plane_change())
                .prop_map(|(index, change)| Step::Plane(index, change)),
            2 => (0..7i32, 0..7i32, any::<u32>())
                .prop_map(|(width, height, seed)| Step::CreateMap(width, height, seed)),
            1 => any::<u32>().prop_map(Step::CreatePriorities),
            3 => (any::<usize>(), 0..440i32, 0..7i32, 0..4i32, any::<i16>())
                .prop_map(|(table, x, y, z, value)| Step::SetCell(table, x, y, z, value)),
            2 => viewport().prop_map(Step::CreateTilemap),
            1 => any::<usize>().prop_map(Step::DisposeTilemap),
            8 => (any::<usize>(), tilemap_change())
                .prop_map(|(index, change)| Step::Tilemap(index, change)),
            3 => Just(Step::Update),
        ]
    }

    /// The elements a random script has made. Disposed ones stay listed, so that the script
    /// also works with stale handles.
    #[derive(Default)]
    struct Made {
        bitmaps: Vec<BitmapId>,
        viewports: Vec<ViewportId>,
        sprites: Vec<SpriteId>,
        windows: Vec<WindowId>,
        planes: Vec<PlaneId>,
        tilemaps: Vec<TilemapId>,
        tables: Vec<SharedTable>,
    }

    /// Numbers that follow from `seed`, for filling tables.
    fn numbers(seed: u32) -> impl Iterator<Item = u32> {
        let mut state = seed;
        std::iter::repeat_with(move || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            state >> 16
        })
    }

    fn pick<T: Copy>(made: &[T], index: usize) -> Option<T> {
        (!made.is_empty()).then(|| made[index % made.len()])
    }

    fn perform(stage: &mut Stage, made: &mut Made, step: Step) {
        let rgba = |[r, g, b, a]: [u8; 4]| {
            Color::new(f64::from(r), f64::from(g), f64::from(b), f64::from(a))
        };
        let toned = |[r, g, b, gray]: [i16; 4]| {
            Tone::new(f64::from(r), f64::from(g), f64::from(b), f64::from(gray))
        };
        let scene = &mut stage.scene;
        match step {
            Step::CreateBitmap(width, height, seed) => {
                let pixels = crate::bitmap::testing::pattern(width, height, seed);
                let id = stage
                    .bitmaps
                    .create_from_rgba(width, height, pixels, &defaults());
                made.bitmaps.push(id.unwrap());
            }
            Step::CreateSkin(seed) => {
                let pixels = crate::bitmap::testing::pattern(192, 128, seed);
                let id = stage
                    .bitmaps
                    .create_from_rgba(192, 128, pixels, &defaults());
                made.bitmaps.push(id.unwrap());
            }
            Step::DisposeBitmap(index) => {
                if let Some(id) = pick(&made.bitmaps, index) {
                    stage.bitmaps.dispose(id);
                }
            }
            Step::Fill(index, rect, pixel) => {
                if let Some(id) = pick(&made.bitmaps, index) {
                    let _ = stage.bitmaps.fill_rect(id, rect, color(pixel));
                }
            }
            Step::CreateViewport(rect) => {
                made.viewports.push(scene.create_viewport(rect).unwrap());
            }
            Step::DisposeViewport(index) => {
                if let Some(id) = pick(&made.viewports, index) {
                    scene.dispose_viewport(id, &mut stage.bitmaps);
                }
            }
            Step::Viewport(index, change) => {
                let Some(id) = pick(&made.viewports, index) else {
                    return;
                };
                if let ViewportChange::Z(z) = change {
                    let _ = scene.set_viewport_z(id, z);
                    return;
                }
                let Ok(viewport) = scene.viewport_mut(id) else {
                    return;
                };
                match change {
                    ViewportChange::Rect(rect) => viewport.rect = rect,
                    ViewportChange::Origin(x, y) => (viewport.ox, viewport.oy) = (x, y),
                    ViewportChange::Z(_) => {}
                    ViewportChange::Visible(visible) => viewport.visible = visible,
                    ViewportChange::Color(pixel) => viewport.color = rgba(pixel),
                    ViewportChange::Tone(tone) => viewport.tone = toned(tone),
                    ViewportChange::Flash(pixel, duration) => {
                        viewport.flash(pixel.map(rgba), duration);
                    }
                }
            }
            Step::CreateSprite(viewport) => {
                let viewport = viewport.and_then(|index| pick(&made.viewports, index));
                if let Ok(id) = scene.create_sprite(viewport) {
                    made.sprites.push(id);
                }
            }
            Step::DisposeSprite(index) => {
                if let Some(id) = pick(&made.sprites, index) {
                    scene.dispose_sprite(id);
                }
            }
            Step::Sprite(index, change) => {
                let Some(id) = pick(&made.sprites, index) else {
                    return;
                };
                if let SpriteChange::Z(z) = change {
                    let _ = scene.set_sprite_z(id, z);
                    return;
                }
                let Ok(sprite) = scene.sprite_mut(id) else {
                    return;
                };
                match change {
                    SpriteChange::Bitmap(bitmap) => {
                        let bitmap = bitmap.and_then(|index| pick(&made.bitmaps, index));
                        sprite.set_bitmap(bitmap, &stage.bitmaps);
                    }
                    SpriteChange::SrcRect(rect) => sprite.src_rect = rect,
                    SpriteChange::Position(x, y) => (sprite.x, sprite.y) = (x, y),
                    SpriteChange::Origin(x, y) => (sprite.ox, sprite.oy) = (x, y),
                    SpriteChange::Z(_) => {}
                    SpriteChange::Zoom(x, y) => (sprite.zoom_x, sprite.zoom_y) = (x, y),
                    SpriteChange::Angle(angle) => sprite.angle = angle,
                    SpriteChange::Mirror(mirror) => sprite.mirror = mirror,
                    SpriteChange::Visible(visible) => sprite.visible = visible,
                    SpriteChange::Opacity(opacity) => sprite.set_opacity(opacity),
                    SpriteChange::BlendType(blend_type) => sprite.blend_type = blend_type,
                    SpriteChange::Color(pixel) => sprite.color = rgba(pixel),
                    SpriteChange::Tone(tone) => sprite.tone = toned(tone),
                    SpriteChange::Flash(pixel, duration) => {
                        sprite.flash(pixel.map(rgba), duration);
                    }
                }
            }
            Step::CreateWindow(viewport) => {
                let viewport = viewport.and_then(|index| pick(&made.viewports, index));
                if let Ok(id) = scene.create_window(viewport) {
                    made.windows.push(id);
                }
            }
            Step::DisposeWindow(index) => {
                if let Some(id) = pick(&made.windows, index) {
                    scene.dispose_window(id, &mut stage.bitmaps);
                }
            }
            Step::Window(index, change) => {
                let Some(id) = pick(&made.windows, index) else {
                    return;
                };
                if let WindowChange::Z(z) = change {
                    let _ = scene.set_window_z(id, z);
                    return;
                }
                let Ok(window) = scene.window_mut(id) else {
                    return;
                };
                match change {
                    WindowChange::Skin(bitmap) => {
                        window.windowskin = bitmap.and_then(|index| pick(&made.bitmaps, index));
                    }
                    WindowChange::Contents(bitmap) => {
                        window.contents = bitmap.and_then(|index| pick(&made.bitmaps, index));
                    }
                    WindowChange::Position(x, y) => (window.x, window.y) = (x, y),
                    WindowChange::Size(width, height) => {
                        (window.width, window.height) = (width, height);
                    }
                    WindowChange::Origin(x, y) => (window.ox, window.oy) = (x, y),
                    WindowChange::Z(_) => {}
                    WindowChange::Stretch(stretch) => window.stretch = stretch,
                    WindowChange::Cursor(rect) => window.cursor_rect = rect,
                    WindowChange::Active(active) => window.active = active,
                    WindowChange::Visible(visible) => window.visible = visible,
                    WindowChange::Pause(pause) => window.pause = pause,
                    WindowChange::Opacities(opacity, back, contents) => {
                        window.set_opacity(opacity);
                        window.set_back_opacity(back);
                        window.set_contents_opacity(contents);
                    }
                }
            }
            Step::CreateSheet(width, height, seed) => {
                let pixels = crate::bitmap::testing::pattern(width, height, seed);
                let id = stage
                    .bitmaps
                    .create_from_rgba(width, height, pixels, &defaults());
                made.bitmaps.push(id.unwrap());
            }
            Step::CreatePlane(viewport) => {
                let viewport = viewport.and_then(|index| pick(&made.viewports, index));
                if let Ok(id) = scene.create_plane(viewport) {
                    made.planes.push(id);
                }
            }
            Step::DisposePlane(index) => {
                if let Some(id) = pick(&made.planes, index) {
                    scene.dispose_plane(id, &mut stage.bitmaps);
                }
            }
            Step::Plane(index, change) => {
                let Some(id) = pick(&made.planes, index) else {
                    return;
                };
                if let PlaneChange::Z(z) = change {
                    let _ = scene.set_plane_z(id, z);
                    return;
                }
                let Ok(plane) = scene.plane_mut(id) else {
                    return;
                };
                match change {
                    PlaneChange::Bitmap(bitmap) => {
                        plane.bitmap = bitmap.and_then(|index| pick(&made.bitmaps, index));
                    }
                    PlaneChange::Origin(x, y) => (plane.ox, plane.oy) = (x, y),
                    PlaneChange::Z(_) => {}
                    PlaneChange::Zoom(x, y) => (plane.zoom_x, plane.zoom_y) = (x, y),
                    PlaneChange::Visible(visible) => plane.visible = visible,
                    PlaneChange::Opacity(opacity) => plane.set_opacity(opacity),
                    PlaneChange::BlendType(blend_type) => plane.blend_type = blend_type,
                    PlaneChange::Color(pixel) => plane.color = rgba(pixel),
                    PlaneChange::Tone(tone) => plane.tone = toned(tone),
                }
            }
            Step::CreateMap(width, height, seed) => {
                // Tiles of every kind: empty, autotiles, the tileset, now and then anything.
                let mut table = Table::new_3d(width, height, 3).unwrap();
                let mut numbers = numbers(seed);
                for z in 0..3 {
                    for y in 0..height {
                        for x in 0..width {
                            let n = numbers.next().unwrap_or(0);
                            let id = match n % 8 {
                                0 => 0,
                                1..=3 => 48 + n / 8 % 336,
                                4..=6 => 384 + n / 8 % 32,
                                _ => n / 8,
                            };
                            table.set(x, y, z, i64::from(id));
                        }
                    }
                }
                made.tables.push(SharedTable::new(table));
            }
            Step::CreatePriorities(seed) => {
                let mut table = Table::new(430).unwrap();
                for (id, n) in (0..430).zip(numbers(seed)) {
                    table.set(id, 0, 0, i64::from(n % 9) - 1);
                }
                made.tables.push(SharedTable::new(table));
            }
            Step::SetCell(index, x, y, z, value) => {
                if let Some(table) = made.tables.get(index % made.tables.len().max(1)) {
                    table.set(x, y, z, i64::from(value));
                }
            }
            Step::CreateTilemap(viewport) => {
                let viewport = viewport.and_then(|index| pick(&made.viewports, index));
                if let Ok(id) = scene.create_tilemap(viewport) {
                    made.tilemaps.push(id);
                }
            }
            Step::DisposeTilemap(index) => {
                if let Some(id) = pick(&made.tilemaps, index) {
                    scene.dispose_tilemap(id, &mut stage.bitmaps);
                }
            }
            Step::Tilemap(index, change) => {
                let Some(id) = pick(&made.tilemaps, index) else {
                    return;
                };
                let Ok(tilemap) = scene.tilemap_mut(id) else {
                    return;
                };
                let bitmap = |index: Option<usize>| index.and_then(|i| pick(&made.bitmaps, i));
                let table = |index: Option<usize>| {
                    let index = index.filter(|_| !made.tables.is_empty())?;
                    Some(made.tables[index % made.tables.len()].clone())
                };
                match change {
                    TilemapChange::Tileset(index) => tilemap.tileset = bitmap(index),
                    TilemapChange::Autotile(slot, index) => {
                        tilemap.autotiles[slot] = bitmap(index);
                    }
                    TilemapChange::MapData(index) => tilemap.map_data = table(index),
                    TilemapChange::Priorities(index) => tilemap.priorities = table(index),
                    TilemapChange::FlashData(index) => tilemap.flash_data = table(index),
                    TilemapChange::Origin(x, y) => (tilemap.ox, tilemap.oy) = (x, y),
                    TilemapChange::Visible(visible) => tilemap.visible = visible,
                }
            }
            Step::Update => {
                for &id in &made.sprites {
                    if let Ok(sprite) = scene.sprite_mut(id) {
                        sprite.update();
                    }
                }
                for &id in &made.viewports {
                    if let Ok(viewport) = scene.viewport_mut(id) {
                        viewport.update();
                    }
                }
                for &id in &made.windows {
                    if let Ok(window) = scene.window_mut(id) {
                        window.update();
                    }
                }
                // Enough updates for an autotile to show its next frame now and then.
                for &id in &made.tilemaps {
                    if let Ok(tilemap) = scene.tilemap_mut(id) {
                        (0..8).for_each(|_| tilemap.update());
                    }
                }
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(400))]

        #[test]
        fn the_reference_compositor_renders_every_extracted_packet(
            steps in proptest::collection::vec(step(), 1..50),
        ) {
            let mut stage = Stage::sized(40, 30);
            let mut made = Made::default();
            // Something to look at from the first frame on: sprites inside and outside a
            // viewport, a window with everything it can show, a plane, and a tilemap with
            // a tileset, autotile sheets, a map and priorities.
            for step in [
                Step::CreateBitmap(12, 9, 1),
                Step::CreateBitmap(70, 50, 2),
                Step::CreateSkin(3),
                Step::CreateViewport(Rect::new(4, 3, 30, 20)),
                Step::CreateSprite(None),
                Step::Sprite(0, SpriteChange::Bitmap(Some(0))),
                Step::CreateSprite(Some(0)),
                Step::Sprite(1, SpriteChange::Bitmap(Some(1))),
                Step::CreateWindow(None),
                Step::Window(0, WindowChange::Skin(Some(2))),
                Step::Window(0, WindowChange::Contents(Some(1))),
                Step::Window(0, WindowChange::Size(38, 36)),
                Step::Window(0, WindowChange::Cursor(Rect::new(0, 0, 6, 4))),
                Step::Window(0, WindowChange::Pause(true)),
                Step::CreatePlane(None),
                Step::Plane(0, PlaneChange::Bitmap(Some(0))),
                Step::CreateSheet(256, 96, 4),
                Step::CreateSheet(288, 128, 5),
                Step::CreateSheet(96, 32, 6),
                Step::CreateMap(5, 4, 7),
                Step::CreatePriorities(8),
                Step::CreateTilemap(Some(0)),
                Step::Tilemap(0, TilemapChange::Tileset(Some(3))),
                Step::Tilemap(0, TilemapChange::Autotile(0, Some(4))),
                Step::Tilemap(0, TilemapChange::Autotile(1, Some(5))),
                Step::Tilemap(0, TilemapChange::MapData(Some(0))),
                Step::Tilemap(0, TilemapChange::Priorities(Some(1))),
            ] {
                perform(&mut stage, &mut made, step);
            }
            for step in steps {
                perform(&mut stage, &mut made, step.clone());
                let packet = stage.scene.extract(&mut stage.bitmaps);
                let rendered = stage.render(&packet);
                prop_assert!(rendered.is_ok(), "after {:?}: {:?}", step, rendered.err());
                // Every delta is for a bitmap an op draws, or destroys one that is gone.
                for delta in &packet.bitmaps {
                    let (BitmapDelta::Create { id, .. }
                    | BitmapDelta::Update { id, .. }
                    | BitmapDelta::Destroy { id }) = delta;
                    let destroyed = matches!(delta, BitmapDelta::Destroy { .. });
                    let drawn = packet.ops.iter().any(
                        |op| matches!(op, vkxp_proto::Op::Quad(quad) if quad.bitmap == *id),
                    );
                    prop_assert_eq!(drawn, !destroyed, "{:?} after {:?}", id, step);
                }
            }
        }
    }
}
