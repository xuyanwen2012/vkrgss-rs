//! The built `vkrgss` executable, run headless on games assembled here: the scenarios of
//! `game-launch` that need a whole program.
//!
//! Each test makes a game directory with the binding's test support (a `Game.ini`, a script
//! archive and, outside the directory, a stand-in built-in scripts file written for tests)
//! and runs the executable on it as a child process, without a window, on the CPU device.
//!
//! Like the other tests that reach Vulkan, these refuse to run unless the loader has been
//! restricted to chosen drivers, and they hand the restriction on to the child
//! (`support`). No test here runs a game of RGSS3, which needs the guest executable of
//! `vkrgss-guest-rgss3`: those are in `rgss3.rs`.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tempfile::TempDir;
use vkrgss_binding::testing::host::Game;
use vkrgss_binding::testing::{self, Harness};
use vkrgss_vfs::Runtime;

mod support;

use support::*;

/// The harness, started as a game of RGSS1.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss1)
}

/// A red sprite of 32 by 32 pixels at 10, 20, as `$sprite`.
const RED_SPRITE: &str = "\
$sprite = Sprite.new
$sprite.bitmap = Bitmap.new(32, 32)
$sprite.bitmap.fill_rect(0, 0, 32, 32, Color.new(255, 0, 0))
$sprite.x = 10
$sprite.y = 20
";

/// A game with the parts a real one has: a sprite that moves with input, a window with a
/// generated skin, and text. It ends by itself after twelve frames.
fn demo(name: &str) -> Game {
    let window = "\
skin = Bitmap.new(192, 128)
skin.fill_rect(0, 0, 128, 128, Color.new(0, 0, 160))
skin.fill_rect(128, 0, 64, 128, Color.new(200, 200, 200))
$window = Window.new
$window.windowskin = skin
$window.x = 320
$window.y = 240
$window.width = 256
$window.height = 96
$window.contents = Bitmap.new(224, 64)
$window.contents.font.name = 'Liberation Sans'
$window.contents.font.size = 28
$window.contents.draw_text(0, 0, 224, 64, 'Hello, vkrgss')
";
    let main = "\
12.times do
  Graphics.update
  Input.update
  $sprite.x += 4 if Input.press?(Input::RIGHT)
end
exit
";
    Game::new(
        harness(),
        name,
        "Demo",
        &[("Sprite", RED_SPRITE), ("Window", window), ("Main", main)],
    )
}

fn demo_arguments() -> Vec<String> {
    let fonts = font_dir().to_string_lossy().into_owned();
    ["--font-dir", &fonts, "--input", "2:RIGHT, 6:"]
        .into_iter()
        .chain(["--dump", "1=first.png", "--dump", "8=later.png"])
        .map(str::to_owned)
        .collect()
}

fn run_demo(game: &Game) -> Ran {
    let arguments = demo_arguments();
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    headless(game, &arguments)
}

#[test]
fn a_game_that_draws_three_frames_and_returns_exits_with_status_0() {
    let game = Game::new(
        harness(),
        "e2e-three",
        "Three",
        &[("Main", "3.times { Graphics.update }")],
    );
    let ran = headless(&game, &["--dump", "2=last.png"]);
    ran.assert_success();
    assert!(ran.file("last.png").is_file());
}

#[test]
fn a_directory_without_game_ini_is_an_error_naming_it() {
    let empty = tempfile::tempdir().unwrap();
    let ran = vkrgss(&[&empty.path().to_string_lossy(), "--headless"]);
    ran.assert_failure();
    assert!(ran.stderr().contains("Game.ini"), "{}", ran.stderr());
}

#[test]
fn a_file_with_a_relative_name_appears_in_the_game_directory() {
    let source = "File.open('note.txt', 'w') { |file| file.write($DEBUG.inspect) }";
    let game = Game::new(harness(), "e2e-relative", "Relative", &[("Main", source)]);
    let ran = headless(&game, &["--debug"]);
    ran.assert_success();
    let note = fs::read_to_string(game.dir().join("note.txt")).expect("the file was written");
    // Debug mode reached the scripts too.
    assert_eq!(note, "true");
    assert!(!ran.file("note.txt").exists());
}

#[test]
fn a_script_error_is_reported_and_fails_the_run() {
    let source = "Graphics.update\nraise 'boom'\n";
    let game = Game::new(
        harness(),
        "e2e-error",
        "Error",
        &[("Setup", "$x = 1"), ("Main", source)],
    );
    // The level changes nothing: the report is shown at every one.
    for log in [None, Some("error")] {
        let work = tempfile::tempdir().expect("a temporary directory");
        let ran = headless_logging(work, &game, &[], log);
        ran.assert_failure();
        // The guest writes the report as it is and the host writes it once more, each line
        // of it after the prefix, the empty one too. Nothing else is written.
        assert_eq!(
            ran.stderr(),
            "Script 'Main' line 2: RuntimeError occurred.\n\
             \n\
             boom\n\
             vkrgss: Script 'Main' line 2: RuntimeError occurred.\n\
             vkrgss: \n\
             vkrgss: boom\n",
            "{log:?}"
        );
    }
}

#[test]
fn the_frame_limit_ends_a_game_that_never_ends_with_status_0() {
    let game = Game::new(
        harness(),
        "e2e-limit",
        "Limit",
        &[("Main", "loop { Graphics.update }")],
    );
    // The tenth frame, number 9, is still composited and written.
    let ran = headless(&game, &["--frames", "10", "--dump", "9=tenth.png"]);
    ran.assert_success();
    assert!(ran.file("tenth.png").is_file());

    // And it is the last one.
    let ran = headless(&game, &["--frames", "10", "--dump", "10=eleventh.png"]);
    ran.assert_failure();
    assert!(!ran.file("eleventh.png").exists());
}

/// The two forms of the guest, as `--guest` names them.
const FORMS: [&str; 2] = ["thread", "process"];

/// Scenario "Option accepted" of `specs/game-launch/spec.md`.
#[test]
fn frame_statistics_of_a_headless_run_are_written_to_the_error_stream() {
    let game = Game::new(
        harness(),
        "e2e-frame-stats",
        "Stats",
        &[("Main", "loop { Graphics.update }")],
    );
    // The rows are the same whether the guest is a thread or a process.
    for form in FORMS {
        let ran = headless(&game, &["--frames", "10", "--frame-stats", "--guest", form]);
        ran.assert_success();
        let stderr = ran.stderr();
        assert!(
            stderr.contains("vkrgss: frame times over 10 frame(s):"),
            "{form}: {stderr}"
        );
        for row in [
            "guest: 9 frame(s)",
            "encode: 10 frame(s)",
            "submit: 10 frame(s)",
        ] {
            assert!(stderr.contains(row), "{form}: no `{row}` in: {stderr}");
        }
    }

    // The option asked for the summary, so it is shown at every level.
    let work = tempfile::tempdir().expect("a temporary directory");
    let ran = headless_logging(
        work,
        &game,
        &["--frames", "10", "--frame-stats"],
        Some("error"),
    );
    ran.assert_success();
    let stderr = ran.stderr();
    assert!(
        stderr.contains("vkrgss: frame times over 10 frame(s):"),
        "{stderr}"
    );

    // Without the option there is no summary.
    let ran = headless(&game, &["--frames", "10"]);
    ran.assert_success();
    assert!(!ran.stderr().contains("frame times"), "{}", ran.stderr());
}

/// The number of frames a row of the frame time statistics counts, if `stderr` has the row.
fn row_count(stderr: &str, name: &str) -> Option<usize> {
    let row = format!("vkrgss:   {name}: ");
    let rest = stderr.lines().find_map(|line| line.strip_prefix(&row))?;
    rest.split(' ').next()?.parse().ok()
}

/// The scenarios of the requirement "Guest time is split" of `specs/host-session/spec.md`.
#[test]
fn the_guest_splits_its_frame_time_when_statistics_are_requested() {
    const ROWS: [&str; 4] = [
        "guest scripts",
        "guest bitmap operations",
        "guest text",
        "guest extraction",
    ];
    let source = "\
text = Bitmap.new(200, 40)
text.font.name = 'Liberation Sans'
text.font.size = 32
sprite = Sprite.new
sprite.bitmap = text
frame = 0
loop do
  text.clear
  text.draw_text(0, 0, 200, 40, frame.to_s)
  frame += 1
  Graphics.update
end
";
    let game = Game::new(harness(), "e2e-guest-stats", "Split", &[("Main", source)]);
    let fonts = font_dir().to_string_lossy().into_owned();
    let run = |extra: &[&str]| {
        let mut all = vec![
            "--font-dir",
            &fonts,
            "--frames",
            "50",
            "--dump",
            "30=out.png",
        ];
        all.extend(extra);
        let ran = headless(&game, &all);
        ran.assert_success();
        ran
    };

    // Rows of a headless run: each counts the frames the guest measured, which are the ones
    // that follow a tick. A guest in a process of its own writes the same rows itself.
    let measured = run(&["--frame-stats"]);
    for ran in [&measured, &run(&["--frame-stats", "--guest", "process"])] {
        let stderr = ran.stderr();
        assert!(
            stderr.contains("vkrgss: guest frame times over "),
            "{stderr}"
        );
        let counts = ROWS.map(|name| row_count(&stderr, name));
        let frames = counts[0].unwrap_or_else(|| panic!("no row of the scripts in: {stderr}"));
        assert!((49..=50).contains(&frames), "{stderr}");
        assert_eq!(counts, [Some(frames); 4], "{stderr}");
    }

    // Not requested: no such row.
    let plain = run(&[]);
    let stderr = plain.stderr();
    for name in ROWS {
        assert!(!stderr.contains(name), "`{name}` in: {stderr}");
    }

    // Same frames.
    assert!(measured.frame("out.png").rgba == plain.frame("out.png").rgba);
}

