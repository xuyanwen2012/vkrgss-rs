//! `cargo xtask xarch`: runs the comparison games in vkrgss alone, on this machine or on an
//! Android phone, and compares two such runs, so that the output of two platforms can be
//! checked to be identical. Run by hand, not by the gate.
//!
//! - `xarch run <OUT> [GAME...]` runs each game of the list as `cargo xtask oracle
//!   --vkrgss-only` does, into `<OUT>/<game>/`. It runs on Linux and on macOS.
//! - `xarch android <SERIAL> <OUT> [GAME...]` builds the APK and installs it on the phone,
//!   pushes the run-time packages, the fonts and the built-in scripts of the list into
//!   [`PHONE_DIR`] of the app's files directory, and runs each game there: the copy that the
//!   oracle makes for vkrgss, probe included, with a `vkrgss.toml` that asks for the options
//!   the oracle gives vkrgss. The copy as the run left it is pulled back to
//!   `<OUT>/<game>/vkrgss/game` and the frames to `<OUT>/<game>/frames/`, where the oracle
//!   puts them, with the app's log and the lines of logcat beside them.
//! - `xarch compare <A> <B>` compares, for each game of `A`, the pixels of every frame and
//!   the bytes of every save file that the game wrote with those of `B`, and fails when one
//!   differs or is missing.
//!
//! `[GAME...]` chooses games of the list by name; without one every game runs. The list is
//! a TOML file, `local/xarch.toml` of the workspace unless `--list FILE` names another; the
//! games in it are the owner's and are never committed. `xtask/xarch.example.toml` shows
//! its keys. See `docs/development.md`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf, absolute};
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;

use crate::android::{adb, run_host};
use crate::apk::{self, FILES_DIR, PACKAGE};
use crate::oracle;
use crate::oracle::report::{self, Picture};

pub(crate) const USAGE: &str = "\
usage: cargo xtask xarch run <OUT> [GAME...] [--list FILE]
       cargo xtask xarch android <SERIAL> <OUT> [GAME...] [--list FILE]
       cargo xtask xarch compare <A> <B>";

/// The directory of the app's files directory that holds what the games share.
const PHONE_DIR: &str = "xarch";

/// How long one game may run on a phone.
const PHONE_TIMEOUT: Duration = Duration::from_secs(3600);

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Run {
        out: PathBuf,
        games: Vec<String>,
        list: Option<PathBuf>,
    },
    Android {
        serial: String,
        out: PathBuf,
        games: Vec<String>,
        list: Option<PathBuf>,
    },
    Compare {
        a: PathBuf,
        b: PathBuf,
    },
}

/// Parses the arguments after `xarch`.
pub(crate) fn parse(arguments: &[&str]) -> Result<Command> {
    let mut list = None;
    let mut rest = Vec::new();
    let mut arguments = arguments.iter().copied();
    while let Some(argument) = arguments.next() {
        match argument {
            "--list" => {
                let file = arguments.next().context("--list needs a file")?;
                list = Some(absolute(file).with_context(|| file.to_owned())?);
            }
            _ if argument.starts_with("--") => bail!("unknown option {argument}"),
            _ => rest.push(argument),
        }
    }
    let path = |text: &str| absolute(text).with_context(|| text.to_owned());
    let names = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect();
    Ok(match rest[..] {
        ["run", out, ref games @ ..] => Command::Run {
            out: path(out)?,
            games: names(games),
            list,
        },
        ["android", serial, out, ref games @ ..] => Command::Android {
            serial: serial.to_owned(),
            out: path(out)?,
            games: names(games),
            list,
        },
        ["compare", a, b] => {
            ensure!(list.is_none(), "compare takes no --list");
            Command::Compare {
                a: path(a)?,
                b: path(b)?,
            }
        }
        _ => bail!("xarch: unknown or incomplete command"),
    })
}

/// Runs the command; `root` is the workspace.
pub(crate) fn run(command: &Command, cargo: &str, root: &Path) -> Result<()> {
    let default_list = || root.join("local").join("xarch.toml");
    match command {
        Command::Run { out, games, list } => {
            let list = List::read(list.clone().unwrap_or_else(default_list).as_path())?;
            run_here(&list, &list.chosen(games)?, out, root)
        }
        Command::Android {
            serial,
            out,
            games,
            list,
        } => {
            let list = List::read(list.clone().unwrap_or_else(default_list).as_path())?;
            run_on_phone(&list, &list.chosen(games)?, serial, out, cargo, root)
        }
        Command::Compare { a, b } => compare(a, b),
    }
}

