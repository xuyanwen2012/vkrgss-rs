//! The built `vkrgss` executable, run headless on games of RGSS3 written here without an
//! interpreter (`support::ace`): the scenarios of `game-launch` and `rgss-runtime` that need
//! the program and its guest executable `vkrgss-guest-rgss3` together, and those of a copy of
//! the program alone in a directory or beside its guest executables.
//!
//! The program finds the guest executable beside its own file, as a player's does; cargo
//! puts the binaries of the workspace into one directory, and builds the binary of
//! `vkrgss-guest-rgss3` before any test runs when that package is selected too. Each test
//! checks first that the file is there.
//!
//! The file is empty on every target but those Ruby 1.9.2 is built for.
#![cfg(any(
    all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_os = "linux",
        target_env = "gnu"
    ),
    all(target_arch = "aarch64", target_os = "macos")
))]

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use tempfile::TempDir;
use vkrgss_binding::testing::host::{Game, RGSS3_STAND_IN};
use vkrgss_binding::testing::{self, Harness};
use vkrgss_vfs::Runtime;

mod support;

use support::ace::{AceGame, script_archive};
use support::*;

/// The harness, started as a game of RGSS1.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss1)
}

/// The command that builds and tests the program together with its guest executable.
const BOTH: &str = "cargo test -p vkrgss -p vkrgss-guest-rgss3";

/// The guest executable beside the program under test. Fails the test, before any child is
/// started, when it is not there.
fn guest_executable() -> PathBuf {
    let guest = Path::new(env!("CARGO_BIN_EXE_vkrgss")).with_file_name("vkrgss-guest-rgss3");
    assert!(
        guest.is_file(),
        "there is no guest executable {} beside the program, which runs the games of RGSS3 \
         in it: build and test both with `{BOTH}`",
        guest.display()
    );
    guest
}

/// Runs `game` headless as [`headless`] does, once the guest executable is known to be there.
fn ace(game: &impl TestGame, arguments: &[&str]) -> Ran {
    guest_executable();
    headless(game, arguments)
}

/// A copy of the program alone in a directory of its own, with the files of `beside` next
/// to it.
fn copy_of_the_program(beside: &[&Path]) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let program = dir.path().join("vkrgss");
    fs::copy(env!("CARGO_BIN_EXE_vkrgss"), &program).expect("the program is copied");
    for file in beside {
        let name = file.file_name().expect("a file name");
        fs::copy(file, dir.path().join(name)).expect("the file is copied");
    }
    (dir, program)
}

/// Runs `program`, a copy of the program, headless on `game`.
fn copy_headless(program: &Path, game: &impl TestGame, arguments: &[&str]) -> Ran {
    let work = tempfile::tempdir().expect("a temporary directory");
    let arguments = headless_arguments(game, arguments);
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    // Another test's thread can fork while the copy is still open for writing; the child
    // holds that descriptor until it executes, and the copy cannot be executed meanwhile.
    let mut tries = 0;
    let output = loop {
        match command_of(program, work.path(), &arguments).output() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy && tries < 50 => {
                tries += 1;
                thread::sleep(Duration::from_millis(20));
            }
            started => break started.expect("the program starts"),
        }
    };
    Ran { output, work }
}

/// A script that writes `value`, a Ruby expression, to `name` in the game directory.
fn writes(name: &str, value: &str) -> String {
    format!("File.open('{name}', 'w') {{ |file| file.write(({value}).to_s) }}")
}

/// A game of RGSS3 whose one script would leave `ran.txt` in the game directory.
fn ran_game(name: &str) -> AceGame {
    AceGame::new(name, "Ace", &[("Main", &writes("ran.txt", "'ran'"))])
}

/// Whether a script of `game` left `ran.txt`.
fn script_ran(game: &impl TestGame) -> bool {
    game.dir().join("ran.txt").exists()
}

/// What a script of `game` wrote to `name`.
fn written(game: &impl TestGame, name: &str) -> String {
    fs::read_to_string(game.dir().join(name))
        .unwrap_or_else(|error| panic!("{name} of {}: {error}", game.dir().display()))
}

// --- The guest executable ---------------------------------------------------------------------

/// Requirement "Executable of the guest process", scenario "Missing guest executable".
#[test]
fn a_copy_of_the_program_alone_names_the_missing_guest_executable() {
    let game = ran_game("ace-alone");
    let (dir, program) = copy_of_the_program(&[]);
    let ran = copy_headless(&program, &game, &[]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("vkrgss-guest-rgss3"), "{stderr}");
    assert!(
        stderr.contains(&dir.path().display().to_string()),
        "{stderr}"
    );
    assert!(!script_ran(&game));
}

/// Requirement "Executable of the guest process", scenario "A guest executable that ends at
/// once".
#[test]
fn a_guest_executable_that_ends_at_once_is_reported_with_its_status() {
    let game = ran_game("ace-ends-at-once");
    let stand_in = tempfile::tempdir().unwrap();
    let script = stand_in.path().join("vkrgss-guest-rgss3");
    fs::write(
        &script,
        "#!/bin/sh\necho 'a stand-in of the guest executable' >&2\nexit 3\n",
    )
    .unwrap();
    // Executable, as the file of a build is; the copy keeps the mode.
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let (_dir, program) = copy_of_the_program(&[&script]);
    let ran = copy_headless(&program, &game, &[]);
    ran.assert_failure();
    let stderr = ran.stderr();
    // The stand-in ran, and the program says how it ended.
    assert!(
        stderr.contains("a stand-in of the guest executable"),
        "{stderr}"
    );
    assert!(stderr.contains("guest process"), "{stderr}");
    assert!(stderr.contains("status 3"), "{stderr}");
    assert!(!script_ran(&game));
}

