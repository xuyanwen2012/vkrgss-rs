//! The arithmetic itself. Every `pub fn` in this file has an entry in [`PROVENANCE`]; a test
//! reads this file and fails if one is missing.
//!
//! Notation used in the formulas: channels are integers from 0 to 255, `/` is integer division
//! rounding down, `div255(x) = (x + 127) / 255` is division by 255 rounded to nearest, and `>>`
//! is an arithmetic shift, which rounds a negative value down.

use crate::{ArithmeticFn, Provenance};

/// A pixel: red, green, blue and straight (non-premultiplied) alpha.
pub type Rgba = [u8; 4];

/// The measurement the formulas for an opaque destination, the colour and the tone offsets
/// were fitted to.
const MEASURED: Provenance = Provenance::ReverseEngineered {
    source: "oracle measurement 2026-10-09, RGSS103J, screen blend probe",
};

/// The measurement the normal blend onto a destination with alpha was fitted to.
const MEASURED_BLT: Provenance = Provenance::ReverseEngineered {
    source: "oracle measurement 2026-10-09, RGSS103J, bitmap blt probe",
};

/// The measurement the grey part of the tone was fitted to.
const MEASURED_TONE: Provenance = Provenance::ReverseEngineered {
    source: "oracle measurement 2026-10-09, RGSS103J, tone probe",
};

/// The provenance of every public function in this file, in declaration order.
pub const PROVENANCE: &[ArithmeticFn] = &[
    ArithmeticFn {
        name: "effective_alpha",
        formula: "a = div255(alpha * opacity)",
        provenance: MEASURED,
    },
    ArithmeticFn {
        name: "blend_normal",
        formula: "for dst.a = 255: a = effective_alpha(src.a, opacity), \
                  out.c = dst.c + (((src.c - dst.c) * (a + 1)) >> 8), out.a = 255, \
                  out = dst when a = 0; \
                  for dst.a < 255: blend_normal_translucent",
        provenance: MEASURED,
    },
    ArithmeticFn {
        name: "blend_normal_translucent",
        formula: "a = effective_alpha(src.a, opacity), \
                  den = 255 * dst.a + 255 * a - dst.a * a, \
                  w = a * 65025 / den, lowered by one for the pairs (a, dst.a) of \
                  BLT_WEIGHT_EXCEPTIONS, \
                  out.c = dst.c + (((src.c - dst.c) * (w + 1)) >> 8), out.c = dst.c when w = 0, \
                  out.a = den / 255, \
                  out = dst when opacity = 0, out = (src.rgb, 0) when den = 0",
        provenance: MEASURED_BLT,
    },
    ArithmeticFn {
        name: "blend_add",
        formula: "for dst.a = 255: a = effective_alpha(src.a, opacity), \
                  c = min(255, dst.c + ((src.c * (a + 1)) >> 8)), out = dst when a = 0; \
                  for dst.a < 255: blend_add_translucent",
        provenance: MEASURED,
    },
    ArithmeticFn {
        name: "blend_add_translucent",
        formula: "c = min(255, dst.c + div255(src.c * a)), a = effective_alpha(src.a, opacity)",
        provenance: Provenance::Textbook,
    },
    ArithmeticFn {
        name: "blend_sub",
        formula: "for dst.a = 255: a = effective_alpha(src.a, opacity), \
                  c = max(0, dst.c - ((src.c * (a + 1)) >> 8)), out = dst when a = 0; \
                  for dst.a < 255: blend_sub_translucent",
        provenance: MEASURED,
    },
    ArithmeticFn {
        name: "blend_sub_translucent",
        formula: "c = max(0, dst.c - div255(src.c * a)), a = effective_alpha(src.a, opacity)",
        provenance: Provenance::Textbook,
    },
    ArithmeticFn {
        name: "blend_color",
        formula: "c = dst.c + (((color.c - dst.c) * (color.a + 1)) >> 8), \
                  out = dst when color.a = 0",
        provenance: MEASURED,
    },
    ArithmeticFn {
        name: "apply_gray",
        formula: "l = (77 * r + 150 * g + 29 * b) >> 8, \
                  c = dst.c + (((l - dst.c) * (gray + 1)) >> 8), \
                  out = dst when gray = 0",
        provenance: MEASURED_TONE,
    },
    ArithmeticFn {
        name: "apply_tone",
        formula: "m = apply_gray(dst, gray), \
                  c = clamp(m.c + clamp(tone.c, -255, 255), 0, 255)",
        provenance: MEASURED,
    },
];

