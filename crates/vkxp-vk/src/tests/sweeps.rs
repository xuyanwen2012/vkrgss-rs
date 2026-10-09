//! Arithmetic sweeps: frames in which every destination channel value meets every source
//! channel value, for each blend mode, the colour blend and the tone.
//!
//! The frames are 256 pixels wide with the destination varying along x. For the blend modes
//! the source varies along y; a rectangle effect has one colour and one tone per op, so there
//! each row gets its own op. This is what catches a GLSL formula drifting from `vkxp-pixel`.

use vkxp_compose_ref::fixtures::bitmap;
use vkxp_proto::{
    BitmapDelta, BitmapId, BlendMode, Color, FramePacket, FrameRate, GuestToHost, Op, Quad, Rect,
    RectEffect, Size, Tone, Transform,
};

use super::differential::Pair;

const N: u32 = 256;

fn packet(height: u32, bitmaps: Vec<BitmapDelta>, ops: Vec<Op>) -> GuestToHost {
    GuestToHost::Frame(FramePacket {
        frame_index: 0,
        logical_size: Size::new(N, height),
        frame_rate: FrameRate::DEFAULT,
        bitmaps,
        ops,
    })
}

/// An `N` by `height` bitmap from a function of the position.
fn pattern(height: u32, pixel: impl Fn(u8, u32) -> [u8; 4]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((N * height * 4) as usize);
    for y in 0..height {
        for x in 0..N {
            rgba.extend_from_slice(&pixel(x as u8, y));
        }
    }
    rgba
}

/// A quad that copies an `N` by `height` bitmap to the screen unscaled.
fn full_quad(id: BitmapId, height: u32, opacity: u8, blend: BlendMode) -> Op {
    Op::Quad(Quad {
        clip: Rect::new(0, 0, N, height),
        bitmap: id,
        src: Rect::new(0, 0, N, height),
        transform: Transform::IDENTITY,
        tone: Tone::NEUTRAL,
        color: Color::new(0, 0, 0, 0),
        opacity,
        blend,
    })
}

fn clear(height: u32, color: [u8; 4]) -> Op {
    Op::Clear {
        clip: Rect::new(0, 0, N, height),
        color: Color::from_rgba(color),
    }
}

/// Renders the packet on both backends and panics, naming the case, when they differ.
fn check(pair: &mut Pair, case: &str, message: &GuestToHost) {
    match pair.step(message) {
        Ok(Ok(Some(_))) => {}
        Ok(other) => panic!("{case}: the reference did not render: {other:?}"),
        Err(difference) => panic!("{case}: {difference}"),
    }
}

const ALPHAS: [u8; 6] = [0, 1, 127, 128, 254, 255];
const OPACITIES: [u8; 5] = [0, 1, 128, 254, 255];

/// Destination pixels with the three colour channels at x, 255 - x and x: against a source of
/// y, y and 255 - y, each channel pairs every destination value with every source value.
fn destination(alpha: u8) -> Vec<u8> {
    pattern(N, |x, _| [x, 255 - x, x, alpha])
}

fn source(alpha: u8) -> Vec<u8> {
    pattern(N, |_, y| [y as u8, y as u8, 255 - y as u8, alpha])
}