/// A game that names the run-time package `Standard` and loads a picture found only there.
/// Returns the game and the directory of the package.
fn rtp_game(name: &str) -> (Game, TempDir) {
    let source = "\
picture = Bitmap.new('Graphics/Pictures/RtpOnly')
File.open('size.txt', 'w') { |file| file.write(\"#{picture.width}x#{picture.height}\") }
";
    let game = Game::new(harness(), name, "Package", &[("Main", source)]);
    let ini = "[Game]\r\nLibrary=RGSS104E.dll\r\nScripts=Data\\Scripts.rxdata\r\n\
               Title=Package\r\nRTP1=Standard\r\n";
    fs::write(game.dir().join("Game.ini"), ini).unwrap();

    let rtp = tempfile::tempdir().unwrap();
    let pictures = rtp.path().join("Graphics").join("Pictures");
    fs::create_dir_all(&pictures).unwrap();
    let file = fs::File::create(pictures.join("RtpOnly.png")).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), 3, 2);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&[200; 3 * 2 * 4]).unwrap();
    writer.finish().unwrap();
    (game, rtp)
}

#[test]
fn a_picture_of_a_run_time_package_loads_when_its_directory_is_given() {
    let (game, rtp) = rtp_game("e2e-rtp");
    let table = format!("Standard={}", rtp.path().display());
    let ran = headless(&game, &["--rtp", &table]);
    ran.assert_success();
    let size = fs::read_to_string(game.dir().join("size.txt")).expect("the script ran");
    assert_eq!(size, "3x2");
}

#[test]
fn a_run_time_package_that_is_not_given_is_an_error_naming_it_and_the_option() {
    let (game, _rtp) = rtp_game("e2e-rtp-missing");
    let ran = headless(&game, &[]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("Standard"), "{stderr}");
    assert!(stderr.contains("--rtp"), "{stderr}");
    assert!(!game.dir().join("size.txt").exists());
}

/// Requirement "Run-time package directories", for a game of VX: `RTP` names the package and
/// `RTP1` is not read.
#[test]
fn a_game_of_vx_names_its_run_time_package_with_the_key_rtp() {
    let (xp, rtp) = rtp_game("e2e-rtp-vx-source");
    let source = "\
picture = Bitmap.new('Graphics/Pictures/RtpOnly')
File.open('size.txt', 'w') { |file| file.write(\"#{picture.width}x#{picture.height}\") }
";
    drop(xp);
    let game = Game::of(
        harness(),
        Runtime::Rgss2,
        "e2e-rtp-vx",
        "Package",
        &[("Main", source)],
    );
    let ini = "[Game]\r\nRTP=RPGVX\r\nLibrary=RGSS202E.dll\r\nScripts=Data\\Scripts.rvdata\r\n\
               Title=Package\r\nRTP1=Standard\r\n";
    fs::write(game.dir().join("Game.ini"), ini).unwrap();
    let table = format!("RPGVX={}", rtp.path().display());
    let ran = headless(&game, &["--rtp", &table]);
    ran.assert_success();
    let size = fs::read_to_string(game.dir().join("size.txt")).expect("the script ran");
    assert_eq!(size, "3x2");
}

/// A fresh directory for a run with `text` as the user's configuration file at its default
/// location below it.
fn work_with_user_config(text: &str) -> TempDir {
    let work = tempfile::tempdir().unwrap();
    fs::create_dir(work.path().join("vkrgss")).unwrap();
    fs::write(work.path().join("vkrgss").join("config.toml"), text).unwrap();
    work
}

/// Scenario "RTP from the file".
#[test]
fn a_run_time_package_named_in_the_user_file_is_found() {
    let (game, rtp) = rtp_game("e2e-rtp-config");
    let work = work_with_user_config(&format!("[rtp]\nStandard = '{}'\n", rtp.path().display()));
    let ran = headless_in(work, &game, &[]);
    ran.assert_success();
    let size = fs::read_to_string(game.dir().join("size.txt")).expect("the script ran");
    assert_eq!(size, "3x2");
}

/// Scenario "Precedence".
#[test]
fn the_seed_of_the_game_file_overrides_the_user_file_and_the_command_line_both() {
    let game = rand_game("e2e-config-seed");
    let run = |user: Option<&str>, arguments: &[&str]| {
        let mut all = vec!["--dump", "2=out.png"];
        all.extend_from_slice(arguments);
        let work = match user {
            Some(text) => work_with_user_config(text),
            None => tempfile::tempdir().unwrap(),
        };
        let ran = headless_in(work, &game, &all);
        ran.assert_success();
        fs::read(ran.file("out.png")).unwrap()
    };
    let seeds: Vec<Vec<u8>> = ["1", "2", "3"]
        .iter()
        .map(|seed| run(None, &["--seed", seed]))
        .collect();
    assert_ne!(seeds[0], seeds[1]);
    assert_ne!(seeds[1], seeds[2]);

    assert_eq!(run(Some("seed = 1"), &[]), seeds[0]);
    fs::write(game.dir().join("vkrgss.toml"), "seed = 2").unwrap();
    assert_eq!(run(Some("seed = 1"), &[]), seeds[1]);
    assert_eq!(run(Some("seed = 1"), &["--seed", "3"]), seeds[2]);
    assert_eq!(
        run(Some("seed = 1"), &["--no-config", "--seed", "1"]),
        seeds[0]
    );
}

/// Scenarios "Ignored" and "Unknown key".
#[test]
fn a_user_file_with_an_error_ends_the_program_unless_it_is_ignored() {
    let game = Game::new(
        harness(),
        "e2e-config-error",
        "Config",
        &[("Main", "3.times { Graphics.update }")],
    );
    let ran = headless_in(work_with_user_config("sead = 1"), &game, &[]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("config.toml"), "{stderr}");
    assert!(stderr.contains("sead"), "{stderr}");

    let ran = headless_in(work_with_user_config("sead = 1"), &game, &["--no-config"]);
    ran.assert_success();
}

/// The settings of a headless run come from the user file alone: the command line names
/// only the game and its built-in scripts. A second run replays what the first recorded.
#[test]
fn a_headless_run_is_set_up_by_the_user_file() {
    let game = demo("e2e-config-headless");
    let by_options = run_demo(&game);
    by_options.assert_success();

    let dir = game.dir().to_string_lossy().into_owned();
    let builtin = game.builtin_scripts().to_string_lossy().into_owned();
    let arguments = [dir.as_str(), "--builtin-scripts", &builtin];
    let common = format!(
        "headless = true\ndevice = '{}'\nfont_dirs = ['{}']\nframes = 12\n\
         dump = ['1=first.png', '8=later.png']\n",
        device(),
        font_dir().display()
    );

    let text = format!("{common}input = '2:RIGHT, 6:'\nrecord = 'session.txt'\n");
    let recorded = vkrgss_in(work_with_user_config(&text), &arguments);
    recorded.assert_success();
    // Paths in the file are relative to the file.
    let config_dir = recorded.file("vkrgss");
    assert!(config_dir.join("session.txt").is_file());
    for name in ["first.png", "later.png"] {
        assert_eq!(
            fs::read(config_dir.join(name)).unwrap(),
            fs::read(by_options.file(name)).unwrap(),
            "{name}"
        );
    }

    let session = config_dir.join("session.txt");
    let text = format!("{common}replay = '{}'\n", session.display());
    let replayed = vkrgss_in(work_with_user_config(&text), &arguments);
    replayed.assert_success();
    assert_eq!(
        fs::read(replayed.file("vkrgss").join("later.png")).unwrap(),
        fs::read(by_options.file("later.png")).unwrap()
    );

    // An input script in a file that the user file names, and frames from the command line.
    let work = work_with_user_config(&format!("{common}input = '@keys.txt'\n"));
    fs::write(work.path().join("vkrgss").join("keys.txt"), "2:RIGHT\n6:\n").unwrap();
    let ran = vkrgss_in(work, &[arguments.as_slice(), &["--frames", "12"]].concat());
    ran.assert_success();
    assert_eq!(
        fs::read(ran.file("vkrgss").join("later.png")).unwrap(),
        fs::read(by_options.file("later.png")).unwrap()
    );
}

/// The checks of the options apply to the final settings.
#[test]
fn settings_of_a_file_that_do_not_go_together_print_the_usage_message() {
    let game = demo("e2e-config-check");
    let dir = game.dir().to_string_lossy().into_owned();
    let builtin = game.builtin_scripts().to_string_lossy().into_owned();
    let headless = "headless = true\nreplay = 'a.txt'\n";
    for (text, extra, named) in [
        // No window opens for these: the check comes first.
        ("frames = 3\n", &[][..], "--headless"),
        (headless, &["--record", "b.txt"], "--replay"),
        (headless, &["--input", "1:UP"], "--input"),
    ] {
        let mut arguments = vec![dir.as_str(), "--builtin-scripts", &builtin];
        arguments.extend_from_slice(extra);
        let ran = vkrgss_in(work_with_user_config(text), &arguments);
        assert_eq!(ran.output.status.code(), Some(2), "{text}");
        let stderr = ran.stderr();
        assert!(stderr.contains(named), "{text}: {stderr}");
        assert!(stderr.contains("usage: vkrgss"), "{text}: {stderr}");
    }
}

#[test]
fn an_unknown_filter_prints_the_usage_message() {
    let ran = vkrgss(&["--filter", "cubic"]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("cubic"), "{stderr}");
    assert!(stderr.contains("usage: vkrgss"), "{stderr}");
}

#[test]
fn an_unknown_option_prints_the_usage_message() {
    let ran = vkrgss(&["--no-such-option"]);
    assert_eq!(ran.output.status.code(), Some(2));
    let stderr = ran.stderr();
    assert!(stderr.contains("--no-such-option"), "{stderr}");
    assert!(stderr.contains("usage: vkrgss"), "{stderr}");
}

#[test]
fn the_version_is_printed_without_a_driver_or_a_game() {
    for option in ["--version", "-V"] {
        let work = tempfile::tempdir().expect("a temporary directory");
        let mut command = command(work.path(), &[option]);
        // A pattern that selects no driver: the loader would find no device.
        for name in ISOLATION_VARIABLES {
            command.env_remove(name);
        }
        command.env("VK_LOADER_DRIVERS_SELECT", "no-such-driver");
        let output = command.output().expect("the program starts");
        assert_eq!(output.status.code(), Some(0), "{option}");
        let expected = format!("vkrgss {}\n", env!("CARGO_PKG_VERSION"));
        assert_eq!(output.stdout, expected.as_bytes(), "{option}");
    }
}

/// Scenario "Help" of the requirement "Help option" of `specs/game-launch/spec.md`.
#[test]
fn the_usage_text_is_printed_without_a_driver_or_a_game() {
    for option in ["--help", "-h"] {
        let work = tempfile::tempdir().expect("a temporary directory");
        let mut command = command(work.path(), &[option]);
        // A pattern that selects no driver: the loader would find no device.
        for name in ISOLATION_VARIABLES {
            command.env_remove(name);
        }
        command.env("VK_LOADER_DRIVERS_SELECT", "no-such-driver");
        let output = command.output().expect("the program starts");
        assert_eq!(output.status.code(), Some(0), "{option}");
        let stdout = String::from_utf8(output.stdout).expect("the usage text is UTF-8");
        assert!(stdout.starts_with("usage: vkrgss"), "{stdout}");
        assert!(stdout.contains("--help"), "{stdout}");
        assert!(stdout.contains("--version"), "{stdout}");
        assert_eq!(output.stderr, b"", "{option}");
    }
}

#[test]
fn a_malformed_input_script_file_prints_the_usage_message() {
    let game = Game::new(
        harness(),
        "e2e-bad-input",
        "Input",
        &[("Main", "Graphics.update")],
    );
    let keys = tempfile::tempdir().unwrap();
    let file = keys.path().join("bad.txt");
    fs::write(&file, "2:SIDEWAYS").unwrap();
    let ran = headless(&game, &["--input", &format!("@{}", file.display())]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("SIDEWAYS"), "{stderr}");
    assert!(stderr.contains("usage: vkrgss"), "{stderr}");
}

#[test]
fn a_dumped_frame_shows_the_red_sprite_where_it_is() {
    let main = format!("{RED_SPRITE}loop {{ Graphics.update }}\n");
    let game = Game::new(harness(), "e2e-dump", "Dump", &[("Main", &main)]);
    let ran = headless(&game, &["--frames", "5", "--dump", "3=out.png"]);
    ran.assert_success();
    let frame = ran.frame("out.png");
    assert_eq!((frame.width, frame.height), (640, 480));
    assert_eq!(frame.pixel(10, 20), RED);
    assert_eq!(frame.pixel(41, 51), RED);
    assert_eq!(frame.pixel(9, 20), [0, 0, 0, 255]);
    assert_eq!(frame.pixel(10, 19), [0, 0, 0, 255]);
}

#[test]
fn dumped_frames_show_a_transition_as_a_window_would() {
    let main = "\
sprite = Sprite.new
sprite.bitmap = Bitmap.new(640, 480)
sprite.bitmap.fill_rect(0, 0, 640, 480, Color.new(255, 255, 255))
Graphics.update
Graphics.freeze
sprite.visible = false
Graphics.transition(4)
Graphics.update
";
    let game = Game::new(harness(), "e2e-transition", "Transition", &[("Main", main)]);
    let names: Vec<String> = (0..6).map(|index| format!("{index}={index}.png")).collect();
    let dumps: Vec<&str> = names.iter().flat_map(|name| ["--dump", name]).collect();
    let ran = headless(&game, &dumps);
    ran.assert_success();

    // The frame before the freeze, the four of the transition, and the one after it. At the
    // steps 0 to 3 of 4 the weights are 0, 63, 127 and 191, and white keeps
    // `(255 * (256 - weight)) >> 8`.
    let expected = [255, 255, 192, 128, 64, 0];
    for (index, level) in expected.into_iter().enumerate() {
        let frame = ran.frame(&format!("{index}.png"));
        assert_eq!((frame.width, frame.height), (640, 480));
        let first = frame.pixel(0, 0);
        assert_eq!(first[0], level, "frame {index}: {first:?}");
        assert_eq!([first[1], first[2], first[3]], [first[0], first[0], 255]);
        assert!(
            frame
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| *pixel == first),
            "frame {index} is of one colour"
        );
    }
}

/// Requirement "Frames of a game of RGSS2 as shown", scenario "Dumped frames of a game": the
/// first ten frames, which the program counts from 0.
#[test]
fn dumped_frames_of_a_game_of_vx_show_a_fade_and_a_transition_after_a_freeze() {
    let main = "\
sprite = Sprite.new
sprite.bitmap = Bitmap.new(544, 416)
sprite.bitmap.fill_rect(0, 0, 544, 416, Color.new(255, 255, 255))
Graphics.update
Graphics.fadeout(4)
Graphics.freeze
Graphics.transition(4)
Graphics.update
";
    let game = Game::of(
        harness(),
        Runtime::Rgss2,
        "e2e-fade-vx",
        "Fade",
        &[("Main", main)],
    );
    let names: Vec<String> = (0..10)
        .map(|index| format!("{index}={index}.png"))
        .collect();
    let dumps: Vec<&str> = names.iter().flat_map(|name| ["--dump", name]).collect();
    let ran = headless(&game, &dumps);
    ran.assert_success();

    // The frame of the update, the four of the fade from white, the four of the transition
    // from the black that was frozen to the scene at the brightness 255, and the frame after.
    let expected = [255, 191, 127, 63, 0, 62, 126, 190, 255, 255];
    for (index, level) in expected.into_iter().enumerate() {
        let frame = ran.frame(&format!("{index}.png"));
        assert_eq!((frame.width, frame.height), (544, 416));
        let first = frame.pixel(0, 0);
        assert_eq!(first[0], level, "frame {index}: {first:?}");
        assert_eq!([first[1], first[2], first[3]], [first[0], first[0], 255]);
        assert!(
            frame
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| *pixel == first),
            "frame {index} is of one colour"
        );
    }
}

