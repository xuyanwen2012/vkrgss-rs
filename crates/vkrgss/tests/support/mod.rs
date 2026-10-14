//! What the test files of the program share: the loader restriction handed to the child, the
//! runs of the built `vkrgss` executable, dumped frames, archives assembled here, and the games
//! of RGSS3 that `ace` writes without an interpreter.
//!
//! Like the other tests that reach Vulkan, these refuse to run unless the loader has been
//! restricted to chosen drivers, and they hand the restriction on to the child.

// Each test file uses a part of this module.
#![allow(dead_code)]

pub(crate) mod ace;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

use tempfile::TempDir;
use vkrgss_binding::testing::host::Game;

/// A game directory made for a test, with a built-in scripts file outside it.
pub(crate) trait TestGame {
    /// The game directory.
    fn dir(&self) -> &Path;
    /// The built-in scripts file of the game.
    fn builtin_scripts(&self) -> PathBuf;
}

impl TestGame for Game {
    fn dir(&self) -> &Path {
        Game::dir(self)
    }

    fn builtin_scripts(&self) -> PathBuf {
        Game::builtin_scripts(self)
    }
}

/// Environment variables that restrict which drivers the Vulkan loader loads. Naming a
/// device does not do that: the loader loads every installed driver to enumerate the devices.
pub(crate) const ISOLATION_VARIABLES: [&str; 3] = [
    "VK_LOADER_DRIVERS_SELECT",
    "VK_DRIVER_FILES",
    "VK_ICD_FILENAMES",
];

/// The loader restriction of this process, for the child. Fails the test, before any child
/// is started, when there is none.
pub(crate) fn isolation() -> Vec<(&'static str, String)> {
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
         default, or the pattern of the driver that VKRGSS_TEST_DEVICE names."
    );
    restriction
}

/// The device the child composites on: llvmpipe, unless `VKRGSS_TEST_DEVICE` names another.
pub(crate) fn device() -> String {
    match std::env::var("VKRGSS_TEST_DEVICE") {
        Ok(value) if !value.is_empty() => value,
        _ => "llvmpipe".to_owned(),
    }
}

/// The test fonts of `vkrgss-text`.
pub(crate) fn font_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../vkrgss-text/tests/fonts")
}

/// How a run of the program went.
pub(crate) struct Ran {
    pub(crate) output: Output,
    /// The directory the program was started in, where relative dump paths end up. It also
    /// stands in for the user's data directory.
    pub(crate) work: TempDir,
}

impl Ran {
    pub(crate) fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }

    #[track_caller]
    pub(crate) fn assert_success(&self) {
        assert!(
            self.output.status.success(),
            "{}: {}",
            self.output.status,
            self.stderr()
        );
    }

    #[track_caller]
    pub(crate) fn assert_failure(&self) {
        assert!(!self.output.status.success(), "the run succeeded");
        assert!(
            self.output.status.code().is_some(),
            "{}",
            self.output.status
        );
    }

    pub(crate) fn file(&self, name: &str) -> PathBuf {
        self.work.path().join(name)
    }

    pub(crate) fn frame(&self, name: &str) -> Frame {
        Frame::read(&self.file(name))
    }
}

/// Runs the executable with these arguments in a fresh directory, with the loader restricted
/// as this process is and no built-in scripts file at the default location.
pub(crate) fn vkrgss(arguments: &[&str]) -> Ran {
    vkrgss_in(
        tempfile::tempdir().expect("a temporary directory"),
        arguments,
    )
}

/// Runs the executable in `work`, which is also where it looks for the user's data and
/// configuration.
pub(crate) fn vkrgss_in(work: TempDir, arguments: &[&str]) -> Ran {
    vkrgss_logging(work, arguments, None)
}

/// Runs the executable as [`vkrgss_in`] does, with the environment variable `VKRGSS_LOG` set to
/// `log` or, without one, not set.
pub(crate) fn vkrgss_logging(work: TempDir, arguments: &[&str], log: Option<&str>) -> Ran {
    let mut command = command(work.path(), arguments);
    if let Some(log) = log {
        command.env("VKRGSS_LOG", log);
    }
    let output = command.output().expect("the program starts");
    Ran { output, work }
}

