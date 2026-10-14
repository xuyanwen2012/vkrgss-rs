//! The built `vkrgss` executable, run headless with the files, environment variables and
//! recordings of versions 0.1.0 to 0.3.0, when the program was named vkxp: the scenarios of
//! the requirements "Files under the former name" and "Environment variables under the former
//! name" of `game-launch` and "Recordings under the former name" of `host-session`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tempfile::TempDir;
use vkrgss_binding::testing::host::Game;

mod support;

#[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
use support::ace::AceGame;
use support::*;

/// A game that draws a red sprite at a random place and moves it right while RIGHT is held,
/// for eight frames: its frames tell the seed and the input of a run.
fn rand_game(name: &str) -> Game {
    let main = "\
$sprite = Sprite.new
$sprite.bitmap = Bitmap.new(32, 32)
$sprite.bitmap.fill_rect(0, 0, 32, 32, Color.new(255, 0, 0))
$sprite.x = rand(400)
$sprite.y = rand(400)
8.times do
  Input.update
  $sprite.x += 4 if Input.press?(Input::RIGHT)
  Graphics.update
end
";
    Game::new(name, "Rand", &[("Main", main)])
}

/// The bytes of the frame that a run of a [`rand_game`] dumped as `out.png`.
fn dumped(ran: &Ran) -> Vec<u8> {
    fs::read(ran.file("out.png")).expect("the frame was dumped")
}

/// The lines of the error stream that name `text`.
fn naming<'a>(stderr: &'a str, text: &str) -> Vec<&'a str> {
    stderr.lines().filter(|line| line.contains(text)).collect()
}

/// The lines of a diagnostic of the program on the error stream of a run at the default
/// level, where only warnings and errors are written.
fn diagnostics(stderr: &str) -> Vec<&str> {
    stderr
        .lines()
        .filter(|line| line.starts_with("vkrgss: "))
        .collect()
}

/// Asserts that the error stream of a run at the default level has exactly one diagnostic,
/// and that it names `former` and `new`.
#[track_caller]
fn assert_one_warning(stderr: &str, former: &str, new: &str) {
    let lines = diagnostics(stderr);
    assert_eq!(lines.len(), 1, "{stderr}");
    assert!(lines[0].contains(former), "{stderr}");
    assert!(lines[0].contains(new), "{stderr}");
}

/// The frame of a run of `game` with the seed `seed` and no configuration file.
fn seeded(game: &Game, seed: &str) -> Vec<u8> {
    let ran = headless(
        game,
        &["--no-config", "--seed", seed, "--dump", "2=out.png"],
    );
    ran.assert_success();
    dumped(&ran)
}

/// A fresh directory for a run, with `text` as the user's configuration file at
/// `<dir>/config.toml` below it.
fn work_with_user_file(dir: &str, text: &str) -> TempDir {
    let work = tempfile::tempdir().expect("a temporary directory");
    fs::create_dir_all(work.path().join(dir)).unwrap();
    fs::write(work.path().join(dir).join("config.toml"), text).unwrap();
    work
}

/// Scenario "User file of version 0.3.0".
#[test]
fn a_user_file_at_its_former_place_is_read_with_a_warning() {
    let game = rand_game("former-user-file");
    let work = work_with_user_file("vkxp", "seed = 1\n");
    let former = work.path().join("vkxp").join("config.toml");
    let new = work.path().join("vkrgss").join("config.toml");
    let ran = headless_in(work, &game, &["--dump", "2=out.png"]);
    ran.assert_success();
    assert_eq!(dumped(&ran), seeded(&game, "1"));
    assert_ne!(dumped(&ran), seeded(&game, "2"));
    assert_one_warning(
        &ran.stderr(),
        &former.to_string_lossy(),
        &new.to_string_lossy(),
    );
}

/// Scenario "Both user files".
#[test]
fn a_user_file_at_its_new_place_is_read_without_a_warning() {
    let game = rand_game("former-both-user-files");
    let work = work_with_user_file("vkxp", "seed = 1\n");
    fs::create_dir(work.path().join("vkrgss")).unwrap();
    fs::write(work.path().join("vkrgss").join("config.toml"), "seed = 2\n").unwrap();
    let former = work.path().join("vkxp").join("config.toml");
    let ran = headless_in(work, &game, &["--dump", "2=out.png"]);
    ran.assert_success();
    assert_eq!(dumped(&ran), seeded(&game, "2"));
    let stderr = ran.stderr();
    assert!(
        naming(&stderr, &former.to_string_lossy()).is_empty(),
        "{stderr}"
    );
}

/// Scenario "Game file of version 0.3.0".
#[test]
fn a_game_file_under_its_former_name_is_read_with_a_warning() {
    let game = rand_game("former-game-file");
    fs::write(game.dir().join("vkxp.toml"), "seed = 2\n").unwrap();
    let ran = headless(&game, &["--dump", "2=out.png"]);
    ran.assert_success();
    assert_eq!(dumped(&ran), seeded(&game, "2"));
    let former = game.dir().join("vkxp.toml");
    let new = game.dir().join("vkrgss.toml");
    assert_one_warning(
        &ran.stderr(),
        &former.to_string_lossy(),
        &new.to_string_lossy(),
    );
}