/// The list of games and what they share.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct List {
    /// The font files that the games with `font = true` draw text with.
    #[serde(default)]
    fonts: Vec<PathBuf>,
    /// The built-in scripts file of each runtime, by its name: `rgss1`, `rgss2`, `rgss3`.
    #[serde(default)]
    builtin_scripts: BTreeMap<String, PathBuf>,
    /// The directory of each run-time package, by its name.
    #[serde(default)]
    rtp: BTreeMap<String, PathBuf>,
    #[serde(default, rename = "game")]
    games: Vec<Game>,
}

/// A game of the list.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Game {
    name: String,
    dir: PathBuf,
    /// The frames to capture.
    frames: Vec<u64>,
    /// The schedule of held buttons, as `--input` of the oracle takes it.
    input: Option<String>,
    /// The names of the run-time packages of `[rtp]` that the game uses.
    #[serde(default)]
    rtp: Vec<String>,
    /// Whether the game draws text with the fonts of the list.
    #[serde(default)]
    font: bool,
    /// `--no-threads` of the oracle.
    #[serde(default)]
    no_threads: bool,
}

impl List {
    /// Reads the list in `file`; its paths are relative to its directory.
    fn read(file: &Path) -> Result<List> {
        let text = fs::read_to_string(file).with_context(|| {
            format!(
                "{}: the list of games cannot be read; `xtask/xarch.example.toml` shows one",
                file.display()
            )
        })?;
        let mut list: List = toml::from_str(&text).with_context(|| file.display().to_string())?;
        let dir = file.parent().unwrap_or(Path::new("."));
        let within = |path: &mut PathBuf| *path = dir.join(&*path);
        list.fonts.iter_mut().for_each(within);
        list.builtin_scripts.values_mut().for_each(within);
        list.rtp.values_mut().for_each(within);
        for game in &mut list.games {
            within(&mut game.dir);
            if let Some(file) = game
                .input
                .as_mut()
                .and_then(|input| input.strip_prefix('@'))
            {
                game.input = Some(format!("@{}", dir.join(file).display()));
            }
            for name in &game.rtp {
                ensure!(
                    list.rtp.contains_key(name),
                    "{}: game `{}` names the run-time package `{name}`, which `[rtp]` lacks",
                    file.display(),
                    game.name
                );
            }
        }
        Ok(list)
    }

    /// The games named, or all of them.
    fn chosen(&self, names: &[String]) -> Result<Vec<Game>> {
        if names.is_empty() {
            return Ok(self.games.clone());
        }
        names
            .iter()
            .map(|name| {
                self.games
                    .iter()
                    .find(|game| &game.name == name)
                    .cloned()
                    .with_context(|| format!("the list has no game `{name}`"))
            })
            .collect()
    }

    /// The built-in scripts file of the game.
    fn builtin_scripts(&self, game: &Game) -> Result<&Path> {
        let runtime = oracle::runtime_of(&game.dir)?.name();
        self.builtin_scripts
            .get(runtime)
            .map(PathBuf::as_path)
            .with_context(|| {
                format!(
                    "game `{}` is of {runtime}, for which `[builtin_scripts]` names no file",
                    game.name
                )
            })
    }

    /// The arguments of `cargo xtask oracle --vkrgss-only` for the game, written to `out`.
    fn oracle_arguments(&self, game: &Game, out: &Path) -> Result<Vec<String>> {
        let text = |path: &Path| path.display().to_string();
        let frames: Vec<String> = game.frames.iter().map(u64::to_string).collect();
        let mut arguments = vec![
            text(&game.dir),
            "--vkrgss-only".to_owned(),
            "--frames".to_owned(),
            frames.join(","),
            "--builtin-scripts".to_owned(),
            text(self.builtin_scripts(game)?),
            "--out".to_owned(),
            text(out),
        ];
        for name in &game.rtp {
            arguments.push("--rtp".to_owned());
            arguments.push(format!("{name}={}", self.rtp[name].display()));
        }
        if game.font {
            for font in &self.fonts {
                arguments.push("--font".to_owned());
                arguments.push(text(font));
            }
        }
        if let Some(input) = &game.input {
            arguments.push("--input".to_owned());
            arguments.push(input.clone());
        }
        if game.no_threads {
            arguments.push("--no-threads".to_owned());
        }
        Ok(arguments)
    }

