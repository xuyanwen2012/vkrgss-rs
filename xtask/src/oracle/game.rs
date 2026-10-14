//! The copy of the game that a run works on, with the probe in its script archive.

use std::fs;
use std::path::{Path, PathBuf};

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

/// Copies the game directory `source` to `copy`, replacing what is there, and adds `probe`
/// as the first script of the copy's archive. `runtime` gives the name of the encrypted
/// archive that the game may have.
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
        return probed.with_context(|| packed.display().to_string());
    }
    let scripts = scripts_file(copy, &name)?;
    let bytes = fs::read(&scripts).with_context(|| scripts.display().to_string())?;
    let probed = archive::with_first(&bytes, probe::NAME, probe)
        .with_context(|| scripts.display().to_string())?;
    fs::write(&scripts, probed).with_context(|| scripts.display().to_string())
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
        }
    }
    Ok(())
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
