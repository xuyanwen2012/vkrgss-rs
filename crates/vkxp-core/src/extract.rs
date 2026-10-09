//! Frame extraction: the scene and the bitmap arena become a protocol frame packet.

use vkxp_proto::{BitmapId, BlendMode, FramePacket, FrameRate, Op, Quad, RectEffect};

use crate::element::Order;
use crate::geometry::Geometry;
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
        if look.opacity == 0 || clip.is_empty() {
            return;
        }
        let Ok(bounds) = self.bitmaps.rect(bitmap) else {
            return;
        };
        let inside = intersect(src.to_proto(), bounds.to_proto());
        if inside.is_empty() {
            return;
        }
        let cut_left = i64::from(inside.x) - i64::from(src.x);
        let cut_top = i64::from(inside.y) - i64::from(src.y);
        // Cuts are at most a rectangle side, which `f64` holds exactly.
        let transform = geometry.transform(f64::from(src.width), cut_left as f64, cut_top as f64);
        let Some(transform) = transform else {
            return;
        };
        self.referenced.push(bitmap);
        self.ops.push(Op::Quad(Quad {
            clip,
            bitmap,
            src: inside,
            transform,
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
        let geometry = Geometry {
            // Sums of two `i32` and an `i64` offset built from two more.
            x: (i64::from(sprite.x) + level.dx) as f64,
            y: (i64::from(sprite.y) + level.dy) as f64,
            ox: f64::from(sprite.ox),
            oy: f64::from(sprite.oy),
            zoom_x: sprite.zoom_x,
            zoom_y: sprite.zoom_y,
            angle: sprite.angle,
            mirror: sprite.mirror,
        };
        let look = Look {
            tone: sprite.tone.to_proto(),
            color: sprite.flash.effective_color(sprite.color),
            opacity: sprite.opacity,
            blend: blend_mode(sprite.blend_type),
        };
        self.quad(bitmap, sprite.src_rect, &geometry, level.clip, look);
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
        // An element without a level is not drawn, but still gives its cache back when it is
        // invisible itself.
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
    fn the_angle_turns_a_sprite_counter_clockwise_about_its_origin() {
        let mut stage = Stage::sized(32, 32);
        let id = a_and_b(&mut stage);
        stage.scene.sprite_mut(id).unwrap().angle = 90.0;
        assert_eq!(drawn(&stage.frame()), [(10, 8, B), (10, 9, A)]);

        stage.scene.sprite_mut(id).unwrap().angle = 180.0;
        assert_eq!(drawn(&stage.frame()), [(8, 9, B), (9, 9, A)]);

        stage.scene.sprite_mut(id).unwrap().angle = 270.0;
        assert_eq!(drawn(&stage.frame()), [(9, 10, A), (9, 11, B)]);

        stage.scene.sprite_mut(id).unwrap().angle = 360.0;
        assert_eq!(drawn(&stage.frame()), [(10, 10, A), (11, 10, B)]);

        // Turned and zoomed by a whole number: still whole pixels.
        let sprite = stage.scene.sprite_mut(id).unwrap();
        sprite.angle = 90.0;
        sprite.zoom_x = 2.0;
        assert_eq!(
            drawn(&stage.frame()),
            [(10, 6, B), (10, 7, B), (10, 8, A), (10, 9, A)]
        );
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
    fn a_flash_decays_linearly_with_each_update() {
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

        // The alpha is rounded down: 100 * 2 / 3 and 100 * 1 / 3.
        let white = Color::new(255.0, 255.0, 255.0, 100.0);
        stage.scene.sprite_mut(id).unwrap().flash(Some(white), 3);
        for alpha in [100, 66, 33] {
            assert_eq!(stage.frame().pixel(0, 0), whitened(alpha), "alpha {alpha}");
            stage.scene.sprite_mut(id).unwrap().update();
        }
        assert_eq!(stage.frame().pixel(0, 0), BLACK);
    }

    #[test]
    fn the_larger_alpha_of_flash_and_own_colour_wins() {
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

        // A new flash replaces the running one, and a duration of 0 ends it.
        stage.scene.sprite_mut(id).unwrap().update();
        stage.scene.sprite_mut(id).unwrap().flash(Some(white), 2);
        assert_eq!(stage.frame().pixel(0, 0), whitened(200));
        stage.scene.sprite_mut(id).unwrap().flash(Some(white), 0);
        assert_eq!(stage.frame().pixel(0, 0), BLACK);
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

        stage.scene.dispose_viewport(viewport);
        assert_eq!(drawn(&stage.frame()).len(), 4);
        // The sprite itself is still alive, and stays hidden when the slot is reused.
        assert!(!stage.scene.is_sprite_disposed(inside));
        stage
            .scene
            .create_viewport(Rect::new(0, 0, 16, 16))
            .unwrap();
        assert_eq!(drawn(&stage.frame()).len(), 4);
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

        // A flash of alpha 200 for 4 updates: 200 and 150 exceed the colour's 100.
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
        for expected in [flashed(150), own(RED), own(RED), own(RED)] {
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
                    scene.dispose_viewport(id);
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