    /// The options of the oracle for the game, written to `out`.
    fn oracle_options(&self, game: &Game, out: &Path, root: &Path) -> Result<oracle::Options> {
        let arguments = self.oracle_arguments(game, out)?;
        let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
        oracle::parse(&arguments, root)
    }
}

/// Runs every game and reports those that failed at the end.
fn each_game(games: &[Game], mut run: impl FnMut(&Game) -> Result<()>) -> Result<()> {
    let mut failed = Vec::new();
    for game in games {
        eprintln!("xarch: {}", game.name);
        if let Err(error) = run(game) {
            eprintln!("xarch: {}: {error:#}", game.name);
            failed.push(game.name.as_str());
        }
    }
    ensure!(failed.is_empty(), "xarch: failed: {}", failed.join(", "));
    Ok(())
}

/// `xarch run`.
fn run_here(list: &List, games: &[Game], out: &Path, root: &Path) -> Result<()> {
    each_game(games, |game| {
        let options = list.oracle_options(game, &out.join(&game.name), root)?;
        oracle::run(&options, root)
    })
}

/// `xarch android`.
fn run_on_phone(
    list: &List,
    games: &[Game],
    serial: &str,
    out: &Path,
    cargo: &str,
    root: &Path,
) -> Result<()> {
    apk::install(cargo, root, serial)?;
    push_shared(list, games, serial)?;
    each_game(games, |game| {
        run_one_on_phone(list, game, serial, out, root)
    })
}

/// The path of a file of [`PHONE_DIR`] as a game directory of the files directory sees it.
fn shared_path(path: &str) -> String {
    format!("../{PHONE_DIR}/{path}")
}

/// The name of the file at `path`.
fn file_name(path: &Path) -> Result<String> {
    Ok(path
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("{}: no file name", path.display()))?
        .to_owned())
}

/// Pushes the run-time packages, the fonts and the built-in scripts that `games` use to
/// [`PHONE_DIR`]: what the phone has already is pushed again only where it changed, and the
/// fonts replace those of an earlier run.
fn push_shared(list: &List, games: &[Game], serial: &str) -> Result<()> {
    let remote = format!("{FILES_DIR}/{PHONE_DIR}");
    let mut pushes: Vec<(PathBuf, String)> = Vec::new();
    for name in games.iter().flat_map(|game| &game.rtp) {
        let dir = &list.rtp[name];
        for entry in fs::read_dir(dir).with_context(|| dir.display().to_string())? {
            let entry = entry.with_context(|| dir.display().to_string())?;
            pushes.push((entry.path(), format!("{remote}/rtp/{name}/")));
        }
    }
    for game in games {
        let file = list.builtin_scripts(game)?;
        pushes.push((file.to_owned(), format!("{remote}/builtin/")));
    }
    if games.iter().any(|game| game.font) {
        for font in &list.fonts {
            pushes.push((font.clone(), format!("{remote}/fonts/")));
        }
    }
    pushes.sort();
    pushes.dedup();
    apk::output(serial, &["shell", &format!("rm -rf {remote}/fonts")])?;
    let mut dirs: Vec<&str> = pushes.iter().map(|(_, dir)| dir.as_str()).collect();
    dirs.dedup();
    for dir in dirs {
        apk::output(serial, &["shell", &format!("mkdir -p {dir}")])?;
    }
    for (path, dir) in &pushes {
        eprintln!("xarch: pushing {} to {dir}", path.display());
        run_host(
            adb(Some(serial))
                .args(["push", "--sync"])
                .arg(path)
                .arg(dir),
        )?;
    }
    Ok(())
}

