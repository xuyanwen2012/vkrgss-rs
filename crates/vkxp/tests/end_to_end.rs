//! The built `vkxp` executable, run headless on games assembled here: the scenarios of
//! `game-launch` that need a whole program.
//!
//! Each test makes a game directory with the binding's test support (a `Game.ini`, a script
//! archive and, outside the directory, a stand-in built-in scripts file written for tests)
//! and runs the executable on it as a child process, without a window, on the CPU device.
//!
//! Like the other tests that reach Vulkan, these refuse to run unless the loader has been
//! restricted to chosen drivers, and they hand the restriction on to the child.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use tempfile::TempDir;
use vkxp_binding::testing::host::Game;

/// Environment variables that restrict which drivers the Vulkan loader loads. Naming a
/// device does not do that: the loader loads every installed driver to enumerate the devices.
const ISOLATION_VARIABLES: [&str; 3] = [
    "VK_LOADER_DRIVERS_SELECT",
    "VK_DRIVER_FILES",
    "VK_ICD_FILENAMES",
];

/// The loader restriction of this process, for the child. Fails the test, before any child
/// is started, when there is none.
fn isolation() -> Vec<(&'static str, String)> {
    let restriction: Vec<_> = ISOLATION_VARIABLES
        .iter()
        .filter_map(|name| Some((*name, std::env::var(name).ok()?)))
        .filter(|(_, value)| !value.trim().is_empty())
        .collect();
    assert!(
        !restriction.is_empty(),
        "the Vulkan loader is not restricted to chosen drivers, so running the program would \
         load the driver of every GPU in the machine. Set VK_LOADER_DRIVERS_SELECT before \
         running these tests: '*lvp*' for the CPU device (lavapipe), which they use by \
         default, or the pattern of the driver that VKXP_TEST_DEVICE names."
    );
    restriction
}

/// The device the child composites on: llvmpipe, unless `VKXP_TEST_DEVICE` names another.
fn device() -> String {
    match std::env::var("VKXP_TEST_DEVICE") {
        Ok(value) if !value.is_empty() => value,
        _ => "llvmpipe".to_owned(),
    }
}

/// The test fonts of `vkxp-text`.
fn font_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../vkxp-text/tests/fonts")
}

/// How a run of the program went.
struct Ran {
    output: Output,
    /// The directory the program was started in, where relative dump paths end up. It also
    /// stands in for the user's data directory.
    work: TempDir,
}

impl Ran {
    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }

    #[track_caller]
    fn assert_success(&self) {
        assert!(
            self.output.status.success(),
            "{}: {}",
            self.output.status,
            self.stderr()
        );
    }

    #[track_caller]
    fn assert_failure(&self) {
        assert!(!self.output.status.success(), "the run succeeded");
        assert!(
            self.output.status.code().is_some(),
            "{}",
            self.output.status
        );
    }

    fn file(&self, name: &str) -> PathBuf {
        self.work.path().join(name)
    }

    fn frame(&self, name: &str) -> Frame {
        Frame::read(&self.file(name))
    }
}

/// Runs the executable with these arguments in a fresh directory, with the loader restricted
/// as this process is and no built-in scripts file at the default location.
fn vkxp(arguments: &[&str]) -> Ran {
    vkxp_in(
        tempfile::tempdir().expect("a temporary directory"),
        arguments,
    )
}

/// Runs the executable in `work`, which is also where it looks for the user's data and
/// configuration.
fn vkxp_in(work: TempDir, arguments: &[&str]) -> Ran {
    vkxp_logging(work, arguments, None)
}

/// Runs the executable as [`vkxp_in`] does, with the environment variable `VKXP_LOG` set to
/// `log` or, without one, not set.
fn vkxp_logging(work: TempDir, arguments: &[&str], log: Option<&str>) -> Ran {
    let restriction = isolation();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vkxp"));
    command
        .args(arguments)
        .current_dir(work.path())
        .env("XDG_DATA_HOME", work.path())
        .env("XDG_CONFIG_HOME", work.path())
        .env_remove("VKXP_LOG");
    if let Some(log) = log {
        command.env("VKXP_LOG", log);
    }
    for (name, value) in restriction {
        command.env(name, value);
    }
    let output = command.output().expect("the program starts");
    Ran { output, work }
}

/// Runs `game` headless with its stand-in built-in scripts file, followed by `arguments`.
fn headless(game: &Game, arguments: &[&str]) -> Ran {
    headless_in(
        tempfile::tempdir().expect("a temporary directory"),
        game,
        arguments,
    )
}

