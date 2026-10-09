//! The arithmetic itself. Every `pub fn` in this file has an entry in [`PROVENANCE`]; a test
//! reads this file and fails if one is missing.
//!
//! Notation used in the formulas: channels are integers from 0 to 255, `/` is integer division
//! rounding down, and `div255(x) = (x + 127) / 255` is division by 255 rounded to nearest.

use crate::{ArithmeticFn, Provenance};

/// A pixel: red, green, blue and straight (non-premultiplied) alpha.
pub type Rgba = [u8; 4];

/// The provenance of every public function in this file, in declaration order.
pub const PROVENANCE: &[ArithmeticFn] = &[
    ArithmeticFn {
        name: "effective_alpha",
        formula: "a = div255(alpha * opacity)",
        provenance: Provenance::Textbook,
    },
    ArithmeticFn {
        name: "blend_normal",
        formula: "c = div255(src.c * a + dst.c * (255 - a)), a = effective_alpha(src.a, opacity)",
        provenance: Provenance::Textbook,
    },
    ArithmeticFn {
        name: "blend_add",
        formula: "c = min(255, dst.c + div255(src.c * a)), a = effective_alpha(src.a, opacity)",
        provenance: Provenance::Textbook,
    },
    ArithmeticFn {
        name: "blend_sub",
        formula: "c = max(0, dst.c - div255(src.c * a)), a = effective_alpha(src.a, opacity)",
        provenance: Provenance::Textbook,
    },
    ArithmeticFn {
        name: "blend_color",
        formula: "c = div255(color.c * color.a + dst.c * (255 - color.a))",
        provenance: Provenance::Textbook,
    },
    ArithmeticFn {
        name: "apply_tone",
        formula: "l = (77 * r + 150 * g + 29 * b) / 256, \
                  m = div255(dst.c * (255 - gray) + l * gray), \
                  c = clamp(m + clamp(tone.c, -255, 255), 0, 255)",
        provenance: Provenance::Textbook,
    },
];

/// Division by 255, rounded to nearest.
const fn div255(value: u32) -> u32 {
    (value + 127) / 255
}

/// `div255(src * alpha + dst * (255 - alpha))`: `dst` at alpha 0, `src` at alpha 255.
const fn mix(dst: u8, src: u8, alpha: u8) -> u8 {
    div255(src as u32 * alpha as u32 + dst as u32 * (255 - alpha as u32)) as u8
}

/// `div255(channel * alpha)`: the share of a source channel that is added or subtracted.
const fn scale(channel: u8, alpha: u8) -> u8 {
    div255(channel as u32 * alpha as u32) as u8
}

/// The alpha a source pixel is drawn with: its own alpha scaled by the opacity of the draw.
///
/// Formula: `a = div255(alpha * opacity)`.
///
/// PROVENANCE: Textbook.
pub const fn effective_alpha(alpha: u8, opacity: u8) -> u8 {
    scale(alpha, opacity)
}

/// Normal blend of `src` over `dst` at `opacity`. The destination alpha is kept.
///
/// Formula, per colour channel `c`, with `a = effective_alpha(src.a, opacity)`:
/// `c = div255(src.c * a + dst.c * (255 - a))`.
///
/// PROVENANCE: Textbook.
pub const fn blend_normal(dst: Rgba, src: Rgba, opacity: u8) -> Rgba {
    let a = effective_alpha(src[3], opacity);
    [
        mix(dst[0], src[0], a),
        mix(dst[1], src[1], a),
        mix(dst[2], src[2], a),
        dst[3],
    ]
}

/// Additive blend of `src` onto `dst` at `opacity`, saturating at 255. The destination alpha
/// is kept.
///
/// Formula, per colour channel `c`, with `a = effective_alpha(src.a, opacity)`:
/// `c = min(255, dst.c + div255(src.c * a))`.
///
/// PROVENANCE: Textbook.
pub const fn blend_add(dst: Rgba, src: Rgba, opacity: u8) -> Rgba {
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
/// Formula, per colour channel `c`, with `a = effective_alpha(src.a, opacity)`:
/// `c = max(0, dst.c - div255(src.c * a))`.
///
/// PROVENANCE: Textbook.
pub const fn blend_sub(dst: Rgba, src: Rgba, opacity: u8) -> Rgba {
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
/// Formula, per colour channel `c`: `c = div255(color.c * color.a + dst.c * (255 - color.a))`.
///
/// PROVENANCE: Textbook.
pub const fn blend_color(dst: Rgba, color: Rgba) -> Rgba {
    [
        mix(dst[0], color[0], color[3]),
        mix(dst[1], color[1], color[3]),
        mix(dst[2], color[2], color[3]),
        dst[3],
    ]
}

/// Applies a tone to `dst`: desaturation by `gray`, then the signed offsets `red`, `green` and
/// `blue`. The destination alpha is kept.
///
/// Formula, with `r`, `g`, `b` the channels of `dst` and offsets clamped to -255..=255:
///
/// ```text
/// l = (77 * r + 150 * g + 29 * b) / 256
/// m = div255(dst.c * (255 - gray) + l * gray)      per colour channel c
/// c = clamp(m + offset.c, 0, 255)
/// ```
///
/// PROVENANCE: Textbook.
pub const fn apply_tone(dst: Rgba, red: i16, green: i16, blue: i16, gray: u8) -> Rgba {
    let luma = ((77 * dst[0] as u32 + 150 * dst[1] as u32 + 29 * dst[2] as u32) / 256) as u8;
    [
        offset(mix(dst[0], luma, gray), red),
        offset(mix(dst[1], luma, gray), green),
        offset(mix(dst[2], luma, gray), blue),
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

    #[test]
    fn zero_alpha_source_leaves_destination_unchanged() {
        for (name, blend) in BLENDS {
            for dst in SAMPLES {
                for opacity in [0, 1, 128, 255] {
                    let src = [255, 128, 7, 0];
                    assert_eq!(blend(dst, src, opacity), dst, "{name} {dst:?} {opacity}");
                }
            }
        }
    }

    #[test]
    fn blends_keep_destination_alpha() {
        for (name, blend) in BLENDS {
            for dst in SAMPLES {
                for src in SAMPLES {
                    assert_eq!(blend(dst, src, 200)[3], dst[3], "{name}");
                }
            }
        }
    }

    #[test]
    fn normal_blend_at_full_alpha_and_opacity_copies_source_colour() {
        for dst in SAMPLES {
            assert_eq!(
                blend_normal(dst, [12, 34, 56, 255], 255),
                [12, 34, 56, dst[3]]
            );
        }
    }

    #[test]
    fn normal_blend_mixes_halfway() {
        assert_eq!(
            blend_normal([0, 100, 255, 255], [255, 200, 0, 255], 128),
            [128, 150, 127, 255]
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
            [128, 150, 127, 7]
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
        assert_eq!(functions.len(), 6, "found {functions:?}");
        let listed: Vec<&str> = crate::provenance().iter().map(|entry| entry.name).collect();
        assert_eq!(listed, functions);
    }

    #[test]
    fn every_listed_function_is_currently_a_placeholder() {
        for entry in crate::provenance() {
            assert_eq!(entry.provenance, Provenance::Textbook, "{}", entry.name);
            assert!(entry.provenance.is_placeholder(), "{}", entry.name);
            assert!(!entry.formula.is_empty(), "{}", entry.name);
        }
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
