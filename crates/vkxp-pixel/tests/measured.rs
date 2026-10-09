//! The samples measured on the original runtime (`tests/data/screen-blend-samples.txt`): every
//! one of them is reproduced exactly. The comment lines of the file describe its columns.

use vkxp_pixel::{apply_tone, blend_add, blend_color, blend_normal, blend_sub};

const SAMPLES: &str = include_str!("data/screen-blend-samples.txt");

/// The samples of one kind, as their numbers; the last one is the measured result.
fn samples(kind: &str) -> Vec<Vec<i32>> {
    let rows: Vec<Vec<i32>> = SAMPLES
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.strip_prefix(kind)?.strip_prefix(' '))
        .map(|row| {
            row.split_whitespace()
                .map(|number| number.parse().expect("a number"))
                .collect()
        })
        .collect();
    assert!(!rows.is_empty(), "no `{kind}` samples");
    rows
}

/// A grey opaque pixel, or one with the given alpha.
fn grey(channel: i32, alpha: i32) -> [u8; 4] {
    let channel = u8::try_from(channel).expect("a channel");
    [
        channel,
        channel,
        channel,
        u8::try_from(alpha).expect("an alpha"),
    ]
}

#[test]
fn every_line_of_the_file_is_a_sample_of_a_known_kind() {
    let kinds = ["normal", "add", "sub", "color", "tone", "combined"];
    let lines = SAMPLES.lines().filter(|line| !line.starts_with('#'));
    let total: usize = kinds.iter().map(|kind| samples(kind).len()).sum();
    assert_eq!(total, lines.count());
}

#[test]
fn blend_modes_reproduce_the_measured_samples() {
    type Blend = fn([u8; 4], [u8; 4], u8) -> [u8; 4];
    let modes: [(&str, Blend); 3] = [
        ("normal", blend_normal),
        ("add", blend_add),
        ("sub", blend_sub),
    ];
    for (kind, blend) in modes {
        for row in samples(kind) {
            let [source, dest, alpha, opacity, result] = row[..] else {
                panic!("{kind} {row:?}");
            };
            let out = blend(grey(dest, 255), grey(source, alpha), opacity as u8);
            assert_eq!(out, grey(result, 255), "{kind} {row:?}");
        }
    }
}

#[test]
fn colour_reproduces_the_measured_samples() {
    for row in samples("color") {
        let [source, colour, alpha, result] = row[..] else {
            panic!("{row:?}");
        };
        let out = blend_color(grey(source, 255), grey(colour, alpha));
        assert_eq!(out, grey(result, 255), "{row:?}");
    }
}

#[test]
fn tone_offsets_reproduce_the_measured_samples() {
    for row in samples("tone") {
        let [source, offset, result] = row[..] else {
            panic!("{row:?}");
        };
        let offset = offset as i16;
        let out = apply_tone(grey(source, 255), offset, offset, offset, 0);
        assert_eq!(out, grey(result, 255), "{row:?}");
    }
}

/// A white source with a colour, drawn translucent at opacity 200: colour, then blend.
#[test]
fn colour_and_blend_combined_reproduce_the_measured_samples() {
    for row in samples("combined") {
        let [dest, colour, colour_alpha, alpha, result] = row[..] else {
            panic!("{row:?}");
        };
        let source = apply_tone(grey(255, alpha), 0, 0, 0, 0);
        let source = blend_color(source, grey(colour, colour_alpha));
        let out = blend_normal(grey(dest, 255), source, 200);
        assert_eq!(out, grey(result, 255), "{row:?}");
    }
}
