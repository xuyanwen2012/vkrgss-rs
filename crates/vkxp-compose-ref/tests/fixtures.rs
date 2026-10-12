//! Every generated fixture must render to exactly its committed expected image.
//!
//! The comparison is on PNG bytes. To add a fixture or accept an intended change, run
//!
//! ```sh
//! VKXP_BLESS=1 cargo test -p vkxp-compose-ref --test fixtures
//! ```
//!
//! which rewrites `tests/expected/*.png` through the same render and encode path, then look at
//! the changed images before committing them.

mod common;

use std::collections::BTreeSet;

use vkxp_compose_ref::{Frame, encode_png, fixtures, render_stream};
use vkxp_proto::GuestToHost;
use vkxp_proto::codec::{decode_stream, encode_stream};

fn render(fixture: &fixtures::Fixture) -> Frame {
    render_stream(&fixture.messages)
        .unwrap_or_else(|error| panic!("fixture `{}` failed: {error}", fixture.name))
        .unwrap_or_else(|| panic!("fixture `{}` produced no frame", fixture.name))
}

/// The pixels of a PNG, to tell a pixel difference from a mere encoding difference.
fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    pixels.truncate(info.buffer_size());
    (info.width, info.height, pixels)
}

#[test]
fn fixtures_match_their_expected_images() {
    let bless = std::env::var_os("VKXP_BLESS").is_some();
    let mut mismatches = Vec::new();
    for fixture in fixtures::all() {
        let frame = render(&fixture);
        let actual = encode_png(&frame).unwrap();
        if bless {
            std::fs::write(common::expected_path(fixture.name), &actual).unwrap();
            continue;
        }
        let expected = common::expected_png(fixture.name);
        if actual == expected {
            continue;
        }
        let (width, height, pixels) = decode_png(&expected);
        let what = if (width, height) != (frame.size.width, frame.size.height) {
            format!("size is {width}x{height} in the expected image")
        } else if pixels != frame.rgba {
            let first = pixels
                .as_chunks::<4>()
                .0
                .iter()
                .zip(frame.rgba.as_chunks::<4>().0)
                .position(|(expected, actual)| expected != actual)
                .unwrap() as u32;
            format!(
                "pixels differ, first at ({}, {})",
                first % width,
                first / width
            )
        } else {
            "pixels are equal but the PNG encoding differs (did the `png` crate change?)".to_owned()
        };
        mismatches.push(format!("{}: {what}", fixture.name));
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn expected_images_decode_to_the_rendered_pixels() {
    for fixture in fixtures::all() {
        let frame = render(&fixture);
        let (width, height, pixels) = decode_png(&common::expected_png(fixture.name));
        assert_eq!(
            (width, height),
            (frame.size.width, frame.size.height),
            "{}",
            fixture.name
        );
        assert!(pixels == frame.rgba, "{}: pixels differ", fixture.name);
    }
}

#[test]
fn fixtures_render_the_same_after_an_encoding_round_trip() {
    for fixture in fixtures::all() {
        let bytes = encode_stream(&fixture.messages).unwrap();
        let decoded: Vec<GuestToHost> = decode_stream(&bytes).unwrap();
        assert_eq!(decoded, fixture.messages, "{}", fixture.name);
        assert_eq!(
            render_stream(&decoded).unwrap(),
            Some(render(&fixture)),
            "{}",
            fixture.name
        );
    }
}

#[test]
fn fixture_names_are_unique_and_cover_the_required_cases() {
    let all = fixtures::all();
    let names: BTreeSet<&str> = all.iter().map(|fixture| fixture.name).collect();
    assert_eq!(names.len(), all.len(), "duplicate fixture name");
    for required in [
        "checkerboard",
        "gradient",
        "alpha_shape",
        "overlap",
        "clip",
        "blend_normal",
        "blend_add",
        "blend_subtract",
        "rect_effect",
        "quad_tone_normal",
        "quad_tone_add",
        "quad_tone_subtract",
        "quad_color_normal",
        "quad_color_add",
        "quad_color_subtract",
        "brightness",
    ] {
        assert!(names.contains(required), "fixture `{required}` is missing");
        assert_eq!(fixtures::by_name(required).unwrap().name, required);
    }
    assert_eq!(fixtures::by_name("no such fixture"), None);
}

#[test]
fn every_expected_image_belongs_to_a_fixture() {
    let names: BTreeSet<String> = fixtures::all()
        .iter()
        .map(|fixture| format!("{}.png", fixture.name))
        .collect();
    let directory = common::expected_path("x").parent().unwrap().to_path_buf();
    let files: BTreeSet<String> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(files, names);
}

#[test]
fn fixtures_are_distinct_images() {
    let frames: Vec<(&str, Frame)> = fixtures::all()
        .iter()
        .map(|fixture| (fixture.name, render(fixture)))
        .collect();
    for (index, (name, frame)) in frames.iter().enumerate() {
        for (other_name, other) in &frames[index + 1..] {
            assert!(
                frame != other,
                "`{name}` and `{other_name}` render the same image"
            );
        }
    }
}
