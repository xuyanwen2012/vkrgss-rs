//! Arithmetic of the guest's bitmap operations. Every `pub fn` in this file has an entry in
//! [`BITMAP_PROVENANCE`]; a test reads this file and fails if one is missing.
//!
//! These functions are listed apart from the compositing functions of `arith.rs`, which a
//! shader mirrors one by one. Hue rotation runs on the CPU alone. Stretch sampling is the
//! rule of `vkrgss_proto::stretch_step` and `vkrgss_proto::stretch_index`, which the compositors
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
        name: "transfer_opacity",
        formula: "draws at opacity o if 1 <= o <= 255, draws nothing otherwise",
        provenance: Provenance::ReverseEngineered {
            source: "oracle measurement 2026-10-12, RGSS103J and RGSS202E, bitmap clip probe",
        },
    },
    ArithmeticFn {
        name: "block_span",
        formula: "n = |sw|, [dl, dr) = [dx, dx + n); \
                  sw >= 0: [sl, sr) = [sx, sx + sw); \
                  sl < 0: dl += -sl, sl = 0; dl < 0: sl += -dl, dl = 0; \
                  sr > ssize: dr += ssize - sr, sr = ssize; dr > dsize: sr += dsize - dr, dr = dsize; \
                  dl + i takes sl + i; \
                  sw < 0: sl = sx + sw, sh = sx; \
                  sl < 0: dl += -sl, sl = 0; dl < 0: sh += dl, dl = 0; \
                  sh > ssize: dr += ssize - sh, sh = ssize; dr > dsize: sl += dr - dsize, dr = dsize; \
                  dl + i takes sh - 1 - i; for i in 0..dr - dl",
        provenance: Provenance::ReverseEngineered {
            source: "oracle measurement 2026-10-12, RGSS103J and RGSS202E, bitmap clip probe",
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

/// The part of one axis of a transfer of equal sizes that is drawn: position `i` of `0..len`
/// lands on destination coordinate `dst + i` and takes source coordinate `src + i`, or
/// `src - i` when `mirrored`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockSpan {
    pub dst: u32,
    pub src: u32,
    pub len: u32,
    pub mirrored: bool,
}

/// Nearest sampling for a stretched transfer: the source index that position `i` of a
/// destination span of `dst_len` takes from a source span of `src_len`.
///
/// Formula: `s = (i * step) >> 20` with the fixed-point step `(src_len << 20) / dst_len`, in
/// 64-bit arithmetic: `vkrgss_proto::stretch_index` at `vkrgss_proto::stretch_step`, the one
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
    vkrgss_proto::stretch_index(i, vkrgss_proto::stretch_step(src_len, dst_len))
}

/// The opacity a transfer (`blt`, `stretch_blt`) draws at, given the integer a script passed:
/// 1 to 255 as given, and `None`, nothing is drawn, for 0, negative values and values above
/// 255. The original does not limit the value to 255.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-12, RGSS103J and RGSS202E, bitmap
/// clip probe: 39 values through `blt` and `stretch_blt`; the comparison is at 0x10096a23 of
/// RGSS202E).
pub const fn transfer_opacity(opacity: i32) -> Option<u8> {
    if opacity >= 1 && opacity <= 255 {
        Some(opacity as u8)
    } else {
        None
    }
}