/// Division by 255, rounded to nearest.
const fn div255(value: u32) -> u32 {
    (value + 127) / 255
}

/// `div255(channel * alpha)`: the share of a source channel that is added or subtracted to a
/// translucent destination.
const fn scale(channel: u8, alpha: u8) -> u8 {
    div255(channel as u32 * alpha as u32) as u8
}

/// `(value * (alpha + 1)) >> 8`: the share of a signed value that an alpha lets through, as the
/// original computes it. The shift is arithmetic.
const fn share(value: i32, alpha: u8) -> i32 {
    (value * (alpha as i32 + 1)) >> 8
}

/// `dst + share(src - dst, alpha)`: `src` at alpha 255, never outside the two.
const fn lerp(dst: u8, src: u8, alpha: u8) -> u8 {
    (dst as i32 + share(src as i32 - dst as i32, alpha)) as u8
}

/// The alpha a source pixel is drawn with: its own alpha scaled by the opacity of the draw.
///
/// Formula: `a = div255(alpha * opacity)`, that is `(alpha * opacity + 127) / 255`.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-09, RGSS103J, screen blend probe).
pub const fn effective_alpha(alpha: u8, opacity: u8) -> u8 {
    div255(alpha as u32 * opacity as u32) as u8
}

/// Normal blend: `src` drawn over `dst` at `opacity`.
///
/// Formula for an opaque destination (`dst.a = 255`, the frame), with
/// `a = effective_alpha(src.a, opacity)`:
///
/// ```text
/// out   = dst                                        when a = 0
/// out.c = dst.c + (((src.c - dst.c) * (a + 1)) >> 8)  per colour channel c
/// out.a = 255
/// ```
///
/// `a = 255` returns `src`. A destination with an alpha below 255 is blended by
/// [`blend_normal_translucent`], which gives the same result for an alpha of 255.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-09, RGSS103J, screen blend probe).
pub const fn blend_normal(dst: Rgba, src: Rgba, opacity: u8) -> Rgba {
    if dst[3] != 255 {
        return blend_normal_translucent(dst, src, opacity);
    }
    let a = effective_alpha(src[3], opacity);
    if a == 0 {
        return dst;
    }
    [
        lerp(dst[0], src[0], a),
        lerp(dst[1], src[1], a),
        lerp(dst[2], src[2], a),
        255,
    ]
}

/// The pairs `(a, dst.a)` for which the weight of [`blend_normal_translucent`] is one below
/// the quotient. They are measured, not derived: the grids of the bitmap blt probe pin the
/// weight for every pair, and it equals the quotient everywhere else.
const BLT_WEIGHT_EXCEPTIONS: [(u8, u8); 5] =
    [(30, 51), (30, 136), (105, 204), (180, 153), (210, 85)];

/// The weight of the source in [`blend_normal_translucent`]: `a * 65025 / den`, lowered by one
/// for the pairs of [`BLT_WEIGHT_EXCEPTIONS`]. `den` is not 0.
const fn blt_weight(a: u8, dst_alpha: u8, den: u32) -> u8 {
    let mut weight = a as u32 * 65025 / den;
    let mut i = 0;
    while i < BLT_WEIGHT_EXCEPTIONS.len() {
        if BLT_WEIGHT_EXCEPTIONS[i].0 == a && BLT_WEIGHT_EXCEPTIONS[i].1 == dst_alpha {
            weight -= 1;
        }
        i += 1;
    }
    weight as u8
}

