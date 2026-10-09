//! The frame packet: bitmap changes and the ordered op list.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{BitmapId, Color, Fixed, Rect, Size, Tone};

/// Everything a backend needs to produce one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FramePacket {
    /// Counts frames from the start of the guest.
    pub frame_index: u64,
    /// Size of the logical screen in pixels. Op coordinates are in this space.
    pub logical_size: Size,
    /// The frame rate the guest runs at. Compositors ignore it; the host paces to it.
    pub frame_rate: FrameRate,
    /// Bitmap changes since the previous packet, applied in order before any op is drawn.
    pub bitmaps: Vec<BitmapDelta>,
    /// Draw ops. Their order is the draw order; backends never reorder them.
    pub ops: Vec<Op>,
}

/// A frame rate in whole frames per second, always from [`FrameRate::MIN`] to
/// [`FrameRate::MAX`].
///
/// The range is that of `Graphics.frame_rate`. A value outside it cannot be constructed and
/// does not decode, so no receiver has to check it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct FrameRate(u8);

impl FrameRate {
    /// The lowest frame rate.
    pub const MIN: FrameRate = FrameRate(10);
    /// The highest frame rate.
    pub const MAX: FrameRate = FrameRate(120);
    /// The frame rate a game starts with.
    pub const DEFAULT: FrameRate = FrameRate(40);

    /// The frame rate of `fps` frames per second, if that is in range.
    pub const fn new(fps: u8) -> Result<FrameRate, FrameRateOutOfRange> {
        if fps >= Self::MIN.0 && fps <= Self::MAX.0 {
            Ok(FrameRate(fps))
        } else {
            Err(FrameRateOutOfRange { found: fps })
        }
    }

    /// The frame rate nearest to `fps`, as `Graphics.frame_rate=` assigns it.
    pub const fn clamped(fps: i64) -> FrameRate {
        if fps < Self::MIN.0 as i64 {
            Self::MIN
        } else if fps > Self::MAX.0 as i64 {
            Self::MAX
        } else {
            FrameRate(fps as u8)
        }
    }

    /// Frames per second.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl Default for FrameRate {
    fn default() -> FrameRate {
        FrameRate::DEFAULT
    }
}

impl TryFrom<u8> for FrameRate {
    type Error = FrameRateOutOfRange;

    fn try_from(fps: u8) -> Result<FrameRate, FrameRateOutOfRange> {
        FrameRate::new(fps)
    }
}

impl From<FrameRate> for u8 {
    fn from(rate: FrameRate) -> u8 {
        rate.get()
    }
}

