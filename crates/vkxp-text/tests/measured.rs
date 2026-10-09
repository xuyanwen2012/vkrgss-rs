//! The samples measured on the original runtime (`tests/data/text-samples.txt`).
//!
//! The samples of Liberation Sans are checked with the committed test font. Those of Noto Sans
//! CJK SC need `local/fonts/NotoSansCJKsc-Regular.otf` under the workspace root, which is not
//! committed: without it they are skipped.

mod common;

use std::path::{Path, PathBuf};

use common::{LIBERATION, engine};
use vkxp_text::{FontSources, TextEngine, TextRequest};

const NOTO: &str = "Noto Sans CJK SC";

/// The sample lines of one kind, split into fields; `|` separates groups and stays a field.
fn samples(kind: &str) -> Vec<Vec<String>> {
    let path = std::env::var_os("VKXP_TEXT_SAMPLES").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/text-samples.txt"),
        PathBuf::from,
    );
    let text = std::fs::read_to_string(path).expect("read the samples");
    let lines: Vec<Vec<String>> = text
        .lines()
        .map(|line| line.split_whitespace().map(str::to_owned).collect())
        .filter(|fields: &Vec<String>| fields.first().is_some_and(|first| first == kind))
        .collect();
    assert!(!lines.is_empty(), "no {kind} samples");
    lines
}

fn number(field: &str) -> u32 {
    field.parse().expect("a number")
}

/// A string written as the hexadecimal digits of its UTF-8 bytes, `-` for the empty string.
fn string(field: &str) -> String {
    if field == "-" {
        return String::new();
    }
    let bytes = (0..field.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&field[at..at + 2], 16).expect("hexadecimal"))
        .collect();
    String::from_utf8(bytes).expect("UTF-8")
}

/// An engine on the measured CJK font, when the file is there.
fn noto_engine() -> Option<TextEngine> {
    let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/fonts");
    if !fonts.join("NotoSansCJKsc-Regular.otf").is_file() {
        eprintln!("skipped: local/fonts/NotoSansCJKsc-Regular.otf is absent");
        return None;
    }
    Some(TextEngine::new(&FontSources::directories_only([fonts])))
}

fn styled(family: &str, size: u32, style: u32) -> TextRequest {
    TextRequest {
        bold: style & 1 != 0,
        italic: style & 2 != 0,
        ..TextRequest::new([family], size)
    }
}

/// `M font size ppem ascent descent ave`.
fn check_sizes(engine: &mut TextEngine, font: &str, family: &str) {
    let mut checked = 0;
    for sample in samples("M").iter().filter(|sample| sample[1] == font) {
        let size = number(&sample[2]);
        let layout = engine.layout(&styled(family, size, 0), "H");
        let expected = [3, 4, 5, 6].map(|field| number(&sample[field]));
        assert_eq!(
            [
                layout.pixels_per_em,
                layout.ascent,
                layout.height - layout.ascent,
                layout.average_width
            ],
            expected,
            "{font} at {size}"
        );
        assert_eq!(
            engine.measure(&styled(family, size, 0), "H").1,
            expected[1] + expected[2]
        );
        checked += 1;
    }
    assert!(checked > 0, "no size samples of {font}");
}

// Requirement: Size, bold and italic

#[test]
fn measured_sizes_of_liberation_sans() {
    check_sizes(&mut engine(), "liberation", LIBERATION);
}

#[test]
fn measured_sizes_of_noto_sans_cjk() {
    let Some(mut engine) = noto_engine() else {
        return;
    };
    check_sizes(&mut engine, "noto", NOTO);
    // The example of the requirement.
    let layout = engine.layout(&styled(NOTO, 22, 0), "H");
    assert_eq!((layout.pixels_per_em, layout.height), (15, 21));
}

// Requirement: Measurement

/// `T size style string | width height | ... | sum of the advances, left overhang`, in every
/// style: the width, which includes the left overhang, the height and the advances.
#[test]
fn measured_widths() {
    let Some(mut engine) = noto_engine() else {
        return;
    };
    let mut wrong = Vec::new();
    let samples = samples("T");
    for sample in &samples {
        let request = styled(NOTO, number(&sample[1]), number(&sample[2]));
        let text = string(&sample[3]);
        let layout = engine.layout(&request, &text);
        let sum: u32 = layout.advances.iter().sum();
        let measured = engine.measure(&request, &text);
        let expected = (number(&sample[5]), number(&sample[6]));
        if sum != number(&sample[11]) || measured != expected {
            wrong.push(format!(
                "{}: {measured:?}, advances {sum}",
                sample.join(" ")
            ));
        }
    }
    let count = (wrong.len(), samples.len());
    assert!(wrong.is_empty(), "{count:?}:\n{}", wrong.join("\n"));
}

/// `O size style character advance left right`: characters with an overhang.
#[test]
fn measured_overhangs() {
    let Some(mut engine) = noto_engine() else {
        return;
    };
    let mut wrong = Vec::new();
    let samples = samples("O");
    for sample in &samples {
        let request = styled(NOTO, number(&sample[1]), number(&sample[2]));
        let layout = engine.layout(&request, &string(&sample[3]));
        let found = (
            layout.advances[0],
            layout.left_overhang,
            layout.right_overhang,
        );
        let expected = (number(&sample[4]), number(&sample[5]), number(&sample[6]));
        if found != expected {
            wrong.push(format!("{}: {found:?}", sample.join(" ")));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {}:\n{}",
        wrong.len(),
        samples.len(),
        wrong.join("\n")
    );
}
