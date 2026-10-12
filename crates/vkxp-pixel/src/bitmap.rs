//! Arithmetic of the guest's bitmap operations. Every `pub fn` in this file has an entry in
//! [`BITMAP_PROVENANCE`]; a test reads this file and fails if one is missing.
//!
//! These functions are listed apart from the compositing functions of `arith.rs`, which a
//! shader mirrors one by one. Hue rotation runs on the CPU alone. Stretch sampling is the
//! rule of `vkxp_proto::stretch_step` and `vkxp_proto::stretch_index`, which the compositors
//! apply to stretched quads, the compute shader among them; [`stretch_index`] calls them.
//!
//! Notation used in the formulas: channels are integers from 0 to 255 and `/` is integer
//! division rounding down.

use crate::{ArithmeticFn, Provenance, Rgba};

/// The provenance of every public function in this file, in declaration order.
pub const BITMAP_PROVENANCE: &[ArithmeticFn] = &[
    ArithmeticFn {
        name: "stretch_index",
        formula: "s = (i * ((src_len << 20) / dst_len)) >> 20",
        provenance: Provenance::ReverseEngineered {
            source: "oracle measurement 2026-10-09, RGSS103J, bitmap stretch probe",
        },
    },
    ArithmeticFn {
        name: "rotate_hue",
        formula: "M = max(r, g, b), m = min(r, g, b), L = (M + m) >> 1, d = M - m; \
                  d = 0: H = 0, S = 0; else S = d * 255 / (M + m if L < 128 else 511 - (M + m)), \
                  H = (g - b) * 60 / d + 360 if r = M, (b - r) * 60 / d + 480 if g = M, \
                  else (r - g) * 60 / d + 600 (division towards zero), minus 360 once if >= 360; \
                  h = degrees, plus 360 once if negative; H = H + h, minus 360 once if >= 360; \
                  q = S + L - ((L * S + 127) * 0x8081 >> 23) if L >= 128 \
                  else ((S + 255) * L * 0x8081) >> 23, p = 2 * L - q; \
                  f(x) = p + (((q - p) * x * 0x8889) >> 21) if x < 60, q if x < 180, \
                  p + (((q - p) * (240 - x) * 0x8889) >> 21) if x < 240, else p; \
                  out.r = f(H + 120, minus 360 if >= 360), out.g = f(H), \
                  out.b = f(H - 120, plus 360 if < 0), each its low 8 bits, in unsigned 32-bit \
                  arithmetic; out = pixel when degrees = 0; out.a = pixel.a",
        provenance: Provenance::ReverseEngineered {
            source: "oracle measurement 2026-10-09, RGSS104E, hue probe",
        },
    },
];

/// Nearest sampling for a stretched transfer: the source index that position `i` of a
/// destination span of `dst_len` takes from a source span of `src_len`.
///
/// Formula: `s = (i * step) >> 20` with the fixed-point step `(src_len << 20) / dst_len`, in
/// 64-bit arithmetic: `vkxp_proto::stretch_index` at `vkxp_proto::stretch_step`, the one
/// definition of the rule, which the compositors use for stretched quads. No interpolation and no pixel centres. The step is rounded down, so a
/// ratio that is not a binary fraction lands one index low where the exact product is a whole
/// number: 16 stretched to 24 takes 0, 0, 1, 1, 2, 3.
///
/// For `i < dst_len`, which is what the callers pass, the result is below `src_len`. When both
/// spans have the same length the result is `i`.
///
/// # Panics
///
/// If `dst_len` is 0.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-09, RGSS103J, bitmap stretch
/// probe).
pub const fn stretch_index(i: u32, src_len: u32, dst_len: u32) -> u32 {
    vkxp_proto::stretch_index(i, vkxp_proto::stretch_step(src_len, dst_len))
}