impl std::fmt::Display for FrameRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A number of frames per second that is not a valid [`FrameRate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("frame rate {found} is outside {min} to {max}", min = FrameRate::MIN, max = FrameRate::MAX)]
pub struct FrameRateOutOfRange {
    pub found: u8,
}

/// A change to the set of bitmaps known to the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BitmapDelta {
    /// Creates a bitmap whose pixels are all transparent black.
    Create {
        id: BitmapId,
        width: u32,
        height: u32,
    },
    /// Replaces the pixels inside `rect`. `rgba` holds `rect.width * rect.height` pixels in row
    /// order, four bytes each (red, green, blue, straight alpha).
    Update {
        id: BitmapId,
        rect: Rect,
        rgba: Vec<u8>,
    },
    /// Destroys a bitmap. Its id is invalid from here on, even if the slot is reused.
    Destroy { id: BitmapId },
}

/// One step of drawing. Every op carries a clip rectangle in logical screen pixels and never
/// changes a pixel outside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    /// Sets every pixel in `clip` to `color`, alpha included.
    Clear { clip: Rect, color: Color },
    /// Draws part of a bitmap.
    Quad(Quad),
    /// Changes the pixels already present in a rectangle.
    RectEffect(RectEffect),
    /// Reserved for planes. Not specified yet; backends reject it.
    Plane { clip: Rect },
    /// Reserved for tilemap layers. Not specified yet; backends reject it.
    TilemapLayer { clip: Rect },
}

impl Op {
    /// The kind of this op, for reports and errors.
    pub const fn kind(&self) -> OpKind {
        match self {
            Op::Clear { .. } => OpKind::Clear,
            Op::Quad(_) => OpKind::Quad,
            Op::RectEffect(_) => OpKind::RectEffect,
            Op::Plane { .. } => OpKind::Plane,
            Op::TilemapLayer { .. } => OpKind::TilemapLayer,
        }
    }

    /// The clip rectangle of this op.
    pub const fn clip(&self) -> Rect {
        match self {
            Op::Clear { clip, .. } | Op::Plane { clip } | Op::TilemapLayer { clip } => *clip,
            Op::Quad(quad) => quad.clip,
            Op::RectEffect(effect) => effect.clip,
        }
    }
}

/// The name of an op variant without its data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpKind {
    Clear,
    Quad,
    RectEffect,
    Plane,
    TilemapLayer,
}

impl OpKind {
    /// Whether the kind is only a reserved name, with no specified behaviour yet.
    pub const fn is_reserved(self) -> bool {
        matches!(self, OpKind::Plane | OpKind::TilemapLayer)
    }
}

impl std::fmt::Display for OpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            OpKind::Clear => "Clear",
            OpKind::Quad => "Quad",
            OpKind::RectEffect => "RectEffect",
            OpKind::Plane => "Plane",
            OpKind::TilemapLayer => "TilemapLayer",
        })
    }
}

/// Draws the `src` rectangle of a bitmap through an affine transform.
///
/// A screen pixel is covered when `transform` maps its centre into the source rectangle; it then
/// takes the texel under that point (nearest sampling). The texel is changed first by `tone`,
/// then by `color` blended over it with the colour's alpha, both keeping the texel's own alpha.
/// The result is blended over the pixel underneath at `opacity` in mode `blend`.
///
/// Tone and colour recolour what the quad draws, never the pixels underneath: that is
/// [`RectEffect`]. [`Tone::NEUTRAL`] and a colour of alpha 0 leave the texel as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quad {
    pub clip: Rect,
    pub bitmap: BitmapId,
    /// The part of the bitmap to draw, in bitmap pixels. It must lie inside the bitmap.
    pub src: Rect,
    pub transform: Transform,
    /// Applied to the sampled texel first.
    pub tone: Tone,
    /// Blended over the toned texel with its alpha; alpha 0 changes nothing.
    pub color: Color,
    /// 0 draws nothing, 255 draws the bitmap at its own alpha.
    pub opacity: u8,
    pub blend: BlendMode,
}

/// How a quad combines with the pixels underneath.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlendMode {
    #[default]
    Normal,
    /// Adds the source to the destination, saturating at 255.
    Add,
    /// Subtracts the source from the destination, saturating at 0.
    Subtract,
}

/// A fixed-point affine map from logical screen space to the source rectangle of a quad.
///
/// For the screen pixel (`px`, `py`), with `X = px + 1/2` and `Y = py + 1/2` (its centre):
///
/// ```text
/// u = xx * X + xy * Y + tx
/// v = yx * X + yy * Y + ty
/// ```
///
/// (`u`, `v`) is a position inside the source rectangle, in bitmap pixels relative to the
/// rectangle's top-left corner. Sending the map in this direction means a backend evaluates it
/// per pixel and never has to invert a matrix.
///
/// The arithmetic is exact: each sum of products is formed without overflow and then rounded
/// towards negative infinity to 16 fractional bits before the translation is added. See
/// [`Transform::source_position`], which every backend must match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Transform {
    pub xx: Fixed,
    pub xy: Fixed,
    pub yx: Fixed,
    pub yy: Fixed,
    pub tx: Fixed,
    pub ty: Fixed,
}

impl Transform {
    /// Draws the source rectangle unscaled with its top-left corner at the origin.
    pub const IDENTITY: Transform = Transform::place_at(0, 0);