/// Scenario "Named file".
#[test]
fn a_named_user_file_is_read_as_named() {
    let game = rand_game("former-named-file");
    let work = work_with_user_file("vkxp", "seed = 1\n");
    let named = work.path().join("named.toml");
    fs::write(&named, "seed = 2\n").unwrap();
    let named = named.to_string_lossy().into_owned();
    let ran = headless_in(work, &game, &["--config", &named, "--dump", "2=out.png"]);
    ran.assert_success();
    assert_eq!(dumped(&ran), seeded(&game, "2"));
    let stderr = ran.stderr();
    assert!(diagnostics(&stderr).is_empty(), "{stderr}");
}

/// A headless run of the game in `dir` without `--builtin-scripts`, in `work`, which stands
/// in for the user's data directory.
fn without_builtin_option(work: TempDir, dir: &Path, arguments: &[&str]) -> Ran {
    let dir = dir.to_string_lossy().into_owned();
    let device = device();
    let mut all = vec![dir.as_str(), "--headless", "--device", &device];
    all.extend_from_slice(arguments);
    vkrgss_in(work, &all)
}

/// A fresh directory for a run, with `builtin` copied to `vkxp/<name>` below it, the place
/// of the default built-in scripts file of version 0.3.0. Returns the former and the new path
/// of the file as well.
fn work_with_former_builtin(builtin: &Path, name: &str) -> (TempDir, PathBuf, PathBuf) {
    let work = tempfile::tempdir().expect("a temporary directory");
    fs::create_dir(work.path().join("vkxp")).unwrap();
    let former = work.path().join("vkxp").join(name);
    fs::copy(builtin, &former).unwrap();
    let new = work.path().join("vkrgss").join(name);
    (work, former, new)
}

/// Scenario "Built-in scripts of version 0.3.0", for a game of RGSS1 with the guest on a
/// thread and in a process.
#[test]
fn a_built_in_scripts_file_at_its_former_place_is_read_with_a_warning() {
    let game = Game::new(
        "former-builtin",
        "Builtin",
        &[(
            "Main",
            "File.open('ran.txt', 'w') { |file| file.write('ran') }",
        )],
    );
    for guest in ["thread", "process"] {
        fs::remove_file(game.dir().join("ran.txt")).ok();
        let (work, former, new) =
            work_with_former_builtin(&game.builtin_scripts(), "rpg_builtin.rb");
        let ran = without_builtin_option(work, game.dir(), &["--guest", guest]);
        ran.assert_success();
        assert!(game.dir().join("ran.txt").is_file(), "{guest}");
        assert_one_warning(
            &ran.stderr(),
            &former.to_string_lossy(),
            &new.to_string_lossy(),
        );
    }
}

/// Scenario "Built-in scripts of version 0.3.0", for a game of RGSS3, whose guest is always a
/// process of the guest executable.
#[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
#[test]
fn a_built_in_scripts_file_of_rgss3_at_its_former_place_is_read_with_a_warning() {
    let guest = Path::new(env!("CARGO_BIN_EXE_vkrgss")).with_file_name("vkrgss-guest-rgss3");
    assert!(
        guest.is_file(),
        "there is no guest executable {} beside the program: build and test both with \
         `cargo test -p vkrgss -p vkrgss-guest-rgss3`",
        guest.display()
    );
    let game = AceGame::new(
        "former-builtin-rgss3",
        "Ace",
        &[(
            "Main",
            "File.open('ran.txt', 'w') { |file| file.write('ran') }",
        )],
    );
    let (work, former, new) =
        work_with_former_builtin(&TestGame::builtin_scripts(&game), "rpg_builtin_rgss3.rb");
    let ran = without_builtin_option(work, game.dir(), &[]);
    ran.assert_success();
    assert!(game.dir().join("ran.txt").is_file());
    assert_one_warning(
        &ran.stderr(),
        &former.to_string_lossy(),
        &new.to_string_lossy(),
    );
}

/// Scenario "No built-in scripts file".
#[test]
fn a_missing_built_in_scripts_file_is_named_at_its_new_place() {
    let game = Game::new(
        "former-builtin-missing",
        "Builtin",
        &[("Main", "Graphics.update")],
    );
    let work = tempfile::tempdir().expect("a temporary directory");
    let new = work.path().join("vkrgss").join("rpg_builtin.rb");
    let former = work.path().join("vkxp").join("rpg_builtin.rb");
    let ran = without_builtin_option(work, game.dir(), &[]);
    ran.assert_failure();
    let stderr = ran.stderr();
    assert!(stderr.contains(&*new.to_string_lossy()), "{stderr}");
    assert!(stderr.contains("cargo xtask extract-rpg"), "{stderr}");
    assert!(!stderr.contains(&*former.to_string_lossy()), "{stderr}");
}