/// Writes a grey picture of `width` by `height` pixels as an RGB PNG file: `level(x, y)` in
/// every channel.
fn write_grey_png(path: &Path, width: u32, height: u32, level: impl Fn(u32, u32) -> u8) {
    let file = fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    let data: Vec<u8> = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .flat_map(|(x, y)| [level(x, y); 3])
        .collect();
    writer.write_image_data(&data).unwrap();
    writer.finish().unwrap();
}

/// Scenario "Mask file of a game" of the host session's requirement that mask bitmaps reach
/// the display.
#[test]
fn a_transition_picture_of_a_game_masks_the_transition() {
    let main = "\
sprite = Sprite.new
sprite.bitmap = Bitmap.new(640, 480)
sprite.bitmap.fill_rect(0, 0, 640, 480, Color.new(255, 255, 255))
Graphics.update
Graphics.freeze
sprite.visible = false
Graphics.transition(10, 'Graphics/Transitions/halves')
Graphics.update
";
    let game = Game::new(harness(), "e2e-mask", "Mask", &[("Main", main)]);
    let transitions = game.dir().join("Graphics").join("Transitions");
    fs::create_dir_all(&transitions).unwrap();
    // Black on the left half, white on the right half.
    write_grey_png(&transitions.join("halves.png"), 640, 480, |x, _| {
        if x < 320 { 0 } else { 255 }
    });

    // Frame 0 is the white one, frames 1 to 10 are the transition to black.
    let ran = headless(&game, &["--dump", "6=inside.png", "--dump", "11=after.png"]);
    ran.assert_success();
    let stderr = ran.stderr();
    assert!(!stderr.contains("mask"), "{stderr}");

    // The newest frame comes first where the mask is dark.
    let inside = ran.frame("inside.png");
    for y in [0, 240, 479] {
        assert_eq!(inside.pixel(0, y), [0, 0, 0, 255]);
        assert_eq!(inside.pixel(319, y), [0, 0, 0, 255]);
        assert_eq!(inside.pixel(320, y), [255, 255, 255, 255]);
        assert_eq!(inside.pixel(639, y), [255, 255, 255, 255]);
    }
    let after = ran.frame("after.png");
    assert_eq!(after.pixel(0, 0), [0, 0, 0, 255]);
    assert_eq!(after.pixel(639, 479), [0, 0, 0, 255]);
}

#[test]
fn scripted_input_moves_the_sprite() {
    let ran = run_demo(&demo("e2e-input"));
    ran.assert_success();
    let first = ran.frame("first.png").leftmost(RED);
    let later = ran.frame("later.png").leftmost(RED);
    assert_eq!(first, Some(10));
    // RIGHT is held for the four frames 2 to 5, each of which moves the sprite 4 pixels.
    assert_eq!(later, Some(26));
}

#[test]
fn two_runs_write_identical_files() {
    let game = demo("e2e-repeat");
    let (one, two) = (run_demo(&game), run_demo(&game));
    one.assert_success();
    two.assert_success();
    for name in ["first.png", "later.png"] {
        let bytes = fs::read(one.file(name)).unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(bytes, fs::read(two.file(name)).unwrap(), "{name}");
    }
}

/// The family is that of the test fonts. The machine may have it installed as well; a font
/// directory comes before the system's fonts either way.
#[test]
fn text_is_drawn_with_a_font_from_a_font_directory() {
    let source = "\
text = Bitmap.new(200, 40)
text.font.name = 'Liberation Sans'
text.font.size = 32
text.font.color = Color.new(255, 255, 255)
text.draw_text(0, 0, 200, 40, 'Hello')
sprite = Sprite.new
sprite.bitmap = text
sprite.x = 100
sprite.y = 100
answers = [Font.exist?('Liberation Sans'), Font.exist?('NoSuchFamilyAnywhere')]
File.open('fonts.txt', 'w') { |file| file.write(answers.inspect) }
2.times { Graphics.update }
";
    let game = Game::new(harness(), "e2e-fonts", "Fonts", &[("Main", source)]);
    let fonts = font_dir().to_string_lossy().into_owned();
    let ran = headless(&game, &["--font-dir", &fonts, "--dump", "1=text.png"]);
    ran.assert_success();

    let frame = ran.frame("text.png");
    let ink = (100..140)
        .flat_map(|y| (100..300).map(move |x| (x, y)))
        .filter(|(x, y)| frame.pixel(*x, *y) != [0, 0, 0, 255])
        .count();
    assert!(ink > 100, "{ink} pixel(s) of ink");
    // Nothing is drawn outside the bitmap.
    assert_eq!(
        frame.leftmost([255, 255, 255, 255]).map(|x| x >= 100),
        Some(true)
    );

    let answers = fs::read_to_string(game.dir().join("fonts.txt")).expect("the script ran");
    assert_eq!(answers, "[true, false]");
}

