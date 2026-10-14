//! Arithmetic of the bitmap operations RGSS2 adds: `blur`, `radial_blur`,
//! `gradient_fill_rect` and `clear_rect`. Every `pub fn` in this file has an entry in
//! [`BITMAP_RGSS2_PROVENANCE`]; a test reads this file and fails if one is missing.
//!
//! All of it runs on the CPU alone and has no GLSL counterpart. The rules were read from the
//! original runtime of VX and then measured on it, on both versions of its DLL.
//!
//! Notation used in the formulas: channels are integers from 0 to 255 with straight alpha,
//! and `/` on integers rounds toward zero.

use crate::{ArithmeticFn, Provenance, Rgba};

/// The measurement every function of this file follows.
const SOURCE: &str =
    "oracle measurement 2026-10-12, RGSS202E 2.0.2.1 and 2.0.0.1, bitmap ops probe";

/// The provenance of every public function in this file, in declaration order.
pub const BITMAP_RGSS2_PROVENANCE: &[ArithmeticFn] = &[
    ArithmeticFn {
        name: "truncated_mean",
        formula: "out.c = sum.c / count for c in r, g, b, a; blur: the sum over the 3 by 3 \
                  pixels around the pixel, the edge pixel repeated beyond the edges, count = 9; \
                  radial blur: the sum over the samples, count = division",
        provenance: Provenance::ReverseEngineered { source: SOURCE },
    },
    ArithmeticFn {
        name: "gradient_color",
        formula: "out.c = (c1.c + (i * (c2.c - c1.c)) / (n - 1)) & 255 for c in r, g, b, a, \
                  in signed arithmetic",
        provenance: Provenance::ReverseEngineered { source: SOURCE },
    },
    ArithmeticFn {
        name: "cleared_pixel",
        formula: "out = (0, 0, 0, 0)",
        provenance: Provenance::ReverseEngineered { source: SOURCE },
    },
    ArithmeticFn {
        name: "radial_blur_turns",
        formula: "angle = clamp(angle, 0, 360), division = clamp(division, 2, 100); in doubles: \
                  step = angle * 6.28318 / 360.0 / (division - 1), \
                  half = (division - 1) * step * 0.5, a_i = step * i - half, \
                  turn_i = (cos(a_i), sin(a_i)) for i in 0 .. division - 1",
        provenance: Provenance::ReverseEngineered { source: SOURCE },
    },
    ArithmeticFn {
        name: "radial_blur_source",
        formula: "cx = w >> 1, cy = h >> 1, dx = x - cx, dy = y - cy; in doubles: \
                  sx = trunc(cos * dx + cx - sin * dy + 0.5), \
                  sy = trunc(sin * dx + cy + cos * dy + 0.5); each mirrored into its side n: \
                  repeat { v < 0: v = -v, stop if v < n; else stop if v < n; v = 2 * n - v - 1 }",
        provenance: Provenance::ReverseEngineered { source: SOURCE },
    },
];

/// The mean of `count` pixels whose channels add up to `sum`, each channel on its own and
/// rounded down: `sum.c / count`.
///
/// This is how `Bitmap#blur` and `Bitmap#radial_blur` of RGSS2 average. Alpha is neither
/// premultiplied nor a weight: a transparent pixel counts with the colour it holds. The blur
/// sums the 3 by 3 pixels around a pixel, read from the bitmap as it was, with the edge pixel
/// repeated beyond the edges, and divides by 9; the radial blur sums one sample per turn and
/// divides by the number of turns.
///
/// # Panics
///
/// If `count` is 0.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-12, RGSS202E, bitmap ops probe;
/// blur: 102 cases, radial blur: 468 cases, no wrong pixel. The routines are at 0x100162d0,
/// 0x10016240 and 0x100165e0 of version 2.0.2.1).
pub const fn truncated_mean(sum: [u32; 4], count: u32) -> Rgba {
    [
        (sum[0] / count) as u8,
        (sum[1] / count) as u8,
        (sum[2] / count) as u8,
        (sum[3] / count) as u8,
    ]
}

/// The colour of column (or row) `i` of a gradient over `n` columns from `from` to `to`, as
/// `Bitmap#gradient_fill_rect` of RGSS2 writes it:
/// `(from.c + (i * (to.c - from.c)) / (n - 1)) & 255` for every channel, alpha included.
///
/// The division is signed and truncates toward zero, so a falling channel is not the mirror
/// image of a rising one. Column 0 is exactly `from` and column `n - 1` exactly `to`. `i` is
/// counted from the unclipped edge of the rectangle and may be any value; the low 8 bits are
/// kept.
///
/// # Panics
///
/// If `n` is below 2: the original draws nothing then, and the caller does not ask.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-12, RGSS202E, bitmap ops probe:
/// 300 cases, no wrong pixel. The routine is at 0x10016980 of version 2.0.2.1).
pub const fn gradient_color(from: Rgba, to: Rgba, i: i64, n: i64) -> Rgba {
    assert!(n >= 2, "a gradient has at least two steps");
    let mut out = [0; 4];
    let mut channel = 0;
    while channel < 4 {
        let (c1, c2) = (from[channel] as i64, to[channel] as i64);
        // Rust's `/` on integers truncates toward zero, as the original's division does.
        out[channel] = ((c1 + (i * (c2 - c1)) / (n - 1)) & 255) as u8;
        channel += 1;
    }
    out
}