/// A game of `runtime` whose one script would leave `ran.txt` in the game directory.
fn ran_game_of(runtime: Runtime, name: &str) -> Game {
    Game::of(
        harness(),
        runtime,
        name,
        "Alone",
        &[("Main", &writes("ran.txt", "'ran'"))],
    )
}

/// Requirement "Executable of the guest process", scenario "XP and VX need their guest
/// executable".
#[test]
fn a_copy_of_the_program_alone_names_the_missing_guest_executable_of_xp_and_vx() {
    let game = ran_game_of(Runtime::Rgss1, "xp-alone-process");
    let (dir, program) = copy_of_the_program(&[]);
    let ran = copy_headless(&program, &game, &["--guest", "process"]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("vkrgss-guest-rgss12"), "{stderr}");
    assert!(
        stderr.contains(&dir.path().display().to_string()),
        "{stderr}"
    );
    assert!(!script_ran(&game));
}

/// Requirement "Executable of the guest process", scenario "XP and VX need no second file".
#[test]
fn a_copy_of_the_program_alone_runs_games_of_xp_and_vx_on_a_thread() {
    let (_dir, program) = copy_of_the_program(&[]);
    for runtime in [Runtime::Rgss1, Runtime::Rgss2] {
        let game = ran_game_of(runtime, &format!("ace-alone-{}", runtime.name()));
        let ran = copy_headless(&program, &game, &["--guest", "thread"]);
        ran.assert_success();
        assert!(script_ran(&game), "{}", runtime.name());
    }
}

/// Requirement "Release package holds the executables", scenario "The two files are
/// enough".
#[test]
fn the_program_and_the_guest_executable_alone_run_a_game_of_vx_ace() {
    let game = ran_game("ace-two-files");
    let (_dir, program) = copy_of_the_program(&[&guest_executable()]);
    copy_headless(&program, &game, &[]).assert_success();
    assert!(script_ran(&game));
}

/// Requirement "Release package holds the executables", scenario "The three files are
/// enough".
#[test]
fn the_program_and_the_two_guest_executables_alone_run_games_of_every_runtime() {
    let rgss12 = Path::new(env!("CARGO_BIN_EXE_vkrgss")).with_file_name("vkrgss-guest-rgss12");
    assert!(
        rgss12.is_file(),
        "there is no guest executable {} beside the program: build and test them with \
         `cargo test -p vkrgss -p vkrgss-guest-rgss12 -p vkrgss-guest-rgss3`",
        rgss12.display()
    );
    let (_dir, program) = copy_of_the_program(&[&rgss12, &guest_executable()]);
    for runtime in [Runtime::Rgss1, Runtime::Rgss2] {
        let game = ran_game_of(runtime, &format!("three-files-{}", runtime.name()));
        copy_headless(&program, &game, &["--guest", "process"]).assert_success();
        assert!(script_ran(&game), "{}", runtime.name());
    }
    let game = ran_game("ace-three-files");
    copy_headless(&program, &game, &["--guest", "process"]).assert_success();
    assert!(script_ran(&game));
}

// --- The form of the guest --------------------------------------------------------------------

/// Requirement "Guest option", scenario "Headless default of VX Ace".
#[test]
fn a_headless_run_of_vx_ace_has_its_guest_in_a_process() {
    let game = AceGame::new("ace-headless-default", "Ace", &[("Main", "nil")]);
    let ran = ace(&game, &["--verbose"]);
    ran.assert_success();
    assert_eq!(
        attached_processes(&ran.stderr()).len(),
        1,
        "{}",
        ran.stderr()
    );
}

/// Requirement "Guest option", scenario "Thread with VX Ace".
#[test]
fn a_thread_chosen_for_vx_ace_ends_the_run_naming_the_process() {
    let game = ran_game("ace-thread");
    let check = |ran: Ran| {
        ran.assert_failure();
        let stderr = ran.stderr();
        assert!(stderr.contains("RPG Maker VX Ace"), "{stderr}");
        assert!(stderr.contains("--guest process"), "{stderr}");
    };
    check(ace(&game, &["--guest", "thread"]));
    let work = tempfile::tempdir().unwrap();
    let arguments = headless_arguments(&game, &[]);
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let output = command(work.path(), &arguments)
        .env("VKRGSS_GUEST", "thread")
        .output()
        .expect("the program starts");
    check(Ran { output, work });
    assert!(!script_ran(&game));
}

// --- A game of VX Ace runs --------------------------------------------------------------------

/// The script of the scenario "Ace game": the version of Ruby, and a red sprite for three
/// frames.
fn red_sprite_script() -> String {
    format!(
        "{}
sprite = Sprite.new
sprite.bitmap = Bitmap.new(32, 32)
sprite.bitmap.fill_rect(0, 0, 32, 32, Color.new(255, 0, 0))
sprite.x = 10
sprite.y = 20
3.times {{ Graphics.update }}",
        writes("version.txt", "RUBY_VERSION")
    )
}

#[track_caller]
fn assert_red_sprite_run(game: &AceGame, ran: &Ran) {
    ran.assert_success();
    assert_eq!(written(game, "version.txt"), "1.9.2");
    let frame = ran.frame("frame.png");
    assert_eq!((frame.width, frame.height), (544, 416));
    assert_eq!(frame.pixel(10, 20), RED);
    assert_ne!(frame.pixel(9, 20), RED);
}

/// Requirement "A game of VX Ace runs", scenario "Ace game".
#[test]
fn a_game_of_vx_ace_runs_over_ruby_1_9_2() {
    let game = AceGame::new("ace-game", "Ace", &[("Main", &red_sprite_script())]);
    assert_red_sprite_run(&game, &ace(&game, &["--dump", "2=frame.png"]));
}