/// Normal blend onto a destination of any alpha, both with straight alpha, as `Bitmap#blt`
/// draws into a bitmap. This is what `blend_normal` computes for a destination alpha below 255.
///
/// Formula, with `a = effective_alpha(src.a, opacity)`:
///
/// ```text
/// out   = dst                                        when opacity = 0
/// den   = 255 * dst.a + 255 * a - dst.a * a          resulting alpha, times 255
/// out   = (src.r, src.g, src.b, 0)                   when den = 0, that is a = 0 and dst.a = 0
/// w     = a * 65025 / den                            lowered by one for five pairs (a, dst.a)
/// out.c = dst.c                                      when w = 0
/// out.c = dst.c + (((src.c - dst.c) * (w + 1)) >> 8)  per colour channel c otherwise
/// out.a = den / 255
/// ```
///
/// The five pairs are those of `BLT_WEIGHT_EXCEPTIONS`. `w` is 255 for `dst.a = 0` and `a` for
/// `dst.a = 255`, so an opaque destination gets the formula of [`blend_normal`].
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-09, RGSS103J, bitmap blt probe).
pub const fn blend_normal_translucent(dst: Rgba, src: Rgba, opacity: u8) -> Rgba {
    if opacity == 0 {
        return dst;
    }
    let a = effective_alpha(src[3], opacity);
    let den = 255 * dst[3] as u32 + 255 * a as u32 - dst[3] as u32 * a as u32;
    if den == 0 {
        return [src[0], src[1], src[2], 0];
    }
    let weight = blt_weight(a, dst[3], den);
    if weight == 0 {
        return [dst[0], dst[1], dst[2], (den / 255) as u8];
    }
    [
        lerp(dst[0], src[0], weight),
        lerp(dst[1], src[1], weight),
        lerp(dst[2], src[2], weight),
        (den / 255) as u8,
    ]
}

/// Additive blend of `src` onto `dst` at `opacity`, saturating at 255. The destination alpha
/// is kept.
///
/// Formula for an opaque destination, per colour channel `c`, with
/// `a = effective_alpha(src.a, opacity)`: `c = min(255, dst.c + ((src.c * (a + 1)) >> 8))`, and
/// `dst` unchanged when `a = 0`. A destination with an alpha below 255 is blended by
/// [`blend_add_translucent`], which is not measured yet.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-09, RGSS103J, screen blend probe).
pub const fn blend_add(dst: Rgba, src: Rgba, opacity: u8) -> Rgba {
    if dst[3] != 255 {
        return blend_add_translucent(dst, src, opacity);
    }
    let a = effective_alpha(src[3], opacity);
    if a == 0 {
        return dst;
    }
    [
        dst[0].saturating_add(share(src[0] as i32, a) as u8),
        dst[1].saturating_add(share(src[1] as i32, a) as u8),
        dst[2].saturating_add(share(src[2] as i32, a) as u8),
        dst[3],
    ]
}

/// Additive blend onto a destination of any alpha, which is kept. This is what `blend_add`
/// computes for a destination alpha below 255.
///
/// Formula, per colour channel `c`, with `a = effective_alpha(src.a, opacity)`:
/// `c = min(255, dst.c + div255(src.c * a))`.
///
/// PROVENANCE: Textbook.
pub const fn blend_add_translucent(dst: Rgba, src: Rgba, opacity: u8) -> Rgba {
    let a = effective_alpha(src[3], opacity);
    [
        dst[0].saturating_add(scale(src[0], a)),
        dst[1].saturating_add(scale(src[1], a)),
        dst[2].saturating_add(scale(src[2], a)),
        dst[3],
    ]
}

/// Subtractive blend of `src` from `dst` at `opacity`, saturating at 0. The destination alpha
/// is kept.
///
/// Formula for an opaque destination, per colour channel `c`, with
/// `a = effective_alpha(src.a, opacity)`: `c = max(0, dst.c - ((src.c * (a + 1)) >> 8))`, and
/// `dst` unchanged when `a = 0`. A destination with an alpha below 255 is blended by
/// [`blend_sub_translucent`], which is not measured yet.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-09, RGSS103J, screen blend probe).
pub const fn blend_sub(dst: Rgba, src: Rgba, opacity: u8) -> Rgba {
    if dst[3] != 255 {
        return blend_sub_translucent(dst, src, opacity);
    }
    let a = effective_alpha(src[3], opacity);
    if a == 0 {
        return dst;
    }
    [
        dst[0].saturating_sub(share(src[0] as i32, a) as u8),
        dst[1].saturating_sub(share(src[1] as i32, a) as u8),
        dst[2].saturating_sub(share(src[2] as i32, a) as u8),
        dst[3],
    ]
}

/// Subtractive blend from a destination of any alpha, which is kept. This is what `blend_sub`
/// computes for a destination alpha below 255.
///
/// Formula, per colour channel `c`, with `a = effective_alpha(src.a, opacity)`:
/// `c = max(0, dst.c - div255(src.c * a))`.
///
/// PROVENANCE: Textbook.
pub const fn blend_sub_translucent(dst: Rgba, src: Rgba, opacity: u8) -> Rgba {
    let a = effective_alpha(src[3], opacity);
    [
        dst[0].saturating_sub(scale(src[0], a)),
        dst[1].saturating_sub(scale(src[1], a)),
        dst[2].saturating_sub(scale(src[2], a)),
        dst[3],
    ]
}