/// Draws the destination onto a transparent screen, which reproduces it exactly, alpha
/// included, then the source over it in the mode under test.
fn blend_sweep(blend: BlendMode) {
    let mut pair = Pair::new();
    // Slots 0.. hold destinations by alpha, 10.. sources by alpha, 20 and 21 the alpha sweep.
    let destination_alphas = [255u8, 128, 1];
    let mut bitmaps = Vec::new();
    for (slot, &alpha) in destination_alphas.iter().enumerate() {
        bitmaps.extend(bitmap(
            BitmapId::new(slot as u32, 0),
            N,
            N,
            destination(alpha),
        ));
    }
    for (slot, &alpha) in ALPHAS.iter().enumerate() {
        bitmaps.extend(bitmap(
            BitmapId::new(10 + slot as u32, 0),
            N,
            N,
            source(alpha),
        ));
    }
    // Alpha against alpha: destination alpha along x, source alpha along y.
    let (alpha_dst, alpha_src) = (BitmapId::new(20, 0), BitmapId::new(21, 0));
    bitmaps.extend(bitmap(
        alpha_dst,
        N,
        N,
        pattern(N, |x, _| [200, 100, 50, x]),
    ));
    bitmaps.extend(bitmap(
        alpha_src,
        N,
        N,
        pattern(N, |_, y| [30, 150, 250, y as u8]),
    ));
    check(&mut pair, "setup", &packet(N, bitmaps, vec![]));

    for (dst_slot, dst_alpha) in destination_alphas.iter().enumerate() {
        for (src_slot, src_alpha) in ALPHAS.iter().enumerate() {
            for opacity in OPACITIES {
                let ops = vec![
                    clear(N, [0, 0, 0, 0]),
                    full_quad(BitmapId::new(dst_slot as u32, 0), N, 255, BlendMode::Normal),
                    full_quad(BitmapId::new(10 + src_slot as u32, 0), N, opacity, blend),
                ];
                check(
                    &mut pair,
                    &format!(
                        "{blend:?}, destination alpha {dst_alpha}, source alpha {src_alpha}, \
                         opacity {opacity}"
                    ),
                    &packet(N, vec![], ops),
                );
            }
        }
    }

    // A transparent destination keeps whatever colour the clear gave it.
    for color in [[0, 0, 0, 0], [255, 255, 255, 0], [90, 60, 30, 0]] {
        for (src_slot, src_alpha) in ALPHAS.iter().enumerate() {
            for opacity in OPACITIES {
                let ops = vec![
                    clear(N, color),
                    full_quad(BitmapId::new(10 + src_slot as u32, 0), N, opacity, blend),
                ];
                check(
                    &mut pair,
                    &format!(
                        "{blend:?}, transparent destination {color:?}, source alpha \
                         {src_alpha}, opacity {opacity}"
                    ),
                    &packet(N, vec![], ops),
                );
            }
        }
    }

    for opacity in OPACITIES {
        let ops = vec![
            clear(N, [0, 0, 0, 0]),
            full_quad(alpha_dst, N, 255, BlendMode::Normal),
            full_quad(alpha_src, N, opacity, blend),
        ];
        check(
            &mut pair,
            &format!("{blend:?}, alpha against alpha, opacity {opacity}"),
            &packet(N, vec![], ops),
        );
    }
}

#[test]
fn normal_blend_sweep_is_byte_identical() {
    blend_sweep(BlendMode::Normal);
}

#[test]
fn additive_blend_sweep_is_byte_identical() {
    blend_sweep(BlendMode::Add);
}

#[test]
fn subtractive_blend_sweep_is_byte_identical() {
    blend_sweep(BlendMode::Subtract);
}

/// The ops that lay down a destination for a rectangle effect: the pattern bitmap at its own
/// alpha, or a uniformly coloured transparent screen.
fn effect_destinations(height: u32) -> Vec<(String, Vec<Op>)> {
    let mut destinations = Vec::new();
    for slot in 0..3 {
        destinations.push((
            format!("destination bitmap {slot}"),
            vec![
                clear(height, [0, 0, 0, 0]),
                full_quad(BitmapId::new(slot, 0), height, 255, BlendMode::Normal),
            ],
        ));
    }
    for color in [[0, 0, 0, 0], [255, 128, 7, 0]] {
        destinations.push((
            format!("transparent destination {color:?}"),
            vec![clear(height, color)],
        ));
    }
    destinations
}

