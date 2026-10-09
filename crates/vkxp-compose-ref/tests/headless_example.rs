//! Runs the `render_frame` example as a process: encoded messages in, PNG out.

mod common;

use std::path::PathBuf;
use std::process::Command;

use vkxp_compose_ref::fixtures;
use vkxp_proto::codec::encode_stream;
use vkxp_proto::{FramePacket, FrameRate, GuestToHost, Op, Rect, Size};

/// The example binary, which `cargo test` builds next to the test binaries.
fn example() -> PathBuf {
    // target/<profile>/deps/<this test> -> target/<profile>/examples/render_frame
    let test_binary = std::env::current_exe().unwrap();
    let profile_dir = test_binary.parent().and_then(|deps| deps.parent()).unwrap();
    let path = profile_dir
        .join("examples")
        .join(format!("render_frame{}", std::env::consts::EXE_SUFFIX));
    assert!(
        path.is_file(),
        "{} is missing; run `cargo test -p vkxp-compose-ref`, which builds examples",
        path.display()
    );
    path
}

#[test]
fn example_writes_the_expected_png_for_every_fixture() {
    let directory = tempfile::tempdir().unwrap();
    for fixture in fixtures::all() {
        let input = directory.path().join(format!("{}.vkxp", fixture.name));
        let output = directory.path().join(format!("{}.png", fixture.name));
        std::fs::write(&input, encode_stream(&fixture.messages).unwrap()).unwrap();

        let run = Command::new(example())
            .arg(&input)
            .arg(&output)
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "{}: {}",
            fixture.name,
            String::from_utf8_lossy(&run.stderr)
        );
        let written = std::fs::read(&output).unwrap();
        assert!(
            written == common::expected_png(fixture.name),
            "{}: the written PNG differs from the committed expected image",
            fixture.name
        );
    }
}

fn run_on(bytes: &[u8]) -> (std::process::Output, bool) {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.vkxp");
    let output = directory.path().join("output.png");
    std::fs::write(&input, bytes).unwrap();
    let run = Command::new(example())
        .arg(&input)
        .arg(&output)
        .output()
        .unwrap();
    (run, output.exists())
}

#[test]
fn example_rejects_a_stream_of_another_protocol_version() {
    let mut bytes = encode_stream(&fixtures::by_name("checkerboard").unwrap().messages).unwrap();
    bytes[4] = bytes[4].wrapping_add(1);
    let (run, wrote_output) = run_on(&bytes);
    assert_eq!(run.status.code(), Some(1));
    assert!(!wrote_output);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("unsupported protocol version"), "{stderr}");
}

#[test]
fn example_reports_a_reserved_op_and_writes_nothing() {
    let size = Size::new(4, 4);
    let messages = [GuestToHost::Frame(FramePacket {
        frame_index: 0,
        logical_size: size,
        frame_rate: FrameRate::DEFAULT,
        bitmaps: vec![],
        ops: vec![Op::TilemapLayer {
            clip: Rect::from_size(size),
        }],
    })];
    let (run, wrote_output) = run_on(&encode_stream(&messages).unwrap());
    assert_eq!(run.status.code(), Some(1));
    assert!(!wrote_output);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("unsupported op kind `TilemapLayer`"),
        "{stderr}"
    );
}

#[test]
fn example_fails_on_a_stream_without_a_frame() {
    let (run, wrote_output) = run_on(&encode_stream(&[GuestToHost::Shutdown]).unwrap());
    assert_eq!(run.status.code(), Some(1));
    assert!(!wrote_output);
    assert!(String::from_utf8_lossy(&run.stderr).contains("no frame"));
}

#[test]
fn example_without_arguments_prints_usage() {
    let run = Command::new(example()).output().unwrap();
    assert_eq!(run.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&run.stderr).contains("usage: render_frame"));
}
