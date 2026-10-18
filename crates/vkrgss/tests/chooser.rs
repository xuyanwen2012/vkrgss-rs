//! The app on macOS started without a game directory: a copy of the built `vkrgss` in an app
//! bundle made here, with a stand-in for `osascript` first in `PATH` that answers as the
//! folder chooser would and records its arguments. Nothing here reaches Vulkan.

#![cfg(target_os = "macos")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

/// A bundle with a copy of the program, a home directory and a stand-in for `osascript`.
struct App {
    dir: TempDir,
}

impl App {
    /// The app, with an `osascript` that runs the shell code `answer` after recording its
    /// arguments.
    fn new(answer: &str) -> App {
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("vkrgss.app/Contents/MacOS/vkrgss");
        fs::create_dir_all(program.parent().unwrap()).unwrap();
        fs::copy(env!("CARGO_BIN_EXE_vkrgss"), &program).unwrap();
        let app = App { dir };
        fs::create_dir_all(app.bin()).unwrap();
        fs::create_dir_all(app.home()).unwrap();
        let osascript = app.bin().join("osascript");
        let calls = app.calls();
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n{answer}\n",
            calls.display()
        );
        fs::write(&osascript, script).unwrap();
        fs::set_permissions(&osascript, fs::Permissions::from_mode(0o755)).unwrap();
        app
    }

    fn bin(&self) -> PathBuf {
        self.dir.path().join("bin")
    }

    fn home(&self) -> PathBuf {
        self.dir.path().join("home")
    }

    /// The arguments of the last call of `osascript`, one per line.
    fn calls(&self) -> PathBuf {
        self.dir.path().join("calls")
    }

    /// Starts the app without arguments and with no environment but `PATH` and `HOME`, as a
    /// double-click does.
    fn start(&self) -> Output {
        let path = format!("{}:/usr/bin:/bin", self.bin().display());
        Command::new(self.dir.path().join("vkrgss.app/Contents/MacOS/vkrgss"))
            .current_dir("/")
            .env_clear()
            .env("PATH", path)
            .env("HOME", self.home())
            .output()
            .expect("the program starts")
    }

    /// The file that keeps the directory chosen last.
    fn last(&self) -> PathBuf {
        self.home()
            .join("Library/Application Support/vkrgss/last-game-dir")
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Scenario "No game directory".
#[test]
fn a_cancelled_chooser_ends_the_app_quietly_with_status_0() {
    let app = App::new("echo '0:409: execution error: User canceled. (-128)' >&2\nexit 1");
    let output = app.start();
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert!(app.calls().is_file(), "the chooser was not asked");
    assert!(!app.last().exists());
}

#[test]
fn another_failure_of_the_chooser_is_an_error() {
    let app = App::new("echo 'execution error: No user interaction allowed. (-1713)' >&2\nexit 1");
    let output = app.start();
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("(-1713)"), "{}", stderr(&output));
}

#[test]
fn the_directory_chosen_is_run_and_offered_the_next_time() {
    let games = tempfile::tempdir().unwrap();
    let game = games.path().join("No Game Here");
    fs::create_dir(&game).unwrap();
    let app = App::new(&format!("echo '{}/'", game.display()));

    // The directory chosen is the game directory of the run; this one has no Game.ini.
    let output = app.start();
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("Game.ini"), "{}", stderr(&output));
    let calls = fs::read_to_string(app.calls()).unwrap();
    assert!(
        calls.ends_with("end run\n"),
        "no default location was offered: {calls}"
    );
    let kept = PathBuf::from(fs::read_to_string(app.last()).unwrap());
    assert_eq!(kept, Path::new(&format!("{}/", game.display())));

    app.start();
    let calls = fs::read_to_string(app.calls()).unwrap();
    assert!(
        calls.ends_with(&format!("end run\n{}/\n", game.display())),
        "{calls}"
    );
}
