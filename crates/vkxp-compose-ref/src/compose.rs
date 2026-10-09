//! Per-pixel, in-order evaluation of the op list.

use vkxp_pixel::Rgba;
use vkxp_proto::{BlendMode, FramePacket, GuestToHost, Op, Quad, Rect, RectEffect, Size};

use crate::{Bitmap, BitmapStore, ComposeError};

/// The colour every pixel starts from before the first op: opaque black.
pub const BACKGROUND: Rgba = [0, 0, 0, 255];

/// Largest logical width or height the compositor accepts.
pub const MAX_LOGICAL_DIMENSION: u32 = 16384;

/// A composited frame: RGBA8 in row order, `size.width * size.height` pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub size: Size,
    pub rgba: Vec<u8>,
}

impl Frame {
    /// The pixel at (`x`, `y`).
    ///
    /// # Panics
    ///
    /// If the position is outside the frame.
    pub fn pixel(&self, x: u32, y: u32) -> Rgba {
        assert!(
            x < self.size.width && y < self.size.height,
            "pixel outside the frame"
        );
        let offset = (y as usize * self.size.width as usize + x as usize) * 4;
        [
            self.rgba[offset],
            self.rgba[offset + 1],
            self.rgba[offset + 2],
            self.rgba[offset + 3],
        ]
    }
}

/// The CPU reference compositor: a bitmap store plus the evaluation of frame packets.
#[derive(Debug, Clone, Default)]
pub struct Compositor {
    bitmaps: BitmapStore,
}

impl Compositor {
    pub fn new() -> Compositor {
        Compositor::default()
    }

    /// The bitmaps currently known.
    pub fn bitmaps(&self) -> &BitmapStore {
        &self.bitmaps
    }

    /// Handles one message. A `Frame` message yields its frame; every other message yields
    /// `None`. Freeze, transition, audio, modal and title messages are outside what this
    /// compositor models and have no effect on later frames.
    pub fn handle(&mut self, message: &GuestToHost) -> Result<Option<Frame>, ComposeError> {
        match message {
            GuestToHost::Frame(packet) => self.render(packet).map(Some),
            _ => Ok(None),
        }
    }

    /// Applies the packet's bitmap deltas, then draws its ops.
    ///
    /// Either a complete frame is returned or an error and no frame. Deltas applied before an
    /// error stay applied, so the store keeps following the guest.
    pub fn render(&mut self, packet: &FramePacket) -> Result<Frame, ComposeError> {
        let size = packet.logical_size;
        let valid = 1..=MAX_LOGICAL_DIMENSION;
        if !valid.contains(&size.width) || !valid.contains(&size.height) {
            return Err(ComposeError::InvalidLogicalSize {
                size,
                limit: MAX_LOGICAL_DIMENSION,
            });
        }
        for (index, delta) in packet.bitmaps.iter().enumerate() {
            self.bitmaps
                .apply(delta)
                .map_err(|source| ComposeError::InvalidDelta { index, source })?;
        }
        let steps = self.prepare(&packet.ops)?;

        let mut rgba = Vec::with_capacity(size.width as usize * size.height as usize * 4);
        for py in 0..size.height as u16 {
            for px in 0..size.width as u16 {
                let mut pixel = BACKGROUND;
                for step in &steps {
                    pixel = step.apply(pixel, px, py);
                }
                rgba.extend_from_slice(&pixel);
            }
        }
        Ok(Frame { size, rgba })
    }