/// One axis of a transfer of equal sizes (`blt`, and `stretch_blt` whose rectangles have equal
/// absolute sizes): what is left of the span after the four clips of the original, or `None`
/// when nothing is. The destination span starts at `dst_pos` inside `0..dst_size` and is
/// `|src_len|` long; the source span starts at `src_pos` inside `0..src_size`.
///
/// ```text
/// n = |src_len|;  destination [dl, dr) = [dst_pos, dst_pos + n)
///
/// src_len >= 0:  source [sl, sr) = [src_pos, src_pos + src_len)
///     sl < 0:         dl += -sl;             sl = 0
///     dl < 0:         sl += -dl;             dl = 0
///     sr > src_size:  dr += src_size - sr;   sr = src_size
///     dr > dst_size:  sr += dst_size - dr;   dr = dst_size
///     destination dl + i takes source sl + i
///
/// src_len < 0:   low end sl = src_pos + src_len, high end sh = src_pos
///     sl < 0:         dl += -sl;             sl = 0
///     dl < 0:         sh += dl;              dl = 0
///     sh > src_size:  dr += src_size - sh;   sh = src_size
///     dr > dst_size:  sl += dr - dst_size;   dr = dst_size
///     destination dl + i takes source sh - 1 - i
/// ```
///
/// A mirrored span that leaves the source is therefore not a clipped mirror image: the first
/// and third clips move the destination as if the span were not mirrored, and what remains is
/// drawn mirrored within itself.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-12, RGSS103J and RGSS202E, bitmap
/// clip probe: 3648 `blt` and 1368 `stretch_blt` cases of equal sizes; the routine is at
/// 0x10095f10 of RGSS202E).
pub const fn block_span(
    dst_pos: i32,
    dst_size: u32,
    src_pos: i32,
    src_len: i32,
    src_size: u32,
) -> Option<BlockSpan> {
    let (dst_size, src_size) = (dst_size as i64, src_size as i64);
    let mirrored = src_len < 0;
    let mut dl = dst_pos as i64;
    let mut dr = dl + (src_len as i64).abs();
    // The low end and the high end of the source span.
    let (mut sl, mut sh) = if mirrored {
        (src_pos as i64 + src_len as i64, src_pos as i64)
    } else {
        (src_pos as i64, src_pos as i64 + src_len as i64)
    };
    if sl < 0 {
        dl -= sl;
        sl = 0;
    }
    if dl < 0 {
        if mirrored {
            sh += dl;
        } else {
            sl -= dl;
        }
        dl = 0;
    }
    if sh > src_size {
        dr += src_size - sh;
        sh = src_size;
    }
    if dr > dst_size {
        if mirrored {
            sl += dr - dst_size;
        } else {
            sh += dst_size - dr;
        }
        dr = dst_size;
    }
    if dr <= dl {
        return None;
    }
    // Both spans keep the same length through the clips, so the source span is inside
    // `0..src_size` here, as the destination span is inside `0..dst_size`.
    Some(BlockSpan {
        dst: dl as u32,
        src: if mirrored { sh - 1 } else { sl } as u32,
        len: (dr - dl) as u32,
        mirrored,
    })
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

    #[test]
    fn opacity_outside_1_to_255_draws_nothing() {
        for opacity in [1, 127, 255] {
            assert_eq!(transfer_opacity(opacity), Some(opacity as u8));
        }
        for opacity in [0, -1, -256, i32::MIN, 256, 300, 65536, 1073741823, i32::MAX] {
            assert_eq!(transfer_opacity(opacity), None, "{opacity}");
        }
    }

    #[test]
    fn a_span_inside_both_bitmaps_is_kept_whole() {
        let plain = BlockSpan {
            dst: 2,
            src: 1,
            len: 3,
            mirrored: false,
        };
        assert_eq!(block_span(2, 8, 1, 3, 4), Some(plain));
        // Mirrored, the first position takes the last texel of the span.
        let mirrored = BlockSpan {
            dst: 2,
            src: 3,
            len: 3,
            mirrored: true,
        };
        assert_eq!(block_span(2, 8, 4, -3, 4), Some(mirrored));
    }

    #[test]
    fn a_mirrored_span_is_clipped_before_it_is_mirrored() {
        // Texels 12 to 19 of a source of 16, mirrored: the four that exist land on the first
        // four positions, where the unmirrored part would, and are reversed among themselves.
        let span = BlockSpan {
            dst: 8,
            src: 15,
            len: 4,
            mirrored: true,
        };
        assert_eq!(block_span(8, 48, 20, -8, 16), Some(span));
        // Starting 3 texels before the source, the destination moves by 3.
        let span = BlockSpan {
            dst: 11,
            src: 4,
            len: 5,
            mirrored: true,
        };
        assert_eq!(block_span(8, 48, 5, -8, 16), Some(span));
        // A destination cut at its start loses the high texels of the mirrored span.
        let span = BlockSpan {
            dst: 0,
            src: 8,
            len: 3,
            mirrored: true,
        };
        assert_eq!(block_span(-5, 48, 14, -8, 16), Some(span));
    }

    #[test]
    fn empty_and_extreme_spans_draw_nothing() {
        assert_eq!(block_span(0, 8, 0, 0, 8), None);
        assert_eq!(block_span(8, 8, 0, 4, 8), None);
        assert_eq!(block_span(0, 8, 8, 4, 8), None);
        assert_eq!(block_span(0, 8, 0, -4, 8), None);
        assert_eq!(block_span(0, 3, 0, i32::MIN, 3), None);
        assert_eq!(block_span(i32::MIN, 3, i32::MIN, i32::MAX, 3), None);
        assert_eq!(
            block_span(i32::MAX, u32::MAX, i32::MAX, i32::MIN, u32::MAX).map(|s| s.len),
            Some(i32::MAX as u32)
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
    fn every_function_is_listed_and_none_is_a_placeholder() {
        let listed: Vec<&str> = crate::bitmap_provenance()
            .iter()
            .map(|entry| entry.name)
            .collect();
        assert_eq!(
            listed,
            [
                "stretch_index",
                "transfer_opacity",
                "block_span",
                "rotate_hue"
            ]
        );
        assert_eq!(listed, public_functions());
        let [stretch, opacity, span, hue] = crate::bitmap_provenance() else {
            panic!("four entries");
        };
        for entry in [opacity, span] {
            assert_eq!(
                entry.provenance.to_string(),
                "reverse-engineered: oracle measurement 2026-10-12, RGSS103J and RGSS202E, \
                 bitmap clip probe"
            );
        }
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
        fn a_block_span_stays_inside_both_bitmaps(
            (dst_pos, dst_size) in (-40..40i32, 0..40u32),
            (src_pos, src_len, src_size) in (-40..40i32, -40..40i32, 0..40u32),
        ) {
            if let Some(span) = block_span(dst_pos, dst_size, src_pos, src_len, src_size) {
                prop_assert!(span.len >= 1 && span.len <= src_len.unsigned_abs());
                prop_assert!(span.dst + span.len <= dst_size);
                prop_assert_eq!(span.mirrored, src_len < 0);
                if span.mirrored {
                    prop_assert!(span.src < src_size && span.src + 1 >= span.len);
                } else {
                    prop_assert!(span.src + span.len <= src_size);
                }
            }
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