/// Requirement "A game of VX Ace runs", scenario "Library under the other name".
#[test]
fn a_game_of_vx_ace_with_the_library_under_the_other_name_runs_the_same_way() {
    let game = AceGame::new("ace-300", "Ace", &[("Main", &red_sprite_script())]);
    let ini = "[Game]\r\nRTP=\r\nLibrary=System\\RGSS300.dll\r\n\
               Scripts=Data\\Scripts.rvdata2\r\nTitle=Ace\r\n";
    fs::write(game.dir().join("Game.ini"), ini).unwrap();
    assert_red_sprite_run(&game, &ace(&game, &["--dump", "2=frame.png"]));
}

/// The bytes of a file that stands for the DLL of a game: other bytes, then the version
/// record of `version` (`VS_FIXEDFILEINFO`, found by its signature), then more bytes.
fn library_file(version: [u16; 4]) -> Vec<u8> {
    let mut bytes = b"MZ".to_vec();
    bytes.extend_from_slice(&[0x11; 500]);
    for word in [
        0xFEEF_04BD_u32,
        0x0001_0000,
        u32::from(version[0]) << 16 | u32::from(version[1]),
        u32::from(version[2]) << 16 | u32::from(version[3]),
    ] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes.extend_from_slice(&[0x22; 100]);
    bytes
}

/// What `RGSS_VERSION` a game of RGSS3 reports whose `Library=System\RGSS300.dll` is a file
/// of the version `version`.
fn reported_version(name: &str, version: [u16; 4]) -> String {
    let game = AceGame::new(
        name,
        "Ace",
        &[("Main", &writes("version.txt", "RGSS_VERSION"))],
    );
    let ini = "[Game]\r\nRTP=\r\nLibrary=System\\RGSS300.dll\r\n\
               Scripts=Data\\Scripts.rvdata2\r\nTitle=Ace\r\n";
    fs::write(game.dir().join("Game.ini"), ini).unwrap();
    fs::create_dir_all(game.dir().join("System")).unwrap();
    fs::write(game.dir().join("System/RGSS300.dll"), library_file(version)).unwrap();
    ace(&game, &[]).assert_success();
    written(&game, "version.txt")
}

/// Requirement "Version of RGSS3 by the library of the game", scenario "Through both
/// executables": the DLL 3.0.0.1.
#[test]
fn a_game_of_vx_ace_with_the_dll_3_0_0_1_reports_3_0_0() {
    assert_eq!(reported_version("ace-dll-3001", [3, 0, 0, 1]), "3.0.0");
}

/// Requirement "Version of RGSS3 by the library of the game", scenario "Through both
/// executables": the DLL 3.0.1.1 under the name `RGSS300.dll`.
#[test]
fn a_game_of_vx_ace_with_the_dll_3_0_1_1_reports_3_0_1() {
    assert_eq!(reported_version("ace-dll-3011", [3, 0, 1, 1]), "3.0.1");
}

/// Requirement "A game of VX Ace runs", scenario "Runtime named by the option".
#[test]
fn a_game_run_as_one_of_vx_ace_runs_over_ruby_1_9_2() {
    let game = Game::new(
        harness(),
        "ace-named",
        "Named",
        &[("Main", &writes("version.txt", "RUBY_VERSION"))],
    );
    fs::write(game.builtin_scripts(), RGSS3_STAND_IN).unwrap();
    ace(&game, &["--runtime", "rgss3"]).assert_success();
    assert_eq!(written(&game, "version.txt"), "1.9.2");
}

/// Requirement "A game of VX Ace runs", scenario "Run-time package not given", and the run
/// with the package given.
#[test]
fn a_game_of_vx_ace_needs_the_run_time_package_it_names() {
    let game = ran_game("ace-rtp");
    let ini = "[Game]\r\nRTP=RPGVXAce\r\nLibrary=System\\RGSS301.dll\r\n\
               Scripts=Data\\Scripts.rvdata2\r\nTitle=Ace\r\n";
    fs::write(game.dir().join("Game.ini"), ini).unwrap();
    let ran = ace(&game, &[]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("RPGVXAce"), "{stderr}");
    assert!(stderr.contains("--rtp"), "{stderr}");
    assert!(!script_ran(&game));

    let rtp = tempfile::tempdir().unwrap();
    let table = format!("RPGVXAce={}", rtp.path().display());
    ace(&game, &["--rtp", &table]).assert_success();
    assert!(script_ran(&game));
}

/// Requirement "A game of VX Ace runs", scenario "No script archive".
#[test]
fn a_game_of_vx_ace_without_a_script_archive_is_an_error_naming_it() {
    let game = ran_game("ace-no-scripts");
    fs::remove_dir_all(game.dir().join("Data")).unwrap();
    let check = |ran: Ran| {
        ran.assert_failure();
        assert!(
            ran.stderr().contains("Data\\Scripts.rvdata2"),
            "{}",
            ran.stderr()
        );
    };
    check(ace(&game, &[]));
    // Nor does an archive that holds something else supply it.
    let archive = rgss3a(&[("Data\\Actors.rvdata2", b"actors")]);
    fs::write(game.dir().join("Game.rgss3a"), archive).unwrap();
    check(ace(&game, &[]));
}