/// Runs `game` headless as [`headless`] does, started in `work`.
fn headless_in(work: TempDir, game: &Game, arguments: &[&str]) -> Ran {
    headless_logging(work, game, arguments, None)
}

/// Runs `game` headless as [`headless`] does, with `VKXP_LOG` set to `log`.
fn headless_logging(work: TempDir, game: &Game, arguments: &[&str], log: Option<&str>) -> Ran {
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
    vkxp_logging(work, &all, log)
}

/// A dumped frame.
struct Frame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Frame {
    fn read(path: &Path) -> Frame {
        let file =
            fs::File::open(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let mut reader = png::Decoder::new(std::io::BufReader::new(file))
            .read_info()
            .expect("a PNG file");
        let info = reader.info();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        assert_eq!(info.bit_depth, png::BitDepth::Eight);
        let (width, height) = (info.width, info.height);
        let mut rgba = vec![0; width as usize * height as usize * 4];
        reader.next_frame(&mut rgba).expect("PNG data");
        Frame {
            width,
            height,
            rgba,
        }
    }

    fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let at = (y as usize * self.width as usize + x as usize) * 4;
        self.rgba[at..at + 4].try_into().expect("four bytes")
    }

    /// The leftmost column that has a pixel of this colour.
    fn leftmost(&self, colour: [u8; 4]) -> Option<u32> {
        (0..self.width).find(|x| (0..self.height).any(|y| self.pixel(*x, y) == colour))
    }
}

const RED: [u8; 4] = [255, 0, 0, 255];

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
$window.contents.draw_text(0, 0, 224, 64, 'Hello, vkxp')
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
    let ran = vkxp(&[&empty.path().to_string_lossy(), "--headless"]);
    ran.assert_failure();
    assert!(ran.stderr().contains("Game.ini"), "{}", ran.stderr());
}

#[test]
fn a_file_with_a_relative_name_appears_in_the_game_directory() {
    let source = "File.open('note.txt', 'w') { |file| file.write($DEBUG.inspect) }";
    let game = Game::new("e2e-relative", "Relative", &[("Main", source)]);
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
             vkxp: Script 'Main' line 2: RuntimeError occurred.\n\
             vkxp: \n\
             vkxp: boom\n",
            "{log:?}"
        );
    }
}