    /// Draws the source rectangle unscaled with its top-left corner at screen pixel (`x`, `y`).
    pub const fn place_at(x: i16, y: i16) -> Transform {
        Transform {
            xx: Fixed::ONE,
            xy: Fixed::ZERO,
            yx: Fixed::ZERO,
            yy: Fixed::ONE,
            tx: Self::offset(x),
            ty: Self::offset(y),
        }
    }

    /// The translation that moves screen coordinate `position` to source coordinate 0. The one
    /// value that does not fit, 32768 for `i16::MIN`, saturates.
    const fn offset(position: i16) -> Fixed {
        let bits = -(position as i64) << Fixed::FRAC_BITS;
        Fixed::from_bits(if bits > i32::MAX as i64 {
            i32::MAX
        } else {
            bits as i32
        })
    }

    /// The position (`u`, `v`) that the centre of screen pixel (`px`, `py`) maps to, as
    /// fixed-point numbers with 16 fractional bits. The texel under it is
    /// (`u >> 16`, `v >> 16`), relative to the source rectangle.
    pub const fn source_position(&self, px: u16, py: u16) -> (i64, i64) {
        let x = ((px as i128) << Fixed::FRAC_BITS) + (1 << (Fixed::FRAC_BITS - 1));
        let y = ((py as i128) << Fixed::FRAC_BITS) + (1 << (Fixed::FRAC_BITS - 1));
        let u = (self.xx.to_bits() as i128 * x + self.xy.to_bits() as i128 * y) >> Fixed::FRAC_BITS;
        let v = (self.yx.to_bits() as i128 * x + self.yy.to_bits() as i128 * y) >> Fixed::FRAC_BITS;
        (
            u as i64 + self.tx.to_bits() as i64,
            v as i64 + self.ty.to_bits() as i64,
        )
    }
}