/// The executable with these arguments, to be started in `work`, with the loader restricted
/// as this process is and none of `VKRGSS_LOG`, `VKRGSS_GUEST` and `VKRGSS_SANDBOX` set, nor
/// their former names, which the program reads for one version. `work` also stands in for
/// the user's run-time directory, where the socket of a guest process is made.
pub(crate) fn command(work: &Path, arguments: &[&str]) -> Command {
    command_of(Path::new(env!("CARGO_BIN_EXE_vkrgss")), work, arguments)
}

/// [`command`] for the executable `program`, a copy of the program.
pub(crate) fn command_of(program: &Path, work: &Path, arguments: &[&str]) -> Command {
    let restriction = isolation();
    let mut command = Command::new(program);
    command
        .args(arguments)
        .current_dir(work)
        .env("XDG_DATA_HOME", work)
        .env("XDG_CONFIG_HOME", work)
        .env("XDG_RUNTIME_DIR", work)
        .env_remove("VKRGSS_LOG")
        .env_remove("VKRGSS_GUEST")
        .env_remove("VKRGSS_SANDBOX")
        .env_remove("VKXP_LOG")
        .env_remove("VKXP_GUEST")
        .env_remove("VKXP_SANDBOX");
    for (name, value) in restriction {
        command.env(name, value);
    }
    command
}

/// Runs `game` headless with its stand-in built-in scripts file, followed by `arguments`.
pub(crate) fn headless(game: &impl TestGame, arguments: &[&str]) -> Ran {
    headless_in(
        tempfile::tempdir().expect("a temporary directory"),
        game,
        arguments,
    )
}

/// Runs `game` headless as [`headless`] does, started in `work`.
pub(crate) fn headless_in(work: TempDir, game: &impl TestGame, arguments: &[&str]) -> Ran {
    headless_logging(work, game, arguments, None)
}

/// The arguments of a headless run of `game` with its stand-in built-in scripts file,
/// followed by `arguments`.
pub(crate) fn headless_arguments(game: &impl TestGame, arguments: &[&str]) -> Vec<String> {
    let dir = game.dir().to_string_lossy().into_owned();
    let builtin = game.builtin_scripts().to_string_lossy().into_owned();
    [
        dir.as_str(),
        "--builtin-scripts",
        &builtin,
        "--headless",
        "--device",
        &device(),
    ]
    .into_iter()
    .chain(arguments.iter().copied())
    .map(str::to_owned)
    .collect()
}

/// Runs `game` headless as [`headless`] does, with `VKRGSS_LOG` set to `log`.
pub(crate) fn headless_logging(
    work: TempDir,
    game: &impl TestGame,
    arguments: &[&str],
    log: Option<&str>,
) -> Ran {
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
    vkrgss_logging(work, &all, log)
}

/// A dumped frame.
pub(crate) struct Frame {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
}

impl Frame {
    pub(crate) fn read(path: &Path) -> Frame {
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

    pub(crate) fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let at = (y as usize * self.width as usize + x as usize) * 4;
        self.rgba[at..at + 4].try_into().expect("four bytes")
    }

    /// The leftmost column that has a pixel of this colour.
    pub(crate) fn leftmost(&self, colour: [u8; 4]) -> Option<u32> {
        (0..self.width).find(|x| (0..self.height).any(|y| self.pixel(*x, y) == colour))
    }
}

pub(crate) const RED: [u8; 4] = [255, 0, 0, 255];

