//! Arithmetic of bitmap operations that run only on the CPU. Every `pub fn` in this file has
//! an entry in [`BITMAP_PROVENANCE`]; a test reads this file and fails if one is missing.
//!
//! Nothing here is mirrored by a shader, which is why these functions are listed apart from
//! the compositing functions of `arith.rs`.
//!
//! Notation used in the formulas: channels are integers from 0 to 255 and `/` is integer
//! division rounding down.

use crate::{ArithmeticFn, Provenance, Rgba};

/// The provenance of every public function in this file, in declaration order.
pub const BITMAP_PROVENANCE: &[ArithmeticFn] = &[
    ArithmeticFn {
        name: "stretch_index",
        formula: "s = (i * src_len) / dst_len",
        provenance: Provenance::Textbook,
    },
    ArithmeticFn {
        name: "rotate_hue",
        formula: "M = max(r, g, b), m = min(r, g, b), d = M - m, \
                  p = hue position in 0..6d (red at 0, green at 2d, blue at 4d), \
                  p' = (p + (h * d + 30) / 60) mod 6d with h = degrees mod 360, \
                  s = p' / d, f = p' - s * d, \
                  out.rgb = (M, m+f, m), (M-f, M, m), (m, M, m+f), (m, M-f, M), (m+f, m, M), \
                  (M, m, M-f) for s = 0..5, \
                  out = pixel when d = 0, out.a = pixel.a",
        provenance: Provenance::Textbook,
    },
];

/// Nearest sampling for a stretched transfer: the source index that position `i` of a
/// destination span of `dst_len` takes from a source span of `src_len`.
///
/// Formula: `s = (i * src_len) / dst_len`.
///
/// For `i < dst_len` the result is below `src_len`. When both spans have the same length the
/// result is `i`.
///
/// # Panics
///
/// If `dst_len` is 0.
///
/// PROVENANCE: Textbook.
pub const fn stretch_index(i: u32, src_len: u32, dst_len: u32) -> u32 {
    ((i as u64 * src_len as u64) / dst_len as u64) as u32
}

/// Rotates the hue of a pixel by `degrees`, keeping its alpha and the values of its largest
/// and smallest colour channels.
///
/// The usual rotation in hue-saturation-value terms, in exact integer arithmetic. With `M` and
/// `m` the largest and smallest of red, green and blue and `d = M - m`, the hue position `p`
/// lies in `0..6d`:
///
/// ```text
/// M is red:    p = g - b            if g >= b, else 6d - (b - g)
/// M is green:  p = 2d + (b - r)
/// M is blue:   p = 4d + (r - g)     (ties resolved red, then green, then blue)
///
/// p' = (p + (h * d + 30) / 60) mod 6d      with h = degrees reduced to 0..359
/// s  = p' / d,   f = p' - s * d
///
/// s = 0: (M, m+f, m)   s = 1: (M-f, M, m)   s = 2: (m, M, m+f)
/// s = 3: (m, M-f, M)   s = 4: (m+f, m, M)   s = 5: (M, m, M-f)
/// ```
///
/// A pixel with `d = 0` (a grey) is returned unchanged. An angle that is a multiple of 360
/// reproduces every pixel exactly, and multiples of 60 permute the channels exactly.
///
/// PROVENANCE: Textbook.
pub const fn rotate_hue(pixel: Rgba, degrees: i32) -> Rgba {
    let (r, g, b) = (pixel[0] as u32, pixel[1] as u32, pixel[2] as u32);
    let max = max3(r, g, b);
    let min = min3(r, g, b);
    let d = max - min;
    if d == 0 {
        return pixel;
    }
    let position = if r == max {
        if g >= b { g - b } else { 6 * d - (b - g) }
    } else if g == max {
        // `b - r` may be negative, but never below `-d`: add before subtracting.
        2 * d + b - r
    } else {
        // Likewise `r - g`.
        4 * d + r - g
    };
    let h = degrees.rem_euclid(360) as u32;
    let rotated = (position + (h * d + 30) / 60) % (6 * d);
    let sector = rotated / d;
    let f = rotated - sector * d;
    let (r, g, b) = match sector {
        0 => (max, min + f, min),
        1 => (max - f, max, min),
        2 => (min, max, min + f),
        3 => (min, max - f, max),
        4 => (min + f, min, max),
        _ => (max, min, max - f),
    };
    [r as u8, g as u8, b as u8, pixel[3]]
}

