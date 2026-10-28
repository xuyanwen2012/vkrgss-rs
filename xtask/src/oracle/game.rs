//! The copy of the game that a run works on, with the probe in its script archive.

use std::fs::{self, File, FileTimes};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail, ensure};

use super::runtime::Runtime;
use super::{archive, probe};
use crate::{rgss3a, rgssad};

/// The runtime of the game in `game`: `chosen` if the command line named one, else what the
/// extension of the script archive tells.
pub(super) fn runtime(game: &Path, chosen: Option<Runtime>) -> Result<Runtime> {
    if let Some(runtime) = chosen {
        return Ok(runtime);
    }
    let name = scripts_name(game)?;
    Runtime::of_scripts(&name).with_context(|| {
        format!(
            "{}: `Scripts={name}` of Game.ini does not tell the runtime of the game; \
             name it with --runtime rgss1, rgss2 or rgss3",
            game.display()
        )
    })
}

/// The modification and access time of the files that the oracle writes into a copy itself:
/// 2000-01-01 00:00:00 UTC, so that a game that compares the times of its files sees the same
/// order on every platform and in every run.
pub(super) fn fixed_time() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(946_684_800)
}

/// Copies the game directory `source` to `copy`, replacing what is there, and adds `probe`
/// as the first script of the copy's archive. `runtime` gives the name of the encrypted
/// archive that the game may have. Every copied file has the times of its source, and the
/// archive with the probe [`fixed_time`].
pub(super) fn prepare(source: &Path, copy: &Path, runtime: Runtime, probe: &str) -> Result<()> {
    if copy.exists() {
        fs::remove_dir_all(copy).with_context(|| copy.display().to_string())?;
    }
    copy_dir(source, copy)?;
    let name = scripts_name(copy)?;
    if let Some(packed) = child(copy, runtime.archive_name()) {
        // The original reads its scripts only from the archive when there is one.
        let probed = match runtime {
            Runtime::Rgss3 => probe_packed_rgss3(&packed, &name, probe),
            Runtime::Rgss1 | Runtime::Rgss2 => probe_packed(&packed, &name, probe),
        };
        probed.with_context(|| packed.display().to_string())?;
        return set_fixed_time(&packed);
    }
    let scripts = scripts_file(copy, &name)?;
    let bytes = fs::read(&scripts).with_context(|| scripts.display().to_string())?;
    let probed = archive::with_first(&bytes, probe::NAME, probe)
        .with_context(|| scripts.display().to_string())?;
    fs::write(&scripts, probed).with_context(|| scripts.display().to_string())?;
    set_fixed_time(&scripts)
}

/// Adds `probe` as the first script of the script archive `name` inside the encrypted
/// archive `packed`. The other entries are written back as they were read.
fn probe_packed(packed: &Path, name: &str, probe: &str) -> Result<()> {
    let mut entries = rgssad::read(&fs::read(packed)?)?;
    let scripts = entries
        .iter_mut()
        .find(|entry| rgssad::names_match(&entry.name, name))
        .with_context(|| format!("the script archive `{name}` that Game.ini names is not in it"))?;
    scripts.data =
        archive::with_first(&scripts.data, probe::NAME, probe).with_context(|| name.to_owned())?;
    fs::write(packed, rgssad::write(&entries))?;
    Ok(())
}

/// Adds `probe` as the first script of the script archive `name` inside the archive of
/// RGSSAD version 3 `packed`. The seed, the keys, the order of the entries and the end of the
/// table are kept, so only the data of the script archive change.
fn probe_packed_rgss3(packed: &Path, name: &str, probe: &str) -> Result<()> {
    let mut archive = rgss3a::read(&fs::read(packed)?)?;
    let scripts = archive
        .entries
        .iter_mut()
        .find(|entry| rgssad::names_match(&entry.name, name))
        .with_context(|| format!("the script archive `{name}` that Game.ini names is not in it"))?;
    scripts.data =
        archive::with_first(&scripts.data, probe::NAME, probe).with_context(|| name.to_owned())?;
    fs::write(packed, rgss3a::write(&archive))?;
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to).with_context(|| to.display().to_string())?;
    for entry in fs::read_dir(from).with_context(|| from.display().to_string())? {
        let entry = entry.with_context(|| from.display().to_string())?;
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        // A link is followed, so that the copy never points back into the game.
        if fs::metadata(&source)
            .with_context(|| source.display().to_string())?
            .is_dir()
        {
            copy_dir(&source, &target)?;
        } else {
            fs::copy(&source, &target).with_context(|| source.display().to_string())?;
            // `fs::copy` keeps the modification time on macOS and renews it on Linux.
            let metadata = fs::metadata(&source).with_context(|| source.display().to_string())?;
            let times = FileTimes::new()
                .set_modified(metadata.modified()?)
                .set_accessed(metadata.accessed()?);
            set_times(&target, times)?;
        }
    }
    Ok(())
}