/// The columns of `frame` that have a pixel that is not black: the first and one past the
/// last.
fn ink_columns(frame: &Frame) -> (u32, u32) {
    let inked = |x: &u32| (0..frame.height).any(|y| frame.pixel(*x, y) != [0, 0, 0, 255]);
    let first = (0..frame.width).find(inked).expect("ink");
    let last = (0..frame.width).rev().find(inked).expect("ink");
    (first, last + 1)
}

/// `Tahoma` is replaced by a font whose own line box is not the one Tahoma's replacements
/// are scaled to: `DejaVu Sans` where the machine has it, else the test font.
#[test]
fn own_font_metrics_change_the_size_of_text_in_a_replaced_font() {
    let source = "\
text = Bitmap.new(600, 80)
text.font.name = 'Tahoma'
text.font.size = 64
text.font.color = Color.new(255, 255, 255)
text.draw_text(0, 0, 600, 80, 'HHHHHHHH')
sprite = Sprite.new
sprite.bitmap = text
File.open('width.txt', 'w') { |file| file.write(text.text_size('HHHHHHHH').width.to_s) }
2.times { Graphics.update }
";
    let fonts = font_dir().to_string_lossy().into_owned();
    let run = |name: &str, arguments: &[&str]| {
        let game = Game::new(harness(), name, "Own metrics", &[("Main", source)]);
        let mut all = vec!["--font-dir", &fonts, "--dump", "1=text.png"];
        all.extend_from_slice(arguments);
        let ran = headless(&game, &all);
        ran.assert_success();
        let width = fs::read_to_string(game.dir().join("width.txt")).expect("the script ran");
        let (left, right) = ink_columns(&ran.frame("text.png"));
        (width.parse::<u32>().expect("a width"), right - left)
    };
    let (scaled_width, scaled_ink) = run("e2e-replaced-metrics", &[]);
    let (own_width, own_ink) = run("e2e-own-metrics", &["--font-own-metrics"]);
    assert!(
        own_width > scaled_width,
        "{own_width} against {scaled_width}"
    );
    assert!(own_ink > scaled_ink, "{own_ink} against {scaled_ink}");
}

#[test]
fn a_missing_built_in_scripts_file_names_the_tool_that_creates_it() {
    let game = Game::new(
        harness(),
        "e2e-builtin",
        "Builtin",
        &[("Main", "Graphics.update")],
    );
    // Without the option; the default location is in the run's own empty directory.
    let ran = vkrgss(&[&game.dir().to_string_lossy(), "--headless"]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("cargo xtask extract-rpg"), "{stderr}");
    assert!(stderr.contains("rpg_builtin.rb"), "{stderr}");
}

/// Scenario "Missing file of VX".
#[test]
fn a_missing_built_in_scripts_file_of_vx_names_the_file_and_the_document() {
    let game = Game::of(
        harness(),
        Runtime::Rgss2,
        "e2e-builtin-vx",
        "Builtin",
        &[("Main", "Graphics.update")],
    );
    let ran = vkrgss(&[&game.dir().to_string_lossy(), "--headless"]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("rpg_builtin_rgss2.rb"), "{stderr}");
    assert!(stderr.contains("docs/running.md"), "{stderr}");
}

/// The built-in scripts file of RGSS2 has a default location of its own.
#[test]
fn the_built_in_scripts_file_of_vx_is_loaded_from_its_default_location() {
    let source = "File.open('rpg.txt', 'w') { |file| file.write(RPG::Cache.class.to_s) }";
    let game = Game::of(
        harness(),
        Runtime::Rgss2,
        "e2e-default-vx",
        "Default",
        &[("Main", source)],
    );
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("vkrgss");
    fs::create_dir_all(&data).unwrap();
    fs::copy(game.builtin_scripts(), data.join("rpg_builtin_rgss2.rb")).unwrap();
    let dir = game.dir().to_string_lossy().into_owned();
    let ran = vkrgss_in(work, &[&dir, "--headless", "--device", &device()]);
    ran.assert_success();
    assert_eq!(
        fs::read_to_string(game.dir().join("rpg.txt")).unwrap(),
        "Module"
    );
}

/// The run is headless, so that this test opens neither a window nor an audio device
/// whatever the program does with the option.
#[test]
fn a_soundfont_that_does_not_exist_is_an_error_naming_it() {
    let game = Game::new(
        harness(),
        "e2e-soundfont",
        "SoundFont",
        &[("Main", "Graphics.update")],
    );
    let absent = game.dir().join("absent.sf2");
    let ran = headless(&game, &["--soundfont", &absent.to_string_lossy()]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("absent.sf2"), "{stderr}");
    assert!(stderr.contains("--soundfont"), "{stderr}");
}

#[test]
fn audio_options_do_not_disturb_a_headless_run() {
    let game = Game::new(
        harness(),
        "e2e-no-audio",
        "NoAudio",
        &[("Main", "Graphics.update")],
    );
    let ran = headless(&game, &["--no-audio"]);
    ran.assert_success();
    assert!(!ran.stderr().contains("audio"), "{}", ran.stderr());
}

#[test]
fn the_built_in_scripts_file_is_loaded_from_the_default_location() {
    let source = "File.open('rpg.txt', 'w') { |file| file.write(RPG::Cache.class.to_s) }";
    let game = Game::new(harness(), "e2e-default", "Default", &[("Main", source)]);
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("vkrgss");
    fs::create_dir_all(&data).unwrap();
    fs::copy(game.builtin_scripts(), data.join("rpg_builtin.rb")).unwrap();
    let dir = game.dir().to_string_lossy().into_owned();
    let ran = vkrgss_in(work, &[&dir, "--headless", "--device", &device()]);
    ran.assert_success();
    assert_eq!(
        fs::read_to_string(game.dir().join("rpg.txt")).unwrap(),
        "Module"
    );
}

#[test]
fn a_game_that_does_not_answer_the_quit_request_ends_with_a_failure() {
    // After its third frame the script ignores the request to quit and never asks again.
    let source = "\
begin
  3.times { Graphics.update }
rescue SystemExit
end
loop { }
";
    let game = Game::new(harness(), "e2e-stuck", "Stuck", &[("Main", source)]);
    let started = Instant::now();
    let ran = headless(&game, &["--frames", "3"]);
    let elapsed = started.elapsed();
    ran.assert_failure();
    assert!(ran.stderr().contains("did not end"), "{}", ran.stderr());
    // The wait is a few seconds; the rest is the start of the program.
    assert!(elapsed < Duration::from_secs(60), "{elapsed:?}");
}

/// A game that puts the red sprite at a `rand` position and runs for three frames.
fn rand_game(name: &str) -> Game {
    let main = format!(
        "{RED_SPRITE}$sprite.x = rand(600)\n$sprite.y = rand(440)\n3.times {{ Graphics.update }}\n"
    );
    Game::new(harness(), name, "Rand", &[("Main", &main)])
}

/// Scenario "Headless default".
#[test]
fn headless_runs_without_a_seed_draw_the_same_rand_position() {
    let game = rand_game("e2e-rand");
    let run = |arguments: &[&str]| {
        let mut all = vec!["--dump", "2=out.png"];
        all.extend_from_slice(arguments);
        let ran = headless(&game, &all);
        ran.assert_success();
        assert!(ran.frame("out.png").leftmost(RED).is_some());
        fs::read(ran.file("out.png")).unwrap()
    };
    let default = run(&[]);
    assert_eq!(default, run(&[]));
    // The default is seed 0, and another seed is another position.
    assert_eq!(default, run(&["--seed", "0"]));
    assert_ne!(default, run(&["--seed", "1"]));
}

/// Scenario "Round trip": a run with an input script is recorded, and the replay of the
/// recording writes the same frames.
#[test]
fn a_replayed_recording_writes_the_frames_of_the_recorded_run() {
    let game = demo("e2e-replay");
    let mut arguments = demo_arguments();
    arguments.extend(["--seed", "5", "--record", "input.txt"].map(str::to_owned));
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let recorded = headless(&game, &arguments);
    recorded.assert_success();
    assert_eq!(
        fs::read_to_string(recorded.file("input.txt")).unwrap(),
        "vkrgss-input 2 seed=5\n2x\n4x RIGHT\n6x\n"
    );

    let recording = recorded.file("input.txt").to_string_lossy().into_owned();
    let fonts = font_dir().to_string_lossy().into_owned();
    let replayed = headless(
        &game,
        &["--font-dir", &fonts, "--replay", &recording]
            .into_iter()
            .chain(["--dump", "1=first.png", "--dump", "8=later.png"])
            .collect::<Vec<_>>(),
    );
    replayed.assert_success();
    assert_eq!(replayed.frame("later.png").leftmost(RED), Some(26));
    for name in ["first.png", "later.png"] {
        let bytes = fs::read(recorded.file(name)).unwrap();
        assert_eq!(bytes, fs::read(replayed.file(name)).unwrap(), "{name}");
    }
}

/// A replay takes its seed from the recording unless one is given.
#[test]
fn a_replay_uses_the_seed_of_the_recording() {
    let game = rand_game("e2e-replay-seed");
    let seeded = headless(&game, &["--seed", "9", "--dump", "2=out.png"]);
    seeded.assert_success();
    let expected = fs::read(seeded.file("out.png")).unwrap();

    let work = tempfile::tempdir().unwrap();
    fs::write(work.path().join("input.txt"), "vkrgss-input 1 seed=9\n3x\n").unwrap();
    let replay = |work: TempDir, arguments: &[&str]| {
        let mut all = vec!["--replay", "input.txt", "--dump", "2=out.png"];
        all.extend_from_slice(arguments);
        let ran = headless_in(work, &game, &all);
        ran.assert_success();
        ran
    };
    let replayed = replay(work, &[]);
    assert_eq!(fs::read(replayed.file("out.png")).unwrap(), expected);
    let reseeded = replay(replayed.work, &["--seed", "10"]);
    assert_ne!(fs::read(reseeded.file("out.png")).unwrap(), expected);
}