/// Blends a flat `color` over `dst` with the colour's alpha. The destination alpha is kept.
///
/// Formula, per colour channel `c`: `c = dst.c + (((color.c - dst.c) * (color.a + 1)) >> 8)`,
/// and `dst` unchanged when `color.a = 0`.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-09, RGSS103J, screen blend probe).
pub const fn blend_color(dst: Rgba, color: Rgba) -> Rgba {
    if color[3] == 0 {
        return dst;
    }
    [
        lerp(dst[0], color[0], color[3]),
        lerp(dst[1], color[1], color[3]),
        lerp(dst[2], color[2], color[3]),
        dst[3],
    ]
}

/// The grey part of a tone: desaturates `dst` by `gray`. The destination alpha is kept.
///
/// Formula, with `r`, `g`, `b` the channels of `dst`:
///
/// ```text
/// out = dst                                        when gray = 0
/// l   = (77 * r + 150 * g + 29 * b) >> 8
/// c   = dst.c + (((l - dst.c) * (gray + 1)) >> 8)   per colour channel c
/// ```
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-09, RGSS103J, tone probe).
pub const fn apply_gray(dst: Rgba, gray: u8) -> Rgba {
    if gray == 0 {
        return dst;
    }
    let luma = ((77 * dst[0] as u32 + 150 * dst[1] as u32 + 29 * dst[2] as u32) >> 8) as u8;
    [
        lerp(dst[0], luma, gray),
        lerp(dst[1], luma, gray),
        lerp(dst[2], luma, gray),
        dst[3],
    ]
}

/// Applies a tone to `dst`: desaturation by `gray` ([`apply_gray`]), then the signed offsets
/// `red`, `green` and `blue`. The destination alpha is kept.
///
/// Formula, with `m = apply_gray(dst, gray)` and offsets clamped to -255..=255, per colour
/// channel `c`: `c = clamp(m.c + offset.c, 0, 255)`.
///
/// PROVENANCE: ReverseEngineered (oracle measurement 2026-10-09, RGSS103J, screen blend probe).
pub const fn apply_tone(dst: Rgba, red: i16, green: i16, blue: i16, gray: u8) -> Rgba {
    let grayed = apply_gray(dst, gray);
    [
        offset(grayed[0], red),
        offset(grayed[1], green),
        offset(grayed[2], blue),
        dst[3],
    ]
}