const fn max3(a: u32, b: u32, c: u32) -> u32 {
    let ab = if a > b { a } else { b };
    if ab > c { ab } else { c }
}

const fn min3(a: u32, b: u32, c: u32) -> u32 {
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
    fn full_turns_are_the_identity() {
        for pixel in [[255, 0, 0, 255], [12, 200, 99, 3], [5, 5, 250, 128]] {
            for degrees in [360, -360, 720, -720, 3600] {
                assert_eq!(rotate_hue(pixel, degrees), pixel, "{degrees}");
            }
        }
    }

    #[test]
    fn thirds_of_a_turn_map_red_to_green_and_blue() {
        let red = [255, 0, 0, 255];
        assert_eq!(rotate_hue(red, 120), [0, 255, 0, 255]);
        assert_eq!(rotate_hue(red, 240), [0, 0, 255, 255]);
    }

    #[test]
    fn multiples_of_60_permute_the_channels() {
        let pixel = [200, 120, 40, 9];
        assert_eq!(rotate_hue(pixel, 120), [40, 200, 120, 9]);
        assert_eq!(rotate_hue(pixel, 240), [120, 40, 200, 9]);
        assert_eq!(rotate_hue([255, 0, 0, 1], 60), [255, 255, 0, 1]);
        assert_eq!(rotate_hue([255, 0, 0, 1], 180), [0, 255, 255, 1]);
        assert_eq!(rotate_hue([255, 0, 0, 1], 300), [255, 0, 255, 1]);
    }

    #[test]
    fn negative_angles_equal_their_positive_counterparts() {
        let pixel = [200, 120, 40, 9];
        for degrees in [1, 59, 120, 200, 359] {
            let expected = rotate_hue(pixel, degrees);
            assert_eq!(rotate_hue(pixel, degrees - 360), expected, "{degrees}");
            assert_eq!(rotate_hue(pixel, degrees - 720), expected, "{degrees}");
            assert_eq!(rotate_hue(pixel, degrees + 360), expected, "{degrees}");
        }
        assert_eq!(rotate_hue(pixel, i32::MIN), rotate_hue(pixel, 232));
        assert_eq!(rotate_hue(pixel, i32::MAX), rotate_hue(pixel, 127));
    }

    #[test]
    fn grey_pixels_never_change() {
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
    fn both_functions_are_listed_as_placeholders() {
        let listed: Vec<&str> = crate::bitmap_provenance()
            .iter()
            .map(|entry| entry.name)
            .collect();
        assert_eq!(listed, ["stretch_index", "rotate_hue"]);
        assert_eq!(listed, public_functions());
        for entry in crate::bitmap_provenance() {
            assert_eq!(entry.provenance, Provenance::Textbook, "{}", entry.name);
            assert!(entry.provenance.is_placeholder(), "{}", entry.name);
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
        assert_eq!(names.len(), 6);
        assert_eq!(
            names,
            [
                "effective_alpha",
                "blend_normal",
                "blend_add",
                "blend_sub",
                "blend_color",
                "apply_tone",
            ]
        );
        for entry in crate::bitmap_provenance() {
            assert!(!names.contains(&entry.name), "{}", entry.name);
        }
    }

    proptest! {
        #[test]
        fn largest_and_smallest_channels_and_alpha_are_preserved(
            pixel in any::<[u8; 4]>(),
            degrees in any::<i32>(),
        ) {
            let rotated = rotate_hue(pixel, degrees);
            prop_assert_eq!(rotated[3], pixel[3]);
            prop_assert_eq!(rotated[..3].iter().max(), pixel[..3].iter().max());
            prop_assert_eq!(rotated[..3].iter().min(), pixel[..3].iter().min());
        }

        #[test]
        fn small_angles_preserve_the_extremes_too(
            pixel in any::<[u8; 4]>(),
            degrees in -720..=720i32,
        ) {
            let rotated = rotate_hue(pixel, degrees);
            prop_assert_eq!(rotated[3], pixel[3]);
            prop_assert_eq!(rotated[..3].iter().max(), pixel[..3].iter().max());
            prop_assert_eq!(rotated[..3].iter().min(), pixel[..3].iter().min());
            prop_assert_eq!(rotated, rotate_hue(pixel, degrees.rem_euclid(360)));
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
