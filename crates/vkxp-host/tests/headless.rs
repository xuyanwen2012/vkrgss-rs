//! The headless-run scenarios of `specs/host-session/spec.md`, on llvmpipe. No window is
//! created and no real time passes.

mod common;

use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use common::{SMALL_DEMO, drive_demo, small_frame, spawn_demo, test_device};
use vkxp_host::demo::TRANSITION_FRAMES;
use vkxp_host::testing::ErrorBuffer;
use vkxp_host::{
    HeadlessEnd, HeadlessError, HeadlessOptions, HeadlessReport, HostLink, InputScript, Severity,
    StateSchedule, headless_link, run_headless,
};
use vkxp_proto::link::{Disconnected, GuestEnd};
use vkxp_proto::{
    BitmapId, BlendMode, Button, Color, GuestToHost, HostToGuest, InputSnapshot, Op, Quad, Rect,
    Sampling, Size, Tone, TransitionRule,
};
use vkxp_vk::DeviceChoice;

#[test]
fn unrestricted_loader_is_refused_before_any_vulkan_call() {
    let error = common::isolation_error(|_| None).expect("nothing restricts the loader");
    assert!(error.contains("VK_LOADER_DRIVERS_SELECT"), "{error}");
    let named = |name: &str| (name == "VKXP_TEST_DEVICE").then(|| "llvmpipe".to_owned());
    assert!(
        common::isolation_error(named).is_some(),
        "a name is no restriction"
    );
    let restricted = |name: &str| (name == "VK_LOADER_DRIVERS_SELECT").then(|| "*lvp*".to_owned());
    assert_eq!(common::isolation_error(restricted), None);
}

/// An op that every compositor rejects: a quad of a bitmap that does not exist.
fn invalid_op() -> Op {
    Op::Quad(Quad {
        clip: Rect::new(0, 0, 4, 4),
        bitmap: BitmapId::new(900, 0),
        src: Rect::new(0, 0, 1, 1),
        sampling: Sampling::place_at(0, 0),
        tone: Tone::NEUTRAL,
        color: Color::new(0, 0, 0, 0),
        opacity: 255,
        blend: BlendMode::Normal,
    })
}

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

    let dumped = |index: usize| {
        let (size, rgba) = read_png(&directory.path().join(format!("frame-{index}.png")));
        assert_eq!(size, SMALL_DEMO.size, "dumps are at the logical size");
        assert_eq!(size, frames[index].size);
        rgba
    };
    assert!(
        dumped(6) == frames[6].rgba,
        "frame 6 differs from the reference"
    );

    // B froze the display and started a transition that frame 12 is inside, so its dump is
    // the mix of the frame that was held and frame 12, which the presenter makes in integers.
    let frames_before = |wanted: fn(&GuestToHost) -> bool| {
        let at = messages.iter().position(wanted).unwrap();
        messages[..at]
            .iter()
            .filter(|message| matches!(message, GuestToHost::Frame(_)))
            .count()
    };
    let held = frames_before(|message| matches!(message, GuestToHost::Freeze)) - 1;
    let first = frames_before(|message| matches!(message, GuestToHost::Transition { .. }));
    assert!(
        held < first && first < 12,
        "frame 12 is past the first step"
    );
    let weight = 255 * (12 - first) as u32 / TRANSITION_FRAMES;
    let mixed = dumped(12);
    assert_ne!(frames[held].rgba, frames[12].rgba);
    for (at, shown) in mixed.iter().enumerate() {
        let (from, to) = (frames[held].rgba[at], frames[12].rgba[at]);
        let exact = (u32::from(from) * (256 - weight) + u32::from(to) * weight) >> 8;
        assert_eq!(
            u32::from(*shown),
            exact,
            "byte {at} of frame 12 is not the mix of {from} and {to} at {weight}"
        );
    }
}

/// A frame of one grey level, of the size of [`small_frame`].
fn grey_frame(frame_index: u64, level: u8) -> GuestToHost {
    let GuestToHost::Frame(mut packet) = small_frame(frame_index) else {
        unreachable!("a small frame is a frame");
    };
    packet.ops = vec![Op::Clear {
        clip: Rect::from_size(packet.logical_size),
        color: Color::new(level, level, level, 255),
    }];
    GuestToHost::Frame(packet)
}

