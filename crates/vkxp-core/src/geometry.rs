//! Geometry of a quad: from position, origin, zoom, angle and mirror to the protocol's
//! fixed-point transform, to the rectangle of a stretch, or to the texels of a turned sprite.
//!
//! This is the one place where floating point becomes protocol data or pixels. The
//! coefficients of a transform are computed in `f64` with operations that give the same bits
//! on every platform (sines and cosines come from the pure-Rust `libm`), and each is then
//! rounded to the nearest multiple of 2^-16. The rectangle of a stretch is computed in `f32`,
//! as the original runtime does, with operations that IEEE 754 defines exactly. A turned
//! sprite is walked in `f64`, as the original runtime does.

use vkxp_proto::{Fixed, Transform};

/// How a source rectangle is placed on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Geometry {
    /// The screen position the origin lands on.
    pub(crate) x: f64,
    pub(crate) y: f64,
    /// The origin, in pixels of the source rectangle.
    pub(crate) ox: f64,
    pub(crate) oy: f64,
    pub(crate) zoom_x: f64,
    pub(crate) zoom_y: f64,
    /// Degrees, counter-clockwise on screen.
    pub(crate) angle: f64,
    pub(crate) mirror: bool,
}

impl Geometry {
    /// The source rectangle unscaled, with its top-left corner at screen pixel (`x`, `y`).
    pub(crate) fn at(x: i64, y: i64) -> Geometry {
        Geometry {
            // Positions are sums of a few `i32`, which `f64` holds exactly.
            x: x as f64,
            y: y as f64,
            ox: 0.0,
            oy: 0.0,
            zoom_x: 1.0,
            zoom_y: 1.0,
            angle: 0.0,
            mirror: false,
        }
    }

    /// The map from a screen point to a position in the source rectangle,
    ///
    /// ```text
    /// u = ( cos a * (X - x) - sin a * (Y - y)) / zoom_x + ox
    /// v = ( sin a * (X - x) + cos a * (Y - y)) / zoom_y + oy
    /// ```
    ///
    /// with `u` replaced by `src_width - u` when mirrored. `cut_left` and `cut_top` are then
    /// subtracted: they are what was cut off the source rectangle at its left and top, so
    /// that the result is relative to the rectangle that is left.
    ///
    /// `None` when the placement draws nothing or cannot be expressed: a zoom of 0, a value
    /// that is not a number, or a coefficient outside the fixed-point range.
    ///
    /// A negative zoom goes into the coefficients with its sign, which flips the image. No
    /// sprite reaches this with one: measured on the original runtime (RGSS103J, negative
    /// zoom probe `ph-scene-negzoom-probe.rb`, 96 cases without a differing pixel), a sprite
    /// at angle 0 with a zoom of 0 or less draws nothing, and a turned sprite follows
    /// [`Geometry::turned_box`] with the signed zoom.
    pub(crate) fn transform(
        &self,
        src_width: f64,
        cut_left: f64,
        cut_top: f64,
    ) -> Option<Transform> {
        if self.zoom_x == 0.0 || self.zoom_y == 0.0 {
            return None;
        }
        let (sin, cos) = sin_cos(self.angle);
        let mut xx = cos / self.zoom_x;
        let mut xy = -sin / self.zoom_x;
        let mut tx = self.ox - (cos * self.x - sin * self.y) / self.zoom_x;
        let yx = sin / self.zoom_y;
        let yy = cos / self.zoom_y;
        let ty = self.oy - (sin * self.x + cos * self.y) / self.zoom_y;
        if self.mirror {
            xx = -xx;
            xy = -xy;
            tx = src_width - tx;
        }
        Some(Transform {
            xx: to_fixed(xx)?,
            xy: to_fixed(xy)?,
            yx: to_fixed(yx)?,
            yy: to_fixed(yy)?,
            tx: to_fixed(tx - cut_left)?,
            ty: to_fixed(ty - cut_top)?,
        })
    }
}

