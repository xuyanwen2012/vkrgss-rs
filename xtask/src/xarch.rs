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
//!   puts them, with the app's log and the lines of logcat beside them. With
//!   `--validation` the APK carries the Vulkan validation layer and every game runs under
//!   it, as with `android-run --validation`, and a game fails when the layer reported an
//!   error.
//! - `xarch compare <A> <B>` compares, for each game of `A`, the pixels of every frame and
//!   the bytes of every save with those of `B`, and fails when one differs or is missing. A
//!   save is a file of the copy that the run wrote, whatever its name: one that the game's
//!   directory of the list lacks or holds with other bytes, apart from the script archive
//!   into which the oracle put the probe and the `vkrgss.toml` of `xarch android` while it
//!   holds what that wrote. A game whose entry sets `saves = N` fails when
//!   fewer than `N` saves were compared on either side.
//! - `xarch baseline [GAME...]` runs the games as `xarch run` does into
//!   `target/xarch/base-<COMMIT>`, the short hash of `HEAD`, with `-dirty` when tracked files
//!   differ from it, and records that directory as the baseline in `target/xarch/baseline`.
//!   With `--at REV` it runs the vkrgss of the revision `REV` instead, from a worktree of it in
//!   [`TREE`], which keeps its own build directory from run to run.
//! - `xarch check [GAME...]` runs the games of the current tree into `target/xarch/check` and
//!   compares the baseline with it as `xarch compare` does, for the games run.
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
       cargo xtask xarch android <SERIAL> <OUT> [GAME...] [--list FILE] [--validation]
       cargo xtask xarch compare <A> <B> [--list FILE]
       cargo xtask xarch baseline [GAME...] [--list FILE] [--at REV]
       cargo xtask xarch check [GAME...] [--list FILE]";

/// The directory below `target/` of the runs of `xarch baseline` and `xarch check`.
const XARCH_DIR: &str = "xarch";

/// The file of [`XARCH_DIR`] that names the directory of the baseline.
const BASELINE_FILE: &str = "baseline";

/// The worktree of [`XARCH_DIR`] in which `xarch baseline --at` builds another revision.
const TREE: &str = "tree";

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
        validation: bool,
    },
    Compare {
        a: PathBuf,
        b: PathBuf,
        list: Option<PathBuf>,
    },
    Baseline {
        games: Vec<String>,
        list: Option<PathBuf>,
        at: Option<String>,
    },
    Check {
        games: Vec<String>,
        list: Option<PathBuf>,
    },
}

/// Parses the arguments after `xarch`.
pub(crate) fn parse(arguments: &[&str]) -> Result<Command> {
    let mut list = None;
    let mut validation = false;
    let mut at = None;
    let mut rest = Vec::new();
    let mut arguments = arguments.iter().copied();
    while let Some(argument) = arguments.next() {
        match argument {
            "--list" => {
                let file = arguments.next().context("--list needs a file")?;
                list = Some(absolute(file).with_context(|| file.to_owned())?);
            }
            "--validation" => validation = true,
            "--at" => {
                at = Some(
                    arguments
                        .next()
                        .context("--at needs a revision")?
                        .to_owned(),
                )
            }
            _ if argument.starts_with("--") => bail!("unknown option {argument}"),
            _ => rest.push(argument),
        }
    }
    let path = |text: &str| absolute(text).with_context(|| text.to_owned());
    let names = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect();
    Ok(match rest[..] {
        [command, ..] if validation && command != "android" => {
            bail!("only android takes --validation")
        }
        [command, ..] if at.is_some() && command != "baseline" => {
            bail!("only baseline takes --at")
        }
        ["baseline", ref games @ ..] => Command::Baseline {
            games: names(games),
            list,
            at,
        },
        ["check", ref games @ ..] => Command::Check {
            games: names(games),
            list,
        },
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
            validation,
        },
        ["compare", a, b] => Command::Compare {
            a: path(a)?,
            b: path(b)?,
            list,
        },
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
            validation,
        } => {
            let list = List::read(list.clone().unwrap_or_else(default_list).as_path())?;
            let phone = Phone {
                serial,
                validation: *validation,
            };
            run_on_phone(&list, &list.chosen(games)?, &phone, out, cargo, root)
        }
        Command::Compare { a, b, list } => {
            let list = List::read(list.clone().unwrap_or_else(default_list).as_path())?;
            compare(a, b, &[], &list)
        }
        Command::Baseline { games, list, at } => {
            let list = List::read(list.clone().unwrap_or_else(default_list).as_path())?;
            baseline(&list, &list.chosen(games)?, at.as_deref(), root)
        }
        Command::Check { games, list } => {
            let list = List::read(list.clone().unwrap_or_else(default_list).as_path())?;
            check(&list, &list.chosen(games)?, root)
        }
    }
}