#[test]
fn the_frame_limit_ends_a_game_that_never_ends_with_status_0() {
    let game = Game::new(
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

/// Scenario "Option accepted" of `specs/game-launch/spec.md`.
#[test]
fn frame_statistics_of_a_headless_run_are_written_to_the_error_stream() {
    let game = Game::new(
        "e2e-frame-stats",
        "Stats",
        &[("Main", "loop { Graphics.update }")],
    );
    let ran = headless(&game, &["--frames", "10", "--frame-stats"]);
    ran.assert_success();
    let stderr = ran.stderr();
    assert!(
        stderr.contains("vkxp: frame times over 10 frame(s):"),
        "{stderr}"
    );
    for row in [
        "guest: 9 frame(s)",
        "encode: 10 frame(s)",
        "submit: 10 frame(s)",
    ] {
        assert!(stderr.contains(row), "no `{row}` in: {stderr}");
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
        stderr.contains("vkxp: frame times over 10 frame(s):"),
        "{stderr}"
    );

    // Without the option there is no summary.
    let ran = headless(&game, &["--frames", "10"]);
    ran.assert_success();
    assert!(!ran.stderr().contains("frame times"), "{}", ran.stderr());
}

/// The number of frames a row of the frame time statistics counts, if `stderr` has the row.
fn row_count(stderr: &str, name: &str) -> Option<usize> {
    let row = format!("vkxp:   {name}: ");
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
    let game = Game::new("e2e-guest-stats", "Split", &[("Main", source)]);
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
    // that follow a tick.
    let measured = run(&["--frame-stats"]);
    let stderr = measured.stderr();
    assert!(stderr.contains("vkxp: guest frame times over "), "{stderr}");
    let counts = ROWS.map(|name| row_count(&stderr, name));
    let frames = counts[0].unwrap_or_else(|| panic!("no row of the scripts in: {stderr}"));
    assert!((49..=50).contains(&frames), "{stderr}");
    assert_eq!(counts, [Some(frames); 4], "{stderr}");

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
    let game = Game::new(name, "Package", &[("Main", source)]);
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

/// A fresh directory for a run with `text` as the user's configuration file at its default
/// location below it.
fn work_with_user_config(text: &str) -> TempDir {
    let work = tempfile::tempdir().unwrap();
    fs::create_dir(work.path().join("vkxp")).unwrap();
    fs::write(work.path().join("vkxp").join("config.toml"), text).unwrap();
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
    fs::write(game.dir().join("vkxp.toml"), "seed = 2").unwrap();
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
    let recorded = vkxp_in(work_with_user_config(&text), &arguments);
    recorded.assert_success();
    // Paths in the file are relative to the file.
    let config_dir = recorded.file("vkxp");
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
    let replayed = vkxp_in(work_with_user_config(&text), &arguments);
    replayed.assert_success();
    assert_eq!(
        fs::read(replayed.file("vkxp").join("later.png")).unwrap(),
        fs::read(by_options.file("later.png")).unwrap()
    );

    // An input script in a file that the user file names, and frames from the command line.
    let work = work_with_user_config(&format!("{common}input = '@keys.txt'\n"));
    fs::write(work.path().join("vkxp").join("keys.txt"), "2:RIGHT\n6:\n").unwrap();
    let ran = vkxp_in(work, &[arguments.as_slice(), &["--frames", "12"]].concat());
    ran.assert_success();
    assert_eq!(
        fs::read(ran.file("vkxp").join("later.png")).unwrap(),
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
        let ran = vkxp_in(work_with_user_config(text), &arguments);
        assert_eq!(ran.output.status.code(), Some(2), "{text}");
        let stderr = ran.stderr();
        assert!(stderr.contains(named), "{text}: {stderr}");
        assert!(stderr.contains("usage: vkxp"), "{text}: {stderr}");
    }
}

#[test]
fn an_unknown_filter_prints_the_usage_message() {
    let ran = vkxp(&["--filter", "cubic"]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("cubic"), "{stderr}");
    assert!(stderr.contains("usage: vkxp"), "{stderr}");
}

#[test]
fn an_unknown_option_prints_the_usage_message() {
    let ran = vkxp(&["--no-such-option"]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("--no-such-option"), "{stderr}");
    assert!(stderr.contains("usage: vkxp"), "{stderr}");
}

#[test]
fn a_malformed_input_script_file_prints_the_usage_message() {
    let game = Game::new("e2e-bad-input", "Input", &[("Main", "Graphics.update")]);
    let keys = tempfile::tempdir().unwrap();
    let file = keys.path().join("bad.txt");
    fs::write(&file, "2:SIDEWAYS").unwrap();
    let ran = headless(&game, &["--input", &format!("@{}", file.display())]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("SIDEWAYS"), "{stderr}");
    assert!(stderr.contains("usage: vkxp"), "{stderr}");
}

#[test]
fn a_dumped_frame_shows_the_red_sprite_where_it_is() {
    let main = format!("{RED_SPRITE}loop {{ Graphics.update }}\n");
    let game = Game::new("e2e-dump", "Dump", &[("Main", &main)]);
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
    let game = Game::new("e2e-transition", "Transition", &[("Main", main)]);
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
    let game = Game::new("e2e-mask", "Mask", &[("Main", main)]);
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
    let game = Game::new("e2e-fonts", "Fonts", &[("Main", source)]);
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
        let game = Game::new(name, "Own metrics", &[("Main", source)]);
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
    let game = Game::new("e2e-builtin", "Builtin", &[("Main", "Graphics.update")]);
    // Without the option; the default location is in the run's own empty directory.
    let ran = vkxp(&[&game.dir().to_string_lossy(), "--headless"]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("cargo xtask extract-rpg"), "{stderr}");
    assert!(stderr.contains("rpg_builtin.rb"), "{stderr}");
}

/// The run is headless, so that this test opens neither a window nor an audio device
/// whatever the program does with the option.
#[test]
fn a_soundfont_that_does_not_exist_is_an_error_naming_it() {
    let game = Game::new("e2e-soundfont", "SoundFont", &[("Main", "Graphics.update")]);
    let absent = game.dir().join("absent.sf2");
    let ran = headless(&game, &["--soundfont", &absent.to_string_lossy()]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("absent.sf2"), "{stderr}");
    assert!(stderr.contains("--soundfont"), "{stderr}");
}

#[test]
fn audio_options_do_not_disturb_a_headless_run() {
    let game = Game::new("e2e-no-audio", "NoAudio", &[("Main", "Graphics.update")]);
    let ran = headless(&game, &["--no-audio"]);
    ran.assert_success();
    assert!(!ran.stderr().contains("audio"), "{}", ran.stderr());
}

#[test]
fn the_built_in_scripts_file_is_loaded_from_the_default_location() {
    let source = "File.open('rpg.txt', 'w') { |file| file.write(RPG::Cache.class.to_s) }";
    let game = Game::new("e2e-default", "Default", &[("Main", source)]);
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("vkxp");
    fs::create_dir_all(&data).unwrap();
    fs::copy(game.builtin_scripts(), data.join("rpg_builtin.rb")).unwrap();
    let dir = game.dir().to_string_lossy().into_owned();
    let ran = vkxp_in(work, &[&dir, "--headless", "--device", &device()]);
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
    let game = Game::new("e2e-stuck", "Stuck", &[("Main", source)]);
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
    Game::new(name, "Rand", &[("Main", &main)])
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
        "vkxp-input 2 seed=5\n2x\n4x RIGHT\n6x\n"
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
    fs::write(work.path().join("input.txt"), "vkxp-input 1 seed=9\n3x\n").unwrap();
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
    let ran = vkxp(&["game", "--record", "a.txt", "--replay", "b.txt"]);
    assert_eq!(ran.output.status.code(), Some(2));
    let stderr = ran.stderr();
    assert!(stderr.contains("--record"), "{stderr}");
    assert!(stderr.contains("usage: vkxp"), "{stderr}");
    assert!(!ran.file("a.txt").exists());
}

/// Scenario "Not a recording".
#[test]
fn a_replay_of_a_file_that_is_no_recording_fails_before_the_first_frame() {
    let game = Game::new(
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
    let game = Game::new("e2e-preload", "Preload", &[("Main", &main)]);
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

/// An RGSSAD version 1 archive of these entries, in the format `vkxp-vfs` documents: after
/// the header, per entry, the length of the name, the name, the size of the data and the
/// data, XORed with a key that advances after every length, name byte and size, the data
/// word by word with a copy of the key.
fn rgssad(entries: &[(&str, &[u8])]) -> Vec<u8> {
    fn step(key: &mut u32) -> u32 {
        let current = *key;
        *key = key.wrapping_mul(7).wrapping_add(3);
        current
    }
    let mut key: u32 = 0xDEAD_CAFE;
    let mut archive = b"RGSSAD\0\x01".to_vec();
    for (name, contents) in entries {
        let length = u32::try_from(name.len()).unwrap();
        archive.extend((length ^ step(&mut key)).to_le_bytes());
        archive.extend(name.bytes().map(|byte| byte ^ step(&mut key) as u8));
        let size = u32::try_from(contents.len()).unwrap();
        archive.extend((size ^ step(&mut key)).to_le_bytes());
        let mut data_key = key;
        for word in contents.chunks(4) {
            let mask = step(&mut data_key).to_le_bytes();
            archive.extend(word.iter().zip(mask).map(|(byte, mask)| byte ^ mask));
        }
    }
    archive
}

/// Scenario "Game in an archive".
#[test]
fn a_game_whose_data_is_in_the_archive_runs_like_the_one_with_loose_files() {
    let main = format!("{RED_SPRITE}$sprite.x = 300\n3.times {{ Graphics.update }}\n");
    let game = Game::new("e2e-archive", "Archive", &[("Main", &main)]);
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

#[test]
fn a_damaged_archive_is_an_error_naming_it() {
    let game = Game::new(
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
    Game::new(name, "Quiet", &[("Main", "3.times { Graphics.update }")])
}

/// Whether the error stream of a run has a line of an informational event. Such a line names
/// its level.
fn has_informational_line(stderr: &str) -> bool {
    stderr.lines().any(|line| line.starts_with("vkxp: INFO "))
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
    assert!(stderr.contains("VKXP_LOG"), "{stderr}");
    assert!(stderr.contains("usage: vkxp"), "{stderr}");
}

/// Scenario "Script error at every level".
#[test]
fn a_script_error_is_reported_at_every_level() {
    let source = "Graphics.update\nraise 'boom'\n";
    let game = Game::new("e2e-log-error", "Error", &[("Main", source)]);
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
    let named = stderr
        .lines()
        .any(|line| line.starts_with("vkxp: INFO ") && line.contains(&*builtin.to_string_lossy()));
    assert!(named, "{stderr}");
}