/// The box of whole screen pixels a turned sprite is walked over, with the sine and cosine
/// its rows are computed from. The last column and the last row, `right` and `bottom`, are
/// not drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TurnedBox {
    pub(crate) left: i64,
    pub(crate) top: i64,
    pub(crate) right: i64,
    pub(crate) bottom: i64,
    /// Of the angle the original runtime turns by, which is the negated angle of the sprite.
    sin: f64,
    cos: f64,
}

/// Beyond this a corner of a turned sprite has no place in a box.
const CORNER_END: f64 = 2_147_483_648.0;

impl Geometry {
    /// The box of a sprite turned by an angle other than 0, as the original runtime computes
    /// it in double precision: the corners of the source rectangle of `width` by `height`,
    ///
    /// ```text
    /// t = ((-angle) + (-angle)) * pi / 360,  s = sin t,  c = cos t
    /// P0 = (x - (ox*c*zoom_x - oy*s*zoom_y),  y - (ox*s*zoom_x + oy*c*zoom_y))
    /// P1 = (w*c*zoom_x + P0.x,  w*s*zoom_x + P0.y)
    /// P2 = (P0.x - h*s*zoom_y,  h*c*zoom_y + P0.y)
    /// P3 = (w*c*zoom_x + P2.x,  w*s*zoom_x + P2.y)
    /// ```
    ///
    /// each coordinate rounded with `floor(v + 0.5)`, and the smallest and largest of them.
    /// The products are formed from left to right, which matters for the last bit.
    ///
    /// `None` when nothing is drawn: a zoom of 0, an empty source rectangle, or a corner that
    /// is not a number or beyond the coordinates of a rectangle.
    ///
    /// Provenance: oracle measurement 2026-10-09, RGSS104E, sprite angle probe (36 whole
    /// frames without a differing pixel); the routine is at 0x10021960 of that DLL.
    ///
    /// A negative zoom takes part with its sign and flips the sprite about its origin:
    /// oracle measurement, RGSS103J, negative zoom probe (`ph-scene-negzoom-probe.rb`: 96
    /// cases without a differing pixel).
    pub(crate) fn turned_box(&self, width: i32, height: i32) -> Option<TurnedBox> {
        if self.zoom_x == 0.0 || self.zoom_y == 0.0 || width <= 0 || height <= 0 {
            return None;
        }
        let (x, y, ox, oy) = (self.x, self.y, self.ox, self.oy);
        let (zx, zy) = (self.zoom_x, self.zoom_y);
        let (w, h) = (f64::from(width), f64::from(height));
        let t = ((-self.angle) + (-self.angle)) * std::f64::consts::PI / 360.0;
        let (s, c) = (libm::sin(t), libm::cos(t));
        let p0 = (
            x - (ox * c * zx - oy * s * zy),
            y - (ox * s * zx + oy * c * zy),
        );
        let p1 = (w * c * zx + p0.0, w * s * zx + p0.1);
        let p2 = (p0.0 - h * s * zy, h * c * zy + p0.1);
        let p3 = (w * c * zx + p2.0, w * s * zx + p2.1);
        let corners = [p0, p1, p2, p3];
        let mut xs = [0i64; 4];
        let mut ys = [0i64; 4];
        for (i, corner) in corners.into_iter().enumerate() {
            let (px, py) = ((corner.0 + 0.5).floor(), (corner.1 + 0.5).floor());
            // Not a number fails both comparisons.
            if !(px.abs() < CORNER_END && py.abs() < CORNER_END) {
                return None;
            }
            (xs[i], ys[i]) = (px as i64, py as i64);
        }
        Some(TurnedBox {
            left: xs.into_iter().min()?,
            top: ys.into_iter().min()?,
            right: xs.into_iter().max()?,
            bottom: ys.into_iter().max()?,
            sin: s,
            cos: c,
        })
    }