/// Scenario "Record and replay together".
#[test]
fn record_and_replay_together_print_the_usage_message() {
    let ran = vkrgss(&["game", "--record", "a.txt", "--replay", "b.txt"]);
    assert_eq!(ran.output.status.code(), Some(2));
    let stderr = ran.stderr();
    assert!(stderr.contains("--record"), "{stderr}");
    assert!(stderr.contains("usage: vkrgss"), "{stderr}");
    assert!(!ran.file("a.txt").exists());
}

/// Scenario "Not a recording".
#[test]
fn a_replay_of_a_file_that_is_no_recording_fails_before_the_first_frame() {
    let game = Game::new(
        harness(),
        "e2e-not-a-recording",
        "Replay",
        &[(
            "Main",
            "File.open('ran.txt', 'w') { }\n3.times { Graphics.update }",
        )],
    );
    let work = tempfile::tempdir().unwrap();
    fs::write(work.path().join("notes.txt"), "2:RIGHT, 6:\n").unwrap();
    let ran = headless_in(
        work,
        &game,
        &["--replay", "notes.txt", "--dump", "0=out.png"],
    );
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("notes.txt"), "{stderr}");
    assert!(stderr.contains("not an input recording"), "{stderr}");
    assert!(!ran.file("out.png").exists());
    assert!(!game.dir().join("ran.txt").exists(), "no script ran");
}

#[test]
fn a_preload_script_patches_the_game() {
    let main = format!("{RED_SPRITE}$sprite.x = PATCHED_X\n3.times {{ Graphics.update }}\n");
    let game = Game::new(harness(), "e2e-preload", "Preload", &[("Main", &main)]);
    let work = tempfile::tempdir().unwrap();
    fs::write(work.path().join("patch.rb"), "PATCHED_X = 200\n").unwrap();
    let ran = headless_in(
        work,
        &game,
        &["--preload", "patch.rb", "--dump", "2=out.png"],
    );
    ran.assert_success();
    assert_eq!(ran.frame("out.png").leftmost(RED), Some(200));

    let ran = headless(&game, &["--preload", "absent.rb"]);
    ran.assert_failure();
    assert!(ran.stderr().contains("absent.rb"), "{}", ran.stderr());
}

/// Scenario "Game in an archive".
#[test]
fn a_game_whose_data_is_in_the_archive_runs_like_the_one_with_loose_files() {
    let main = format!("{RED_SPRITE}$sprite.x = 300\n3.times {{ Graphics.update }}\n");
    let game = Game::new(harness(), "e2e-archive", "Archive", &[("Main", &main)]);
    let loose = headless(&game, &["--dump", "2=out.png"]);
    loose.assert_success();

    let data = game.dir().join("Data");
    let scripts = fs::read(data.join("Scripts.rxdata")).unwrap();
    fs::write(
        game.dir().join("Game.rgssad"),
        rgssad(&[("Data\\Scripts.rxdata", &scripts)]),
    )
    .unwrap();
    fs::remove_dir_all(&data).unwrap();
    let archived = headless(&game, &["--dump", "2=out.png"]);
    archived.assert_success();
    assert_eq!(archived.frame("out.png").leftmost(RED), Some(300));
    assert_eq!(
        fs::read(loose.file("out.png")).unwrap(),
        fs::read(archived.file("out.png")).unwrap()
    );
}

/// Scenario "Game of VX in an archive".
#[test]
fn a_game_of_vx_whose_data_is_in_the_archive_runs_like_the_one_with_loose_files() {
    let main = format!("{RED_SPRITE}$sprite.x = 300\n3.times {{ Graphics.update }}\n");
    let game = Game::of(
        harness(),
        Runtime::Rgss2,
        "e2e-archive-vx",
        "Archive",
        &[("Main", &main)],
    );
    let loose = headless(&game, &["--dump", "2=out.png"]);
    loose.assert_success();

    let data = game.dir().join("Data");
    let scripts = fs::read(data.join("Scripts.rvdata")).unwrap();
    fs::write(
        game.dir().join("Game.rgss2a"),
        rgssad(&[("Data\\Scripts.rvdata", &scripts)]),
    )
    .unwrap();
    fs::remove_dir_all(&data).unwrap();
    let archived = headless(&game, &["--dump", "2=out.png"]);
    archived.assert_success();
    assert_eq!(archived.frame("out.png").leftmost(RED), Some(300));
    assert_eq!(
        fs::read(loose.file("out.png")).unwrap(),
        fs::read(archived.file("out.png")).unwrap()
    );
}

/// Requirement "Symbol numbers follow the runtime": the numbers measured on the original
/// runtime of VX, whether the interpreter is started for a thread or in a guest process.
#[test]
fn the_scripts_of_a_game_of_vx_see_the_symbol_numbers_of_rgss2() {
    let source = "zz_first = 1; zz_second = 2
        File.open('symbols.txt', 'w') { |file| file.write([
          [:zz_first.to_i, :zz_second.to_i], [:initialize.to_i, :to_s.to_i, :update.to_i]
        ].inspect) }";
    let game = Game::of(
        harness(),
        Runtime::Rgss2,
        "e2e-symbols-vx",
        "Symbols",
        &[("Main", source)],
    );
    for form in FORMS {
        let written = game.dir().join("symbols.txt");
        let _ = fs::remove_file(&written);
        let ran = headless(&game, &["--guest", form]);
        ran.assert_success();
        assert_eq!(
            fs::read_to_string(written).unwrap(),
            "[[16089, 16097], [2961, 3161, 6481]]",
            "{form}"
        );
    }
}

/// Requirement "Frame size of the run", scenarios "Dump of a game of VX" and "Dump of a
/// game of XP".
#[test]
fn the_dump_of_the_first_frame_has_the_frame_size_of_the_runtime() {
    for (runtime, name, size) in [
        (Runtime::Rgss2, "e2e-size-vx", (544, 416)),
        (Runtime::Rgss1, "e2e-size-xp", (640, 480)),
    ] {
        let game = Game::of(
            harness(),
            runtime,
            name,
            "Size",
            &[("Main", "Graphics.update")],
        );
        for form in FORMS {
            let ran = headless(&game, &["--guest", form, "--dump", "0=first.png"]);
            ran.assert_success();
            let frame = ran.frame("first.png");
            assert_eq!((frame.width, frame.height), size, "{name}, {form}");
        }
    }
}

/// Requirement "Resizing the screen", scenario "Dump after a resize".
#[test]
fn dumped_frames_of_a_game_of_vx_follow_a_resize_of_the_screen() {
    let main = "\
sprite = Sprite.new
sprite.bitmap = Bitmap.new(8, 8)
sprite.bitmap.fill_rect(0, 0, 8, 8, Color.new(255, 0, 0))
sprite.x = 600
sprite.y = 440
Graphics.update
Graphics.resize_screen(640, 480)
Graphics.update
";
    let game = Game::of(
        harness(),
        Runtime::Rgss2,
        "e2e-resize-vx",
        "Resize",
        &[("Main", main)],
    );
    for form in FORMS {
        let dumps = ["--dump", "0=before.png", "--dump", "1=after.png"];
        let ran = headless(&game, &[&["--guest", form], &dumps[..]].concat());
        ran.assert_success();

        let before = ran.frame("before.png");
        assert_eq!((before.width, before.height), (544, 416), "{form}");
        assert_eq!(before.leftmost(RED), None, "{form}");

        let after = ran.frame("after.png");
        assert_eq!((after.width, after.height), (640, 480), "{form}");
        assert_eq!(after.pixel(600, 440), RED, "{form}");
        assert_eq!(after.pixel(607, 447), RED, "{form}");
        assert_eq!(after.pixel(599, 440), [0, 0, 0, 255], "{form}");
        assert_eq!(after.pixel(608, 447), [0, 0, 0, 255], "{form}");
        assert_eq!(after.pixel(543, 415), [0, 0, 0, 255], "{form}");
    }
}

/// Requirement "Supported runtimes say nothing about their state", scenario "VX", whether
/// the guest is a thread or a process.
#[test]
fn a_run_of_vx_does_not_say_that_it_is_experimental() {
    let game = Game::of(
        harness(),
        Runtime::Rgss2,
        "e2e-vx-notice",
        "VX",
        &[("Main", "nil")],
    );
    for form in ["thread", "process"] {
        let ran = headless(&game, &["--guest", form]);
        ran.assert_success();
        let stderr = ran.stderr();
        assert!(experimental_lines(&stderr).is_empty(), "{form}: {stderr}");
    }
}

/// Requirement "Runtime of the run", scenario "Detected".
#[test]
fn a_run_of_xp_sets_the_globals_of_debug_mode_of_rgss1() {
    let source = "File.open('modes.txt', 'w') { |file| file.write([$TEST, $DEBUG].inspect) }";
    let game = Game::new(harness(), "e2e-xp-notice", "XP", &[("Main", source)]);
    let ran = headless(&game, &["--debug"]);
    ran.assert_success();
    let modes = fs::read_to_string(game.dir().join("modes.txt")).expect("the script ran");
    assert_eq!(modes, "[nil, true]");
}

/// A game whose `Library` is that of XP and whose scripts file is that of VX. Its script
/// writes the globals of test mode to `modes.txt`.
fn conflicting_game(name: &str) -> Game {
    let source = "File.open('modes.txt', 'w') { |file| file.write([$TEST, $DEBUG].inspect) }";
    let game = Game::of(
        harness(),
        Runtime::Rgss2,
        name,
        "Conflict",
        &[("Main", source)],
    );
    let ini =
        "[Game]\r\nLibrary=RGSS104E.dll\r\nScripts=Data\\Scripts.rvdata\r\nTitle=Conflict\r\n";
    fs::write(game.dir().join("Game.ini"), ini).unwrap();
    game
}