/// Scenario "Level under the former name".
#[test]
fn the_level_is_read_under_its_former_name_with_a_warning() {
    let game = rand_game("former-log");
    let work = tempfile::tempdir().expect("a temporary directory");
    let arguments = headless_arguments(&game, &[]);
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let output = command(work.path(), &arguments)
        .env("VKXP_LOG", "info")
        .output()
        .expect("the program starts");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(
        stderr.lines().any(|line| line.starts_with("vkrgss: INFO ")),
        "{stderr}"
    );
    let warnings: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("vkrgss: WARN "))
        .collect();
    assert_eq!(warnings.len(), 1, "{stderr}");
    assert!(warnings[0].contains("VKXP_LOG"), "{stderr}");
    assert!(warnings[0].contains("VKRGSS_LOG"), "{stderr}");
}

/// A game that writes the number of the process its scripts run in to `pid.txt`.
fn pid_game(name: &str) -> Game {
    let main = "File.open('pid.txt', 'w') { |file| file.write($$.to_s) }\nGraphics.update\n";
    Game::new(name, "Process", &[("Main", main)])
}

/// Runs a [`pid_game`] headless with the environment variables `variables` set, and returns
/// whether its scripts ran in a process other than the program's, and the error stream.
fn in_a_child(name: &str, variables: &[(&str, &str)]) -> (bool, String) {
    let game = pid_game(name);
    let work = tempfile::tempdir().expect("a temporary directory");
    let arguments = headless_arguments(&game, &[]);
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let mut command = command(work.path(), &arguments);
    command.envs(variables.iter().copied());
    let child = command
        .stderr(Stdio::piped())
        .spawn()
        .expect("the program starts");
    let program = child.id();
    let output = child.wait_with_output().expect("the program ends");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "{stderr}");
    let pid: u32 = fs::read_to_string(game.dir().join("pid.txt"))
        .expect("the game ran")
        .trim()
        .parse()
        .expect("a process number");
    (pid != program, stderr)
}

/// Scenario "Guest under the former name".
#[test]
fn the_guest_is_read_under_its_former_name_with_a_warning() {
    let (child, stderr) = in_a_child("former-guest", &[("VKXP_GUEST", "process")]);
    assert!(child, "{stderr}");
    assert_one_warning(&stderr, "VKXP_GUEST", "VKRGSS_GUEST");
}

/// Scenario "Both names".
#[test]
fn the_new_name_of_the_guest_wins_without_a_warning() {
    let (child, stderr) = in_a_child(
        "former-guest-both",
        &[("VKRGSS_GUEST", "thread"), ("VKXP_GUEST", "process")],
    );
    assert!(!child, "{stderr}");
    assert!(naming(&stderr, "VKXP_GUEST").is_empty(), "{stderr}");
}

/// Scenario "Sandbox under the former name": the game of "Write outside" writes its file
/// beside the game directory.
#[test]
fn the_sandbox_is_read_under_its_former_name_with_a_warning() {
    let outside = tempfile::tempdir().expect("a temporary directory");
    let main = format!(
        "File.open('{}/out.txt', 'w') {{ |file| file.write('x') }}\nGraphics.update\n",
        outside.path().display()
    );
    let game = Game::new("former-sandbox", "Outside", &[("Main", &main)]);
    let work = tempfile::tempdir().expect("a temporary directory");
    let arguments = headless_arguments(&game, &["--guest", "process"]);
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let output = command(work.path(), &arguments)
        .env("VKXP_SANDBOX", "off")
        .output()
        .expect("the program starts");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(outside.path().join("out.txt").is_file(), "{stderr}");
    assert_one_warning(&stderr, "VKXP_SANDBOX", "VKRGSS_SANDBOX");
}

/// Scenarios "Written" and "Recording of version 0.3.0".
#[test]
fn a_recording_under_the_former_name_replays_without_a_warning() {
    let game = rand_game("former-recording");
    let recorded = headless(
        &game,
        &[
            "--seed",
            "7",
            "--input",
            "2:RIGHT, 5:",
            "--record",
            "input.txt",
        ],
    );
    recorded.assert_success();
    let text = fs::read_to_string(recorded.file("input.txt")).unwrap();
    assert_eq!(text.lines().next(), Some("vkrgss-input 2 seed=7"));

    let replay = |text: &str| {
        let work = tempfile::tempdir().expect("a temporary directory");
        fs::write(work.path().join("input.txt"), text).unwrap();
        let ran = headless_in(
            work,
            &game,
            &[
                "--replay",
                "input.txt",
                "--dump",
                "3=early.png",
                "--dump",
                "7=out.png",
            ],
        );
        ran.assert_success();
        ran
    };
    let written = replay(&text);
    let former = replay(&text.replacen("vkrgss-input", "vkxp-input", 1));
    for name in ["early.png", "out.png"] {
        let bytes = fs::read(written.file(name)).unwrap();
        assert_eq!(bytes, fs::read(former.file(name)).unwrap(), "{name}");
    }
    let stderr = former.stderr();
    assert!(diagnostics(&stderr).is_empty(), "{stderr}");
}