/// `xarch baseline`.
fn baseline(list: &List, games: &[Game], at: Option<&str>, root: &Path) -> Result<()> {
    let dir = root.join("target").join(XARCH_DIR);
    let (tree, name) = match at {
        None => {
            let commit = git(root, &["rev-parse", "--short", "HEAD"])?;
            let dirty = !git(root, &["status", "--porcelain", "--untracked-files=no"])?.is_empty();
            let suffix = if dirty { "-dirty" } else { "" };
            (root.to_owned(), format!("base-{commit}{suffix}"))
        }
        Some(revision) => {
            let commit = git(
                root,
                &["rev-parse", "--short", &format!("{revision}^{{commit}}")],
            )?;
            (
                worktree_at(&dir.join(TREE), &commit, root)?,
                format!("base-{commit}"),
            )
        }
    };
    let out = dir.join(&name);
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| out.display().to_string())?;
    }
    run_here(list, games, &out, &tree)?;
    fs::write(dir.join(BASELINE_FILE), format!("{name}\n")).context("the baseline file")?;
    eprintln!("xarch: the baseline is now {}", out.display());
    Ok(())
}

/// `xarch check`.
fn check(list: &List, games: &[Game], root: &Path) -> Result<()> {
    let dir = root.join("target").join(XARCH_DIR);
    let file = dir.join(BASELINE_FILE);
    let name = fs::read_to_string(&file).with_context(|| {
        format!(
            "{}: no baseline; make one with `cargo xtask xarch baseline`",
            file.display()
        )
    })?;
    let base = dir.join(name.trim());
    let out = dir.join("check");
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| out.display().to_string())?;
    }
    run_here(list, games, &out, root)?;
    println!("xarch: comparing {} with {}", base.display(), out.display());
    let names: Vec<String> = games.iter().map(|game| game.name.clone()).collect();
    compare(&base, &out, &names, list)
}

/// A worktree of the commit `commit` of `root` at `tree`, made or moved there.
fn worktree_at(tree: &Path, commit: &str, root: &Path) -> Result<PathBuf> {
    let text = tree.display().to_string();
    if tree.join(".git").exists() {
        git(tree, &["checkout", "--detach", "--force", commit])?;
    } else {
        git(root, &["worktree", "prune"])?;
        git(
            root,
            &["worktree", "add", "--detach", "--force", &text, commit],
        )?;
    }
    Ok(tree.to_owned())
}

/// The standard output of git with `args` in `dir`, trimmed.
fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .context("could not start git")?;
    ensure!(
        output.status.success(),
        "`git {}` ended with {}: {}",
        args.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
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
    /// How many saves a comparison of two runs is to compare at least.
    saves: Option<usize>,
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

/// The phone of `xarch android`, and whether its games run under the validation layer.
struct Phone<'a> {
    serial: &'a str,
    validation: bool,
}