    /// Row `j` of the box of a turned sprite, as the original runtime walks it: the position
    /// in the bitmap is computed at the left edge of the box,
    ///
    /// ```text
    /// U = (src.x + ox) + ((left - x)*c + (j - y)*s) / zoom_x
    /// V = (src.y + oy) + ((-s)*(left - x) + (j - y)*c) / zoom_y
    /// mirrored: U = (src.x + w - 1) - (U - src.x)
    /// ```
    ///
    /// and then advanced pixel by pixel, `U` by `c / zoom_x` (subtracted when mirrored) and
    /// `V` by `(-s) / zoom_y`. A pixel shows the texel at the truncated position when that
    /// is inside `src`. The sums are accumulated, not computed anew, which decides the texel
    /// wherever the exact position is a whole number.
    ///
    /// `show` is called with the column, counted from the left edge of the box, and the
    /// texel, for every pixel of `columns` that shows one.
    ///
    /// Provenance: as [`Geometry::turned_box`]; the loop is at 0x10022720.
    pub(crate) fn turned_row(
        &self,
        bounds: &TurnedBox,
        src: crate::Rect,
        j: i64,
        columns: std::ops::Range<i64>,
        mut show: impl FnMut(i64, i32, i32),
    ) {
        let (s, c) = (bounds.sin, bounds.cos);
        // Whole numbers far inside what `f64` holds exactly, and so are their differences.
        let dx = bounds.left as f64 - self.x;
        let dy = j as f64 - self.y;
        let (src_x, src_y) = (f64::from(src.x), f64::from(src.y));
        let (src_right, src_bottom) = (src_x + f64::from(src.width), src_y + f64::from(src.height));
        let mut u = (src_x + self.ox) + (dx * c + dy * s) / self.zoom_x;
        let mut v = (src_y + self.oy) + ((-s) * dx + dy * c) / self.zoom_y;
        if self.mirror {
            u = (src_right - 1.0) - (u - src_x);
        }
        let du = c / self.zoom_x;
        let dv = (-s) / self.zoom_y;
        for i in 0..columns.end {
            if i >= columns.start && src_x <= u && src_y <= v && u < src_right && v < src_bottom {
                // Inside `src`, whose coordinates are `i32`.
                show(i, u as i32, v as i32);
            }
            u = if self.mirror { u - du } else { du + u };
            v += dv;
        }
    }
}

/// The largest whole number below 2^32 and 2^31 that `f32` holds is below these.
const F32_U32_END: f32 = 4_294_967_296.0;
const F32_I32_END: f32 = 2_147_483_648.0;

/// The number of screen pixels that `length` source pixels are stretched into at `zoom`, as
/// the original runtime finds it: `floor(length * zoom)` in 32-bit floating point, the zoom
/// rounded to that precision first. `None` when nothing is drawn: a zoom of 0 or less or not
/// a number, or a result of 0 or beyond what a rectangle can hold.
pub(crate) fn stretch_length(length: i32, zoom: f64) -> Option<u32> {
    let zoom = zoom as f32;
    if zoom.is_nan() || zoom <= 0.0 {
        return None;
    }
    let pixels = (length as f32 * zoom).floor();
    (1.0..F32_U32_END)
        .contains(&pixels)
        .then_some(pixels as u32)
}

/// The number of screen pixels one tile of a plane of RGSS2 takes along an axis `length`
/// texels long at `zoom`: `floor(length * zoom)` in double precision. `None` as for
/// [`stretch_length`].
///
/// Provenance: oracle measurement 2026-10-12, RGSS202E versions 2.0.0.1 and 2.0.2.1,
/// `vx-v1-sprite-REPORT.md` section 4 (topics `plane`, `size` and `size2`: 197 pairs on both
/// axes without a differing pixel, where the rule of 32-bit floats is wrong for all 93 pairs
/// of `size2`).
pub(crate) fn stretch_length_f64(length: i32, zoom: f64) -> Option<u32> {
    if zoom.is_nan() || zoom <= 0.0 {
        return None;
    }
    let pixels = (f64::from(length) * zoom).floor();
    (1.0..f64::from(F32_U32_END))
        .contains(&pixels)
        .then_some(pixels as u32)
}