/// Requirement "Runtime of the run", scenario "Conflict".
#[test]
fn a_game_that_does_not_tell_its_runtime_is_an_error_naming_the_option() {
    let game = conflicting_game("e2e-runtime-conflict");
    let ran = headless(&game, &[]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("--runtime"), "{stderr}");
    assert!(stderr.contains("RGSS104E.dll"), "{stderr}");
    assert!(stderr.contains("Scripts.rvdata"), "{stderr}");
    assert!(!game.dir().join("modes.txt").exists());
}

/// Requirement "Runtime of the run", scenario "Option decides".
#[test]
fn the_runtime_option_decides_for_a_game_that_does_not_tell_it() {
    let game = conflicting_game("e2e-runtime-option");
    let ran = headless(&game, &["--runtime", "rgss2", "--debug"]);
    ran.assert_success();
    let stderr = ran.stderr();
    assert!(!stderr.contains("--runtime"), "{stderr}");
    let modes = fs::read_to_string(game.dir().join("modes.txt")).expect("the script ran");
    assert_eq!(modes, "[true, false]");
}

/// Requirement "Runtime of the run", scenario "Bad value".
#[test]
fn an_unknown_runtime_prints_the_usage_message() {
    let ran = vkrgss(&["--runtime", "vx"]);
    assert_eq!(ran.output.status.code(), Some(2));
    let stderr = ran.stderr();
    assert!(stderr.contains("`vx`"), "{stderr}");
    assert!(stderr.contains("usage: vkrgss"), "{stderr}");
}

#[test]
fn a_damaged_archive_is_an_error_naming_it() {
    let game = Game::new(
        harness(),
        "e2e-damaged-archive",
        "Archive",
        &[("Main", "3.times { Graphics.update }")],
    );
    let scripts = fs::read(game.dir().join("Data").join("Scripts.rxdata")).unwrap();
    let mut archive = rgssad(&[("Data\\Scripts.rxdata", &scripts)]);
    archive.truncate(archive.len() - 5);
    fs::write(game.dir().join("Game.rgssad"), archive).unwrap();
    let ran = headless(&game, &["--dump", "0=out.png"]);
    ran.assert_failure();
    assert!(ran.stderr().contains("Game.rgssad"), "{}", ran.stderr());
    assert!(!ran.file("out.png").exists());
}

/// A game that shows three frames and ends.
fn quiet_game(name: &str) -> Game {
    Game::new(
        harness(),
        name,
        "Quiet",
        &[("Main", "3.times { Graphics.update }")],
    )
}

/// Whether the error stream of a run has a line of an informational event. Such a line names
/// its level.
fn has_informational_line(stderr: &str) -> bool {
    stderr.lines().any(|line| line.starts_with("vkrgss: INFO "))
}

/// Scenario "Default level".
#[test]
fn a_run_without_verbose_writes_no_informational_line() {
    let game = quiet_game("e2e-log-default");
    let ran = headless(&game, &[]);
    ran.assert_success();
    let stderr = ran.stderr();
    assert!(!has_informational_line(&stderr), "{stderr}");
    assert!(!stderr.contains("built-in scripts"), "{stderr}");
}

/// Scenario "Level from the environment".
#[test]
fn the_level_of_the_environment_comes_before_verbose() {
    let game = quiet_game("e2e-log-environment");
    let work = tempfile::tempdir().expect("a temporary directory");
    let ran = headless_logging(work, &game, &["--verbose"], Some("error"));
    ran.assert_success();
    let stderr = ran.stderr();
    assert!(!has_informational_line(&stderr), "{stderr}");
    assert!(!stderr.contains("built-in scripts"), "{stderr}");
}

/// Scenario "Malformed filter".
#[test]
fn a_malformed_filter_ends_the_program_naming_the_variable() {
    let game = quiet_game("e2e-log-malformed");
    let work = tempfile::tempdir().expect("a temporary directory");
    let ran = headless_logging(work, &game, &[], Some("="));
    ran.assert_failure();
    assert_eq!(ran.output.status.code(), Some(2));
    let stderr = ran.stderr();
    assert!(stderr.contains("VKRGSS_LOG"), "{stderr}");
    assert!(stderr.contains("usage: vkrgss"), "{stderr}");
}

/// Scenario "Script error at every level".
#[test]
fn a_script_error_is_reported_at_every_level() {
    let source = "Graphics.update\nraise 'boom'\n";
    let game = Game::new(harness(), "e2e-log-error", "Error", &[("Main", source)]);
    let work = tempfile::tempdir().expect("a temporary directory");
    let ran = headless_logging(work, &game, &[], Some("error"));
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(
        stderr.contains("Script 'Main' line 2: RuntimeError occurred."),
        "{stderr}"
    );
    assert!(stderr.contains("boom"), "{stderr}");
}

/// Scenario "Verbose".
#[test]
fn verbose_names_the_built_in_scripts_file() {
    let game = quiet_game("e2e-log-verbose");
    let ran = headless(&game, &["--verbose"]);
    ran.assert_success();
    let stderr = ran.stderr();
    let builtin = game.builtin_scripts();
    let named = stderr.lines().any(|line| {
        line.starts_with("vkrgss: INFO ") && line.contains(&*builtin.to_string_lossy())
    });
    assert!(named, "{stderr}");
}

/// A game for the scenarios of the guest in a process: it writes the number of the process
/// its scripts run in to `pid.txt` in the game directory, then does what `rest` says.
fn pid_game(name: &str, rest: &str) -> Game {
    let main = format!(
        "File.open('pid.tmp', 'w') {{ |file| file.write($$.to_s) }}\n\
         File.rename('pid.tmp', 'pid.txt')\n\
         {rest}\n"
    );
    Game::new(harness(), name, "Process", &[("Main", &main)])
}