/// Requirement "A game of VX Ace runs", scenario "Script error".
#[test]
fn a_script_error_of_vx_ace_is_reported_in_the_form_of_rgss3() {
    let game = AceGame::new(
        "ace-script-error",
        "Ace",
        &[("Main", "nil"), ("Scene_Title", "x = 1\nraise 'boom'\n")],
    );
    let ran = ace(&game, &[]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(
        stderr.contains("Script 'Scene_Title' line 2: RuntimeError occurred."),
        "{stderr}"
    );
    assert!(stderr.contains("boom"), "{stderr}");
}

/// Requirement "Magic comments of the scripts of RGSS3", scenario "Unknown names"
/// (`ace-v1-api-REPORT.md` section 1.1).
#[test]
fn an_unknown_magic_comment_ends_a_game_of_vx_ace_with_status_1() {
    for name in ["euc-jp", "binary", "nosuch"] {
        let source = format!("# coding: {name}\n{}", writes("ran.txt", "'ran'"));
        let game = AceGame::new(&format!("ace-magic-{name}"), "Ace", &[("Main", &source)]);
        let ran = ace(&game, &[]);
        assert_eq!(
            ran.output.status.code(),
            Some(1),
            "{name}: {}",
            ran.stderr()
        );
        let stderr = ran.stderr();
        let report = format!("ArgumentError occurred.\n\nunknown encoding name: {name}");
        assert!(stderr.contains(&report), "{name}: {stderr}");
        assert!(!script_ran(&game), "{name}");
    }
}

/// Requirement "Long reports and the status after an error in RGSS3", scenarios "A long
/// message" and "Status whatever the line": vkrgss's rules, where the original shows an empty
/// box above 1023 UTF-16 units and ends with a code that follows the line in most cases.
#[test]
fn every_uncaught_error_of_vx_ace_ends_with_status_1_and_its_whole_report() {
    let long = format!(
        "Script 's' line 1: RuntimeError occurred.\n\n{}\n",
        "x".repeat(2000)
    );
    for (index, source) in [
        "a = 1\nb = 2\nraise 'third line'",
        "raise Errno::ENOENT, 'Data/Gone.rvdata2'",
        "e = RuntimeError.new('adrift')\ne.set_backtrace([])\nraise e",
        "raise RuntimeError, 'x' * 2000",
    ]
    .into_iter()
    .enumerate()
    {
        let game = AceGame::new(&format!("ace-status-{index}"), "Ace", &[("s", source)]);
        let ran = ace(&game, &[]);
        let stderr = ran.stderr();
        assert_eq!(ran.output.status.code(), Some(1), "{source}: {stderr}");
        if index == 3 {
            assert!(stderr.contains(&long), "{stderr}");
        }
    }
}

/// Requirement "A script archive of RGSS3 that cannot be loaded", scenarios "Through the
/// program" and "Empty archive" (`ace-v1-api-REPORT.md` section 1.1).
#[test]
fn an_archive_of_vx_ace_that_cannot_be_loaded_shows_failed_to_load_script() {
    let game = ran_game("ace-unloadable");
    fs::write(game.archive(), b"not a dump").unwrap();
    let ran = ace(&game, &[]);
    assert_eq!(ran.output.status.code(), Some(1), "{}", ran.stderr());
    let stderr = ran.stderr();
    assert!(
        stderr.lines().any(|line| line == "Failed to load script."),
        "{stderr}"
    );
    assert!(
        stderr
            .lines()
            .any(|line| line.contains("Data\\Scripts.rvdata2") && line.contains("archive")),
        "{stderr}"
    );
    assert!(!script_ran(&game));

    // An empty Array: nothing runs, and nothing failed.
    fs::write(game.archive(), b"\x04\x08[\x00").unwrap();
    let ran = ace(&game, &[]);
    assert_eq!(ran.output.status.code(), Some(0), "{}", ran.stderr());
    assert!(
        !ran.stderr().contains("Failed to load script."),
        "{}",
        ran.stderr()
    );
}

/// Requirement "at_exit and exit! in RGSS3", scenario "Never run" (`ace-v1-api-REPORT.md`
/// section 1.6).
#[test]
fn an_at_exit_block_of_vx_ace_never_runs() {
    let register = "at_exit { File.open('at_exit.txt', 'w') { } }";
    for (index, ending) in ["nil", "exit(3)", "raise 'ended'", "exit!(0)"]
        .into_iter()
        .enumerate()
    {
        let game = AceGame::new(
            &format!("ace-at-exit-{index}"),
            "Ace",
            &[("Register", register), ("Ends", ending)],
        );
        ace(&game, &[]);
        assert!(!game.dir().join("at_exit.txt").exists(), "{ending}");
    }
}

/// Requirement "at_exit and exit! in RGSS3", scenario "exit!" (`ace-v1-api-REPORT.md`
/// section 1.6): the program's status says success or failure.
#[test]
fn exit_bang_ends_a_game_of_vx_ace_at_once() {
    for (index, (ending, status)) in [
        (
            "begin; exit!(0); ensure; File.open('ensure.txt', 'w') { }; end",
            0,
        ),
        ("exit!(4)", 1),
        ("exit!(true)", 0),
        ("exit!", 1),
        ("Process.exit!(0)", 0),
    ]
    .into_iter()
    .enumerate()
    {
        let game = AceGame::new(
            &format!("ace-exit-bang-{index}"),
            "Ace",
            &[("Ends", ending), ("Later", &writes("ran.txt", "'ran'"))],
        );
        let ran = ace(&game, &[]);
        assert_eq!(
            ran.output.status.code(),
            Some(status),
            "{ending}: {}",
            ran.stderr()
        );
        assert!(!game.dir().join("ensure.txt").exists(), "{ending}");
        assert!(!script_ran(&game), "{ending}");
    }
}

/// Requirement "A game of VX Ace runs", scenario "Frame limit and input".
#[test]
fn the_frame_limit_and_the_input_script_drive_a_game_of_vx_ace() {
    let source = format!(
        "{}
sprite = Sprite.new
sprite.bitmap = Bitmap.new(32, 32)
sprite.bitmap.fill_rect(0, 0, 32, 32, Color.new(255, 0, 0))
loop do
  Graphics.update
  Input.update
  sprite.x += 4 if Input.press?(Input::RIGHT)
end",
        writes("pid.txt", "$$")
    );
    let game = AceGame::new("ace-frames", "Ace", &[("Main", &source)]);
    let ran = ace(
        &game,
        &[
            "--frames",
            "12",
            "--input",
            "2:RIGHT, 6:",
            "--dump",
            "1=first.png",
            "--dump",
            "8=later.png",
        ],
    );
    ran.assert_success();
    let first = ran.frame("first.png").leftmost(RED).expect("the sprite");
    let later = ran.frame("later.png").leftmost(RED).expect("the sprite");
    assert!(later > first, "{first} then {later}");
    let pid = written(&game, "pid.txt");
    assert!(
        !process_exists(&pid),
        "the guest process {pid} is still there"
    );
}

/// Requirement "A game of VX Ace runs", scenario "Two runs".
#[test]
fn two_runs_of_vx_ace_with_one_seed_write_the_same_picture() {
    let source = "sprite = Sprite.new
sprite.bitmap = Bitmap.new(16, 16)
sprite.bitmap.fill_rect(0, 0, 16, 16, Color.new(255, 0, 0))
sprite.x = rand(500)
sprite.y = rand(400)
3.times { Graphics.update }";
    let game = AceGame::new("ace-two-runs", "Ace", &[("Main", source)]);
    let arguments = ["--seed", "42", "--dump", "2=frame.png"];
    let (one, two) = (ace(&game, &arguments), ace(&game, &arguments));
    one.assert_success();
    two.assert_success();
    assert_eq!(
        fs::read(one.file("frame.png")).unwrap(),
        fs::read(two.file("frame.png")).unwrap()
    );
}

// --- The built-in scripts file and the archive ------------------------------------------------

/// Requirement "Built-in scripts file", scenarios "Missing file of VX Ace", "Default
/// location of VX Ace" and "A file of VX Ace that is not Ruby".
#[test]
fn the_built_in_scripts_file_of_vx_ace_is_found_at_its_default_location() {
    guest_executable();
    let game = ran_game("ace-builtin");
    let dir = game.dir().to_string_lossy().into_owned();
    let device = device();
    let arguments = [dir.as_str(), "--headless", "--device", &device, "--verbose"];
    let ran = vkrgss(&arguments);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains("rpg_builtin_rgss3.rb"), "{stderr}");
    assert!(stderr.contains("docs/running.md"), "{stderr}");
    assert!(attached_processes(&stderr).is_empty(), "{stderr}");

    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("vkrgss");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("rpg_builtin_rgss3.rb"), RGSS3_STAND_IN).unwrap();
    let ran = vkrgss_in(work, &arguments[..4]);
    ran.assert_success();
    assert!(
        !ran.stderr().contains("built-in scripts"),
        "{}",
        ran.stderr()
    );
    assert!(script_ran(&game));

    fs::remove_file(game.dir().join("ran.txt")).unwrap();
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("vkrgss");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("rpg_builtin_rgss3.rb"), "not ) Ruby").unwrap();
    let ran = vkrgss_in(work, &arguments[..4]);
    ran.assert_failure();
    assert!(ran.stderr().contains("SyntaxError"), "{}", ran.stderr());
    assert!(!script_ran(&game));
}

