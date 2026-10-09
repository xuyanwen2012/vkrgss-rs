//! The headless-run scenarios of `specs/host-session/spec.md`, on llvmpipe. No window is
//! created and no real time passes.

mod common;

use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use common::{SMALL_DEMO, drive_demo, small_frame, spawn_demo, test_device};
use vkxp_host::testing::ErrorBuffer;
use vkxp_host::{
    HeadlessEnd, HeadlessError, HeadlessOptions, HeadlessReport, HostLink, InputScript,
    headless_link, run_headless,
};
use vkxp_proto::link::{Disconnected, GuestEnd};
use vkxp_proto::{Button, GuestToHost, HostToGuest, InputSnapshot, Op, Rect, Size};
use vkxp_vk::DeviceChoice;

fn options() -> HeadlessOptions {
    HeadlessOptions {
        device: test_device(),
        ..HeadlessOptions::default()
    }
}

/// Runs `guest` on its own thread against a headless run and returns both results.
fn run_with<T: Send + 'static>(
    options: HeadlessOptions,
    guest: impl FnOnce(GuestEnd) -> T + Send + 'static,
) -> (Result<HeadlessReport, HeadlessError>, T) {
    let (guest_end, host): (GuestEnd, HostLink) = headless_link();
    let guest = thread::spawn(move || guest(guest_end));
    let result = run_headless(options, host);
    (result, guest.join().expect("the guest did not panic"))
}

/// The size and RGBA bytes of a PNG file.
fn read_png(path: &Path) -> (Size, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::BufReader::new(fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut rgba).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    rgba.truncate(info.buffer_size());
    (Size::new(info.width, info.height), rgba)
}

/// What the guest was sent while it waited for the tick of each frame, the tick left out.
type Received = Vec<Vec<HostToGuest>>;

/// A guest that sends `frames` small frames and then shuts down, noting what arrives.
fn counting_guest(frames: u64) -> impl FnOnce(GuestEnd) -> Received + Send + 'static {
    move |link| {
        let mut received = Vec::new();
        for index in 0..frames {
            link.send(small_frame(index)).unwrap();
            let mut before_tick = Vec::new();
            loop {
                match link.recv().unwrap() {
                    HostToGuest::Tick => break,
                    other => before_tick.push(other),
                }
            }
            received.push(before_tick);
        }
        link.send(GuestToHost::Shutdown).unwrap();
        received
    }
}

const SCRIPT: &str = "2:RIGHT, 5:RIGHT+C, 8:DOWN, 9:B, 11:";

/// Runs the small demo for 14 frames with [`SCRIPT`], dumping frames 6 and 12 into `directory`.
fn dump_demo(directory: &Path) -> HeadlessReport {
    let (host, demo) = spawn_demo(SMALL_DEMO);
    let report = run_headless(
        HeadlessOptions {
            frames: Some(14),
            script: InputScript::parse(SCRIPT).unwrap(),
            dumps: vec![
                (6, directory.join("frame-6.png")),
                (12, directory.join("frame-12.png")),
            ],
            ..options()
        },
        host,
    )
    .unwrap();
    assert_eq!(
        demo.join().unwrap(),
        Ok(()),
        "the demo shut down on the quit request"
    );
    report
}

#[test]
fn dumped_frames_equal_the_reference_compositor() {
    let directory = tempfile::tempdir().unwrap();
    let report = dump_demo(directory.path());
    assert_eq!(report.end, HeadlessEnd::FrameLimit);
    assert_eq!(report.frames, 14);

    // The same guest with the same input, composited by the reference.
    let messages = drive_demo(SMALL_DEMO, &InputScript::parse(SCRIPT).unwrap(), 14);
    let mut reference = vkxp_compose_ref::Compositor::new();
    let frames: Vec<vkxp_compose_ref::Frame> = messages
        .iter()
        .filter_map(|message| reference.handle(message).unwrap())
        .collect();
    assert_eq!(frames.len(), 14);

    for index in [6, 12] {
        let (size, rgba) = read_png(&directory.path().join(format!("frame-{index}.png")));
        assert_eq!(size, SMALL_DEMO.size, "dumps are at the logical size");
        assert_eq!(size, frames[index].size);
        assert!(
            rgba == frames[index].rgba,
            "frame {index} differs from the reference"
        );
    }
    // The two dumps show different moments: the script moved the sprite in between.
    assert_ne!(frames[6].rgba, frames[12].rgba);
}