/// The file of the copy `copy` into which [`prepare`] put the probe: the encrypted archive
/// of the runtime if there is one, else the script archive that `Game.ini` names.
pub(super) fn probed(copy: &Path) -> Result<PathBuf> {
    let runtime = runtime(copy, None)?;
    match child(copy, runtime.archive_name()) {
        Some(packed) => Ok(packed),
        None => scripts_file(copy, &scripts_name(copy)?),
    }
}

/// Gives the file `path`, which the oracle wrote, [`fixed_time`].
pub(super) fn set_fixed_time(path: &Path) -> Result<()> {
    set_times(
        path,
        FileTimes::new()
            .set_modified(fixed_time())
            .set_accessed(fixed_time()),
    )
}

fn set_times(path: &Path, times: FileTimes) -> Result<()> {
    // The owner of a file may set its times through a descriptor opened for reading, so a
    // copy of a read-only file gets them too.
    File::open(path)
        .and_then(|file| file.set_times(times))
        .with_context(|| format!("{}: setting the file times", path.display()))
}

/// The path of the script archive that `Game.ini` in `game` names with `Scripts=`.
fn scripts_name(game: &Path) -> Result<String> {
    ini_value(game, "Scripts")?
        .with_context(|| format!("{}: no `Scripts=` line in Game.ini", game.display()))
}

/// The value of the line `key=` of `Game.ini` in `game`, if it has one.
fn ini_value(game: &Path, key: &str) -> Result<Option<String>> {
    let ini = child(game, "Game.ini")
        .with_context(|| format!("{}: no Game.ini, so this is not a game", game.display()))?;
    // The line is looked for in the bytes: the title may be in any encoding.
    let bytes = fs::read(&ini).with_context(|| ini.display().to_string())?;
    let key = format!("{key}=");
    let value = bytes
        .split(|byte| matches!(byte, b'\r' | b'\n'))
        .find_map(|line| {
            let (found, value) = line.split_at_checked(key.len())?;
            found.eq_ignore_ascii_case(key.as_bytes()).then_some(value)
        });
    Ok(value.map(|value| String::from_utf8_lossy(value).trim().to_owned()))
}

/// The path of the DLL of a game of RGSS3, which `Library=` of its `Game.ini` names relative
/// to the game directory.
pub(super) struct Library {
    /// The path as `Game.ini` writes it.
    pub written: String,
    /// Its folders and file name.
    parts: Vec<String>,
}

impl Library {
    /// Reads `Library=` of `Game.ini` in `game`. A missing line, an empty value and a path
    /// that may leave the game directory (a drive, a leading separator, `.` or `..`) are
    /// errors.
    pub(super) fn of(game: &Path) -> Result<Library> {
        let written = ini_value(game, "Library")?.with_context(|| {
            format!(
                "{}: Game.ini has no `Library=` line, which names the DLL of a game of RGSS3",
                game.display()
            )
        })?;
        let parts: Vec<String> = written.split(['\\', '/']).map(str::to_owned).collect();
        let leaves = parts
            .first()
            .is_some_and(|first| first.is_empty() || first.contains(':'))
            || parts.iter().any(|part| part == "." || part == "..");
        ensure!(
            !leaves,
            "{}: the DLL `{written}` that `Library=` of Game.ini names is not a path inside \
             the game directory",
            game.display()
        );
        let parts: Vec<String> = parts.into_iter().filter(|part| !part.is_empty()).collect();
        ensure!(
            !parts.is_empty(),
            "{}: `Library=` of Game.ini names no DLL",
            game.display()
        );
        Ok(Library { written, parts })
    }

    /// The file of `game` at this path, each part matched whatever the case of its letters.
    pub(super) fn find(&self, game: &Path) -> Option<PathBuf> {
        let mut path = game.to_path_buf();
        for part in &self.parts {
            path = child(&path, part)?;
        }
        path.is_file().then_some(path)
    }

    /// The place of the file in `game`: each folder matched whatever the case of its
    /// letters, and made when it is missing; the file as it exists, or as it is written.
    pub(super) fn place(&self, game: &Path) -> Result<PathBuf> {
        let (file, folders) = self.parts.split_last().expect("a path has a part");
        let mut dir = game.to_path_buf();
        for folder in folders {
            dir = match child(&dir, folder) {
                Some(found) => found,
                None => {
                    let made = dir.join(folder);
                    fs::create_dir(&made).with_context(|| made.display().to_string())?;
                    made
                }
            };
        }
        Ok(child(&dir, file).unwrap_or_else(|| dir.join(file)))
    }
}

/// The loose script archive of `game` that `Game.ini` names `name`.
fn scripts_file(game: &Path, name: &str) -> Result<PathBuf> {
    let mut path = game.to_path_buf();
    for part in name.split(['\\', '/']).filter(|part| !part.is_empty()) {
        path = match child(&path, part) {
            Some(found) => found,
            None => bail!(
                "{}: the script archive `{name}` that Game.ini names does not exist",
                game.display()
            ),
        };
    }
    Ok(path)
}

/// The entry of `dir` with this name, whatever the case of its letters.
fn child(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.exists() {
        return Some(exact);
    }
    fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.file_name().to_string_lossy().to_lowercase() == name.to_lowercase())
        .map(|entry| entry.path())
}
