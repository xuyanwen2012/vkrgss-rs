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

    use super::testing::*;
    use crate::{Rect, SpriteId, ViewportId};

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
}