/// The script of the scenario "Game of VX Ace in an archive": it reads a data file and
/// shows a red sprite.
fn archived_game(name: &str) -> AceGame {
    let source = format!(
        "{}
sprite = Sprite.new
sprite.bitmap = Bitmap.new(32, 32)
sprite.bitmap.fill_rect(0, 0, 32, 32, Color.new(255, 0, 0))
sprite.x = 10
sprite.y = 20
Graphics.update",
        writes("value.txt", "load_data('Data/Thing.rvdata2').inspect")
    );
    let game = AceGame::new(name, "Ace", &[("Main", &source)]);
    // `[1, "two"]`, its string without an encoding.
    fs::write(
        game.dir().join("Data/Thing.rvdata2"),
        b"\x04\x08[\x07i\x06\"\x08two",
    )
    .unwrap();
    game
}

/// Requirement "Encrypted archive of the game", scenario "Game of VX Ace in an archive".
#[test]
fn a_game_of_vx_ace_whose_data_is_in_the_archive_runs_like_the_one_with_loose_files() {
    let game = archived_game("ace-archive");
    let loose = ace(&game, &["--dump", "0=frame.png"]);
    loose.assert_success();
    let value = written(&game, "value.txt");
    assert_eq!(value, "[1, \"two\"]");

    let data = game.dir().join("Data");
    let archive = rgss3a(&[
        (
            "Data\\Thing.rvdata2",
            &fs::read(data.join("Thing.rvdata2")).unwrap(),
        ),
        ("Data\\Scripts.rvdata2", &fs::read(game.archive()).unwrap()),
    ]);
    fs::write(game.dir().join("Game.rgss3a"), archive).unwrap();
    fs::remove_dir_all(&data).unwrap();
    fs::remove_file(game.dir().join("value.txt")).unwrap();
    let archived = ace(&game, &["--dump", "0=frame.png"]);
    archived.assert_success();
    assert_eq!(written(&game, "value.txt"), value);
    assert_eq!(
        fs::read(archived.file("frame.png")).unwrap(),
        fs::read(loose.file("frame.png")).unwrap()
    );
}