#[test]
fn two_runs_write_byte_identical_files() {
    let (first, second) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let reports = (dump_demo(first.path()), dump_demo(second.path()));
    assert_eq!(reports.0, reports.1);
    for name in ["frame-6.png", "frame-12.png"] {
        let bytes = fs::read(first.path().join(name)).unwrap();
        assert!(bytes.len() > 100, "{name}");
        assert!(
            bytes == fs::read(second.path().join(name)).unwrap(),
            "{name}"
        );
    }
}

#[test]
fn scripted_input_arrives_before_the_tick_of_its_frame() {
    let script = InputScript::parse("10:RIGHT, 20:").unwrap();
    let (result, received) = run_with(
        HeadlessOptions {
            script,
            ..options()
        },
        counting_guest(25),
    );
    let report = result.unwrap();
    assert_eq!(report.end, HeadlessEnd::Shutdown);
    assert_eq!(report.frames, 25);

    let right = InputSnapshot::EMPTY.with(Button::Right);
    for (frame, before_tick) in received.iter().enumerate() {
        let expected = match frame {
            10 => vec![HostToGuest::Input(right)],
            20 => vec![HostToGuest::Input(InputSnapshot::EMPTY)],
            _ => Vec::new(),
        };
        assert_eq!(*before_tick, expected, "before the tick of frame {frame}");
    }
}

#[test]
fn script_entry_that_changes_nothing_sends_nothing() {
    let script = InputScript::parse("0:, 3:A, 4:A, 6:").unwrap();
    let (result, received) = run_with(
        HeadlessOptions {
            script,
            ..options()
        },
        counting_guest(8),
    );
    result.unwrap();
    let sent: Vec<usize> = received.iter().map(Vec::len).collect();
    assert_eq!(sent, [0, 0, 0, 1, 0, 0, 1, 0]);
}

#[test]
fn frame_limit_asks_a_guest_that_never_shuts_down_to_quit_and_ends() {
    /// Draws for ever, whatever the host says.
    fn stubborn(link: GuestEnd) -> (u64, Vec<HostToGuest>) {
        let mut log = Vec::new();
        let mut frames = 0;
        loop {
            if link.send(small_frame(frames)).is_err() {
                return (frames, log);
            }
            frames += 1;
            loop {
                match link.recv() {
                    Ok(HostToGuest::Tick) => {
                        log.push(HostToGuest::Tick);
                        break;
                    }
                    Ok(other) => log.push(other),
                    Err(Disconnected) => return (frames, log),
                }
            }
        }
    }

    let (result, (sent, log)) = run_with(
        HeadlessOptions {
            frames: Some(30),
            ..options()
        },
        stubborn,
    );
    let report = result.unwrap();
    assert_eq!(report.end, HeadlessEnd::FrameLimit);
    assert_eq!(report.frames, 30);
    assert_eq!(
        report.elapsed,
        Duration::from_millis(29 * 25),
        "virtual time"
    );

    // Thirty ticks, then the quit request, then nothing: the run did not wait for an answer.
    let mut expected = vec![HostToGuest::Tick; 30];
    expected.push(HostToGuest::Quit);
    assert_eq!(log[..31], expected);
    assert!(sent == 30 || sent == 31, "{sent} frames sent");
    assert!(log[31..].is_empty(), "{:?}", &log[31..]);
}

#[test]
fn run_without_a_limit_lasts_until_the_guest_shuts_down() {
    let (result, received) = run_with(options(), |link| {
        link.send(GuestToHost::SetTitle {
            title: "counting".to_owned(),
        })
        .unwrap();
        counting_guest(40)(link)
    });
    let report = result.unwrap();
    assert_eq!(
        report,
        HeadlessReport {
            end: HeadlessEnd::Shutdown,
            frames: 40,
            elapsed: Duration::from_millis(39 * 25),
            title: "counting".to_owned(),
        }
    );
    assert_eq!(received.len(), 40);
}

