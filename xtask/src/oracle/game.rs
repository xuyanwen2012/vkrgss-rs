//! The copy of the game that a run works on, with the probe in its script archive.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::runtime::Runtime;
use super::{archive, probe};
use crate::rgssad;

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
        return probe_packed(&packed, &name, probe).with_context(|| packed.display().to_string());
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
    let ini = child(game, "Game.ini")
        .with_context(|| format!("{}: no Game.ini, so this is not a game", game.display()))?;
    // The line is looked for in the bytes: the title may be in any encoding.
    let bytes = fs::read(&ini).with_context(|| ini.display().to_string())?;
    let value = bytes
        .split(|byte| matches!(byte, b'\r' | b'\n'))
        .find_map(|line| {
            let (key, value) = line.split_at_checked(8)?;
            key.eq_ignore_ascii_case(b"Scripts=").then_some(value)
        })
        .with_context(|| format!("{}: no `Scripts=` line", ini.display()))?;
    Ok(String::from_utf8_lossy(value).trim().to_owned())
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