/// Requirements "Data files of RGSS3" of `game-files` and "Source and errors of load_data
/// in RGSS3" (`ace-v1-api-REPORT.md` sections 2.1 and 2.2): an entry of the archive is found
/// without its extension and comes before a loose file of the same name, and is handed to
/// `Marshal.load` as a `String`, a loose file as a `File`.
#[test]
fn load_data_of_vx_ace_asks_the_archive_first_by_the_name_without_its_extension() {
    let source = format!(
        "class << Marshal
  alias vkrgss_marshal_load load
  def load(source, *rest)
    ($vkrgss_sources ||= []) << source.class.name
    vkrgss_marshal_load(source, *rest)
  end
end
{}",
        writes(
            "value.txt",
            "[load_data('Data/Thing'), load_data('Data\\\\Thing.rvdata2'), \
             load_data('Data/Loose.rvdata2'), $vkrgss_sources].inspect"
        )
    );
    let game = AceGame::new("ace-data-lookup", "Ace", &[("Main", &source)]);
    let data = game.dir().join("Data");
    // `"archive"`, `"loose"` and `"loose only"`, without an encoding.
    let archive = rgss3a(&[
        ("Data\\Scripts.rvdata2", &fs::read(game.archive()).unwrap()),
        ("Data\\Thing.rvdata2", b"\x04\x08\"\x0carchive"),
    ]);
    fs::write(game.dir().join("Game.rgss3a"), archive).unwrap();
    fs::remove_file(game.archive()).unwrap();
    fs::write(data.join("Thing.rvdata2"), b"\x04\x08\"\x0aloose").unwrap();
    fs::write(data.join("Loose.rvdata2"), b"\x04\x08\"\x0floose only").unwrap();
    ace(&game, &[]).assert_success();
    assert_eq!(
        written(&game, "value.txt"),
        r#"["archive", "archive", "loose only", ["String", "String", "File"]]"#
    );
}

/// Requirement "Encrypted archive of the game", scenario "Damaged archive of VX Ace".
#[test]
fn a_damaged_archive_of_vx_ace_is_an_error_naming_it() {
    let game = ran_game("ace-damaged-archive");
    let scripts = fs::read(game.archive()).unwrap();
    let mut archive = rgss3a(&[("Data\\Scripts.rvdata2", &scripts)]);
    // Inside the name of the one record of the table.
    archive.truncate(12 + 16 + 7);
    fs::write(game.dir().join("Game.rgss3a"), archive).unwrap();
    let check = |ran: Ran| {
        ran.assert_failure();
        assert!(ran.stderr().contains("Game.rgss3a"), "{}", ran.stderr());
    };
    check(ace(&game, &[]));
    // An archive of version 1 under that name is damaged too.
    let archive = rgssad(&[("Data\\Scripts.rvdata2", &scripts)]);
    fs::write(game.dir().join("Game.rgss3a"), archive).unwrap();
    check(ace(&game, &[]));
    assert!(!script_ran(&game));
}

/// Requirement "Script archive of RGSS3", scenario "Both forms of the strings".
#[test]
fn the_strings_of_the_archive_may_carry_their_encoding_or_not() {
    let source = writes("name.txt", "$RGSS_SCRIPTS[0][1].encoding.name");
    for (name, game) in [
        (
            "labelled",
            AceGame::new("ace-labelled", "Ace", &[("Main", &source)]),
        ),
        (
            "plain",
            AceGame::plain("ace-plain", "Ace", &[("Main", &source)]),
        ),
    ] {
        ace(&game, &[]).assert_success();
        assert_eq!(written(&game, "name.txt"), "UTF-8", "{name}");
    }
    // The two forms differ in their bytes.
    let scripts = [("Main", source.as_str())];
    assert_ne!(
        script_archive(&scripts, true),
        script_archive(&scripts, false)
    );
}

// --- The state of VX Ace ----------------------------------------------------------------------

/// Requirement "Supported runtimes say nothing about their state", scenario "VX Ace".
#[test]
fn a_run_of_vx_ace_does_not_say_that_it_is_experimental() {
    let game = AceGame::new("ace-supported", "Ace", &[("Main", "nil")]);
    let ran = ace(&game, &[]);
    ran.assert_success();
    let stderr = ran.stderr();
    assert!(experimental_lines(&stderr).is_empty(), "{stderr}");
}

// --- The start-up of RGSS3 --------------------------------------------------------------------

/// Requirement "Message boxes and console output of RGSS3", scenario "Console output".
#[test]
fn p_print_and_puts_write_to_the_standard_output() {
    let source = format!(
        "$r = p(\"a\", 1); print \"b\", 2; puts \"c\"\n{}",
        writes("r.txt", "$r.inspect")
    );
    let game = AceGame::new("ace-console", "Ace", &[("Main", &source)]);
    for arguments in [&[][..], &["--log-p"]] {
        let ran = ace(&game, arguments);
        ran.assert_success();
        let stdout = String::from_utf8_lossy(&ran.output.stdout);
        assert_eq!(stdout, "\"a\"\n1\nb2c\n", "{arguments:?}");
        assert_eq!(written(&game, "r.txt"), "[\"a\", 1]");
        assert!(!ran.stderr().contains("\"a\""), "{}", ran.stderr());
    }
}

/// Requirement "End of a game of RGSS3", scenario "Exit with a status".
#[test]
fn every_exit_of_a_game_of_vx_ace_ends_with_status_0() {
    for (index, ending) in ["exit", "exit(3)", "exit(false)", "abort"]
        .into_iter()
        .enumerate()
    {
        let game = AceGame::new(
            &format!("ace-exit-{index}"),
            "Ace",
            &[("Ends", ending), ("Later", &writes("ran.txt", "'ran'"))],
        );
        let ran = ace(&game, &[]);
        assert_eq!(
            ran.output.status.code(),
            Some(0),
            "{ending}: {}",
            ran.stderr()
        );
        assert!(!script_ran(&game), "{ending}");
    }
}

/// Requirement "End of a game of RGSS3", scenario "Exit inside the block of the main
/// function and inside a Fiber".
#[test]
fn an_exit_inside_the_main_function_or_a_fiber_ends_with_status_0() {
    for (name, source) in [
        ("ace-exit-main", "rgss_main { exit }"),
        ("ace-exit-fiber", "Fiber.new { exit }.resume"),
    ] {
        let game = AceGame::new(name, "Ace", &[("Main", source)]);
        let ran = ace(&game, &[]);
        assert_eq!(
            ran.output.status.code(),
            Some(0),
            "{name}: {}",
            ran.stderr()
        );
    }
}