/// Rotates the hue of a pixel by `degrees`, keeping its alpha, as the original runtime's
/// `Bitmap#hue_change` does: through an integer hue, saturation and lightness form and back.
///
/// ```text
/// h = degrees, plus 360 once when negative
///
/// M, m = largest and smallest of r, g, b;  L = (M + m) >> 1
/// M = m:  H = 0, S = 0
/// else    d = M - m;  S = d * 255 / (M + m  if L < 128  else  511 - (M + m))
///         r = M:  H = (g - b) * 60 / d + 360      (division towards zero)
///         g = M:  H = (b - r) * 60 / d + 480
///         else:   H = (r - g) * 60 / d + 600
///         H >= 360:  H = H - 360
///
/// H = H + h;  H >= 360:  H = H - 360              (once)
///
/// L >= 128:  q = S + L - ((L * S + 127) * 0x8081 >> 23)
/// else       q = ((S + 255) * L * 0x8081) >> 23
/// p = 2 * L - q
/// f(x) = p + (((q - p) * x * 0x8889) >> 21)             if x < 60
///        q                                              if x < 180
///        p + (((q - p) * (240 - x) * 0x8889) >> 21)     if x < 240
///        p                                              otherwise
/// red = f(H + 120, minus 360 if >= 360),  green = f(H),  blue = f(H - 120, plus 360 if < 0)
/// ```
///
/// The products and sums of the way back are unsigned 32-bit and wrap; the low 8 bits of
/// each result are kept. `0x8081 >> 23` divides by 255 and `0x8889 >> 21` by 60.
///
/// The round trip is not exact: an angle of 360 changes many colours by a little. A grey has
/// saturation 0 and comes back as it was; an angle of exactly 0 returns every pixel
/// unchanged. Angles are reduced once, not modulo 360, so an angle of 720 or more, or below
/// -360, leaves the hue outside its range; it is then kept in 16 bits, as in the model that
/// reproduces the samples.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-09, RGSS104E, hue probe; the
/// routines are at 0x100d0200, 0x100d7440 and 0x100d72e0 of that DLL).
pub const fn rotate_hue(pixel: Rgba, degrees: i32) -> Rgba {
    if degrees == 0 {
        return pixel;
    }
    let turn = if degrees < 0 {
        degrees as i64 + 360
    } else {
        degrees as i64
    };
    let (r, g, b) = (pixel[0] as i32, pixel[1] as i32, pixel[2] as i32);
    let max = max3(r, g, b);
    let min = min3(r, g, b);
    let sum = max + min;
    let lightness = (sum >> 1) as u32;
    let (mut hue, saturation) = if max == min {
        (0, 0)
    } else {
        let d = max - min;
        let saturation = d * 255 / if lightness < 128 { sum } else { 511 - sum };
        let hue = if r == max {
            (g - b) * 60 / d + 360
        } else if g == max {
            (b - r) * 60 / d + 480
        } else {
            (r - g) * 60 / d + 600
        };
        (
            if hue >= 360 { hue - 360 } else { hue } as i64,
            (saturation & 255) as u32,
        )
    };
    hue += turn;
    if hue >= 360 {
        hue -= 360;
    }
    let hue = hue & 0xffff;

    let q = if lightness >= 128 {
        (saturation + lightness)
            .wrapping_sub((lightness * saturation + 127).wrapping_mul(0x8081) >> 23)
    } else {
        ((saturation + 255) * lightness).wrapping_mul(0x8081) >> 23
    };
    let p = (2 * lightness).wrapping_sub(q);
    let above = if hue + 120 >= 360 {
        hue - 240
    } else {
        hue + 120
    };
    let below = if hue < 120 { hue + 240 } else { hue - 120 };
    [
        hue_channel(p, q, above),
        hue_channel(p, q, hue),
        hue_channel(p, q, below),
        pixel[3],
    ]
}

/// `f` of [`rotate_hue`]: one channel on the way back from hue, saturation and lightness.
const fn hue_channel(p: u32, q: u32, x: i64) -> u8 {
    let value = if x < 60 {
        hue_ramp(p, q, x)
    } else if x < 180 {
        q
    } else if x < 240 {
        hue_ramp(p, q, 240 - x)
    } else {
        p
    };
    value as u8
}