/// Runs a guest that shows a white frame, sends `between`, shows `black` black frames and
/// shuts down, with every frame dumped. Returns the red channel of each dump, which is a
/// frame of one colour.
fn dumped_levels(between: Vec<GuestToHost>, black: u64) -> Vec<u8> {
    let directory = tempfile::tempdir().unwrap();
    let path = |index: u64| directory.path().join(format!("frame-{index}.png"));
    let frames = 1 + black;
    let guest = move |link: GuestEnd| {
        let show = |message| {
            link.send(message).unwrap();
            assert_eq!(link.recv(), Ok(HostToGuest::Tick));
        };
        show(grey_frame(0, 255));
        for message in between {
            link.send(message).unwrap();
        }
        for index in 1..frames {
            show(grey_frame(index, 0));
        }
        link.send(GuestToHost::Shutdown).unwrap();
    };
    let errors = ErrorBuffer::new();
    let (result, ()) = run_with(
        HeadlessOptions {
            dumps: (0..frames).map(|index| (index, path(index))).collect(),
            errors: Box::new(errors.clone()),
            ..options()
        },
        guest,
    );
    assert_eq!(result.unwrap().frames, frames);
    assert_eq!(errors.text(), "");
    (0..frames)
        .map(|index| {
            let (size, rgba) = read_png(&path(index));
            assert_eq!(size, Size::new(16, 12), "dumps are at the logical size");
            let (pixels, rest) = rgba.as_chunks::<4>();
            assert!(rest.is_empty());
            let first = pixels[0];
            assert_eq!([first[1], first[2], first[3]], [first[0], first[0], 255]);
            assert!(
                pixels.iter().all(|pixel| *pixel == first),
                "frame {index} is of one colour"
            );
            first[0]
        })
        .collect()
}

fn transition(duration: u32) -> GuestToHost {
    GuestToHost::Transition {
        duration,
        mask: None,
        vague: 40,
        rule: TransitionRule::Rgss1,
    }
}

#[test]
fn dump_of_a_frame_under_a_freeze_is_the_held_frame() {
    assert_eq!(dumped_levels(vec![GuestToHost::Freeze], 2), [255, 255, 255]);
}

#[test]
fn dump_in_the_middle_of_a_transition_is_the_mix() {
    let levels = dumped_levels(vec![GuestToHost::Freeze, transition(4)], 6);
    // The black frames of the transition are at steps 0 to 3 of 4, with the weights 0, 63,
    // 127 and 191: white keeps `(255 * (256 - weight)) >> 8`.
    assert_eq!(levels, [255, 255, 192, 128, 64, 0, 0]);
}

#[test]
fn dump_under_a_transition_without_a_freeze_is_the_frame_that_was_on_screen() {
    let levels = dumped_levels(vec![transition(2)], 4);
    assert_eq!(levels, [255, 255, 255, 0, 0]);
}