/// The process number a [`pid_game`] wrote, once it has.
fn guest_pid(game: &Game) -> Option<u32> {
    fs::read_to_string(game.dir().join("pid.txt"))
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// Scenario "Same frames" of the requirement "Guest in a process".
#[test]
fn a_guest_in_a_process_writes_the_frames_of_a_guest_on_a_thread() {
    let main = "\
Graphics.update
Graphics.freeze
$sprite.y = 200
Graphics.transition(4)
12.times do
  Graphics.update
  Input.update
  $sprite.x += 4 if Input.press?(Input::RIGHT)
end
exit
";
    let game = Game::new(
        harness(),
        "e2e-process-frames",
        "Frames",
        &[("Sprite", RED_SPRITE), ("Main", main)],
    );
    let names: Vec<String> = (0..17)
        .map(|index| format!("{index}={index}.png"))
        .collect();
    let mut arguments = vec!["--input", "7:RIGHT, 12:"];
    arguments.extend(names.iter().flat_map(|name| ["--dump", name.as_str()]));
    let [thread, process] = FORMS.map(|form| {
        let mut all = arguments.clone();
        all.extend(["--guest", form]);
        let ran = headless(&game, &all);
        ran.assert_success();
        ran
    });
    for index in 0..17 {
        let name = format!("{index}.png");
        let written = fs::read(thread.file(&name)).expect("a dumped frame");
        assert!(
            written == fs::read(process.file(&name)).expect("a dumped frame"),
            "frame {index} differs"
        );
    }
    // The transition and the input both show.
    let (first, last) = (thread.frame("0.png"), thread.frame("16.png"));
    assert_eq!(first.pixel(10, 20), RED);
    assert_eq!(last.pixel(10, 200), [0, 0, 0, 255]);
    assert_eq!(last.leftmost(RED), Some(10 + 4 * 5));
}

/// Scenario "Script error" of the requirement "Guest in a process".
#[test]
fn a_script_error_of_a_guest_in_a_process_is_reported_as_that_of_a_thread() {
    let source = "Graphics.update\nraise 'boom'\n";
    let game = Game::new(harness(), "e2e-process-error", "Error", &[("Main", source)]);
    let [thread, process] = FORMS.map(|form| headless(&game, &["--guest", form]));
    thread.assert_failure();
    process.assert_failure();
    assert!(process.stderr().contains("boom"), "{}", process.stderr());
    assert_eq!(process.stderr(), thread.stderr());
}

/// Scenario "Guest process dies" of the requirement "Guest in a process".
#[test]
fn a_guest_process_ended_by_a_signal_fails_the_run_naming_the_signal() {
    let game = pid_game("e2e-process-dies", "loop { Graphics.update }");
    let work = tempfile::tempdir().expect("a temporary directory");
    let child = start_headless(work.path(), &game, &["--guest", "process"]);
    let started = Instant::now();
    let pid = loop {
        if let Some(pid) = guest_pid(&game) {
            break pid;
        }
        assert!(started.elapsed() < Duration::from_secs(60), "no guest yet");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_ne!(pid, child.id(), "the guest is a process of its own");
    let killed = Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .status()
        .expect("kill starts");
    assert!(killed.success());

    let output = child.wait_with_output().expect("the program ends");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(output.status.code().is_some(), "{}", output.status);
    assert!(stderr.contains("SIGKILL"), "{stderr}");
}

/// Scenario "Frame limit" of the requirement "Guest in a process".
#[test]
fn the_frame_limit_ends_a_guest_process_too() {
    let game = pid_game("e2e-process-limit", "loop { Graphics.update }");
    let ran = headless(&game, &["--guest", "process", "--frames", "5"]);
    ran.assert_success();
    let pid = guest_pid(&game).expect("the game ran");
    assert!(
        !Path::new(&format!("/proc/{pid}")).exists(),
        "the guest process {pid} is still there"
    );
}

/// A guest process whose game never asks for the quit request is ended.
#[test]
fn a_guest_process_that_does_not_answer_the_quit_request_is_ended() {
    let source = "\
begin
  3.times { Graphics.update }
rescue SystemExit
end
loop { }";
    let game = pid_game("e2e-process-stuck", source);
    let ran = headless(&game, &["--guest", "process", "--frames", "3"]);
    ran.assert_failure();
    assert!(ran.stderr().contains("did not end"), "{}", ran.stderr());
    let pid = guest_pid(&game).expect("the game ran");
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
}

/// Scenario "Unknown value" of the requirement "Guest option".
#[test]
fn an_unknown_form_of_the_guest_prints_the_usage_message() {
    let ran = vkrgss(&["--guest", "fiber"]);
    assert_eq!(ran.output.status.code(), Some(2));
    let stderr = ran.stderr();
    assert!(stderr.contains("fiber"), "{stderr}");
    assert!(stderr.contains("is not one of thread, process"), "{stderr}");
    assert!(stderr.contains("usage: vkrgss"), "{stderr}");
}

/// Scenario "Environment" of the requirement "Guest option", and the order of the three
/// places the form is named in.
#[test]
fn the_form_of_the_guest_comes_from_the_command_line_the_environment_or_a_file() {
    // Whether the scripts of a run with this environment, user file and command line ran in
    // a process other than the program's.
    let in_a_child = |variable: Option<&str>, file: &str, arguments: &[&str]| {
        let game = pid_game("e2e-process-option", "Graphics.update");
        let work = work_with_user_config(file);
        let dir = game.dir().to_string_lossy().into_owned();
        let builtin = game.builtin_scripts().to_string_lossy().into_owned();
        let device = device();
        let mut all = vec![
            dir.as_str(),
            "--builtin-scripts",
            &builtin,
            "--headless",
            "--device",
            &device,
        ];
        all.extend_from_slice(arguments);
        let mut command = command(work.path(), &all);
        if let Some(variable) = variable {
            command.env("VKRGSS_GUEST", variable);
        }
        let child = command
            .stderr(Stdio::piped())
            .spawn()
            .expect("the program starts");
        let program = child.id();
        let output = child.wait_with_output().expect("the program ends");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{stderr}");
        guest_pid(&game).expect("the game ran") != program
    };
    assert!(!in_a_child(None, "", &[]), "the default is a thread");
    assert!(in_a_child(Some("process"), "", &[]));
    assert!(in_a_child(None, "guest = \"process\"", &[]));
    assert!(!in_a_child(Some("thread"), "guest = \"process\"", &[]));
    assert!(!in_a_child(Some("process"), "", &["--guest", "thread"]));
    assert!(in_a_child(Some("thread"), "", &["--guest", "process"]));

    // Another value of the variable is an error that names the two.
    let work = tempfile::tempdir().expect("a temporary directory");
    let output = command(work.path(), &[])
        .env("VKRGSS_GUEST", "fiber")
        .output()
        .expect("the program starts");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("VKRGSS_GUEST: `fiber` is not one of thread, process"),
        "{stderr}"
    );
}

/// Requirement "Sandbox option": the default, and the order of the three places the sandbox
/// is chosen in. Whether a guest process was confined is seen by what became of a file it
/// wrote beside the game directory.
#[test]
fn the_sandbox_comes_from_the_command_line_the_environment_or_a_file() {
    let name = "the_sandbox_comes_from_the_command_line_the_environment_or_a_file";
    let missing = landlock_missing();
    if let Some(reason) = &missing {
        eprintln!(
            "{name}: the runs that are confined are skipped, a guest process cannot confine \
             itself here. {reason}"
        );
    }
    // The environment, the user file, the command line, and whether the guest is confined.
    let cases: [(Option<&str>, &str, &[&str], bool); 9] = [
        (None, "", &[], true),
        (None, "sandbox = false", &[], false),
        (None, "sandbox = true", &[], true),
        (Some("off"), "", &[], false),
        (Some("on"), "sandbox = false", &[], true),
        (Some("off"), "sandbox = true", &[], false),
        (Some("off"), "", &["--sandbox", "on"], true),
        (Some("on"), "", &["--sandbox", "off"], false),
        (Some("on"), "sandbox = true", &["--sandbox", "off"], false),
    ];
    for (variable, file, arguments, confined) in cases {
        if confined && missing.is_some() {
            continue;
        }
        let case = format!("VKRGSS_SANDBOX {variable:?}, file `{file}`, arguments {arguments:?}");
        let outside = tempfile::tempdir().expect("a temporary directory");
        let game = outside_game("e2e-sandbox-option", outside.path());
        let work = work_with_user_config(file);
        let dir = game.dir().to_string_lossy().into_owned();
        let builtin = game.builtin_scripts().to_string_lossy().into_owned();
        let device = device();
        let mut all = vec![
            dir.as_str(),
            "--builtin-scripts",
            &builtin,
            "--headless",
            "--device",
            &device,
            "--guest",
            "process",
        ];
        all.extend_from_slice(arguments);
        let mut command = command(work.path(), &all);
        if let Some(variable) = variable {
            command.env("VKRGSS_SANDBOX", variable);
        }
        let output = command.output().expect("the program starts");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{case}: {stderr}");
        let result = fs::read_to_string(game.dir().join("result.txt")).expect("the game ran");
        let expected = if confined { "Errno::EACCES" } else { "written" };
        assert_eq!(result, expected, "{case}");
        assert_eq!(outside.path().join("out.txt").exists(), !confined, "{case}");
    }
}

// Save states: the scenarios of `specs/host-session/spec.md` and `specs/game-launch/spec.md`
// that need the processes.

/// A game that draws from a counter and from `rand`, without an end: the red sprite goes
/// right by the counter and is put at a height that `rand` gives, and one pixel of its
/// bitmap is painted in a colour that `rand` gives.
fn counter_and_rand_game(name: &str) -> Game {
    let main = "\
counter = 0
loop do
  counter += 1
  $sprite.x = counter * 3
  $sprite.y = rand(400)
  $sprite.bitmap.set_pixel(rand(32), rand(32), Color.new(0, rand(256), counter % 256))
  Graphics.update
end
";
    Game::new(
        harness(),
        name,
        "States",
        &[("Sprite", RED_SPRITE), ("Main", main)],
    )
}

/// Scenario "Back to the saved frame".
#[test]
fn a_loaded_state_goes_back_to_the_saved_frame() {
    let game = counter_and_rand_game("e2e-states-back");
    let plain_frames: Vec<u64> = (61..=90).collect();
    let plain = states_run(&game, 91, "", &plain_frames);
    let loaded_frames: Vec<u64> = (100..=130).collect();
    let loaded = states_run(&game, 131, "60:save 0, 100:load 0", &loaded_frames);
    for k in 1..=30 {
        assert!(
            dumped_bytes(&loaded, 100 + k) == dumped_bytes(&plain, 60 + k),
            "frame {} is not frame {} of the run without states",
            100 + k,
            60 + k
        );
    }
    // The run did go on to frame 100 before it went back, and its frames do differ.
    assert!(dumped_bytes(&loaded, 100) != dumped_bytes(&loaded, 101));
    assert!(dumped_bytes(&plain, 61) != dumped_bytes(&plain, 62));
    let stderr = loaded.stderr();
    assert!(stderr.contains("state saved into slot 0"), "{stderr}");
    assert!(stderr.contains("state loaded from slot 0"), "{stderr}");
}

/// Scenario "Loading twice".
#[test]
fn a_state_can_be_loaded_twice() {
    let game = counter_and_rand_game("e2e-states-twice");
    let ran = states_run(
        &game,
        151,
        "60:save 0, 100:load 0, 140:load 0",
        &[109, 110, 150],
    );
    assert!(dumped_bytes(&ran, 110) == dumped_bytes(&ran, 150));
    assert!(dumped_bytes(&ran, 109) != dumped_bytes(&ran, 110));
    let stderr = ran.stderr();
    assert_eq!(stderr.matches("state loaded from slot 0").count(), 2);
    assert_eq!(stderr.matches("state saved into slot 0").count(), 1);
}

/// Scenario "Empty slot".
#[test]
fn loading_an_empty_slot_is_reported_and_the_run_goes_on() {
    let game = counter_and_rand_game("e2e-states-empty");
    let ran = states_run(&game, 20, "5:load 3", &[19]);
    assert!(
        ran.stderr().contains("slot 3 holds no state"),
        "{}",
        ran.stderr()
    );
}

/// Scenario "Nothing left behind".
#[test]
fn no_process_of_a_run_with_states_is_left_when_it_ends() {
    let game = counter_and_rand_game("e2e-states-left");
    let schedule = "5:save 0, 8:save 1, 11:save 2, 14:save 1, 20:load 2";
    let ran = states_run(&game, 30, schedule, &[]);
    let attached = attached_processes(&ran.stderr());
    // The child, four copies for the saves, and the copy the resumed one left of itself.
    assert_eq!(attached.len(), 6, "{}", ran.stderr());
    for pid in attached {
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "the guest process {pid} is still there"
        );
    }
    // Neither is the directory of the socket.
    let left: Vec<_> = fs::read_dir(ran.work.path())
        .expect("the work directory")
        .flatten()
        .map(|entry| entry.file_name())
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

/// Scenario "Script error after a load". The game leaves a file behind after the state was
/// saved, which the game that goes on from the state finds.
#[test]
fn a_script_error_after_a_load_is_reported_and_fails_the_run() {
    let main = "\
counter = 0
loop do
  counter += 1
  File.open('mark.txt', 'w') { |file| file.write('x') } if counter == 20
  raise 'after the load' if counter == 15 && File.exist?('mark.txt')
  Graphics.update
end
";
    let game = Game::new(harness(), "e2e-states-error", "Error", &[("Main", main)]);
    let ran = headless(
        &game,
        &["--guest", "process", "--states", "10:save 4, 30:load 4"],
    );
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("after the load"), "{stderr}");
    assert!(stderr.contains("RuntimeError"), "{stderr}");
}

/// Scenario "Thread form".
#[test]
fn a_states_schedule_with_the_guest_on_a_thread_is_an_error_naming_the_option() {
    let game = counter_and_rand_game("e2e-states-thread");
    for form in [&[][..], &["--guest", "thread"]] {
        let mut arguments = vec!["--states", "5:save 0", "--frames", "8"];
        arguments.extend_from_slice(form);
        let ran = headless(&game, &arguments);
        assert_eq!(ran.output.status.code(), Some(2));
        assert!(ran.stderr().contains("--guest process"), "{}", ran.stderr());
    }
}

/// Scenario "Malformed schedule".
#[test]
fn a_malformed_states_schedule_is_an_error_naming_the_entry() {
    for entry in ["5:keep 0", "5:save 10"] {
        let ran = vkrgss(&["--headless", "--guest", "process", "--states", entry]);
        assert_eq!(ran.output.status.code(), Some(2));
        let stderr = ran.stderr();
        assert!(stderr.contains(&format!("`{entry}`")), "{stderr}");
        assert!(stderr.contains("--states"), "{stderr}");
    }
}

