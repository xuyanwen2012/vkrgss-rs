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

/// Runs the executable in `work`, which is also where it looks for the user's data.
fn vkxp_in(work: TempDir, arguments: &[&str]) -> Ran {
    let restriction = isolation();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vkxp"));
    command
        .args(arguments)
        .current_dir(work.path())
        .env("XDG_DATA_HOME", work.path());
    for (name, value) in restriction {
        command.env(name, value);
    }
    let output = command.output().expect("the program starts");
    Ran { output, work }
}

/// Runs `game` headless with its stand-in built-in scripts file, followed by `arguments`.
fn headless(game: &Game, arguments: &[&str]) -> Ran {
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
    vkxp(&all)
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
    let ran = headless(&game, &[]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(
        stderr.contains("Script 'Main' line 2: RuntimeError occurred."),
        "{stderr}"
    );
    assert!(stderr.contains("boom"), "{stderr}");
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