/// The first screen pixel of an unrotated sprite along one axis, as the original runtime
/// finds it: `floor(position - origin * zoom)` in 32-bit floating point. `None` when it is
/// not a number or beyond the coordinates of a rectangle.
pub(crate) fn stretch_start(position: i32, origin: i32, zoom: f64) -> Option<i32> {
    let start = (position as f32 - origin as f32 * zoom as f32).floor();
    (-F32_I32_END..F32_I32_END)
        .contains(&start)
        .then_some(start as i32)
}

/// Sine and cosine of an angle in degrees. Multiples of 90 degrees give exactly 0, 1 and -1.
fn sin_cos(degrees: f64) -> (f64, f64) {
    // The remainder of a floating-point division is exact.
    let turn = degrees.rem_euclid(360.0);
    if turn == 0.0 {
        (0.0, 1.0)
    } else if turn == 90.0 {
        (1.0, 0.0)
    } else if turn == 180.0 {
        (0.0, -1.0)
    } else if turn == 270.0 {
        (-1.0, 0.0)
    } else {
        let radians = turn.to_radians();
        (libm::sin(radians), libm::cos(radians))
    }
}

/// The multiple of 2^-16 nearest to `value` (halves away from zero), when `value` is a number
/// inside the fixed-point range.
fn to_fixed(value: f64) -> Option<Fixed> {
    // Scaling by a power of two is exact, so this rounds only once.
    let bits = (value * 65536.0).round();
    (bits >= f64::from(i32::MIN) && bits <= f64::from(i32::MAX))
        .then(|| Fixed::from_bits(bits as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The texel under the centre of screen pixel (`px`, `py`).
    fn texel(transform: &Transform, px: u16, py: u16) -> (i64, i64) {
        let (u, v) = transform.source_position(px, py);
        (u >> 16, v >> 16)
    }

    fn bits(transform: &Transform) -> [i32; 4] {
        [transform.xx, transform.xy, transform.yx, transform.yy].map(Fixed::to_bits)
    }

    const ONE: i32 = 1 << 16;

    #[test]
    fn quarter_turns_have_exact_coefficients() {
        let cases = [
            (0.0, [ONE, 0, 0, ONE]),
            (90.0, [0, -ONE, ONE, 0]),
            (180.0, [-ONE, 0, 0, -ONE]),
            (270.0, [0, ONE, -ONE, 0]),
            (360.0, [ONE, 0, 0, ONE]),
            (-90.0, [0, ONE, -ONE, 0]),
            (-180.0, [-ONE, 0, 0, -ONE]),
            (450.0, [0, -ONE, ONE, 0]),
            (-720.0, [ONE, 0, 0, ONE]),
        ];
        for (angle, expected) in cases {
            let geometry = Geometry {
                angle,
                ..Geometry::at(0, 0)
            };
            let transform = geometry.transform(8.0, 0.0, 0.0).unwrap();
            assert_eq!(bits(&transform), expected, "angle {angle}");
            assert_eq!((transform.tx, transform.ty), (Fixed::ZERO, Fixed::ZERO));
        }
    }

    #[test]
    fn quarter_turns_keep_whole_translations_whole() {
        for angle in [0.0, 90.0, 180.0, 270.0] {
            let geometry = Geometry {
                ox: 3.0,
                oy: 5.0,
                angle,
                ..Geometry::at(101, -57)
            };
            let transform = geometry.transform(8.0, 0.0, 0.0).unwrap();
            assert_eq!(transform.tx.fract_bits(), 0, "angle {angle}");
            assert_eq!(transform.ty.fract_bits(), 0, "angle {angle}");
        }
    }

    #[test]
    fn other_angles_are_rounded_to_the_nearest_step() {
        let geometry = Geometry {
            angle: 30.0,
            ..Geometry::at(0, 0)
        };
        let transform = geometry.transform(8.0, 0.0, 0.0).unwrap();
        // cos 30 = 0.8660254..., sin 30 = 0.5 up to the last bit of the sine.
        assert_eq!(bits(&transform), [56756, -32768, 32768, 56756]);
    }

    #[test]
    fn plain_placement_maps_pixels_one_to_one() {
        let transform = Geometry::at(10, 20).transform(4.0, 0.0, 0.0).unwrap();
        assert_eq!(transform, Transform::place_at(10, 20));
        assert_eq!(texel(&transform, 10, 20), (0, 0));
        assert_eq!(texel(&transform, 13, 22), (3, 2));
    }

    #[test]
    fn the_origin_lands_on_the_position() {
        let geometry = Geometry {
            ox: 1.0,
            oy: 1.0,
            ..Geometry::at(10, 20)
        };
        let transform = geometry.transform(4.0, 0.0, 0.0).unwrap();
        assert_eq!(texel(&transform, 10, 20), (1, 1));
        assert_eq!(texel(&transform, 9, 19), (0, 0));
    }

    #[test]
    fn zoom_scales_about_the_origin() {
        // A 4 by 4 source at three times its width and twice its height, its texel (1, 1)
        // starting at screen pixel (20, 30).
        let geometry = Geometry {
            ox: 1.0,
            oy: 1.0,
            zoom_x: 3.0,
            zoom_y: 2.0,
            ..Geometry::at(20, 30)
        };
        let transform = geometry.transform(4.0, 0.0, 0.0).unwrap();
        for px in 17..29u16 {
            for py in 28..36u16 {
                let expected = (i64::from(px - 17) / 3, i64::from(py - 28) / 2);
                assert_eq!(texel(&transform, px, py), expected, "({px}, {py})");
            }
        }
        assert_eq!(texel(&transform, 16, 28).0, -1);
        assert_eq!(texel(&transform, 29, 28).0, 4);
    }

    #[test]
    fn mirror_flips_the_source_width_in_place() {
        let geometry = Geometry {
            mirror: true,
            ..Geometry::at(10, 10)
        };
        let transform = geometry.transform(4.0, 0.0, 0.0).unwrap();
        for column in 0..4u16 {
            assert_eq!(
                texel(&transform, 10 + column, 10),
                (3 - i64::from(column), 0)
            );
        }
        assert_eq!(texel(&transform, 9, 10).0, 4);
        assert_eq!(texel(&transform, 14, 10).0, -1);

        // Mirrored and zoomed, about an origin.
        let geometry = Geometry {
            ox: 2.0,
            zoom_x: 2.0,
            mirror: true,
            ..Geometry::at(10, 10)
        };
        let transform = geometry.transform(4.0, 0.0, 0.0).unwrap();
        let columns: Vec<i64> = (6..14).map(|px| texel(&transform, px, 10).0).collect();
        assert_eq!(columns, [3, 3, 2, 2, 1, 1, 0, 0]);
    }

    #[test]
    fn a_quarter_turn_is_counter_clockwise_on_screen() {
        let geometry = Geometry {
            angle: 90.0,
            ..Geometry::at(10, 10)
        };
        let transform = geometry.transform(2.0, 0.0, 0.0).unwrap();
        assert_eq!(texel(&transform, 10, 9), (0, 0));
        assert_eq!(texel(&transform, 10, 8), (1, 0));
    }

    #[test]
    fn a_cut_shifts_the_map() {
        let transform = Geometry::at(10, 10).transform(4.0, 2.0, 1.0).unwrap();
        // What was texel (2, 1) of the full rectangle is texel (0, 0) of the rest.
        assert_eq!(texel(&transform, 12, 11), (0, 0));
    }

    #[test]
    fn placements_that_draw_nothing_have_no_transform() {
        let base = Geometry::at(0, 0);
        for geometry in [
            Geometry {
                zoom_x: 0.0,
                ..base
            },
            Geometry {
                zoom_y: 0.0,
                ..base
            },
            Geometry {
                zoom_x: -0.0,
                ..base
            },
            Geometry {
                zoom_x: f64::NAN,
                ..base
            },
            Geometry {
                zoom_y: f64::INFINITY,
                angle: f64::NAN,
                ..base
            },
            Geometry {
                angle: f64::NAN,
                ..base
            },
            Geometry {
                angle: f64::INFINITY,
                ..base
            },
            Geometry {
                zoom_x: 1e-9,
                ..base
            },
            Geometry { x: 40000.0, ..base },
        ] {
            assert_eq!(geometry.transform(4.0, 0.0, 0.0), None, "{geometry:?}");
        }
    }

    #[test]
    fn stretch_lengths_are_floored_in_single_precision() {
        assert_eq!(stretch_length(16, 1.0), Some(16));
        assert_eq!(stretch_length(16, 2.5), Some(40));
        assert_eq!(stretch_length(16, 0.14), Some(2));
        // 16 * 1.1 is 17.6 either way, but 0.7 as a 32-bit float is above 0.7: 10 * 0.7 is 7
        // there and 6.999... in double precision.
        assert_eq!(stretch_length(16, 1.1), Some(17));
        assert_eq!(stretch_length(10, 0.7), Some(7));
        assert_eq!((10.0 * 0.7f64).floor(), 7.0);
        assert_eq!(stretch_length(100, 0.29), Some(29));
        assert_eq!((100.0 * 0.29f64).floor(), 28.0);
        for (length, zoom) in [
            (16, 0.0),
            (16, -1.0),
            (16, -0.0),
            (16, f64::NAN),
            (16, f64::INFINITY),
            (16, 0.06),
            (0, 3.0),
            (-16, 3.0),
            (i32::MAX, 2.0),
            (16, 1e30),
        ] {
            assert_eq!(stretch_length(length, zoom), None, "{length} at {zoom}");
        }
        assert_eq!(stretch_length(i32::MAX, 1.0), Some(1 << 31));
    }

    #[test]
    fn stretch_starts_are_floored_in_single_precision() {
        assert_eq!(stretch_start(10, 0, 3.3), Some(10));
        assert_eq!(stretch_start(260, 5, 0.5), Some(257));
        assert_eq!(stretch_start(260, 5, 0.08), Some(259));
        assert_eq!(stretch_start(260, 5, 1.02), Some(254));
        assert_eq!(stretch_start(-3, 1, 1.5), Some(-5));
        assert_eq!(stretch_start(0, 1, 1e30), None);
        assert_eq!(stretch_start(0, 1, f64::NAN), None);
        assert_eq!(stretch_start(i32::MIN, 0, 1.0), Some(i32::MIN));
        assert_eq!(stretch_start(i32::MAX, 0, 1.0), None);
    }

    #[test]
    fn the_fixed_point_range_is_respected_at_its_ends() {
        assert_eq!(to_fixed(-32768.0), Some(Fixed::from_bits(i32::MIN)));
        assert_eq!(to_fixed(32768.0), None);
        assert_eq!(to_fixed(32767.99999), Some(Fixed::from_bits(i32::MAX)));
        assert_eq!(to_fixed(-0.0), Some(Fixed::ZERO));
        assert_eq!(to_fixed(0.5), Some(Fixed::HALF));
        assert_eq!(to_fixed(1.0 / 3.0), Some(Fixed::from_bits(21845)));
        assert_eq!(to_fixed(-1.0 / 3.0), Some(Fixed::from_bits(-21845)));
    }
}