/// Requirement "End of a game of RGSS3", scenario "A loop driven by a Fiber".
#[test]
fn a_loop_driven_by_a_fiber_moves_the_sprite() {
    let source = "sprite = Sprite.new
sprite.bitmap = Bitmap.new(32, 32)
sprite.bitmap.fill_rect(0, 0, 32, 32, Color.new(255, 0, 0))
mover = Fiber.new do
  loop do
    sprite.x += 3
    Fiber.yield
  end
end
10.times do
  mover.resume
  Graphics.update
end";
    let game = AceGame::new("ace-fiber-loop", "Ace", &[("Main", source)]);
    let ran = ace(&game, &["--dump", "2=early.png", "--dump", "9=late.png"]);
    ran.assert_success();
    let early = ran.frame("early.png").leftmost(RED).expect("the sprite");
    let late = ran.frame("late.png").leftmost(RED).expect("the sprite");
    assert!(late > early, "{early} then {late}");
}

/// Requirement "Process functions of Ruby 1.9 are off by default", scenarios "Functions of
/// Ruby 1.9" and "Open with a file name".
#[test]
fn the_functions_of_ruby_1_9_that_start_processes_are_off() {
    let calls = "[spawn('touch a'), Process.spawn('touch b'), open('|touch c'), \
                 system('touch', 'd'), `touch e`, IO.popen(['touch', 'f'])]";
    let source = format!(
        "{}\nopen('note.txt', 'w') {{ |file| file.write('x') }}",
        writes("results.txt", &format!("{calls}.inspect"))
    );
    let game = AceGame::new("ace-process-functions", "Ace", &[("Main", &source)]);
    let ran = ace(&game, &["--sandbox", "off"]);
    ran.assert_success();
    assert_eq!(
        written(&game, "results.txt"),
        "[nil, nil, nil, nil, nil, nil]"
    );
    for name in ["a", "b", "c", "d", "e", "f"] {
        assert!(!game.dir().join(name).exists(), "{name}");
    }
    let stderr = ran.stderr();
    let lines = stderr.lines().filter(|line| line.contains("is turned off"));
    assert_eq!(lines.count(), 6, "{stderr}");
    assert_eq!(written(&game, "note.txt"), "x");
}

/// Requirement "Process functions of Ruby 1.9 are off by default", scenario "Enabled by the
/// caller".
#[test]
fn the_functions_that_start_processes_work_when_enabled() {
    let source = writes(
        "results.txt",
        "[system('true'), Process.wait(spawn('true')).is_a?(Integer)].inspect",
    );
    let game = AceGame::new("ace-process-enabled", "Ace", &[("Main", &source)]);
    ace(&game, &["--allow-process-functions"]).assert_success();
    assert_eq!(written(&game, "results.txt"), "[true, true]");
}

// --- States and the sandbox -------------------------------------------------------------------

/// A game of RGSS3 that never ends: a red sprite moved by a counter of frames and placed by
/// `rand`, with a pixel of its bitmap changed in every frame.
fn counter_and_rand_game(name: &str) -> AceGame {
    let source = "sprite = Sprite.new
sprite.bitmap = Bitmap.new(32, 32)
sprite.bitmap.fill_rect(0, 0, 32, 32, Color.new(255, 0, 0))
counter = 0
loop do
  counter += 1
  sprite.x = counter * 3
  sprite.y = rand(380)
  sprite.bitmap.set_pixel(rand(32), rand(32), Color.new(0, rand(256), counter % 256))
  Graphics.update
end";
    AceGame::new(name, "States", &[("Main", source)])
}

/// Runs `game` as [`states_run`] does, with `arguments` added.
fn states_run_with(
    game: &AceGame,
    frames: u64,
    states: &str,
    dumped: &[u64],
    arguments: &[&str],
) -> Ran {
    guest_executable();
    let frames = frames.to_string();
    let names: Vec<String> = dumped
        .iter()
        .map(|index| format!("{index}={index}.png"))
        .collect();
    let mut all = vec![
        "--guest", "process", "--frames", &frames, "--states", states,
    ];
    all.extend(names.iter().flat_map(|name| ["--dump", name.as_str()]));
    all.extend_from_slice(arguments);
    let work = tempfile::tempdir().expect("a temporary directory");
    let ran = headless_logging(work, game, &all, Some("info"));
    ran.assert_success();
    ran
}

/// Requirement "States and the sandbox with a game of RGSS3", scenario "A state is loaded".
#[test]
fn a_loaded_state_of_vx_ace_goes_back_to_the_saved_frame() {
    guest_executable();
    let game = counter_and_rand_game("ace-states-back");
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
    assert!(dumped_bytes(&loaded, 100) != dumped_bytes(&loaded, 101));
    let stderr = loaded.stderr();
    assert!(stderr.contains("state saved into slot 0"), "{stderr}");
    assert!(stderr.contains("state loaded from slot 0"), "{stderr}");
}

/// Requirement "States and the sandbox with a game of RGSS3", scenario "With and without the
/// sandbox".
#[test]
fn a_guest_of_vx_ace_saves_and_loads_states_confined_or_not() {
    let game = counter_and_rand_game("ace-states-sandbox");
    let frames: Vec<u64> = (95..=110).collect();
    let runs: Vec<Ran> = ["on", "off"]
        .into_iter()
        .map(|sandbox| {
            states_run_with(
                &game,
                111,
                "60:save 0, 100:load 0",
                &frames,
                &["--sandbox", sandbox],
            )
        })
        .collect();
    for index in &frames {
        assert!(
            dumped_bytes(&runs[0], *index) == dumped_bytes(&runs[1], *index),
            "frame {index}"
        );
    }
    for ran in &runs {
        let attached = attached_processes(&ran.stderr());
        assert!(attached.len() >= 2, "{}", ran.stderr());
        for pid in attached {
            assert!(!process_exists(pid), "the process {pid} is still there");
        }
    }
}