/// An RGSSAD version 1 archive of these entries, in the format `vkrgss-vfs` documents: after
/// the header, per entry, the length of the name, the name, the size of the data and the
/// data, XORed with a key that advances after every length, name byte and size, the data
/// word by word with a copy of the key.
pub(crate) fn rgssad(entries: &[(&str, &[u8])]) -> Vec<u8> {
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

/// The lines of the error stream that say a runtime is experimental.
pub(crate) fn experimental_lines(stderr: &str) -> Vec<&str> {
    stderr
        .lines()
        .filter(|line| line.contains("experimental"))
        .collect()
}

/// An RGSSAD version 3 archive of these entries, in the format `vkrgss-vfs` documents: after
/// the header a seed, then a table of records XORed with the key seed * 9 + 3 (the offset
/// of the data, its size, its key, the length of the name, and the name), a word that
/// decrypts to 0, and the data, each XORed word by word with a key that starts at the key
/// of its record. The seed and the keys are those the editor wrote in the measured archives.
pub(crate) fn rgss3a(entries: &[(&str, &[u8])]) -> Vec<u8> {
    const SEED: u32 = 0x2A38;
    const FILE_KEYS: [u32; 3] = [41, 18467, 6334];
    let key = SEED.wrapping_mul(9).wrapping_add(3);
    let table: usize = entries.iter().map(|(name, _)| 16 + name.len()).sum();
    let mut offset = 12 + table + 4;
    let mut archive = b"RGSSAD\0\x03".to_vec();
    archive.extend(SEED.to_le_bytes());
    for ((name, contents), file_key) in entries.iter().zip(FILE_KEYS) {
        for word in [offset, contents.len(), file_key as usize, name.len()] {
            archive.extend((u32::try_from(word).unwrap() ^ key).to_le_bytes());
        }
        let mask = key.to_le_bytes();
        archive.extend(name.bytes().enumerate().map(|(i, byte)| byte ^ mask[i % 4]));
        offset += contents.len();
    }
    archive.extend(key.to_le_bytes());
    for ((_, contents), mut data_key) in entries.iter().zip(FILE_KEYS) {
        for word in contents.chunks(4) {
            let mask = data_key.to_le_bytes();
            data_key = data_key.wrapping_mul(7).wrapping_add(3);
            archive.extend(word.iter().zip(mask).map(|(byte, mask)| byte ^ mask));
        }
    }
    archive
}

/// Starts `game` headless with `arguments`, its error stream kept.
pub(crate) fn start_headless(work: &Path, game: &impl TestGame, arguments: &[&str]) -> Child {
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
    command(work, &all)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the program starts")
}

/// Runs `game` headless with the guest in a process for `frames` frames, with the states
/// schedule `states` unless it is empty, and writes every frame of `dumped`.
pub(crate) fn states_run(game: &impl TestGame, frames: u64, states: &str, dumped: &[u64]) -> Ran {
    let frames = frames.to_string();
    let names: Vec<String> = dumped
        .iter()
        .map(|index| format!("{index}={index}.png"))
        .collect();
    let mut arguments = vec!["--guest", "process", "--frames", &frames];
    if !states.is_empty() {
        arguments.extend(["--states", states]);
    }
    arguments.extend(names.iter().flat_map(|name| ["--dump", name.as_str()]));
    let work = tempfile::tempdir().expect("a temporary directory");
    let ran = headless_logging(work, game, &arguments, Some("info"));
    ran.assert_success();
    ran
}

pub(crate) fn dumped_bytes(ran: &Ran, index: u64) -> Vec<u8> {
    fs::read(ran.file(&format!("{index}.png"))).expect("a dumped frame")
}

/// The numbers of the guest processes that attached during a run, from its informational
/// lines.
pub(crate) fn attached_processes(stderr: &str) -> Vec<u32> {
    let numbers = stderr.lines().filter_map(|line| {
        let (_, rest) = line.split_once("guest process ")?;
        let (number, rest) = rest.split_once(' ')?;
        rest.starts_with("attached").then(|| number.parse().ok())?
    });
    numbers.collect()
}

/// Why a guest process cannot confine itself here, if it cannot: the kernel has no Landlock,
/// or the environment the tests run in does not let it be used. A thread that is made for
/// it puts a restriction without consequence on itself.
pub(crate) fn landlock_missing() -> Option<String> {
    use landlock::{AccessFs, Ruleset, RulesetAttr, RulesetStatus};

    let probe = || {
        Ruleset::default()
            .handle_access(AccessFs::MakeBlock)?
            .create()?
            .restrict_self()
    };
    match std::thread::spawn(probe).join().expect("the probe ends") {
        Ok(status) if status.ruleset == RulesetStatus::NotEnforced => {
            Some(format!("Landlock is not available: {:?}", status.landlock))
        }
        Ok(_) => None,
        Err(error) => Some(format!("Landlock cannot be used: {error}")),
    }
}

/// Whether the test that calls this is skipped, with the reason printed.
pub(crate) fn skipped_without_landlock(test: &str) -> bool {
    let missing = landlock_missing();
    if let Some(reason) = &missing {
        eprintln!("{test}: skipped, a guest process cannot confine itself here. {reason}");
    }
    missing.is_some()
}
