//! Frame extraction: the scene and the bitmap arena become a protocol frame packet.

use vkxp_proto::{BitmapId, FramePacket, FrameRate, Op};

use crate::{Bitmaps, Scene};

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
        let ops: Vec<Op> = Vec::new();
        let referenced: Vec<BitmapId> = Vec::new();
        // Extraction refers to live bitmaps only, which is all `deltas` requires.
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
