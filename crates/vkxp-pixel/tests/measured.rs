//! The samples measured on the original runtime (the files of `tests/data`): every one of
//! them is reproduced exactly. The comment lines of each file describe its columns.

use vkxp_pixel::{apply_tone, blend_add, blend_color, blend_normal, blend_sub, stretch_index};

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

/// The numbers of a line, whatever separates them.
fn numbers(text: &str) -> Vec<i32> {
    text.split(|c: char| !(c.is_ascii_digit() || c == '-'))
        .filter(|word| !word.is_empty() && *word != "-")
        .map(|word| word.parse().expect("a number"))
        .collect()
}

/// Four numbers as a pixel.
fn pixel(channels: &[i32]) -> [u8; 4] {
    let [r, g, b, a] = channels else {
        panic!("{channels:?}");
    };
    [*r, *g, *b, *a].map(|channel| u8::try_from(channel).expect("a channel"))
}

/// Three numbers as an opaque pixel.
fn opaque(channels: &[i32]) -> [u8; 4] {
    pixel(&[channels[0], channels[1], channels[2], 255])
}

/// A tone from its four numbers.
fn tone(pixel: [u8; 4], tone: &[i32]) -> [u8; 4] {
    let gray = u8::try_from(tone[3]).expect("a grey amount");
    apply_tone(pixel, tone[0] as i16, tone[1] as i16, tone[2] as i16, gray)
}

/// `Bitmap#blt` into a bitmap with any alpha: the blend for a destination with alpha.
#[test]
fn blt_reproduces_the_measured_samples() {
    let samples = include_str!("data/bitmap-blt-samples.txt");
    let mut count = 0;
    for line in samples.lines().filter(|line| !line.starts_with('#')) {
        let row = numbers(line);
        assert_eq!(row.len(), 13, "{line}");
        let opacity = u8::try_from(row[8]).expect("an opacity");
        let out = blend_normal(pixel(&row[0..4]), pixel(&row[4..8]), opacity);
        assert_eq!(out, pixel(&row[9..13]), "{line}");
        count += 1;
    }
    assert!(count > 200, "{count} samples");
}

/// Tones with a grey part on coloured pixels: alone, followed by a colour, and as a viewport
/// tone and colour over a sprite blended onto the background.
#[test]
fn tone_with_grey_reproduces_the_measured_samples() {
    const BACKGROUND: [u8; 4] = [88, 120, 200, 255];
    let samples = include_str!("data/tone-samples.txt");
    let mut counts = [0; 3];
    for line in samples.lines().filter(|line| !line.starts_with('#')) {
        let (kind, rest) = line.split_once(' ').expect("a kind");
        let (inputs, result) = rest.split_once("->").expect("a result");
        let parts: Vec<Vec<i32>> = inputs.split('|').map(numbers).collect();
        let out = match (kind, &parts[..]) {
            ("tone", [source, t]) => {
                counts[0] += 1;
                tone(opaque(source), t)
            }
            ("tonecolor", [source, t, color]) => {
                counts[1] += 1;
                blend_color(tone(opaque(source), t), pixel(color))
            }
            ("viewport", [source, opacity, t, color]) => {
                counts[2] += 1;
                let opacity = u8::try_from(opacity[0]).expect("an opacity");
                let shown = blend_normal(BACKGROUND, pixel(source), opacity);
                blend_color(tone(shown, t), pixel(color))
            }
            _ => panic!("{line}"),
        };
        assert_eq!(out, opaque(&numbers(result)), "{line}");
    }
    assert!(counts.iter().all(|&count| count >= 50), "{counts:?}");
}

/// Whole bitmaps stretched to whole bitmaps: the source column of each destination column and
/// the source row of each destination row. The clipped and mirrored samples of the file are
/// evaluated by the bitmap operations of `vkxp-core`.
#[test]
fn stretch_step_reproduces_the_measured_samples() {
    let samples = include_str!("data/bitmap-stretch-samples.txt");
    let mut lines = samples.lines().filter(|line| !line.starts_with('#'));
    let mut count = 0;
    while let Some(header) = lines.next() {
        let sizes: Vec<u32> = numbers(header).iter().map(|&n| n as u32).collect();
        let axes: Vec<(u32, u32)> = if header.starts_with("full ") {
            vec![(sizes[0], sizes[2]), (sizes[1], sizes[3])]
        } else if header.starts_with("big ") {
            vec![(sizes[0], sizes[1])]
        } else {
            // A clipped or mirrored case: one line of cells follows.
            lines.next().expect("cells");
            continue;
        };
        for (src_len, dst_len) in axes {
            let measured = numbers(lines.next().expect("indices"));
            let taken: Vec<i32> = (0..dst_len)
                .map(|i| stretch_index(i, src_len, dst_len) as i32)
                .collect();
            assert_eq!(taken, measured, "{header}");
            count += 1;
        }
    }
    assert!(count > 80, "{count} rows");
}