/// `p + (((q - p) * steps * 0x8889) >> 21)`, wrapping.
const fn hue_ramp(p: u32, q: u32, steps: i64) -> u32 {
    (q.wrapping_sub(p)
        .wrapping_mul(steps as u32)
        .wrapping_mul(0x8889)
        >> 21)
        .wrapping_add(p)
}

const fn max3(a: i32, b: i32, c: i32) -> i32 {
    let ab = if a > b { a } else { b };
    if ab > c { ab } else { c }
}

const fn min3(a: i32, b: i32, c: i32) -> i32 {
    let ab = if a < b { a } else { b };
    if ab < c { ab } else { c }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// Values of the channel that is sampled rather than enumerated.
    const THIRD: [u8; 9] = [0, 1, 2, 63, 127, 128, 200, 254, 255];

    #[test]
    fn angle_0_leaves_every_pixel_unchanged() {
        for a in 0..=255u8 {
            for b in 0..=255u8 {
                for c in THIRD {
                    // The sampled channel takes each of the three positions.
                    for pixel in [[a, b, c, 77], [a, c, b, 77], [c, a, b, 77]] {
                        assert_eq!(rotate_hue(pixel, 0), pixel);
                    }
                }
            }
        }
    }

    #[test]
    fn a_full_turn_is_not_the_identity() {
        // The way through hue, saturation and lightness loses a little.
        assert_eq!(rotate_hue([255, 0, 0, 255], 360), [254, 0, 0, 255]);
        assert_eq!(rotate_hue([81, 81, 0, 255], 360), [80, 80, 0, 255]);
        assert_eq!(rotate_hue([200, 120, 40, 9], 360), [200, 120, 40, 9]);
    }

    #[test]
    fn thirds_of_a_turn_map_red_to_green_and_blue_up_to_the_rounding() {
        let red = [255, 0, 0, 255];
        assert_eq!(rotate_hue(red, 120), [0, 254, 0, 255]);
        assert_eq!(rotate_hue(red, 240), [0, 0, 254, 255]);
        assert_eq!(rotate_hue(red, 60), [254, 254, 0, 255]);
        assert_eq!(rotate_hue(red, 180), [0, 254, 254, 255]);
        assert_eq!(rotate_hue([200, 120, 40, 9], 120), [40, 200, 120, 9]);
        assert_eq!(rotate_hue([12, 200, 99, 3], 200), [199, 13, 53, 3]);
    }

    #[test]
    fn angles_are_reduced_once() {
        let pixel = [200, 120, 40, 9];
        // A negative angle is raised by 360 once.
        assert_eq!(rotate_hue(pixel, -30), rotate_hue(pixel, 330));
        assert_eq!(rotate_hue(pixel, -30), [200, 40, 40, 9]);
        assert_eq!(rotate_hue([0, 0, 255, 255], -30), [0, 127, 254, 255]);
        // The sum is lowered by 360 once: two turns leave the hue out of its range.
        assert_eq!(rotate_hue(pixel, 720), [200, 40, 40, 9]);
        assert_eq!(rotate_hue(pixel, -400), [40, 40, 40, 9]);
    }

    #[test]
    fn grey_pixels_never_change() {
        // Saturation 0: both ends of the ramp are the lightness.
        for value in 0..=255u8 {
            for degrees in (-720..=720).step_by(7) {
                let pixel = [value, value, value, value.wrapping_mul(3)];
                assert_eq!(rotate_hue(pixel, degrees), pixel);
            }
        }
    }

    #[test]
    fn stretch_index_doubles() {
        let taken: Vec<u32> = (0..4).map(|i| stretch_index(i, 2, 4)).collect();
        assert_eq!(taken, [0, 0, 1, 1]);
        // Rows of the same scenario: a source of 1 stretched to 2.
        assert_eq!([stretch_index(0, 1, 2), stretch_index(1, 1, 2)], [0, 0]);
    }

    #[test]
    fn stretch_index_of_sixteen_to_twenty_four_lands_one_low() {
        let taken: Vec<u32> = (0..6).map(|i| stretch_index(i, 16, 24)).collect();
        assert_eq!(taken, [0, 0, 1, 1, 2, 3]);
    }

    #[test]
    fn stretch_index_halves() {
        let taken: Vec<u32> = (0..2).map(|i| stretch_index(i, 4, 2)).collect();
        assert_eq!(taken, [0, 2]);
    }

    #[test]
    fn stretch_index_rounds_down_without_overflow() {
        let taken: Vec<u32> = (0..5).map(|i| stretch_index(i, 3, 5)).collect();
        assert_eq!(taken, [0, 0, 1, 1, 2]);
        assert_eq!(
            stretch_index(u32::MAX - 1, u32::MAX, u32::MAX),
            u32::MAX - 1
        );
    }

    /// Names of the `pub fn`s declared in this file, in order.
    fn public_functions() -> Vec<&'static str> {
        include_str!("bitmap.rs")
            .lines()
            .filter_map(|line| {
                let rest = line
                    .strip_prefix("pub const fn ")
                    .or_else(|| line.strip_prefix("pub fn "))?;
                rest.split('(').next()
            })
            .collect()
    }

    #[test]
    fn both_functions_are_listed_and_neither_is_a_placeholder() {
        let listed: Vec<&str> = crate::bitmap_provenance()
            .iter()
            .map(|entry| entry.name)
            .collect();
        assert_eq!(listed, ["stretch_index", "rotate_hue"]);
        assert_eq!(listed, public_functions());
        let [stretch, hue] = crate::bitmap_provenance() else {
            panic!("two entries");
        };
        assert_eq!(
            stretch.provenance.to_string(),
            "reverse-engineered: oracle measurement 2026-10-09, RGSS103J, bitmap stretch probe"
        );
        assert_eq!(
            hue.provenance.to_string(),
            "reverse-engineered: oracle measurement 2026-10-09, RGSS104E, hue probe"
        );
        assert!(!hue.provenance.is_placeholder());
        for entry in crate::bitmap_provenance() {
            assert!(!entry.formula.is_empty(), "{}", entry.name);
        }
    }

    #[test]
    fn every_function_documents_its_provenance() {
        let notes = include_str!("bitmap.rs")
            .lines()
            .filter(|line| line.starts_with("/// PROVENANCE: "))
            .count();
        assert_eq!(notes, public_functions().len());
    }

    #[test]
    fn compositing_provenance_is_unchanged() {
        let names: Vec<&str> = crate::provenance().iter().map(|entry| entry.name).collect();
        assert_eq!(names.len(), 9);
        assert_eq!(
            names,
            [
                "effective_alpha",
                "blend_normal",
                "blend_normal_translucent",
                "blend_add",
                "blend_sub",
                "blend_window",
                "blend_color",
                "apply_gray",
                "apply_tone",
            ]
        );
        for entry in crate::bitmap_provenance() {
            assert!(!names.contains(&entry.name), "{}", entry.name);
        }
    }

    proptest! {
        #[test]
        fn alpha_is_kept_at_any_angle(
            pixel in any::<[u8; 4]>(),
            degrees in any::<i32>(),
        ) {
            prop_assert_eq!(rotate_hue(pixel, degrees)[3], pixel[3]);
        }

        #[test]
        fn stretch_index_stays_inside_the_source(
            (dst_len, i) in (1..5000u32).prop_flat_map(|len| (Just(len), 0..len)),
            src_len in 1..5000u32,
        ) {
            prop_assert!(stretch_index(i, src_len, dst_len) < src_len);
            prop_assert_eq!(stretch_index(i, dst_len, dst_len), i);
        }
    }
}