/// `clamp(channel + clamp(amount, -255, 255), 0, 255)`.
const fn offset(channel: u8, amount: i16) -> u8 {
    let sum = channel as i32 + amount as i32;
    if sum < 0 {
        0
    } else if sum > 255 {
        255
    } else {
        sum as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Destination and source pixels that exercise ends and middles of every channel.
    const SAMPLES: [Rgba; 6] = [
        [0, 0, 0, 0],
        [255, 255, 255, 255],
        [10, 128, 250, 255],
        [200, 100, 50, 128],
        [1, 254, 127, 1],
        [90, 60, 30, 0],
    ];

    type Blend = fn(Rgba, Rgba, u8) -> Rgba;

    const BLENDS: [(&str, Blend); 3] = [
        ("blend_normal", blend_normal),
        ("blend_add", blend_add),
        ("blend_sub", blend_sub),
    ];

    #[test]
    fn div255_rounds_to_nearest() {
        assert_eq!(div255(0), 0);
        assert_eq!(div255(127), 0);
        assert_eq!(div255(128), 1);
        assert_eq!(div255(255), 1);
        assert_eq!(div255(255 * 255), 255);
    }

    #[test]
    fn effective_alpha_ends() {
        for value in 0..=255u8 {
            assert_eq!(effective_alpha(value, 255), value);
            assert_eq!(effective_alpha(255, value), value);
            assert_eq!(effective_alpha(value, 0), 0);
            assert_eq!(effective_alpha(0, value), 0);
        }
        assert_eq!(effective_alpha(128, 128), 64);
    }

    #[test]
    fn zero_opacity_leaves_destination_unchanged() {
        for (name, blend) in BLENDS {
            for dst in SAMPLES {
                for src in SAMPLES {
                    assert_eq!(blend(dst, src, 0), dst, "{name} {dst:?} {src:?}");
                }
            }
        }
    }

    /// The normal blend onto a fully transparent destination is the exception: see
    /// `nothing_underneath_and_nothing_drawn_gives_the_source_colour_with_alpha_0`.
    #[test]
    fn zero_alpha_source_leaves_destination_unchanged() {
        for (name, blend) in BLENDS {
            for dst in SAMPLES {
                if name == "blend_normal" && dst[3] == 0 {
                    continue;
                }
                for opacity in [0, 1, 128, 255] {
                    let src = [255, 128, 7, 0];
                    assert_eq!(blend(dst, src, opacity), dst, "{name} {dst:?} {opacity}");
                }
            }
        }
    }

    #[test]
    fn nothing_underneath_and_nothing_drawn_gives_the_source_colour_with_alpha_0() {
        for dst in [[0, 0, 0, 0], [90, 60, 30, 0]] {
            // A source alpha of 0, and one that the opacity scales down to 0.
            assert_eq!(blend_normal(dst, [255, 128, 7, 0], 255), [255, 128, 7, 0]);
            assert_eq!(blend_normal(dst, [255, 128, 7, 0], 1), [255, 128, 7, 0]);
            assert_eq!(blend_normal(dst, [255, 128, 7, 1], 127), [255, 128, 7, 0]);
            // Opacity 0 comes first and leaves the destination.
            assert_eq!(blend_normal(dst, [255, 128, 7, 0], 0), dst);
            assert_eq!(blend_normal(dst, [255, 128, 7, 255], 0), dst);
        }
    }

    /// For a destination alpha of 255 the formula for a destination with alpha is the screen
    /// formula: every destination channel, source channel and drawn alpha.
    #[test]
    fn an_opaque_destination_gets_the_screen_formula() {
        for dst in 0..=255i32 {
            for src in 0..=255i32 {
                for alpha in 0..=255i32 {
                    let expected = if alpha == 0 {
                        dst
                    } else {
                        dst + (((src - dst) * (alpha + 1)) >> 8)
                    };
                    let d = [dst as u8, 0, 255, 255];
                    let s = [src as u8, 255, 0, alpha as u8];
                    let out = blend_normal_translucent(d, s, 255);
                    assert_eq!(
                        (out[0], out[3]),
                        (expected as u8, 255),
                        "{dst} {src} {alpha}"
                    );
                }
            }
        }
        // The drawn alpha is all that the source alpha and the opacity contribute.
        for alpha in 0..=255u8 {
            for opacity in 1..=255u8 {
                let s = [200, 100, 7, alpha];
                let drawn = [200, 100, 7, effective_alpha(alpha, opacity)];
                for d in [[10, 128, 250, 255], [255, 0, 77, 255]] {
                    assert_eq!(
                        blend_normal_translucent(d, s, opacity),
                        blend_normal_translucent(d, drawn, 255)
                    );
                }
            }
        }
    }

    /// The weight is the quotient except for the five measured pairs.
    #[test]
    fn the_weight_is_one_below_the_quotient_for_the_five_measured_pairs() {
        let mut lowered = Vec::new();
        for a in 0..=255u32 {
            for da in 0..=255u32 {
                let den = 255 * da + 255 * a - da * a;
                if den == 0 {
                    continue;
                }
                let quotient = a * 65025 / den;
                let weight = u32::from(blt_weight(a as u8, da as u8, den));
                if weight != quotient {
                    assert_eq!(weight + 1, quotient);
                    lowered.push((a as u8, da as u8));
                }
                if da == 0 {
                    assert_eq!(weight, 255);
                }
                if da == 255 {
                    assert_eq!(weight, a);
                }
            }
        }
        assert_eq!(lowered, BLT_WEIGHT_EXCEPTIONS);
        // (30, 51): den = 255 * 51 + 255 * 30 - 51 * 30 = 19125, 30 * 65025 / 19125 = 102.
        assert_eq!(
            blend_normal([0, 0, 0, 51], [255, 255, 255, 30], 255),
            [101, 101, 101, 75]
        );
    }

    #[test]
    fn additive_and_subtractive_blends_keep_destination_alpha() {
        for (name, blend) in &BLENDS[1..] {
            for dst in SAMPLES {
                for src in SAMPLES {
                    assert_eq!(blend(dst, src, 200)[3], dst[3], "{name}");
                }
            }
        }
    }

    #[test]
    fn normal_blend_of_an_opaque_source_copies_it_alpha_included() {
        for dst in SAMPLES {
            for src in [[12, 34, 56, 255], [255, 0, 0, 255], [0, 0, 0, 255]] {
                assert_eq!(blend_normal(dst, src, 255), src, "{dst:?}");
            }
        }
    }

    #[test]
    fn normal_blend_onto_transparent_destination_gives_the_source_at_its_drawn_alpha() {
        for dst in [[0, 0, 0, 0], [90, 60, 30, 0]] {
            assert_eq!(blend_normal(dst, [255, 0, 0, 128], 255), [255, 0, 0, 128]);
            assert_eq!(
                blend_normal(dst, [200, 100, 7, 255], 128),
                [200, 100, 7, 128]
            );
            // a = div255(128 * 128) = 64.
            assert_eq!(
                blend_normal(dst, [200, 100, 7, 128], 128),
                [200, 100, 7, 64]
            );
            assert_eq!(blend_normal(dst, [200, 100, 7, 1], 255), [200, 100, 7, 1]);
        }
    }

    #[test]
    fn normal_blend_onto_translucent_destination_follows_the_measured_formula() {
        // a = 128, den = 255 * 128 + 255 * 128 - 128 * 128 = 48896, alpha = den / 255 = 191,
        // w = 128 * 65025 / den = 170.
        // red   = 0 + ((200 * 171) >> 8) = 133
        // green = 50 + ((50 * 171) >> 8) = 83
        // blue  = 255 + ((-255 * 171) >> 8) = 255 - 171 = 84
        assert_eq!(
            blend_normal([0, 50, 255, 128], [200, 100, 0, 128], 255),
            [133, 83, 84, 191]
        );
        // The same drawn alpha reached through opacity gives the same pixel.
        assert_eq!(
            blend_normal([0, 50, 255, 128], [200, 100, 0, 255], 128),
            [133, 83, 84, 191]
        );
        // An opaque source replaces a translucent destination completely.
        assert_eq!(
            blend_normal([0, 50, 255, 128], [200, 100, 0, 255], 255),
            [200, 100, 0, 255]
        );
    }

    #[test]
    fn normal_blend_alpha_never_drops_and_colour_stays_between() {
        for dst_alpha in [0u8, 1, 64, 128, 254, 255] {
            for src_alpha in [0u8, 1, 64, 128, 254, 255] {
                for opacity in [0u8, 1, 128, 255] {
                    for (dst_c, src_c) in [(0u8, 255u8), (255, 0), (10, 200), (77, 77)] {
                        let dst = [dst_c, dst_c, dst_c, dst_alpha];
                        let src = [src_c, src_c, src_c, src_alpha];
                        let out = blend_normal(dst, src, opacity);
                        let a = effective_alpha(src_alpha, opacity);
                        assert!(out[3] >= dst_alpha.max(a), "{dst:?} {src:?} {opacity}");
                        assert!(out[0] >= dst_c.min(src_c) && out[0] <= dst_c.max(src_c));
                    }
                }
            }
        }
    }

    /// The formula of the original, written out, for every input on an opaque destination.
    #[test]
    fn blends_onto_opaque_destination_follow_the_measured_formula() {
        for dst in 0..=255i32 {
            for src in [0i32, 1, 100, 127, 128, 254, 255] {
                for alpha in 0..=255i32 {
                    let d = [dst as u8, 0, 0, 255];
                    let s = [src as u8, 0, 0, alpha as u8];
                    let (normal, add, sub) = if alpha == 0 {
                        (dst, dst, dst)
                    } else {
                        (
                            dst + (((src - dst) * (alpha + 1)) >> 8),
                            (dst + ((src * (alpha + 1)) >> 8)).min(255),
                            (dst - ((src * (alpha + 1)) >> 8)).max(0),
                        )
                    };
                    let case = format!("{dst} {src} {alpha}");
                    assert_eq!(blend_normal(d, s, 255), [normal as u8, 0, 0, 255], "{case}");
                    assert_eq!(blend_add(d, s, 255), [add as u8, 0, 0, 255], "{case}");
                    assert_eq!(blend_sub(d, s, 255), [sub as u8, 0, 0, 255], "{case}");
                }
            }
        }
    }

    /// A destination that is darker than the source by one step of alpha is not pulled below
    /// itself: the arithmetic shift rounds down, so alpha 0 has to leave the pixel alone.
    #[test]
    fn zero_alpha_is_not_the_formula_at_zero() {
        assert_eq!(blend_normal([200, 0, 0, 255], [0, 0, 0, 0], 255)[0], 200);
        assert_eq!(blend_normal([200, 0, 0, 255], [0, 0, 0, 1], 255)[0], 198);
        assert_eq!(blend_color([200, 0, 0, 255], [0, 0, 0, 0])[0], 200);
        assert_eq!(blend_color([200, 0, 0, 255], [0, 0, 0, 1])[0], 198);
    }

    #[test]
    fn a_translucent_destination_takes_the_path_for_a_destination_with_alpha() {
        for dst in SAMPLES {
            for src in SAMPLES {
                if dst[3] == 255 {
                    continue;
                }
                assert_eq!(
                    blend_normal(dst, src, 200),
                    blend_normal_translucent(dst, src, 200)
                );
                assert_eq!(
                    blend_add(dst, src, 200),
                    blend_add_translucent(dst, src, 200)
                );
                assert_eq!(
                    blend_sub(dst, src, 200),
                    blend_sub_translucent(dst, src, 200)
                );
            }
        }
    }

    #[test]
    fn normal_blend_mixes_halfway() {
        assert_eq!(
            blend_normal([0, 100, 255, 255], [255, 200, 0, 255], 128),
            [128, 150, 126, 255]
        );
        // Half alpha and half opacity: a = div255(128 * 128) = 64.
        assert_eq!(
            blend_normal([0, 0, 0, 255], [255, 255, 255, 128], 128),
            [64, 64, 64, 255]
        );
    }

    #[test]
    fn normal_blend_stays_between_destination_and_source() {
        for dst in 0..=255u8 {
            for src in [0u8, 1, 127, 128, 254, 255] {
                for opacity in [1u8, 64, 128, 200, 254] {
                    let out = blend_normal([dst, 0, 0, 255], [src, 0, 0, 255], opacity)[0];
                    assert!(out >= dst.min(src) && out <= dst.max(src));
                }
            }
        }
    }

    #[test]
    fn additive_blend_adds_and_saturates_at_255() {
        assert_eq!(
            blend_add([10, 20, 30, 255], [1, 2, 3, 255], 255),
            [11, 22, 33, 255]
        );
        assert_eq!(
            blend_add([200, 250, 255, 255], [100, 6, 1, 255], 255),
            [255, 255, 255, 255]
        );
        // Half opacity adds half of the source.
        assert_eq!(
            blend_add([100, 250, 0, 255], [100, 100, 100, 255], 128),
            [150, 255, 50, 255]
        );
    }

    #[test]
    fn subtractive_blend_subtracts_and_saturates_at_0() {
        assert_eq!(
            blend_sub([10, 20, 30, 255], [1, 2, 3, 255], 255),
            [9, 18, 27, 255]
        );
        assert_eq!(
            blend_sub([50, 5, 0, 255], [100, 6, 1, 255], 255),
            [0, 0, 0, 255]
        );
        assert_eq!(
            blend_sub([100, 20, 255, 255], [100, 100, 100, 255], 128),
            [50, 0, 205, 255]
        );
    }

    #[test]
    fn color_blend_ends() {
        for dst in SAMPLES {
            assert_eq!(blend_color(dst, [9, 99, 199, 0]), dst);
            assert_eq!(blend_color(dst, [9, 99, 199, 255]), [9, 99, 199, dst[3]]);
        }
        assert_eq!(
            blend_color([0, 100, 255, 7], [255, 200, 0, 128]),
            [128, 150, 126, 7]
        );
    }

    #[test]
    fn neutral_tone_is_the_identity() {
        for value in 0..=255u8 {
            let dst = [value, 255 - value, value / 2, value];
            assert_eq!(apply_tone(dst, 0, 0, 0, 0), dst);
        }
        for dst in SAMPLES {
            assert_eq!(apply_tone(dst, 0, 0, 0, 0), dst);
        }
    }

    #[test]
    fn tone_offsets_add_and_saturate() {
        assert_eq!(
            apply_tone([100, 100, 100, 9], 50, -50, 0, 0),
            [150, 50, 100, 9]
        );
        assert_eq!(
            apply_tone([200, 30, 128, 9], 100, -100, 255, 0),
            [255, 0, 255, 9]
        );
        assert_eq!(
            apply_tone([200, 30, 128, 9], -255, 255, -255, 0),
            [0, 255, 0, 9]
        );
    }

    #[test]
    fn tone_offsets_beyond_range_behave_as_the_range_ends() {
        for dst in SAMPLES {
            assert_eq!(
                apply_tone(dst, i16::MAX, i16::MIN, 300, 0),
                apply_tone(dst, 255, -255, 255, 0)
            );
        }
    }

    #[test]
    fn full_gray_removes_colour() {
        // l = (77 * 255) / 256 = 76 for pure red.
        assert_eq!(
            apply_tone([255, 0, 0, 255], 0, 0, 0, 255),
            [76, 76, 76, 255]
        );
        assert_eq!(
            apply_tone([255, 255, 255, 255], 0, 0, 0, 255),
            [255, 255, 255, 255]
        );
        assert_eq!(apply_tone([0, 0, 0, 255], 0, 0, 0, 255), [0, 0, 0, 255]);
    }

    #[test]
    fn gray_leaves_grey_pixels_unchanged() {
        for value in 0..=255u8 {
            for gray in [1u8, 100, 255] {
                let dst = [value, value, value, 255];
                assert_eq!(apply_tone(dst, 0, 0, 0, gray), dst, "{value} {gray}");
            }
        }
    }

    #[test]
    fn gray_follows_the_measured_formula() {
        // l = (77 * 240) >> 8 = 72; red = 240 + ((-168 * 165) >> 8) = 240 - 109,
        // green = blue = 0 + ((72 * 165) >> 8) = 46.
        assert_eq!(
            apply_tone([240, 0, 0, 255], 0, 0, 0, 164),
            [131, 46, 46, 255]
        );
        // A grey amount of 1 already moves a channel that lies above the luminance.
        assert_eq!(apply_gray([255, 0, 0, 7], 1), [253, 0, 0, 7]);
        assert_eq!(apply_gray([255, 0, 0, 7], 0), [255, 0, 0, 7]);
    }

    #[test]
    fn gray_is_applied_before_offsets() {
        // Full gray turns red into 76 everywhere, then only red is raised.
        assert_eq!(
            apply_tone([255, 0, 0, 255], 100, 0, 0, 255),
            [176, 76, 76, 255]
        );
    }

    /// Names of the `pub fn`s declared in this file, in order.
    fn public_functions() -> Vec<&'static str> {
        include_str!("arith.rs")
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
    fn every_public_function_is_listed_once_in_order() {
        let functions = public_functions();
        assert_eq!(functions.len(), 10, "found {functions:?}");
        let listed: Vec<&str> = crate::provenance().iter().map(|entry| entry.name).collect();
        assert_eq!(listed, functions);
    }

    #[test]
    fn measured_functions_name_the_measurement_and_the_rest_are_placeholders() {
        const PLACEHOLDERS: [&str; 2] = ["blend_add_translucent", "blend_sub_translucent"];
        for entry in crate::provenance() {
            assert!(!entry.formula.is_empty(), "{}", entry.name);
            if PLACEHOLDERS.contains(&entry.name) {
                assert_eq!(entry.provenance, Provenance::Textbook, "{}", entry.name);
                assert!(entry.provenance.is_placeholder(), "{}", entry.name);
            } else {
                let probe = match entry.name {
                    "blend_normal_translucent" => "bitmap blt probe",
                    "apply_gray" => "tone probe",
                    _ => "screen blend probe",
                };
                assert_eq!(
                    entry.provenance.to_string(),
                    format!("reverse-engineered: oracle measurement 2026-10-09, RGSS103J, {probe}"),
                    "{}",
                    entry.name
                );
            }
        }
        let placeholders = crate::provenance()
            .iter()
            .filter(|entry| entry.provenance.is_placeholder())
            .count();
        assert_eq!(placeholders, PLACEHOLDERS.len());
    }

    #[test]
    fn every_function_documents_its_provenance() {
        let source = include_str!("arith.rs");
        let notes = source
            .lines()
            .filter(|line| line.starts_with("/// PROVENANCE: "))
            .count();
        assert_eq!(notes, public_functions().len());
    }

    #[test]
    fn provenance_describes_itself() {
        assert_eq!(Provenance::Textbook.to_string(), "textbook placeholder");
        let derived = Provenance::ReverseEngineered {
            source: "specs/blend-normal",
        };
        assert!(!derived.is_placeholder());
        assert_eq!(
            derived.to_string(),
            "reverse-engineered: specs/blend-normal"
        );
    }
}
