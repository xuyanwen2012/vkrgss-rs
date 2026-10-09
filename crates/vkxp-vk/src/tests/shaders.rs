//! The GLSL arithmetic on its own: that the pixel include covers `vkxp-pixel`, and that the
//! 32-bit-word transform evaluation equals `Transform::source_position` on the device.

use vkxp_proto::{Fixed, Transform};

use super::harness::{Rng, TestCompositor};
use crate::records::RECORD_WORDS;
use crate::resources::read_words;

const PIXEL_GLSL: &str = include_str!("../../shaders/pixel.glsl");
const COMPOSE_SPIRV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/compose.comp.spv"));
const TRANSFORM_TEST_SPIRV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/transform_test.comp.spv"));

/// Names of the functions a GLSL source defines at top level, each with the comment lines
/// directly above it.
fn glsl_functions(source: &str) -> Vec<(&str, String)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut functions = Vec::new();
    for (number, line) in lines.iter().enumerate() {
        let Some(rest) = ["uint ", "uvec4 ", "int ", "void "]
            .iter()
            .find_map(|ty| line.strip_prefix(ty))
        else {
            continue;
        };
        let Some((name, _)) = rest.split_once('(') else {
            continue;
        };
        let comment: Vec<&str> = lines[..number]
            .iter()
            .rev()
            .take_while(|line| line.starts_with("//"))
            .copied()
            .collect();
        functions.push((name, comment.join("\n")));
    }
    functions
}

#[test]
fn every_pixel_function_has_a_glsl_function_of_the_same_name() {
    let functions = glsl_functions(PIXEL_GLSL);
    assert!(!vkxp_pixel::provenance().is_empty());
    for entry in vkxp_pixel::provenance() {
        let matching: Vec<_> = functions
            .iter()
            .filter(|(name, _)| *name == entry.name)
            .collect();
        assert_eq!(matching.len(), 1, "GLSL function `{}`", entry.name);
        let counterpart = format!("Mirrors `vkxp_pixel::{}`", entry.name);
        assert!(
            matching[0].1.contains(&counterpart),
            "the comment of GLSL `{}` must name its Rust counterpart",
            entry.name
        );
    }
}

#[test]
fn every_glsl_pixel_function_names_what_it_mirrors() {
    let functions = glsl_functions(PIXEL_GLSL);
    assert!(functions.len() >= vkxp_pixel::provenance().len());
    for (name, comment) in functions {
        assert!(comment.contains("Mirrors `"), "GLSL `{name}`: {comment:?}");
    }
}

/// The values of every `ArrayStride` decoration in a SPIR-V binary.
fn array_strides(spirv: &[u8]) -> Vec<u32> {
    const OP_DECORATE: u32 = 71;
    const DECORATION_ARRAY_STRIDE: u32 = 6;
    let words = read_words(spirv);
    let mut strides = Vec::new();
    // Instructions follow the five-word header; each starts with its length and opcode.
    let mut position = 5;
    while position < words.len() {
        let (length, opcode) = ((words[position] >> 16) as usize, words[position] & 0xFFFF);
        if opcode == OP_DECORATE && words[position + 2] == DECORATION_ARRAY_STRIDE {
            strides.push(words[position + 3]);
        }
        position += length;
    }
    strides
}

/// The record layout is written by hand on both sides. A record of another size in the shader
/// would go unnoticed by every frame that has a single op.
#[test]
fn shader_reads_op_records_at_the_stride_they_are_written_with() {
    let mut strides = array_strides(COMPOSE_SPIRV);
    strides.sort_unstable();
    // Tile op indices, tile headers, op records.
    assert_eq!(strides, [4, 8, RECORD_WORDS as u32 * 4]);
}

/// One input of the transform test shader.
#[derive(Debug, Clone, Copy)]
struct Case {
    transform: Transform,
    px: u16,
    py: u16,
}

/// Number of 32-bit words of a case and of an outcome in the shader's buffers.
const CASE_WORDS: usize = 8;
const OUTCOME_WORDS: usize = 6;