/// The `vkrgss.toml` that asks the app for what the oracle asks of vkrgss: the frames, the
/// built-in scripts, the run-time packages and the fonts with their own line boxes, without
/// sound.
fn phone_options(list: &List, game: &Game) -> Result<String> {
    let quoted = |text: &str| format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""));
    let last = game.frames.iter().max().context("no frame to capture")?;
    let builtin = shared_path(&format!(
        "builtin/{}",
        file_name(list.builtin_scripts(game)?)?
    ));
    let dumps: Vec<String> = game
        .frames
        .iter()
        .map(|frame| quoted(&format!("{frame}={frame}-vkrgss.png")))
        .collect();
    let mut text = format!(
        "builtin_scripts = {}\naudio = false\nframes = {}\ndump = [{}]\n",
        quoted(&builtin),
        last + 1,
        dumps.join(", ")
    );
    if game.font && !list.fonts.is_empty() {
        text += &format!(
            "font_dirs = [{}]\nfont_own_metrics = true\n",
            quoted(&shared_path("fonts"))
        );
    }
    if !game.rtp.is_empty() {
        text += "\n[rtp]\n";
        for name in &game.rtp {
            text += &format!(
                "{} = {}\n",
                quoted(name),
                quoted(&shared_path(&format!("rtp/{name}")))
            );
        }
    }
    Ok(text)
}

/// Runs one game on the phone; see the module documentation.
fn run_one_on_phone(list: &List, game: &Game, serial: &str, out: &Path, root: &Path) -> Result<()> {
    let out = out.join(&game.name);
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| out.display().to_string())?;
    }
    let options = list.oracle_options(game, &out, root)?;
    let copy = out.join("vkrgss").join("game");
    oracle::prepare_vkrgss_copy(&options, &copy)?;
    let toml = copy.join("vkrgss.toml");
    fs::write(&toml, phone_options(list, game)?).with_context(|| toml.display().to_string())?;

    let remote = format!("{FILES_DIR}/game");
    let shell = |script: &str| apk::output(serial, &["shell", script]);
    shell(&format!("rm -rf {remote} {FILES_DIR}/vkrgss.log"))?;
    eprintln!("xarch: pushing {} to {remote}", copy.display());
    run_host(adb(Some(serial)).arg("push").arg(&copy).arg(&remote))?;
    fs::remove_dir_all(&copy).with_context(|| copy.display().to_string())?;
    // A screen that is off or locked sends the activity to the background, where the game
    // waits.
    shell("input keyevent KEYCODE_WAKEUP")?;
    shell("wm dismiss-keyguard")?;
    shell(&format!("am force-stop {PACKAGE}"))?;
    apk::output(serial, &["logcat", "-c"])?;
    let ran = apk::start_and_wait(serial, PHONE_TIMEOUT);

    let logcat = apk::output(serial, &["logcat", "-d", "-s", "vkrgss"])?;
    fs::write(out.join("logcat.txt"), logcat).context("logcat.txt")?;
    run_host(
        adb(Some(serial))
            .arg("pull")
            .arg(format!("{FILES_DIR}/vkrgss.log"))
            .arg(out.join("vkrgss.log")),
    )?;
    let parent = copy.parent().expect("the copy is in a directory");
    run_host(adb(Some(serial)).arg("pull").arg(&remote).arg(parent))?;
    ran?;
    let frames = out.join("frames");
    fs::create_dir_all(&frames).with_context(|| frames.display().to_string())?;
    let mut missing = Vec::new();
    for frame in &game.frames {
        let name = format!("{frame}-vkrgss.png");
        let target = out.join(report::vkrgss_file(*frame));
        if fs::rename(copy.join(&name), &target).is_err() {
            missing.push(name);
        }
    }
    ensure!(
        missing.is_empty(),
        "the app wrote no {}; see {}",
        missing.join(", "),
        out.join("vkrgss.log").display()
    );
    Ok(())
}

/// Whether the file at `path`, below a game directory, is a save file of the game.
fn is_save(path: &Path) -> bool {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let extension = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let data = ["rxdata", "rvdata", "rvdata2", "cdat"].contains(&extension.as_str());
    (data && name.contains("save")) || name.starts_with("save") || name.starts_with("autosave")
}

/// The files below `dir`, recursively.
fn files_below(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }
    for entry in fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let path = entry.with_context(|| dir.display().to_string())?.path();
        if path.is_dir() {
            files.extend(files_below(&path)?);
        } else {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// The frame numbers of the pictures of vkrgss in `frames`, in ascending order.
fn captured_frames(frames: &Path) -> Result<Vec<u64>> {
    let mut numbers: Vec<u64> = fs::read_dir(frames)
        .with_context(|| frames.display().to_string())?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.strip_suffix("-vkrgss.png")?.parse().ok()
        })
        .collect();
    numbers.sort_unstable();
    Ok(numbers)
}