/// The pixel `Bitmap#clear_rect` of RGSS2 writes: all four channels 0. Clearing is the fill
/// with this pixel and has no arithmetic beyond it.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-12, RGSS202E, bitmap ops probe:
/// 44 cases, no wrong pixel).
pub const fn cleared_pixel() -> Rgba {
    [0; 4]
}

/// The turns of `Bitmap#radial_blur(angle, division)` of RGSS2: the cosine and the sine of
/// each angle the bitmap is sampled at, in the order of the samples.
///
/// ```text
/// angle = clamp(angle, 0, 360);  division = clamp(division, 2, 100)
/// step  = angle * 6.28318 / 360.0 / (division - 1)        (doubles, in this order)
/// half  = (division - 1) * step * 0.5
/// a_i   = step * i - half                                  for i in 0 .. division - 1
/// ```
///
/// The constant is 6.28318 as the original has it, not twice pi, so 360 degrees is not a
/// whole turn. The samples are spread evenly from `-angle / 2` to `angle / 2`.
///
/// The cosine and the sine are those of the Rust standard library, where the original calls
/// its C library. The measurement found no pixel among 530 858 that depends on the last bit
/// of them; a sample within rounding error of a pixel boundary is where a difference from
/// the original would come from.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-12, RGSS202E, bitmap ops probe:
/// 468 cases, no wrong pixel. The routine is at 0x100165e0 of version 2.0.2.1).
pub fn radial_blur_turns(angle: i32, division: i32) -> Vec<(f64, f64)> {
    let angle = f64::from(angle.clamp(0, 360));
    let division = division.clamp(2, 100);
    // The constant of the original, which is not `TAU`: a better one gives other pixels.
    #[allow(clippy::approx_constant)]
    let step = angle * 6.28318 / 360.0 / f64::from(division - 1);
    let half = f64::from(division - 1) * step * 0.5;
    (0..division)
        .map(|i| {
            let a = step * f64::from(i) - half;
            (a.cos(), a.sin())
        })
        .collect()
}

