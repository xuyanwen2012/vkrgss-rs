//! The copy of the game that a run works on, with the probe in its script archive.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::{archive, probe};

/// Copies the game directory `source` to `copy`, replacing what is there, and adds `probe`
/// as the first script of the copy's archive.
pub fn prepare(source: &Path, copy: &Path, probe: &str) -> Result<()> {
    if copy.exists() {
        fs::remove_dir_all(copy).with_context(|| copy.display().to_string())?;
    }
    copy_dir(source, copy)?;
    let scripts = scripts_file(copy)?;
    let bytes = fs::read(&scripts).with_context(|| scripts.display().to_string())?;
    let probed = archive::with_first(&bytes, probe::NAME, probe)
        .with_context(|| scripts.display().to_string())?;
    fs::write(&scripts, probed).with_context(|| scripts.display().to_string())
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

/// The script archive that `Game.ini` in `game` names with `Scripts=`.
fn scripts_file(game: &Path) -> Result<PathBuf> {
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
    let value = String::from_utf8_lossy(value);
    let mut path = game.to_path_buf();
    for part in value
        .trim()
        .split(['\\', '/'])
        .filter(|part| !part.is_empty())
    {
        path = match child(&path, part) {
            Some(found) => found,
            None => bail!(
                "{}: the script archive `{}` that Game.ini names does not exist",
                game.display(),
                value.trim()
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