/// How the output of one game in `a` compares with that in `b`.
#[derive(Debug, Default, PartialEq, Eq)]
struct Tally {
    frames_same: usize,
    frames_different: usize,
    saves_same: usize,
    saves_different: usize,
}

/// Compares the frames and the saves of the game `game` of `a` with those of `b`, and prints
/// each difference.
fn compare_game(a: &Path, b: &Path, game: &str) -> Result<Tally> {
    let (a, b) = (a.join(game), b.join(game));
    let mut tally = Tally::default();
    for frame in captured_frames(&a.join("frames"))? {
        let file = report::vkrgss_file(frame);
        let theirs = b.join(&file);
        if !theirs.is_file() {
            println!("{game}: frame {frame} is missing in B");
            tally.frames_different += 1;
            continue;
        }
        let (ours, theirs) = (Picture::read(&a.join(&file))?, Picture::read(&theirs)?);
        if ours == theirs {
            tally.frames_same += 1;
            continue;
        }
        tally.frames_different += 1;
        match report::compare(&ours, &theirs) {
            Ok(difference) => println!(
                "{game}: frame {frame} differs in {} pixels, by up to {}",
                difference.differing_pixels, difference.largest_difference
            ),
            Err(error) => println!("{game}: frame {frame} differs: {error:#}"),
        }
    }
    let copy = a.join("vkrgss").join("game");
    for save in files_below(&copy)?.into_iter().filter(|path| is_save(path)) {
        let relative = save.strip_prefix(&a).expect("below the game's output");
        let theirs = b.join(relative);
        if theirs.is_file() && fs::read(&theirs)? == fs::read(&save)? {
            tally.saves_same += 1;
        } else {
            tally.saves_different += 1;
            let how = if theirs.is_file() {
                "differs"
            } else {
                "is missing in B"
            };
            println!("{game}: save {} {how}", relative.display());
        }
    }
    Ok(tally)
}