/// `xarch android`.
fn run_on_phone(
    list: &List,
    games: &[Game],
    phone: &Phone<'_>,
    out: &Path,
    cargo: &str,
    root: &Path,
) -> Result<()> {
    let flavor = match phone.validation {
        true => apk::Flavor::Validation,
        false => apk::Flavor::Debug,
    };
    apk::install(cargo, root, phone.serial, flavor)?;
    push_shared(list, games, phone.serial)?;
    each_game(games, |game| run_one_on_phone(list, game, phone, out, root))
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

/// The files that `xarch android` puts into the copy of `game` beside what the oracle puts
/// there, each relative to the copy and with its bytes: the `vkrgss.toml` of
/// [`phone_options`]. A file of a copy that still holds these bytes is the harness's, not a
/// save.
fn harness_files(list: &List, game: &Game) -> Result<Vec<(PathBuf, Vec<u8>)>> {
    let options = phone_options(list, game)
        .with_context(|| format!("the vkrgss.toml that `xarch android` gives `{}`", game.name))?;
    Ok(vec![(PathBuf::from("vkrgss.toml"), options.into_bytes())])
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
fn run_one_on_phone(
    list: &List,
    game: &Game,
    phone: &Phone<'_>,
    out: &Path,
    root: &Path,
) -> Result<()> {
    let serial = phone.serial;
    let out = out.join(&game.name);
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| out.display().to_string())?;
    }
    let options = list.oracle_options(game, &out, root)?;
    let copy = out.join("vkrgss").join("game");
    oracle::prepare_vkrgss_copy(&options, &copy)?;
    for (file, bytes) in harness_files(list, game)? {
        let file = copy.join(file);
        fs::write(&file, bytes).with_context(|| file.display().to_string())?;
    }

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
    let ran = apk::start_and_wait(serial, PHONE_TIMEOUT, phone.validation);

    let logcat = apk::output(serial, &["logcat", "-d", "-s", "vkrgss"])?;
    fs::write(out.join("logcat.txt"), &logcat).context("logcat.txt")?;
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
    if phone.validation {
        apk::validation_outcome(&logcat)
            .with_context(|| format!("see {}", out.join("logcat.txt").display()))?;
    }
    Ok(())
}

/// The saves in `copy`, a copy of the game in `source` as a run left it: the files that
/// `source` lacks or holds with other bytes, apart from the one into which the oracle put
/// the probe and those of `harness` that still hold the bytes it wrote. Each is given
/// relative to `copy`. A copy that does not exist holds none.
fn saves(copy: &Path, source: &Path, harness: &[(PathBuf, Vec<u8>)]) -> Result<Vec<PathBuf>> {
    if !copy.is_dir() {
        return Ok(Vec::new());
    }
    let probed = oracle::probed_file(copy)?;
    let mut saves = Vec::new();
    for file in files_below(copy)? {
        if file == probed {
            continue;
        }
        let relative = file.strip_prefix(copy).expect("below the copy");
        let original = source.join(relative);
        let bytes = fs::read(&file).with_context(|| file.display().to_string())?;
        if harness
            .iter()
            .any(|(path, written)| path == relative && *written == bytes)
        {
            continue;
        }
        if !original.is_file()
            || fs::read(&original).with_context(|| original.display().to_string())? != bytes
        {
            saves.push(relative.to_owned());
        }
    }
    Ok(saves)
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
    /// How many saves the copy of `a` and that of `b` hold.
    saves_of: (usize, usize),
}

