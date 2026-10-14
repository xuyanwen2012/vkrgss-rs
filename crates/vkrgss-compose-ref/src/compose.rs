//! Per-pixel, in-order evaluation of the op list.

use vkrgss_pixel::Rgba;
use vkrgss_proto::{
    BlendMode, FramePacket, GuestToHost, MAX_LOGICAL_DIMENSION, Op, Quad, Rect, RectEffect, Size,
};

use crate::tilemap::Layer;
use crate::tilemap_rgss2::PlaneStep;
use crate::{Bitmap, BitmapStore, ComposeError, TableStore};

/// The colour every pixel starts from before the first op: opaque black.
pub const BACKGROUND: Rgba = [0, 0, 0, 255];

/// A pixel of the frame at `brightness`: each colour channel `v` becomes
/// `(v * brightness) >> 8`, and the alpha stays. 255 is the identity and is not multiplied.
/// Measured on the original runtime of VX: the level 200 at brightness 100 shows 78.
fn darken(pixel: Rgba, brightness: u8) -> Rgba {
    if brightness == 255 {
        return pixel;
    }
    let channel = |value: u8| ((u16::from(value) * u16::from(brightness)) >> 8) as u8;
    [
        channel(pixel[0]),
        channel(pixel[1]),
        channel(pixel[2]),
        pixel[3],
    ]
}

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

/// The CPU reference compositor: a bitmap store, a table store and the evaluation of frame
/// packets.
#[derive(Debug, Clone, Default)]
pub struct Compositor {
    bitmaps: BitmapStore,
    tables: TableStore,
}

impl Compositor {
    pub fn new() -> Compositor {
        Compositor::default()
    }

    /// The bitmaps currently known.
    pub fn bitmaps(&self) -> &BitmapStore {
        &self.bitmaps
    }

    /// The tables currently known.
    pub fn tables(&self) -> &TableStore {
        &self.tables
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

    /// Applies the packet's bitmap deltas and then its table deltas, then draws its ops and
    /// darkens the result by the packet's brightness.
    ///
    /// Either a complete frame is returned or an error and no frame. Deltas applied before an
    /// error stay applied, so the stores keep following the guest.
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
        for (index, delta) in packet.tables.iter().enumerate() {
            self.tables
                .apply(delta)
                .map_err(|source| ComposeError::InvalidTableDelta { index, source })?;
        }
        let steps = self.prepare(&packet.ops)?;

        let mut rgba = Vec::with_capacity(size.width as usize * size.height as usize * 4);
        for py in 0..size.height as u16 {
            for px in 0..size.width as u16 {
                let mut pixel = BACKGROUND;
                for step in &steps {
                    pixel = step.apply(pixel, size, px, py);
                }
                rgba.extend_from_slice(&darken(pixel, packet.brightness));
            }
        }
        Ok(Frame { size, rgba })
    }

    /// Checks every op and resolves its bitmaps and tables, so drawing itself cannot fail.
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
                Op::TilemapLayer(layer) => {
                    let bitmap = |id| {
                        self.bitmaps
                            .get(id)
                            .map_err(|source| ComposeError::InvalidOpBitmap { index, source })
                    };
                    let table = |id| {
                        self.tables
                            .get(id)
                            .map_err(|source| ComposeError::InvalidOpTable { index, source })
                    };
                    let tileset = layer.tileset.map(bitmap).transpose()?;
                    let mut autotiles = [None; 7];
                    for (found, id) in autotiles.iter_mut().zip(layer.autotiles) {
                        *found = id.map(bitmap).transpose()?;
                    }
                    Step::Tilemap(Layer {
                        op: layer,
                        map: table(layer.map)?,
                        priorities: table(layer.priorities)?,
                        tileset,
                        autotiles,
                    })
                }
                Op::TilemapPlane(plane) => {
                    let table = |id| {
                        self.tables
                            .get(id)
                            .map_err(|source| ComposeError::InvalidOpTable { index, source })
                    };
                    let mut bitmaps = [None; 9];
                    for (found, id) in bitmaps.iter_mut().zip(plane.bitmaps) {
                        let bitmap = id.map(|id| self.bitmaps.get(id)).transpose();
                        *found = bitmap
                            .map_err(|source| ComposeError::InvalidOpBitmap { index, source })?;
                    }
                    Step::TilemapPlane(PlaneStep {
                        op: plane,
                        map: table(plane.map)?,
                        passages: plane.passages.map(table).transpose()?,
                        flash: plane.flash.map(table).transpose()?,
                        bitmaps,
                    })
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
    Tilemap(Layer<'a>),
    TilemapPlane(PlaneStep<'a>),
}

impl Step<'_> {
    /// The value of pixel (`px`, `py`) of a frame of `frame` pixels after this op, given its
    /// value before.
    fn apply(&self, pixel: Rgba, frame: Size, px: u16, py: u16) -> Rgba {
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
                    vkrgss_pixel::apply_tone(texel, tone.red, tone.green, tone.blue, tone.gray);
                let texel = vkrgss_pixel::blend_color(texel, quad.color.to_rgba());
                match quad.blend {
                    BlendMode::Normal => vkrgss_pixel::blend_normal(pixel, texel, quad.opacity),
                    BlendMode::Add => vkrgss_pixel::blend_add(pixel, texel, quad.opacity),
                    BlendMode::Subtract => vkrgss_pixel::blend_sub(pixel, texel, quad.opacity),
                    BlendMode::Window if quad.is_window_tail(frame, px) => {
                        vkrgss_pixel::blend_normal(pixel, texel, quad.opacity)
                    }
                    BlendMode::Window => vkrgss_pixel::blend_window(pixel, texel, quad.opacity),
                }
            }
            Step::Tilemap(layer) => {
                if !layer.op.clip.contains(x, y) {
                    return pixel;
                }
                match layer.texel(px, py) {
                    Some(texel) => vkrgss_pixel::blend_normal(pixel, texel, 255),
                    None => pixel,
                }
            }
            Step::TilemapPlane(plane) => {
                if !plane.op.clip.contains(x, y) {
                    return pixel;
                }
                plane.apply(pixel, frame, px, py)
            }
            Step::RectEffect(effect) => {
                if !effect.clip.contains(x, y) {
                    return pixel;
                }
                let tone = effect.tone;
                let toned =
                    vkrgss_pixel::apply_tone(pixel, tone.red, tone.green, tone.blue, tone.gray);
                vkrgss_pixel::blend_color(toned, effect.color.to_rgba())
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