/// Runs the cases through the GLSL helpers on the device and compares every result with
/// `Transform::source_position`.
fn assert_matches_source_position(cases: &[Case]) {
    let mut input = Vec::with_capacity(cases.len() * CASE_WORDS);
    for case in cases {
        let t = &case.transform;
        for fixed in [t.xx, t.xy, t.yx, t.yy, t.tx, t.ty] {
            input.push(fixed.to_bits() as u32);
        }
        input.extend([u32::from(case.px), u32::from(case.py)]);
    }
    let mut compositor = TestCompositor::new();
    let output = compositor.run_buffer_shader(
        TRANSFORM_TEST_SPIRV,
        &input,
        cases.len() * OUTCOME_WORDS,
        cases.len() as u32,
        64,
    );

    for (case, outcome) in cases.iter().zip(output.as_chunks::<OUTCOME_WORDS>().0) {
        let (u, v) = case.transform.source_position(case.px, case.py);
        let join = |hi: u32, lo: u32| ((u64::from(hi) << 32) | u64::from(lo)) as i64;
        assert_eq!(
            (join(outcome[0], outcome[1]), join(outcome[2], outcome[3])),
            (u, v),
            "source position of {case:?}"
        );
        // The texel is the floor of the position, and must not have been truncated.
        assert_eq!(
            (i64::from(outcome[4] as i32), i64::from(outcome[5] as i32)),
            (u >> 16, v >> 16),
            "texel of {case:?}"
        );
    }
}

/// Largest pixel coordinate of the largest logical size.
const MAX_PIXEL: u16 = 16383;

#[test]
fn transform_matches_source_position_for_extreme_coefficients() {
    let extremes = [
        i32::MIN,
        i32::MIN + 1,
        -0x8000_0000 / 2,
        -65537,
        -65536,
        -1,
        0,
        1,
        65535,
        65536,
        0x7FFF_FFFF / 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    let pixels = [0, 1, MAX_PIXEL - 1, MAX_PIXEL];
    let mut cases = Vec::new();
    for a in extremes {
        for b in extremes {
            for t in [i32::MIN, -1, 0, 0x8000, i32::MAX] {
                for px in pixels {
                    for py in pixels {
                        // The two axes use the same helper; swapping the coefficients between
                        // them exercises both outputs with every pair.
                        cases.push(Case {
                            transform: Transform {
                                xx: Fixed::from_bits(a),
                                xy: Fixed::from_bits(b),
                                yx: Fixed::from_bits(b),
                                yy: Fixed::from_bits(a),
                                tx: Fixed::from_bits(t),
                                ty: Fixed::from_bits(t.wrapping_neg()),
                            },
                            px,
                            py,
                        });
                    }
                }
            }
        }
    }
    assert_matches_source_position(&cases);
}

#[test]
fn transform_matches_source_position_for_random_coefficients() {
    let mut rng = Rng::new(0x5EED_0001);
    let mut cases = Vec::new();
    for round in 0..60_000 {
        // Full-range coefficients, and small ones around the scales real sprites use, where
        // the fractional bits decide the texel.
        let coefficient = |rng: &mut Rng| {
            let bits = rng.next_u32() as i32;
            Fixed::from_bits(if round % 2 == 0 { bits } else { bits >> 13 })
        };
        cases.push(Case {
            transform: Transform {
                xx: coefficient(&mut rng),
                xy: coefficient(&mut rng),
                yx: coefficient(&mut rng),
                yy: coefficient(&mut rng),
                tx: coefficient(&mut rng),
                ty: coefficient(&mut rng),
            },
            px: rng.below(u32::from(MAX_PIXEL) + 1) as u16,
            py: rng.below(u32::from(MAX_PIXEL) + 1) as u16,
        });
    }
    assert_matches_source_position(&cases);
}

#[test]
fn transform_matches_source_position_for_placements_and_scales() {
    let mut cases = Vec::new();
    let transforms = [
        Transform::IDENTITY,
        Transform::place_at(10, 20),
        Transform::place_at(-4, -6),
        Transform::place_at(i16::MAX, i16::MIN),
        Transform {
            xx: Fixed::HALF,
            yy: Fixed::HALF,
            ..Transform::IDENTITY
        },
        Transform {
            xx: Fixed::from_ratio(1, 3).unwrap(),
            yy: Fixed::from_ratio(-7, 5).unwrap(),
            ..Transform::place_at(3, 3)
        },
        Transform {
            xx: Fixed::ZERO,
            xy: Fixed::ONE,
            yx: Fixed::from_int(-1),
            yy: Fixed::ZERO,
            tx: Fixed::from_int(-4),
            ty: Fixed::from_int(44),
        },
    ];
    for transform in transforms {
        for px in (0..640).step_by(7).chain([MAX_PIXEL]) {
            for py in (0..480).step_by(11).chain([MAX_PIXEL]) {
                cases.push(Case { transform, px, py });
            }
        }
    }
    assert_matches_source_position(&cases);
}