/// With the functions that start processes there are no states: a save is answered with
/// the failure.
#[test]
fn a_guest_that_may_start_processes_saves_no_state() {
    let game = counter_and_rand_game("e2e-states-functions");
    let work = tempfile::tempdir().expect("a temporary directory");
    let arguments = [
        "--guest",
        "process",
        "--allow-process-functions",
        "--frames",
        "12",
        "--states",
        "3:save 0, 6:load 0",
    ];
    let ran = headless_logging(work, &game, &arguments, Some("info"));
    ran.assert_success();
    let stderr = ran.stderr();
    assert!(
        stderr.contains("no state could be saved into slot 0"),
        "{stderr}"
    );
    assert!(stderr.contains("slot 0 holds no state"), "{stderr}");
}

// The confined guest process: the scenarios of the requirements "Confined guest process" and
// "Sandbox option".

/// A game that opens `out.txt` in the directory `outside` for writing and notes in
/// `result.txt` in the game directory how that went.
fn outside_game(name: &str, outside: &Path) -> Game {
    let main = format!(
        "result = begin\n\
           File.open('{}/out.txt', 'w') {{ |file| file.write('x') }}\n\
           'written'\n\
         rescue SystemCallError => error\n\
           error.class.name\n\
         end\n\
         File.open('result.txt', 'w') {{ |file| file.write(result) }}\n\
         Graphics.update\n",
        outside.display()
    );
    Game::new(harness(), name, "Outside", &[("Main", &main)])
}

/// Scenario "Write inside".
#[test]
fn a_confined_guest_writes_below_the_game_directory() {
    if skipped_without_landlock("a_confined_guest_writes_below_the_game_directory") {
        return;
    }
    let main = "\
Dir.mkdir('saves')
File.open('saves/one.tmp', 'w') { |file| file.write('saved') }
File.rename('saves/one.tmp', 'Save1.rxdata')
File.open('/dev/null', 'w') { |file| file.write('nothing') }
Graphics.update
";
    let game = Game::new(harness(), "e2e-sandbox-inside", "Inside", &[("Main", main)]);
    let work = tempfile::tempdir().expect("a temporary directory");
    let ran = headless_logging(work, &game, &["--guest", "process"], Some("info"));
    ran.assert_success();
    let saved = fs::read_to_string(game.dir().join("Save1.rxdata")).expect("the file was written");
    assert_eq!(saved, "saved");
    let stderr = ran.stderr();
    assert!(
        stderr.contains("the guest process is confined with Landlock"),
        "{stderr}"
    );
}

/// Scenario "Write outside".
#[test]
fn a_confined_guest_cannot_write_beside_the_game_directory() {
    if skipped_without_landlock("a_confined_guest_cannot_write_beside_the_game_directory") {
        return;
    }
    let outside = tempfile::tempdir().expect("a temporary directory");
    let game = outside_game("e2e-sandbox-outside", outside.path());
    let ran = headless(&game, &["--guest", "process"]);
    ran.assert_success();
    let result = fs::read_to_string(game.dir().join("result.txt")).expect("the game ran");
    assert_eq!(result, "Errno::EACCES");
    assert!(!outside.path().join("out.txt").exists());
}

/// Scenario "Off", and the two runs that are not confined whatever the option says, which
/// say so.
#[test]
fn a_guest_that_is_not_confined_writes_beside_the_game_directory() {
    let functions = "the guest process is not confined: --allow-process-functions is given";
    let thread = "the guest is a thread and is not confined";
    let not_confined: [(&[&str], Option<&str>); 5] = [
        (&["--guest", "process", "--sandbox", "off"], None),
        (
            &["--guest", "process", "--allow-process-functions"],
            Some(functions),
        ),
        (&["--guest", "thread", "--sandbox", "on"], Some(thread)),
        // The two say so with the sandbox off as well.
        (
            &[
                "--sandbox",
                "off",
                "--guest",
                "process",
                "--allow-process-functions",
            ],
            Some(functions),
        ),
        (&["--sandbox", "off", "--guest", "thread"], Some(thread)),
    ];
    for (arguments, said) in not_confined {
        let outside = tempfile::tempdir().expect("a temporary directory");
        let game = outside_game("e2e-sandbox-off", outside.path());
        let work = tempfile::tempdir().expect("a temporary directory");
        let ran = headless_logging(work, &game, arguments, Some("info"));
        ran.assert_success();
        let result = fs::read_to_string(game.dir().join("result.txt")).expect("the game ran");
        assert_eq!(result, "written", "{arguments:?}");
        assert!(outside.path().join("out.txt").is_file(), "{arguments:?}");
        let stderr = ran.stderr();
        assert!(!stderr.contains("is confined"), "{arguments:?}: {stderr}");
        for message in [functions, thread] {
            let times = usize::from(said == Some(message));
            assert_eq!(
                stderr.matches(message).count(),
                times,
                "{arguments:?}: {stderr}"
            );
        }
    }
}

/// Scenario "States" of the requirement "Confined guest process".
#[test]
fn a_confined_guest_saves_and_loads_states_as_one_that_is_not() {
    if skipped_without_landlock("a_confined_guest_saves_and_loads_states_as_one_that_is_not") {
        return;
    }
    let dumped = [39, 40, 41, 79, 80, 81, 82, 100];
    let names: Vec<String> = dumped
        .iter()
        .map(|index| format!("{index}={index}.png"))
        .collect();
    let run = |sandbox: &'static str| {
        let game = counter_and_rand_game("e2e-sandbox-states");
        let mut arguments = vec!["--guest", "process", "--sandbox", sandbox];
        arguments.extend(["--frames", "101", "--states", "40:save 0, 80:load 0"]);
        arguments.extend(names.iter().flat_map(|name| ["--dump", name.as_str()]));
        let work = tempfile::tempdir().expect("a temporary directory");
        let ran = headless_logging(work, &game, &arguments, Some("info"));
        ran.assert_success();
        let stderr = ran.stderr();
        assert!(stderr.contains("state saved into slot 0"), "{stderr}");
        assert!(stderr.contains("state loaded from slot 0"), "{stderr}");
        assert_eq!(
            stderr.contains("the guest process is confined with Landlock"),
            sandbox == "on",
            "{stderr}"
        );
        ran
    };
    let (confined, free) = (run("on"), run("off"));
    for index in dumped {
        assert!(
            dumped_bytes(&confined, index) == dumped_bytes(&free, index),
            "frame {index} differs under the restriction"
        );
    }
    // The load did go back: the frame after it is the one after the save.
    assert!(dumped_bytes(&confined, 81) == dumped_bytes(&confined, 41));
    assert!(dumped_bytes(&confined, 80) != dumped_bytes(&confined, 81));
}

/// Scenario "Unknown value" of the requirement "Sandbox option".
#[test]
fn an_unknown_value_of_the_sandbox_option_prints_the_usage_message() {
    let ran = vkrgss(&["--sandbox", "maybe"]);
    assert_eq!(ran.output.status.code(), Some(2));
    let stderr = ran.stderr();
    assert!(
        stderr.contains("--sandbox: `maybe` is not one of on, off"),
        "{stderr}"
    );
    assert!(stderr.contains("usage: vkrgss"), "{stderr}");

    let work = tempfile::tempdir().expect("a temporary directory");
    let output = command(work.path(), &[])
        .env("VKRGSS_SANDBOX", "maybe")
        .output()
        .expect("the program starts");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("VKRGSS_SANDBOX: `maybe` is not one of on, off"),
        "{stderr}"
    );
}

// The guest record: the scenarios of the requirement "Record of a guest process" that run
// the program.

/// Scenario "Configuration files are read once": the guest process reads none.
#[test]
fn a_guest_process_does_not_read_the_configuration_files_again() {
    let game = quiet_game("e2e-record-config");
    let work = work_with_user_config("seed = 1\n");
    let ran = headless_in(work, &game, &["--guest", "process", "--verbose"]);
    ran.assert_success();
    let stderr = ran.stderr();
    let read = stderr
        .lines()
        .filter(|line| line.contains("read the configuration file"))
        .count();
    assert_eq!(read, 1, "{stderr}");
}

/// Scenario "Settings of the run": a directory and a file, the file by a relative path,
/// reach a guest process as they reach a guest on a thread.
#[test]
fn a_guest_process_is_handed_the_settings_of_the_run() {
    let (game, rtp) = rtp_game("e2e-record-rtp");
    let table = format!("Standard={}", rtp.path().display());
    let ran = headless(&game, &["--guest", "process", "--rtp", &table]);
    ran.assert_success();
    let size = fs::read_to_string(game.dir().join("size.txt")).expect("the script ran");
    assert_eq!(size, "3x2");

    let main = format!("{RED_SPRITE}$sprite.x = PATCHED_X\n3.times {{ Graphics.update }}\n");
    let game = Game::new(
        harness(),
        "e2e-record-preload",
        "Preload",
        &[("Main", &main)],
    );
    let work = tempfile::tempdir().unwrap();
    fs::write(work.path().join("patch.rb"), "PATCHED_X = 200\n").unwrap();
    let arguments = [
        "--guest",
        "process",
        "--preload",
        "patch.rb",
        "--dump",
        "2=out.png",
    ];
    let ran = headless_in(work, &game, &arguments);
    ran.assert_success();
    assert_eq!(ran.frame("out.png").leftmost(RED), Some(200));
}

/// Scenario "Level of the guest process": `--verbose` reaches it without `VKRGSS_LOG`.
#[test]
fn a_guest_process_takes_the_level_of_verbose() {
    let game = quiet_game("e2e-record-verbose");
    let arguments = [
        "--guest",
        "process",
        "--allow-process-functions",
        "--verbose",
    ];
    let ran = headless(&game, &arguments);
    ran.assert_success();
    let stderr = ran.stderr();
    let said = stderr.lines().any(|line| {
        line.starts_with("vkrgss: INFO ") && line.contains("the guest process is not confined")
    });
    assert!(said, "{stderr}");
}