/// Changes the pixels already present inside `clip`: first the tone, then the colour blended
/// over the result with the colour's alpha. No bitmap is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RectEffect {
    /// The rectangle the effect applies to.
    pub clip: Rect,
    pub tone: Tone,
    pub color: Color,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_reports_kind_and_clip() {
        let clip = Rect::new(1, 2, 3, 4);
        let quad = Quad {
            clip,
            bitmap: BitmapId::new(0, 0),
            src: Rect::new(0, 0, 1, 1),
            transform: Transform::IDENTITY,
            tone: Tone::NEUTRAL,
            color: Color::default(),
            opacity: 255,
            blend: BlendMode::Normal,
        };
        assert_eq!(quad.tone, Tone::new(0, 0, 0, 0));
        assert_eq!(quad.color.a, 0);
        let effect = RectEffect {
            clip,
            tone: Tone::NEUTRAL,
            color: Color::default(),
        };
        let cases = [
            (
                Op::Clear {
                    clip,
                    color: Color::default(),
                },
                OpKind::Clear,
                "Clear",
            ),
            (Op::Quad(quad), OpKind::Quad, "Quad"),
            (Op::RectEffect(effect), OpKind::RectEffect, "RectEffect"),
            (Op::Plane { clip }, OpKind::Plane, "Plane"),
            (
                Op::TilemapLayer { clip },
                OpKind::TilemapLayer,
                "TilemapLayer",
            ),
        ];
        for (op, kind, name) in cases {
            assert_eq!(op.kind(), kind);
            assert_eq!(op.clip(), clip);
            assert_eq!(kind.to_string(), name);
        }
    }

    #[test]
    fn only_plane_and_tilemap_layer_are_reserved() {
        assert!(OpKind::Plane.is_reserved());
        assert!(OpKind::TilemapLayer.is_reserved());
        assert!(!OpKind::Clear.is_reserved());
        assert!(!OpKind::Quad.is_reserved());
        assert!(!OpKind::RectEffect.is_reserved());
    }

    #[test]
    fn frame_rate_accepts_exactly_10_to_120() {
        assert_eq!(FrameRate::new(9), Err(FrameRateOutOfRange { found: 9 }));
        assert_eq!(FrameRate::new(10), Ok(FrameRate::MIN));
        assert_eq!(FrameRate::new(120), Ok(FrameRate::MAX));
        assert_eq!(FrameRate::new(121), Err(FrameRateOutOfRange { found: 121 }));
        for fps in 0..=u8::MAX {
            let rate = FrameRate::new(fps);
            assert_eq!(rate.is_ok(), (10..=120).contains(&fps), "{fps}");
            assert_eq!(FrameRate::try_from(fps), rate);
            if let Ok(rate) = rate {
                assert_eq!(rate.get(), fps);
                assert_eq!(u8::from(rate), fps);
            }
        }
        assert_eq!(
            FrameRate::new(9).unwrap_err().to_string(),
            "frame rate 9 is outside 10 to 120"
        );
    }

    #[test]
    fn frame_rate_defaults_to_40() {
        assert_eq!(FrameRate::default().get(), 40);
        assert_eq!(FrameRate::DEFAULT, FrameRate::default());
        assert_eq!(FrameRate::DEFAULT.to_string(), "40");
    }

    #[test]
    fn frame_rate_clamps_like_graphics_frame_rate() {
        assert_eq!(FrameRate::clamped(i64::MIN), FrameRate::MIN);
        assert_eq!(FrameRate::clamped(-1), FrameRate::MIN);
        assert_eq!(FrameRate::clamped(9), FrameRate::MIN);
        assert_eq!(FrameRate::clamped(10), FrameRate::MIN);
        assert_eq!(FrameRate::clamped(11).get(), 11);
        assert_eq!(FrameRate::clamped(60).get(), 60);
        assert_eq!(FrameRate::clamped(119).get(), 119);
        assert_eq!(FrameRate::clamped(120), FrameRate::MAX);
        assert_eq!(FrameRate::clamped(121), FrameRate::MAX);
        assert_eq!(
            FrameRate::clamped(376),
            FrameRate::MAX,
            "376 is 120 mod 256"
        );
        assert_eq!(FrameRate::clamped(i64::MAX), FrameRate::MAX);
    }

    fn texel(transform: &Transform, px: u16, py: u16) -> (i64, i64) {
        let (u, v) = transform.source_position(px, py);
        (u >> 16, v >> 16)
    }

    #[test]
    fn placement_maps_pixels_to_texels_one_to_one() {
        let transform = Transform::place_at(10, 20);
        assert_eq!(texel(&transform, 10, 20), (0, 0));
        assert_eq!(texel(&transform, 13, 25), (3, 5));
        assert_eq!(texel(&transform, 9, 20), (-1, 0));
        assert_eq!(texel(&Transform::place_at(-4, -6), 0, 0), (4, 6));
        assert_eq!(texel(&Transform::IDENTITY, 7, 8), (7, 8));
    }

    #[test]
    fn half_scale_map_doubles_the_image() {
        let transform = Transform {
            xx: Fixed::HALF,
            yy: Fixed::HALF,
            ..Transform::IDENTITY
        };
        assert_eq!(texel(&transform, 0, 0), (0, 0));
        assert_eq!(texel(&transform, 1, 1), (0, 0));
        assert_eq!(texel(&transform, 2, 3), (1, 1));
        assert_eq!(texel(&transform, 5, 4), (2, 2));
    }

    #[test]
    fn extreme_transforms_do_not_overflow() {
        let max = Fixed::from_bits(i32::MAX);
        let min = Fixed::from_bits(i32::MIN);
        let transform = Transform {
            xx: max,
            xy: max,
            yx: min,
            yy: min,
            tx: max,
            ty: min,
        };
        let (u, v) = transform.source_position(u16::MAX, u16::MAX);
        assert!(u > 0);
        assert!(v < 0);
    }
}
