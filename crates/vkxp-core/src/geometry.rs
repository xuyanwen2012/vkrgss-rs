//! Geometry of a quad: from position, origin, zoom, angle and mirror to the protocol's
//! fixed-point transform.
//!
//! This is the one place where floating point becomes protocol data. The coefficients are
//! computed in `f64` with operations that give the same bits on every platform (sines and
//! cosines come from the pure-Rust `libm`), and each is then rounded to the nearest multiple
//! of 2^-16.

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
    /// PLACEHOLDER: the rounding of rotated and zoomed geometry is not yet checked against the
    /// original runtime; neither is what a negative zoom draws (here: the flipped image).
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