/// `xarch compare`.
fn compare(a: &Path, b: &Path) -> Result<()> {
    let mut games: Vec<String> = fs::read_dir(a)
        .with_context(|| a.display().to_string())?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("frames").is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    games.sort();
    ensure!(!games.is_empty(), "{}: no game output in it", a.display());
    let mut differences = 0;
    for game in &games {
        if !b.join(game).is_dir() {
            println!("{game}: missing in B");
            differences += 1;
            continue;
        }
        let tally = compare_game(a, b, game)?;
        println!(
            "{game}: frames {} same, {} different; saves {} same, {} different",
            tally.frames_same, tally.frames_different, tally.saves_same, tally.saves_different
        );
        differences += tally.frames_different + tally.saves_different;
    }
    ensure!(differences == 0, "{differences} difference(s)");
    println!("all identical");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_commands_are_parsed_with_the_list_anywhere() {
        let command = parse(&["run", "out", "mh", "--list", "games.toml", "vv"]).unwrap();
        assert_eq!(
            command,
            Command::Run {
                out: absolute("out").unwrap(),
                games: vec!["mh".to_owned(), "vv".to_owned()],
                list: Some(absolute("games.toml").unwrap()),
            }
        );
        let command = parse(&["android", "SERIAL", "out"]).unwrap();
        assert_eq!(
            command,
            Command::Android {
                serial: "SERIAL".to_owned(),
                out: absolute("out").unwrap(),
                games: Vec::new(),
                list: None,
            }
        );
        assert!(parse(&["compare", "a"]).is_err());
        assert!(parse(&["run", "out", "--fast"]).is_err());
    }

    /// A list with one game of RGSS1, in a directory of its own.
    fn listed() -> (tempfile::TempDir, List) {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("games/one");
        fs::create_dir_all(&game).unwrap();
        fs::write(
            game.join("Game.ini"),
            "[Game]\r\nScripts=Data\\Scripts.rxdata\r\n",
        )
        .unwrap();
        let file = dir.path().join("xarch.toml");
        fs::write(
            &file,
            r#"
                fonts = ["fonts/cjk.otf"]
                [builtin_scripts]
                rgss1 = "builtin/rpg_builtin.rb"
                [rtp]
                Standard = "rtp/xp"
                [[game]]
                name = "one"
                dir = "games/one"
                frames = [4, 90]
                input = "@schedule.txt"
                rtp = ["Standard"]
                font = true
            "#,
        )
        .unwrap();
        let list = List::read(&file).unwrap();
        (dir, list)
    }

    #[test]
    fn the_paths_of_the_list_are_relative_to_its_directory() {
        let (dir, list) = listed();
        let game = &list.games[0];
        assert_eq!(game.dir, dir.path().join("games/one"));
        assert_eq!(list.rtp["Standard"], dir.path().join("rtp/xp"));
        let schedule = dir.path().join("schedule.txt");
        assert_eq!(game.input, Some(format!("@{}", schedule.display())));
        let arguments = list.oracle_arguments(game, Path::new("/out/one")).unwrap();
        let joined = arguments.join(" ");
        for part in [
            "--vkrgss-only --frames 4,90",
            &format!("--rtp Standard={}", dir.path().join("rtp/xp").display()),
            &format!("--font {}", dir.path().join("fonts/cjk.otf").display()),
            "--out /out/one",
        ] {
            assert!(joined.contains(part), "{part} is not in {joined}");
        }
    }

    #[test]
    fn a_game_that_names_a_package_the_list_lacks_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("xarch.toml");
        fs::write(
            &file,
            "[[game]]\nname = \"one\"\ndir = \"one\"\nframes = [1]\nrtp = [\"RPGVX\"]\n",
        )
        .unwrap();
        let message = format!("{:#}", List::read(&file).unwrap_err());
        assert!(message.contains("RPGVX"), "{message}");
    }

    #[test]
    fn the_options_of_the_phone_ask_for_what_the_oracle_asks_of_vkrgss() {
        let (_dir, list) = listed();
        let text = phone_options(&list, &list.games[0]).unwrap();
        let table: toml::Table = toml::from_str(&text).unwrap();
        assert_eq!(
            table["builtin_scripts"].as_str(),
            Some("../xarch/builtin/rpg_builtin.rb")
        );
        assert_eq!(table["frames"].as_integer(), Some(91));
        assert_eq!(table["audio"].as_bool(), Some(false));
        assert_eq!(table["font_own_metrics"].as_bool(), Some(true));
        let dumps: Vec<&str> = table["dump"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(toml::Value::as_str)
            .collect();
        assert_eq!(dumps, ["4=4-vkrgss.png", "90=90-vkrgss.png"]);
        assert_eq!(
            table["rtp"]["Standard"].as_str(),
            Some("../xarch/rtp/Standard")
        );
    }

    #[test]
    fn saves_are_told_by_their_names() {
        for save in [
            "Save1.rxdata",
            "Saves/SAVE4.rxdata",
            "AutoSave.rvdata2",
            "save01.dat",
        ] {
            assert!(is_save(Path::new(save)), "{save}");
        }
        for other in ["Data/Map001.rxdata", "Game.ini", "MHCI.rxdata"] {
            assert!(!is_save(Path::new(other)), "{other}");
        }
    }

    #[test]
    fn a_frame_or_a_save_that_differs_is_counted_and_equal_ones_are_not() {
        let dir = tempfile::tempdir().unwrap();
        let picture = |red: u8| Picture {
            width: 2,
            height: 1,
            rgba: vec![red, 0, 0, 255, 0, 0, 0, 255],
        };
        for (side, red, save) in [("a", 10, "one"), ("b", 10, "one"), ("c", 11, "two")] {
            let out = dir.path().join(side).join("game");
            fs::create_dir_all(out.join("frames")).unwrap();
            fs::create_dir_all(out.join("vkrgss/game")).unwrap();
            picture(red)
                .write(&out.join(report::vkrgss_file(4)))
                .unwrap();
            fs::write(out.join("vkrgss/game/Save1.rxdata"), save).unwrap();
            fs::write(out.join("vkrgss/game/Map001.rxdata"), side).unwrap();
        }
        let path = |side: &str| dir.path().join(side);
        let same = compare_game(&path("a"), &path("b"), "game").unwrap();
        assert_eq!(
            same,
            Tally {
                frames_same: 1,
                saves_same: 1,
                ..Tally::default()
            }
        );
        compare(&path("a"), &path("b")).unwrap();
        let different = compare_game(&path("a"), &path("c"), "game").unwrap();
        assert_eq!(
            different,
            Tally {
                frames_different: 1,
                saves_different: 1,
                ..Tally::default()
            }
        );
        assert!(compare(&path("a"), &path("c")).is_err());
    }
}
