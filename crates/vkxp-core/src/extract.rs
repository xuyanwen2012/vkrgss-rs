//! Frame extraction: the scene and the bitmap arena become a protocol frame packet.

use vkxp_proto::{BitmapId, BlendMode, FramePacket, FrameRate, Op, Quad, RectEffect};

use crate::element::Order;
use crate::geometry::Geometry;
use crate::scene::Key;
use crate::{Bitmaps, Rect, Scene, Sprite, Viewport};

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
    ops: Vec<Op>,
    /// The bitmap of every quad in `ops`.
    referenced: Vec<BitmapId>,
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
            blend: match sprite.blend_type {
                1 => BlendMode::Add,
                2 => BlendMode::Subtract,
                _ => BlendMode::Normal,
            },
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
        let mut place = |viewport: Option<crate::ViewportId>, entry: Entry| match viewport {
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
        sort(&mut top);

        let screen = Level {
            clip: vkxp_proto::Rect::from_size(self.logical_size),
            dx: 0,
            dy: 0,
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
                    let clip = intersect(viewport.rect.to_proto(), screen.clip);
                    if !viewport.visible || clip.is_empty() {
                        continue;
                    }
                    let level = Level {
                        clip,
                        dx: i64::from(viewport.rect.x) - i64::from(viewport.ox),
                        dy: i64::from(viewport.rect.y) - i64::from(viewport.oy),
                    };
                    if !viewport.flash.hides() {
                        let entries = &mut inside[key.index as usize];
                        sort(entries);
                        for entry in entries.iter() {
                            self.item(&mut assembly, entry.item, &level);
                        }
                    }
                    assembly.viewport_effect(viewport, clip);
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
            Item::Viewport(_) => {}
        }
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use vkxp_compose_ref::{Compositor, Frame};
    use vkxp_pixel::Rgba;
    use vkxp_proto::{BitmapId, Size};

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
            self.compositor
                .render(&packet)
                .expect("the reference compositor accepts every extracted packet")
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

    use vkxp_pixel::Rgba;

    use super::testing::*;
    use crate::{Color, Rect, SpriteId, Tone, ViewportId};

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
}