/// The pixel of a bitmap of `width` by `height` that the pixel (`x`, `y`) takes for the turn
/// (`cos`, `sin`) of [`radial_blur_turns`]: the nearest pixel to (`x`, `y`) turned about the
/// centre, mirrored at the edges until it is inside.
///
/// ```text
/// cx = width >> 1;  cy = height >> 1;  dx = x - cx;  dy = y - cy
/// sx = trunc(cos * dx + cx - sin * dy + 0.5)          (doubles, the terms in this order)
/// sy = trunc(sin * dx + cy + cos * dy + 0.5)
/// mirror(v, n): repeat { if v < 0: v = -v, and stop if v < n
///                        else if v < n: stop
///                        v = 2 * n - v - 1 }
/// ```
///
/// The centre of an even side is the pixel right of or below the middle. Rounding adds 0.5
/// and truncates, so it goes toward zero for negative values. A negative coordinate `v`
/// reads `-v`, not `-v - 1`; a coordinate `v >= n` reads `2 * n - 1 - v`.
///
/// # Panics
///
/// If `width` or `height` is 0.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-12, RGSS202E, bitmap ops probe:
/// 468 cases, no wrong pixel. The routine is at 0x100165e0 of version 2.0.2.1).
pub fn radial_blur_source(
    (cos, sin): (f64, f64),
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> (u32, u32) {
    assert!(width > 0 && height > 0, "a bitmap has pixels");
    let cx = f64::from(width >> 1);
    let cy = f64::from(height >> 1);
    let dx = f64::from(x) - cx;
    let dy = f64::from(y) - cy;
    // `as` truncates toward zero.
    let sx = (cos * dx + cx - sin * dy + 0.5) as i64;
    let sy = (sin * dx + cy + cos * dy + 0.5) as i64;
    (mirror(sx, i64::from(width)), mirror(sy, i64::from(height)))
}

/// `mirror` of [`radial_blur_source`].
fn mirror(mut v: i64, n: i64) -> u32 {
    loop {
        if v < 0 {
            v = -v;
            if v < n {
                return v as u32;
            }
        } else if v < n {
            return v as u32;
        }
        v = 2 * n - v - 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mean_rounds_down_in_every_channel() {
        assert_eq!(truncated_mean([8, 9, 17, 2295], 9), [0, 1, 1, 255]);
        assert_eq!(truncated_mean([720, 0, 0, 255], 9), [80, 0, 0, 28]);
        assert_eq!(truncated_mean([199, 200, 201, 0], 100), [1, 2, 2, 0]);
    }

    #[test]
    fn a_gradient_starts_and_ends_on_its_colours() {
        let (from, to) = ([0, 0, 0, 255], [255, 10, 0, 0]);
        let columns: Vec<Rgba> = (0..4).map(|i| gradient_color(from, to, i, 4)).collect();
        assert_eq!(
            columns,
            [
                [0, 0, 0, 255],
                [85, 3, 0, 170],
                [170, 6, 0, 85],
                [255, 10, 0, 0]
            ]
        );
    }

    #[test]
    fn a_falling_channel_truncates_toward_zero() {
        // 1 * -10 / 3 is -3, not -4: 10 - 3.
        assert_eq!(gradient_color([10; 4], [0; 4], 1, 4), [7; 4]);
        assert_eq!(gradient_color([0; 4], [10; 4], 1, 4), [3; 4]);
        // Beyond the last step the low 8 bits are kept.
        assert_eq!(gradient_color([0; 4], [200; 4], 2, 2), [144; 4]);
    }

    #[test]
    fn the_cleared_pixel_is_transparent_black() {
        assert_eq!(cleared_pixel(), [0, 0, 0, 0]);
    }

    #[test]
    fn the_arguments_of_a_radial_blur_are_clamped() {
        assert_eq!(radial_blur_turns(1000, 1), radial_blur_turns(360, 2));
        assert_eq!(radial_blur_turns(30, -3), radial_blur_turns(30, 2));
        assert_eq!(radial_blur_turns(30, 101), radial_blur_turns(30, 100));
        assert_eq!(radial_blur_turns(-10, 3), radial_blur_turns(0, 3));
        assert_eq!(radial_blur_turns(0, 3), [(1.0, 0.0); 3]);
    }

    #[test]
    fn the_turns_are_spread_evenly_about_zero() {
        let turns = radial_blur_turns(90, 3);
        assert_eq!(turns.len(), 3);
        assert_eq!(turns[1], (1.0, 0.0));
        assert_eq!(turns[0].0, turns[2].0);
        assert_eq!(turns[0].1, -turns[2].1);
        // 6.28318 is a little less than a whole turn: half of 360 degrees is not pi.
        let whole = radial_blur_turns(360, 2);
        assert!(whole[1].1 > 0.0 && whole[1].1 < 1e-5);
    }

    #[test]
    fn no_turn_takes_the_pixel_itself() {
        for (x, y) in [(0, 0), (4, 2), (6, 5)] {
            assert_eq!(radial_blur_source((1.0, 0.0), x, y, 7, 6), (x, y));
        }
    }

    #[test]
    fn a_sample_outside_is_mirrored() {
        assert_eq!(mirror(-1, 5), 1);
        assert_eq!(mirror(-4, 5), 4);
        assert_eq!(mirror(5, 5), 4);
        assert_eq!(mirror(9, 5), 0);
        // Mirrored at one edge and then at the other.
        assert_eq!(mirror(-5, 5), 4);
        assert_eq!(mirror(10, 5), 1);
        assert_eq!(mirror(-7, 3), 2);
        // A quarter turn about the centre (2, 1) of a 5 by 3 bitmap takes (4, 1) to (2, 3),
        // below the bitmap, which reads row 2.
        assert_eq!(radial_blur_source((0.0, 1.0), 4, 1, 5, 3), (2, 2));
    }

    /// Names of the `pub fn`s declared in this file, in order.
    fn public_functions() -> Vec<&'static str> {
        include_str!("bitmap_rgss2.rs")
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
    fn every_function_is_listed_as_measured() {
        let listed: Vec<&str> = crate::bitmap_rgss2_provenance()
            .iter()
            .map(|entry| entry.name)
            .collect();
        assert_eq!(listed, public_functions());
        for entry in crate::bitmap_rgss2_provenance() {
            assert!(!entry.provenance.is_placeholder(), "{}", entry.name);
            assert!(!entry.formula.is_empty(), "{}", entry.name);
            for other in crate::provenance().iter().chain(crate::bitmap_provenance()) {
                assert_ne!(entry.name, other.name);
            }
        }
    }

    #[test]
    fn every_function_documents_its_provenance() {
        let notes = include_str!("bitmap_rgss2.rs")
            .lines()
            .filter(|line| line.starts_with("/// PROVENANCE: "))
            .count();
        assert_eq!(notes, public_functions().len());
    }
}