#[test]
fn dump_after_a_transition_of_duration_zero_is_the_newest_frame() {
    let levels = dumped_levels(vec![GuestToHost::Freeze, transition(0)], 2);
    assert_eq!(levels, [255, 0, 0]);
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
fn rejected_frame_gets_its_tick_and_then_fails_the_run_with_the_error_reported() {
    let errors = ErrorBuffer::new();
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("rejected.png");
    let (result, received) = run_with(
        HeadlessOptions {
            errors: Box::new(errors.clone()),
            dumps: vec![(2, dump.clone())],
            ..options()
        },
        |link| {
            // Everything the host sends, in order, until it is gone.
            let mut received = Vec::new();
            for index in 0..3 {
                let mut frame = small_frame(index);
                if let (2, GuestToHost::Frame(packet)) = (index, &mut frame) {
                    packet.ops.push(invalid_op());
                }
                link.send(frame)
                    .expect("the host is there for the first three frames");
                received.push(link.recv());
            }
            // The run has failed by now or is about to. Whether this frame still gets into the
            // link or not, it is not answered: the next thing the guest learns is that the
            // host has gone.
            let _ = link.send(small_frame(3));
            received.push(link.recv());
            received.push(link.recv());
            received
        },
    );

    // One tick per frame, the rejected third one included, and only then the disconnection.
    assert_eq!(
        received,
        [
            Ok(HostToGuest::Tick),
            Ok(HostToGuest::Tick),
            Ok(HostToGuest::Tick),
            Err(Disconnected),
            Err(Disconnected),
        ]
    );
    let ticks = received
        .iter()
        .filter(|message| **message == Ok(HostToGuest::Tick))
        .count();
    assert_eq!(ticks, 3);

    let error = result.unwrap_err();
    assert!(
        matches!(error, HeadlessError::FrameRejected { frame: 2 }),
        "{error}"
    );
    let text = errors.text();
    assert!(text.contains("frame 2 was rejected"), "{text}");
    assert!(text.contains("op 1 is invalid"), "{text}");
    assert_eq!(errors.severities("was rejected"), [Severity::Error]);
    assert!(!dump.exists(), "a rejected frame is not dumped");
}

#[test]
fn input_scripted_for_a_rejected_frame_still_arrives_before_its_tick() {
    let (result, received) = run_with(
        HeadlessOptions {
            script: InputScript::parse("0:A").unwrap(),
            errors: Box::new(ErrorBuffer::new()),
            ..options()
        },
        |link| {
            let GuestToHost::Frame(mut packet) = small_frame(0) else {
                unreachable!("small_frame makes a frame");
            };
            packet.ops.clear();
            packet.ops.push(invalid_op());
            link.send(GuestToHost::Frame(packet)).unwrap();
            [link.recv(), link.recv(), link.recv()]
        },
    );
    assert!(matches!(
        result,
        Err(HeadlessError::FrameRejected { frame: 0 })
    ));
    assert_eq!(
        received,
        [
            Ok(HostToGuest::Input(InputSnapshot::EMPTY.with(Button::A))),
            Ok(HostToGuest::Tick),
            Err(Disconnected),
        ]
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
    common::require_isolation();
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

/// Scenario "Headless" of the audio commands requirement. A headless run has the sink that
/// plays nothing: this crate opens an audio device only in the windowed runner.
#[test]
fn a_game_that_plays_music_runs_headless_without_an_audio_report() {
    use vkxp_proto::{AudioChannel, AudioCommand};

    let errors = ErrorBuffer::new();
    let (result, ()) = run_with(
        HeadlessOptions {
            errors: Box::new(errors.clone()),
            ..options()
        },
        |link| {
            let tick = |index| {
                link.send(small_frame(index)).unwrap();
                assert_eq!(link.recv(), Ok(HostToGuest::Tick));
            };
            let audio = |command| link.send(GuestToHost::Audio(command)).unwrap();
            tick(0);
            for channel in [AudioChannel::Bgm, AudioChannel::Se] {
                audio(AudioCommand::Play {
                    channel,
                    // No sink of a headless run looks at the data.
                    data: b"MThd music".to_vec(),
                    volume: 100,
                    pitch: 100,
                });
            }
            tick(1);
            audio(AudioCommand::Fade {
                channel: AudioChannel::Bgm,
                duration_ms: 1000,
            });
            tick(2);
            audio(AudioCommand::Stop {
                channel: AudioChannel::Bgm,
            });
            link.send(GuestToHost::Shutdown).unwrap();
        },
    );
    let report = result.unwrap();
    assert_eq!(report.end, HeadlessEnd::Shutdown);
    assert_eq!(report.frames, 3);
    assert_eq!(errors.text(), "", "nothing is reported about audio");
}

/// The rows of a frame time summary: the name of each with the number of frames it counts.
fn summary_rows(text: &str) -> Vec<(String, u64)> {
    text.lines()
        .filter_map(|line| {
            let (name, rest) = line.strip_prefix("vkxp:   ")?.split_once(": ")?;
            let count = rest.split(' ').next()?.parse().ok()?;
            Some((name.to_owned(), count))
        })
        .collect()
}

/// Scenario "Headless summary".
#[test]
fn frame_statistics_of_a_headless_run_are_one_summary_at_its_end() {
    let errors = ErrorBuffer::new();
    let (result, _) = run_with(
        HeadlessOptions {
            frame_stats: true,
            errors: Box::new(errors.clone()),
            ..options()
        },
        counting_guest(50),
    );
    assert_eq!(result.unwrap().frames, 50);
    let text = errors.text();
    assert_eq!(text.matches("frame times over").count(), 1, "{text}");
    assert!(
        text.contains("vkxp: frame times over 50 frame(s):"),
        "{text}"
    );
    let rows = summary_rows(&text);
    let count = |name: &str| {
        let row = rows.iter().find(|(row, _)| row == name);
        row.unwrap_or_else(|| panic!("no row `{name}` in:\n{text}"))
            .1
    };
    // The first frame follows no answer; every other one was waited for.
    assert_eq!(count("guest"), 49);
    for stage in ["encode", "bin", "upload", "submit"] {
        assert_eq!(count(stage), 50, "{stage}:\n{text}");
    }
    // A headless run presents nothing.
    assert_eq!(count("present"), 0);
}

/// Scenario "Not requested".
#[test]
fn frame_statistics_are_not_written_unless_requested() {
    let errors = ErrorBuffer::new();
    let (result, _) = run_with(
        HeadlessOptions {
            errors: Box::new(errors.clone()),
            ..options()
        },
        counting_guest(50),
    );
    assert_eq!(result.unwrap().frames, 50);
    assert_eq!(errors.text(), "");
}

/// Scenario "Same frames".
#[test]
fn frame_statistics_do_not_change_a_dumped_frame() {
    let directory = tempfile::tempdir().unwrap();
    let dumped = |frame_stats: bool| {
        let path = directory.path().join(format!("stats-{frame_stats}.png"));
        let (host, demo) = spawn_demo(SMALL_DEMO);
        run_headless(
            HeadlessOptions {
                frames: Some(14),
                script: InputScript::parse("2:RIGHT, 6:").unwrap(),
                dumps: vec![(12, path.clone())],
                frame_stats,
                errors: Box::new(ErrorBuffer::new()),
                ..options()
            },
            host,
        )
        .unwrap();
        assert_eq!(demo.join().unwrap(), Ok(()));
        fs::read(path).unwrap()
    };
    assert_eq!(dumped(true), dumped(false));
}

// The states schedule.

/// Scenario "a scheduled request reaching the guest at its frame".
#[test]
fn scheduled_state_requests_reach_the_guest_before_the_tick_of_their_frame() {
    let schedule = StateSchedule::parse("2:save 3, 4:save 0, 4:save 1").unwrap();
    let (result, received) = run_with(
        HeadlessOptions {
            states: schedule,
            ..options()
        },
        counting_guest(6),
    );
    assert_eq!(result.unwrap().frames, 6);
    let save = |slot| HostToGuest::SaveState { slot };
    let expected: Received = vec![
        vec![],
        vec![],
        vec![save(3)],
        vec![],
        vec![save(0), save(1)],
        vec![],
    ];
    assert_eq!(received, expected);
}

/// A frame that draws the bitmap 1 of generation 0 through the table-less ops of a sprite,
/// and first creates that bitmap when `create` says so.
fn sprite_frame(index: u64, create: bool) -> GuestToHost {
    use vkxp_compose_ref::fixtures;
    let id = BitmapId::new(1, 0);
    let size = Size::new(16, 16);
    let rgba = [index as u8 * 60 + 40, 200, 30, 255].repeat(16);
    GuestToHost::Frame(vkxp_proto::FramePacket {
        frame_index: index,
        logical_size: size,
        frame_rate: vkxp_proto::FrameRate::DEFAULT,
        brightness: 255,
        tables: if create {
            vec![vkxp_proto::TableDelta::Set {
                id: vkxp_proto::TableId::new(0),
                xsize: 2,
                ysize: 1,
                zsize: 1,
                data: vec![1, 2],
            }]
        } else {
            vec![]
        },
        bitmaps: if create {
            fixtures::bitmap(id, 4, 4, rgba).to_vec()
        } else {
            vec![]
        },
        ops: vec![
            Op::Clear {
                clip: Rect::from_size(size),
                color: Color::new(0, 0, 0, 255),
            },
            Op::Quad(fixtures::sprite(id, 4, 4, 3, 5)),
        ],
    })
}

/// Waits for the tick of a frame and returns what arrived before it.
fn until_tick(link: &GuestEnd) -> Vec<HostToGuest> {
    let mut before = Vec::new();
    loop {
        match link.recv().unwrap() {
            HostToGuest::Tick => return before,
            other => before.push(other),
        }
    }
}

/// Scenario "Stale resources": a guest, and what keeps its states, that loads a state when
/// asked: from there on it sends as a guest of which the host holds nothing.
#[test]
fn after_a_loaded_state_a_frame_creates_the_bitmaps_and_tables_the_display_held() {
    let directory = tempfile::tempdir().unwrap();
    let dumps = [0, 2].map(|frame| (frame, directory.path().join(format!("{frame}.png"))));
    let guest = |link: GuestEnd| {
        link.send(sprite_frame(0, true)).unwrap();
        assert_eq!(until_tick(&link), []);
        link.send(sprite_frame(1, false)).unwrap();
        // No tick answers the frame at which the state is loaded.
        assert_eq!(link.recv(), Ok(HostToGuest::LoadState { slot: 2 }));
        link.send(GuestToHost::StateLoaded { slot: 2 }).unwrap();
        link.send(sprite_frame(0, true)).unwrap();
        let before = until_tick(&link);
        link.send(GuestToHost::Shutdown).unwrap();
        before
    };
    let errors = ErrorBuffer::new();
    let (result, before) = run_with(
        HeadlessOptions {
            states: StateSchedule::parse("1:load 2").unwrap(),
            dumps: dumps.to_vec(),
            errors: Box::new(errors.clone()),
            ..options()
        },
        guest,
    );
    let report = result.unwrap_or_else(|error| panic!("{error}: {}", errors.text()));
    assert_eq!(report.frames, 3);
    // The current input is sent again for the guest that goes on.
    assert_eq!(before, [HostToGuest::Input(InputSnapshot::EMPTY)]);
    assert_eq!(read_png(&dumps[0].1), read_png(&dumps[1].1));
    assert_eq!(errors.severities("was rejected"), []);
    assert_eq!(
        errors.severities("state loaded from slot 2"),
        [Severity::Info]
    );
}

/// Without the load the same frame is rejected: the display has the bitmap.
#[test]
fn a_frame_that_creates_a_bitmap_the_display_holds_is_rejected() {
    let guest = |link: GuestEnd| {
        link.send(sprite_frame(0, true)).unwrap();
        until_tick(&link);
        link.send(sprite_frame(0, true)).unwrap();
        until_tick(&link);
    };
    let errors = ErrorBuffer::new();
    let (result, ()) = run_with(
        HeadlessOptions {
            errors: Box::new(errors.clone()),
            ..options()
        },
        guest,
    );
    assert!(matches!(
        result,
        Err(HeadlessError::FrameRejected { frame: 1 })
    ));
}

/// Scenario "Empty slot", as far as the host goes.
#[test]
fn a_load_that_fails_is_reported_and_the_run_goes_on() {
    let guest = |link: GuestEnd| {
        for index in 0..4 {
            link.send(small_frame(index)).unwrap();
            // The tick of the frame waits for the answer.
            loop {
                match link.recv().unwrap() {
                    HostToGuest::Tick => break,
                    HostToGuest::LoadState { slot } => {
                        assert_eq!(index, 1);
                        link.send(GuestToHost::StateFailed { slot, load: true })
                            .unwrap();
                    }
                    other => panic!("{other:?} is not a request to load"),
                }
            }
        }
        link.send(GuestToHost::Shutdown).unwrap();
    };
    let errors = ErrorBuffer::new();
    let (result, ()) = run_with(
        HeadlessOptions {
            states: StateSchedule::parse("1:load 3").unwrap(),
            errors: Box::new(errors.clone()),
            ..options()
        },
        guest,
    );
    let report = result.unwrap();
    assert_eq!((report.end, report.frames), (HeadlessEnd::Shutdown, 4));
    assert_eq!(errors.severities("slot 3 holds no state"), [Severity::Info]);
}