/// Compares the frames and the saves of `game` of `list` in `a` with those in `b`, and prints
/// each difference.
fn compare_game(a: &Path, b: &Path, list: &List, game: &Game) -> Result<Tally> {
    let name = &game.name;
    let (a, b) = (a.join(name), b.join(name));
    let mut tally = Tally::default();
    for frame in captured_frames(&a.join("frames"))? {
        let file = report::vkrgss_file(frame);
        let theirs = b.join(&file);
        if !theirs.is_file() {
            println!("{name}: frame {frame} is missing in B");
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
                "{name}: frame {frame} differs in {} pixels, by up to {}",
                difference.differing_pixels, difference.largest_difference
            ),
            Err(error) => println!("{name}: frame {frame} differs: {error:#}"),
        }
    }
    let (copy_a, copy_b) = (a.join("vkrgss/game"), b.join("vkrgss/game"));
    let harness = harness_files(list, game)?;
    let saves_a = saves(&copy_a, &game.dir, &harness)?;
    let saves_b = saves(&copy_b, &game.dir, &harness)?;
    tally.saves_of = (saves_a.len(), saves_b.len());
    for save in &saves_a {
        let (ours, theirs) = (copy_a.join(save), copy_b.join(save));
        let how = if !theirs.is_file() {
            "is missing in B"
        } else if fs::read(&theirs)? != fs::read(&ours)? {
            "differs"
        } else {
            tally.saves_same += 1;
            continue;
        };
        tally.saves_different += 1;
        println!("{name}: save {} {how}", save.display());
    }
    // What `b` has and `a` has not.
    let frames = b.join("frames");
    if frames.is_dir() {
        for frame in captured_frames(&frames)? {
            if !a.join(report::vkrgss_file(frame)).is_file() {
                println!("{name}: frame {frame} is missing in A");
                tally.frames_different += 1;
            }
        }
    }
    for save in saves_b.iter().filter(|save| !saves_a.contains(save)) {
        // A file of `a` that the run of `a` did not write differs from a save of `b`.
        let how = if copy_a.join(save).is_file() {
            "differs"
        } else {
            "is missing in A"
        };
        tally.saves_different += 1;
        println!("{name}: save {} {how}", save.display());
    }
    Ok(tally)
}