#[test]
fn modal_is_written_to_the_error_stream_and_dismissed() {
    let errors = ErrorBuffer::new();
    let (result, answer) = run_with(
        HeadlessOptions {
            errors: Box::new(errors.clone()),
            ..options()
        },
        |link| {
            link.send(GuestToHost::Modal {
                text: "Hello from the guest".to_owned(),
            })
            .unwrap();
            let answer = link.recv();
            link.send(GuestToHost::Shutdown).unwrap();
            answer
        },
    );
    assert_eq!(result.unwrap().frames, 0);
    assert_eq!(answer, Ok(HostToGuest::ModalDismissed));
    assert!(errors.text().contains("Hello from the guest"));
}

#[test]
fn rejected_frame_fails_the_run_with_the_error_on_the_error_stream() {
    let errors = ErrorBuffer::new();
    let (result, _) = run_with(
        HeadlessOptions {
            errors: Box::new(errors.clone()),
            ..options()
        },
        |link| {
            for index in 0..5 {
                let mut frame = small_frame(index);
                if let (2, GuestToHost::Frame(packet)) = (index, &mut frame) {
                    packet.ops.push(Op::Plane {
                        clip: Rect::new(0, 0, 4, 4),
                    });
                }
                if link.send(frame).is_err() || link.recv() != Ok(HostToGuest::Tick) {
                    return;
                }
            }
        },
    );
    let error = result.unwrap_err();
    assert!(
        matches!(error, HeadlessError::FrameRejected { frame: 2 }),
        "{error}"
    );
    let text = errors.text();
    assert!(text.contains("frame 2 was rejected"), "{text}");
    assert!(
        text.contains("unsupported op kind `Plane` (op 1)"),
        "{text}"
    );
}

#[test]
fn guest_that_vanishes_fails_the_run() {
    let (result, ()) = run_with(options(), |link| {
        link.send(small_frame(0)).unwrap();
        assert_eq!(link.recv(), Ok(HostToGuest::Tick));
    });
    let error = result.unwrap_err();
    assert!(
        matches!(error, HeadlessError::GuestDisconnected { frames: 1 }),
        "{error}"
    );
}

#[test]
fn dump_of_a_frame_the_run_never_reached_is_an_error() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("late.png");
    let (result, _) = run_with(
        HeadlessOptions {
            dumps: vec![(5, path.clone())],
            ..options()
        },
        counting_guest(5),
    );
    let error = result.unwrap_err();
    assert!(
        matches!(
            error,
            HeadlessError::DumpNotReached {
                frame: 5,
                frames: 5
            }
        ),
        "{error}"
    );
    assert!(!path.exists());
}

#[test]
fn dump_that_cannot_be_written_fails_the_run() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("missing").join("frame.png");
    let (result, _) = run_with(
        HeadlessOptions {
            dumps: vec![(1, path.clone())],
            ..options()
        },
        |link| {
            for index in 0..3 {
                if link.send(small_frame(index)).is_err() || link.recv().is_err() {
                    return;
                }
            }
        },
    );
    let error = result.unwrap_err();
    assert!(
        matches!(&error, HeadlessError::Dump { frame: 1, path: at, .. } if *at == path),
        "{error}"
    );
}

#[test]
fn frame_limit_of_zero_quits_before_any_frame() {
    let (result, first) = run_with(
        HeadlessOptions {
            frames: Some(0),
            ..options()
        },
        |link| link.recv(),
    );
    let report = result.unwrap();
    assert_eq!((report.end, report.frames), (HeadlessEnd::FrameLimit, 0));
    assert_eq!(first, Ok(HostToGuest::Quit));
}

#[test]
fn unknown_device_fails_the_run_before_any_message() {
    let (result, _) = run_with(
        HeadlessOptions {
            device: DeviceChoice::Name("no such device".to_owned()),
            ..HeadlessOptions::default()
        },
        |link| link.recv(),
    );
    let error = result.unwrap_err();
    assert!(matches!(error, HeadlessError::Compositor(_)), "{error}");
    assert!(error.to_string().contains("cannot start the compositor"));
}