/// Creates the three destination bitmaps of `effect_destinations`: opaque, half and barely
/// visible, with the channels at x, 255 - x and a third value that differs from both.
fn create_effect_destinations(pair: &mut Pair, height: u32) {
    let mut bitmaps = Vec::new();
    for (slot, alpha) in [255u8, 128, 1].into_iter().enumerate() {
        bitmaps.extend(bitmap(
            BitmapId::new(slot as u32, 0),
            N,
            height,
            pattern(height, |x, _| [x, 255 - x, x.wrapping_mul(7), alpha]),
        ));
    }
    check(pair, "setup", &packet(height, bitmaps, vec![]));
}

fn row_effect(row: u32, tone: Tone, color: Color) -> Op {
    Op::RectEffect(RectEffect {
        clip: Rect::new(0, row as i32, N, 1),
        tone,
        color,
    })
}

#[test]
fn color_blend_sweep_is_byte_identical() {
    let mut pair = Pair::new();
    create_effect_destinations(&mut pair, N);
    for (name, destination) in effect_destinations(N) {
        // Colour channel along y, at fixed alphas and with the alpha along y as well.
        let alphas = ALPHAS.iter().map(|&alpha| Some(alpha)).chain([None]);
        for alpha in alphas {
            let mut ops = destination.clone();
            ops.extend((0..N).map(|row| {
                let y = row as u8;
                let color = Color::new(y, 255 - y, y, alpha.unwrap_or(y));
                row_effect(row, Tone::NEUTRAL, color)
            }));
            check(
                &mut pair,
                &format!("colour blend, {name}, alpha {alpha:?}"),
                &packet(N, vec![], ops),
            );
        }
    }
}

#[test]
fn tone_sweep_is_byte_identical() {
    // One row per offset from -255 to 255.
    const ROWS: u32 = 511;
    let mut pair = Pair::new();
    create_effect_destinations(&mut pair, ROWS);
    let none = Color::new(0, 0, 0, 0);
    for (name, destination) in effect_destinations(ROWS) {
        for gray in [0u8, 1, 128, 254, 255] {
            let mut ops = destination.clone();
            ops.extend((0..ROWS).map(|row| {
                let offset = row as i16 - 255;
                row_effect(row, Tone::new(offset, -offset, offset, gray), none)
            }));
            check(
                &mut pair,
                &format!("tone offsets, {name}, gray {gray}"),
                &packet(ROWS, vec![], ops),
            );
        }
        // Gray along y, alone and with offsets on top.
        for offsets in [(0, 0, 0), (40, -40, 255), (-255, 3, -3)] {
            let mut ops = destination.clone();
            ops.extend((0..N).map(|row| {
                let tone = Tone::new(offsets.0, offsets.1, offsets.2, row as u8);
                row_effect(row, tone, none)
            }));
            check(
                &mut pair,
                &format!("tone gray, {name}, offsets {offsets:?}"),
                &packet(ROWS, vec![], ops),
            );
        }
        // Offsets beyond the meaningful range behave as the range ends.
        let mut ops = destination.clone();
        ops.extend((0..ROWS).map(|row| {
            let offset = if row % 2 == 0 { i16::MAX } else { i16::MIN };
            row_effect(row, Tone::new(offset, -300, 300, (row / 2) as u8), none)
        }));
        check(
            &mut pair,
            &format!("tone out of range, {name}"),
            &packet(ROWS, vec![], ops),
        );
    }
}

/// Tone and colour in one op: the tone is applied first.
#[test]
fn tone_then_color_sweep_is_byte_identical() {
    let mut pair = Pair::new();
    create_effect_destinations(&mut pair, N);
    for (name, destination) in effect_destinations(N) {
        let mut ops = destination.clone();
        ops.extend((0..N).map(|row| {
            let y = row as u8;
            row_effect(
                row,
                Tone::new(row as i16 - 128, 128 - row as i16, 64, 255 - y),
                Color::new(255 - y, y, 99, y),
            )
        }));
        check(
            &mut pair,
            &format!("tone then colour, {name}"),
            &packet(N, vec![], ops),
        );
    }
}