/// `xarch compare`, of the games named in `only`, or of all games of `a`, each of which
/// `list` has to hold. A game of `only` that `a` lacks is a difference, as is a game that
/// expects more saves than either side holds.
fn compare(a: &Path, b: &Path, only: &[String], list: &List) -> Result<()> {
    let mut names: Vec<String> = fs::read_dir(a)
        .with_context(|| a.display().to_string())?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("frames").is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|game| only.is_empty() || only.contains(game))
        .collect();
    names.sort();
    ensure!(!names.is_empty(), "{}: no game output in it", a.display());
    let mut differences = 0;
    let mut short = String::new();
    for name in only.iter().filter(|name| !names.contains(name)) {
        println!("{name}: missing in A");
        differences += 1;
    }
    for name in &names {
        let game = list
            .games
            .iter()
            .find(|game| &game.name == name)
            .with_context(|| {
                format!(
                    "{name}: the list has no game of this name, so its saves cannot be told \
                 from the files of the game"
                )
            })?;
        if !b.join(name).is_dir() {
            println!("{name}: missing in B");
            differences += 1;
            continue;
        }
        let tally = compare_game(a, b, list, game)?;
        let expected = match game.saves {
            Some(saves) => format!(" of {saves} expected"),
            None => String::new(),
        };
        println!(
            "{name}: frames {} same, {} different; saves {} same, {} different{expected}",
            tally.frames_same, tally.frames_different, tally.saves_same, tally.saves_different
        );
        differences += tally.frames_different + tally.saves_different;
        let (of_a, of_b) = tally.saves_of;
        if let Some(saves) = game.saves.filter(|&saves| of_a.min(of_b) < saves) {
            let line = format!("{name}: {saves} saves expected, A holds {of_a} and B {of_b}");
            println!("{line}");
            short.push_str(&format!("; {line}"));
        }
    }
    ensure!(
        differences == 0 && short.is_empty(),
        "{differences} difference(s){short}"
    );
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
                validation: false,
            }
        );
        let command = parse(&["android", "--validation", "SERIAL", "out", "mh"]).unwrap();
        assert!(matches!(
            command,
            Command::Android {
                validation: true,
                ..
            }
        ));
        assert!(parse(&["run", "out", "--validation"]).is_err());
        assert!(parse(&["compare", "a", "b", "--validation"]).is_err());
        assert!(parse(&["compare", "a"]).is_err());
        let command = parse(&["compare", "a", "b", "--list", "games.toml"]).unwrap();
        assert_eq!(
            command,
            Command::Compare {
                a: absolute("a").unwrap(),
                b: absolute("b").unwrap(),
                list: Some(absolute("games.toml").unwrap()),
            }
        );
        assert!(parse(&["run", "out", "--fast"]).is_err());
        let command = parse(&["baseline", "mh", "--at", "origin/main"]).unwrap();
        assert_eq!(
            command,
            Command::Baseline {
                games: vec!["mh".to_owned()],
                list: None,
                at: Some("origin/main".to_owned()),
            }
        );
        let command = parse(&["check", "--list", "games.toml"]).unwrap();
        assert_eq!(
            command,
            Command::Check {
                games: Vec::new(),
                list: Some(absolute("games.toml").unwrap()),
            }
        );
        assert!(parse(&["check", "--at", "HEAD"]).is_err());
        assert!(parse(&["baseline", "--validation"]).is_err());
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

    /// A game of RGSS2 in `dir/source`, and the copy of it that a run left in `dir/<side>`,
    /// with the probe in its script archive and `written` written by the run.
    fn ran(dir: &Path, side: &str, red: u8, written: &[(&str, &str)]) {
        let files = [
            ("Game.ini", "[Game]\r\nScripts=Data\\Scripts.rvdata\r\n"),
            ("Data/Scripts.rvdata", "scripts"),
            ("Data/Map001.rvdata", "map"),
        ];
        let write = |game: &Path| {
            fs::create_dir_all(game.join("Data")).unwrap();
            for (file, bytes) in files {
                fs::write(game.join(file), bytes).unwrap();
            }
        };
        write(&dir.join("source"));
        let out = dir.join(side).join("game");
        fs::create_dir_all(out.join("frames")).unwrap();
        let picture = Picture {
            width: 2,
            height: 1,
            rgba: vec![red, 0, 0, 255, 0, 0, 0, 255],
        };
        picture.write(&out.join(report::vkrgss_file(4))).unwrap();
        let copy = out.join("vkrgss/game");
        write(&copy);
        fs::write(copy.join("Data/Scripts.rvdata"), "probe and scripts").unwrap();
        for (file, bytes) in written {
            fs::create_dir_all(copy.join(file).parent().unwrap()).unwrap();
            fs::write(copy.join(file), bytes).unwrap();
        }
    }

    /// The list of the game of [`ran`], which expects `saves`.
    fn listed_game(dir: &Path, saves: Option<usize>) -> List {
        let game = Game {
            name: "game".to_owned(),
            dir: dir.join("source"),
            frames: vec![4],
            input: None,
            rtp: Vec::new(),
            font: false,
            no_threads: false,
            saves,
        };
        List {
            builtin_scripts: [("rgss2".to_owned(), dir.join("rpg_builtin_rgss2.rb"))].into(),
            games: vec![game],
            ..List::default()
        }
    }

    /// Writes the files of `xarch android` into the copy of [`ran`] in `dir/<side>`.
    fn on_phone(dir: &Path, side: &str, list: &List) {
        let copy = dir.join(side).join("game/vkrgss/game");
        for (file, bytes) in harness_files(list, &list.games[0]).unwrap() {
            fs::write(copy.join(file), bytes).unwrap();
        }
    }

    #[test]
    fn a_numbered_save_that_differs_is_counted_and_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = |side: &str| dir.path().join(side);
        ran(dir.path(), "a", 10, &[("Save/1.rvdata", "one")]);
        ran(dir.path(), "b", 10, &[("Save/1.rvdata", "one")]);
        ran(dir.path(), "c", 11, &[("Save/1.rvdata", "two")]);
        let list = listed_game(dir.path(), Some(1));
        let game = &list.games[0];
        let same = compare_game(&path("a"), &path("b"), &list, game).unwrap();
        assert_eq!(
            same,
            Tally {
                frames_same: 1,
                saves_same: 1,
                saves_of: (1, 1),
                ..Tally::default()
            }
        );
        compare(&path("a"), &path("b"), &[], &list).unwrap();
        let different = compare_game(&path("a"), &path("c"), &list, game).unwrap();
        assert_eq!(
            different,
            Tally {
                frames_different: 1,
                saves_different: 1,
                saves_of: (1, 1),
                ..Tally::default()
            }
        );
        assert!(compare(&path("a"), &path("c"), &[], &list).is_err());
        // A save that only the second wrote, a data file that only the second changed, and a
        // game that only the second ran.
        fs::write(path("b").join("game/vkrgss/game/1.cdat"), "two").unwrap();
        fs::write(path("b").join("game/vkrgss/game/Data/Map001.rvdata"), "new").unwrap();
        let extra = compare_game(&path("a"), &path("b"), &list, game).unwrap();
        assert_eq!((extra.saves_same, extra.saves_different), (1, 2));
        compare(&path("a"), &path("a"), &["game".to_owned()], &list).unwrap();
        assert!(compare(&path("a"), &path("a"), &["other".to_owned()], &list).is_err());
        // A game that the list lacks cannot be compared.
        assert!(compare(&path("a"), &path("b"), &[], &List::default()).is_err());
    }

    #[test]
    fn data_files_and_the_probed_archive_are_not_saves() {
        let dir = tempfile::tempdir().unwrap();
        ran(dir.path(), "a", 10, &[("Save/1.rvdata", "one")]);
        let copy = dir.path().join("a/game/vkrgss/game");
        let saves = saves(&copy, &dir.path().join("source"), &[]).unwrap();
        assert_eq!(saves, [PathBuf::from("Save/1.rvdata")]);
        assert!(copy.join("Data/Map001.rvdata").is_file());
    }

    #[test]
    fn a_game_that_expects_saves_fails_without_them() {
        let dir = tempfile::tempdir().unwrap();
        let path = |side: &str| dir.path().join(side);
        ran(dir.path(), "a", 10, &[]);
        ran(dir.path(), "b", 10, &[]);
        compare(&path("a"), &path("b"), &[], &listed_game(dir.path(), None)).unwrap();
        let error = compare(
            &path("a"),
            &path("b"),
            &[],
            &listed_game(dir.path(), Some(2)),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("game: 2 saves expected"), "{error}");
    }

    #[test]
    fn the_configuration_of_the_phone_is_no_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = |side: &str| dir.path().join(side);
        let list = listed_game(dir.path(), Some(1));
        // A run on this machine and one on a phone, which wrote the same save.
        ran(dir.path(), "here", 10, &[("Save/1.rvdata", "one")]);
        ran(dir.path(), "phone", 10, &[("Save/1.rvdata", "one")]);
        on_phone(dir.path(), "phone", &list);
        let tally = compare_game(&path("here"), &path("phone"), &list, &list.games[0]).unwrap();
        assert_eq!((tally.saves_same, tally.saves_different), (1, 0));
        assert_eq!(tally.saves_of, (1, 1));
        compare(&path("here"), &path("phone"), &[], &list).unwrap();
        compare(&path("phone"), &path("here"), &[], &list).unwrap();
    }

    #[test]
    fn the_configuration_of_the_phone_meets_no_expected_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = |side: &str| dir.path().join(side);
        let list = listed_game(dir.path(), Some(1));
        for side in ["a", "b"] {
            ran(dir.path(), side, 10, &[]);
            on_phone(dir.path(), side, &list);
        }
        let error = compare(&path("a"), &path("b"), &[], &list)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("game: 1 saves expected, A holds 0 and B 0"),
            "{error}"
        );
    }

    #[test]
    fn a_configuration_that_the_game_changed_is_a_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = |side: &str| dir.path().join(side);
        let list = listed_game(dir.path(), None);
        ran(dir.path(), "a", 10, &[]);
        ran(dir.path(), "b", 10, &[("vkrgss.toml", "audio = true\n")]);
        let tally = compare_game(&path("a"), &path("b"), &list, &list.games[0]).unwrap();
        assert_eq!(tally.saves_of, (0, 1));
        assert_eq!(tally.saves_different, 1);
        assert!(compare(&path("a"), &path("b"), &[], &list).is_err());
    }
}