    /// Checks every op and resolves its bitmap, so drawing itself cannot fail.
    fn prepare<'a>(&'a self, ops: &'a [Op]) -> Result<Vec<Step<'a>>, ComposeError> {
        let mut steps = Vec::with_capacity(ops.len());
        for (index, op) in ops.iter().enumerate() {
            steps.push(match op {
                Op::Clear { clip, color } => Step::Clear {
                    clip: *clip,
                    color: color.to_rgba(),
                },
                Op::Quad(quad) => {
                    let bitmap = self
                        .bitmaps
                        .get(quad.bitmap)
                        .map_err(|source| ComposeError::InvalidOpBitmap { index, source })?;
                    if !bitmap.bounds().contains_rect(&quad.src) {
                        return Err(ComposeError::SourceOutOfBounds {
                            index,
                            bitmap: quad.bitmap,
                            src: quad.src,
                            width: bitmap.width(),
                            height: bitmap.height(),
                        });
                    }
                    Step::Quad { quad, bitmap }
                }
                Op::RectEffect(effect) => Step::RectEffect(effect),
                Op::Plane { .. } | Op::TilemapLayer { .. } => {
                    return Err(ComposeError::UnsupportedOp {
                        kind: op.kind(),
                        index,
                    });
                }
            });
        }
        Ok(steps)
    }
}

/// A checked op, ready to be evaluated for any pixel.
enum Step<'a> {
    Clear { clip: Rect, color: Rgba },
    Quad { quad: &'a Quad, bitmap: &'a Bitmap },
    RectEffect(&'a RectEffect),
}

impl Step<'_> {
    /// The value of screen pixel (`px`, `py`) after this op, given its value before.
    fn apply(&self, pixel: Rgba, px: u16, py: u16) -> Rgba {
        let (x, y) = (i32::from(px), i32::from(py));
        match self {
            Step::Clear { clip, color } => {
                if clip.contains(x, y) {
                    *color
                } else {
                    pixel
                }
            }
            Step::Quad { quad, bitmap } => {
                if !quad.clip.contains(x, y) {
                    return pixel;
                }
                let Some(texel) = sample(quad, bitmap, px, py) else {
                    return pixel;
                };
                // Tone, then colour, on the texel alone; both keep its alpha.
                let tone = quad.tone;
                let texel =
                    vkxp_pixel::apply_tone(texel, tone.red, tone.green, tone.blue, tone.gray);
                let texel = vkxp_pixel::blend_color(texel, quad.color.to_rgba());
                match quad.blend {
                    BlendMode::Normal => vkxp_pixel::blend_normal(pixel, texel, quad.opacity),
                    BlendMode::Add => vkxp_pixel::blend_add(pixel, texel, quad.opacity),
                    BlendMode::Subtract => vkxp_pixel::blend_sub(pixel, texel, quad.opacity),
                }
            }
            Step::RectEffect(effect) => {
                if !effect.clip.contains(x, y) {
                    return pixel;
                }
                let tone = effect.tone;
                let toned =
                    vkxp_pixel::apply_tone(pixel, tone.red, tone.green, tone.blue, tone.gray);
                vkxp_pixel::blend_color(toned, effect.color.to_rgba())
            }
        }
    }
}

/// The source texel that screen pixel (`px`, `py`) takes through the quad's sampling, or `None`
/// when the quad does not cover the pixel.
fn sample(quad: &Quad, bitmap: &Bitmap, px: u16, py: u16) -> Option<Rgba> {
    let (column, row) = quad
        .sampling
        .source_texel(quad.src.width, quad.src.height, px, py)?;
    // `prepare` checked that the source rectangle lies inside the bitmap.
    Some(bitmap.pixel(
        (i64::from(quad.src.x) + i64::from(column)) as u32,
        (i64::from(quad.src.y) + i64::from(row)) as u32,
    ))
}

/// Feeds a whole message sequence to a fresh compositor and returns the last frame, or `None`
/// if the sequence contains no frame packet.
pub fn render_stream<'a, I>(messages: I) -> Result<Option<Frame>, ComposeError>
where
    I: IntoIterator<Item = &'a GuestToHost>,
{
    let mut compositor = Compositor::new();
    let mut last = None;
    for message in messages {
        if let Some(frame) = compositor.handle(message)? {
            last = Some(frame);
        }
    }
    Ok(last)
}