/// The lines of the guest that say that the state of slot 0 is not saved because of a
/// thread.
fn thread_lines(stderr: &str) -> Vec<&str> {
    stderr
        .lines()
        .filter(|line| line.contains("slot 0") && line.contains("thread"))
        .collect()
}

/// Requirement "States and the sandbox with a game of RGSS3", scenario "A script thread is
/// alive".
#[test]
fn a_state_is_not_saved_while_a_script_thread_is_alive() {
    let game = AceGame::new(
        "ace-states-thread",
        "Ace",
        &[
            ("Thread", "$t = Thread.new { sleep }"),
            ("Main", "loop { Graphics.update }"),
        ],
    );
    let ran = states_run_with(&game, 20, "5:save 0, 10:load 0", &[], &[]);
    let stderr = ran.stderr();
    assert_eq!(thread_lines(&stderr).len(), 1, "{stderr}");
    assert!(stderr.contains("slot 0 holds no state"), "{stderr}");
}

/// Requirement "States and the sandbox with a game of RGSS3", scenario "A script thread has
/// ended".
#[test]
fn a_state_is_saved_once_the_script_thread_has_ended() {
    let game = AceGame::new(
        "ace-states-thread-ended",
        "Ace",
        &[
            ("Thread", "Thread.new { 1 + 1 }.value"),
            ("Main", "loop { Graphics.update }"),
        ],
    );
    let ran = states_run_with(&game, 20, "5:save 0, 10:load 0", &[], &[]);
    let stderr = ran.stderr();
    assert!(thread_lines(&stderr).is_empty(), "{stderr}");
    assert!(stderr.contains("state saved into slot 0"), "{stderr}");
    assert!(stderr.contains("state loaded from slot 0"), "{stderr}");
}

/// A game of RGSS3 that opens `out.txt` in the directory `outside` and `in.txt` in its own
/// directory for writing, from `context` (`%s` stands for the code), and notes in
/// `result.txt` how that went.
fn confined_game(name: &str, outside: &Path, context: &str) -> AceGame {
    let attempt = format!(
        "[File.join({:?}, 'out.txt'), 'in.txt'].map do |path|
  begin
    File.open(path, 'w') {{ |file| file.write('x') }}
    'written'
  rescue SystemCallError => error
    error.class.name
  end
end",
        outside.display().to_string()
    );
    let source = format!(
        "{}\nGraphics.update",
        writes("result.txt", &context.replace("%s", &attempt))
    );
    AceGame::new(name, "Confined", &[("Main", &source)])
}

/// Requirement "States and the sandbox with a game of RGSS3", scenarios "Write outside" and
/// "A script thread is confined too".
#[test]
fn a_confined_guest_of_vx_ace_writes_below_its_game_directory_only() {
    if skipped_without_landlock("a_confined_guest_of_vx_ace_writes_below_its_game_directory_only") {
        return;
    }
    for (name, context) in [
        ("ace-confined", "(%s).inspect"),
        ("ace-confined-thread", "Thread.new { %s }.value.inspect"),
    ] {
        let outside = tempfile::tempdir().unwrap();
        let game = confined_game(name, outside.path(), context);
        ace(&game, &[]).assert_success();
        assert_eq!(
            written(&game, "result.txt"),
            "[\"Errno::EACCES\", \"written\"]",
            "{name}"
        );
        assert!(!outside.path().join("out.txt").exists(), "{name}");
        assert_eq!(written(&game, "in.txt"), "x", "{name}");
    }
}

// --- Win32API ---------------------------------------------------------------------------------

/// The script of a game that makes `Win32API` objects while it loads, as the games of the
/// corpus do: of `wininet` and of `user32` with Array and spaced declarations, and of a
/// library that is a file of the game; it calls some of them and writes their values.
const WIN32API_AT_LOAD: &str = "
open = Win32API.new('wininet', 'InternetOpenA', ['p', 'l', 'p', 'p', 'l'], 'l')
key = Win32API.new('user32', 'GetKeyState', ['i'], 'i')
metrics = Win32API.new('user32', 'GetSystemMetrics', 'i', 'i')
blur = Win32API.new('tktk_bitmap', 'Blur', 'n i i i i', 'i')
error = Win32API.new('Kernel32', 'GetLastError', [], 'l')
Graphics.update
values = [open.call('vkrgss', 0, nil, nil, 0), key.call(0x09), metrics.call(5),
          blur.call(Bitmap.new(1, 1).object_id, 1, 2, 3, 4), error.call]
File.open('win32api.txt', 'w') { |file| file.write(values.inspect) }
";

/// Requirement "Libraries of Win32API in RGSS3" and "Declarations and count of Win32API in
/// RGSS3": a game that calls `Win32API` while it loads runs to its end through both
/// executables.
#[test]
fn a_game_of_vx_ace_that_calls_win32api_at_load_runs_to_its_end() {
    let game = AceGame::new("ace-win32api", "Ace", &[("Main", WIN32API_AT_LOAD)]);
    fs::write(game.dir().join("tktk_bitmap.dll"), b"").unwrap();
    let ran = ace(&game, &[]);
    assert_eq!(ran.output.status.code(), Some(0), "{}", ran.stderr());
    assert_eq!(written(&game, "win32api.txt"), "[0, 0, 0, 0, 0]");
    let stderr = ran.stderr();
    for pair in [
        "wininet InternetOpenA",
        "tktk_bitmap Blur",
        "Kernel32 GetLastError",
    ] {
        assert!(
            stderr.contains(&format!("vkrgss: Win32API {pair} is not emulated")),
            "{stderr}"
        );
    }
}
