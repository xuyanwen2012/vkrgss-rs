//! The options of a run on Android: `vkrgss.toml` in the game directory.
//!
//! The file has the name, the keys and the meaning of the game's configuration file on Linux,
//! and paths in it are relative to its directory, as there. Android has no command line and
//! no user's file, so this file is the only layer, and it holds only the keys that apply to
//! a run of the app; any other key is an error that names it. Without the file every option
//! has its default.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use vkrgss_host::InputScript;

/// The name of the file in the game directory.
pub const FILE: &str = "vkrgss.toml";

/// What the file holds.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    rtp: BTreeMap<String, PathBuf>,
    builtin_scripts: Option<PathBuf>,
    font_dirs: Option<Vec<PathBuf>>,
    font_own_metrics: Option<bool>,
    audio: Option<bool>,
    soundfont: Option<PathBuf>,
    seed: Option<u64>,
    debug: Option<bool>,
    log_p: Option<bool>,
    frames: Option<u64>,
    input: Option<String>,
    dump: Option<Vec<String>>,
}

/// The options of a run, with every path made absolute.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// The directory of each run-time package, by its name.
    pub rtps: BTreeMap<String, PathBuf>,
    /// The built-in scripts file, instead of the default.
    pub builtin_scripts: Option<PathBuf>,
    /// Font directories, searched before those of the system.
    pub font_dirs: Vec<PathBuf>,
    /// Whether a font that replaces another keeps its own line box, as
    /// `--font-own-metrics` says on the command line.
    pub font_own_metrics: bool,
    /// Whether the run plays no sound.
    pub no_audio: bool,
    pub soundfont: Option<PathBuf>,
    /// The random seed; without one it is taken from the clock.
    pub seed: Option<u64>,
    pub debug: bool,
    pub log_p: bool,
    /// The number of frames after which the game is asked to quit.
    pub frames: Option<u64>,
    /// The buttons held from given frames on, in place of those of the keys.
    pub input: Option<InputScript>,
    /// Frames to write as PNG files, by frame number.
    pub dumps: Vec<(u64, PathBuf)>,
}

impl Options {
    /// Reads the file of `game_dir`, or gives the defaults when it has none. Every error
    /// names the file.
    pub fn read(game_dir: &Path) -> Result<Options> {
        let path = game_dir.join(FILE);
        if !path.exists() {
            return Ok(Options::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read the configuration file {}", path.display()))?;
        Options::parse(&text, game_dir)
            .with_context(|| format!("in the configuration file {}", path.display()))
    }

    /// Reads the text of a file that lies in `dir`.
    fn parse(text: &str, dir: &Path) -> Result<Options> {
        let file: File = toml::from_str(text)?;
        let input = match file.input {
            None => None,
            Some(text) => Some(script(&text, dir).map_err(|error| anyhow!("input: {error}"))?),
        };
        let dumps = file
            .dump
            .unwrap_or_default()
            .iter()
            .map(|text| dump(text, dir).map_err(|error| anyhow!("dump: {error}")))
            .collect::<Result<_>>()?;
        Ok(Options {
            rtps: file
                .rtp
                .into_iter()
                .map(|(name, path)| (name, dir.join(path)))
                .collect(),
            builtin_scripts: file.builtin_scripts.map(|path| dir.join(path)),
            font_dirs: file
                .font_dirs
                .unwrap_or_default()
                .iter()
                .map(|path| dir.join(path))
                .collect(),
            font_own_metrics: file.font_own_metrics == Some(true),
            no_audio: file.audio == Some(false),
            soundfont: file.soundfont.map(|path| dir.join(path)),
            seed: file.seed,
            debug: file.debug == Some(true),
            log_p: file.log_p == Some(true),
            frames: file.frames,
            input,
            dumps,
        })
    }
}

/// An input script, given in place or as `@FILE`, as `--input` takes it.
fn script(text: &str, dir: &Path) -> Result<InputScript> {
    let text = match text.strip_prefix('@') {
        Some(name) => {
            let path = dir.join(name);
            std::fs::read_to_string(&path)
                .with_context(|| format!("cannot read the input script {}", path.display()))?
        }
        None => text.to_owned(),
    };
    Ok(InputScript::parse(&text)?)
}

/// A frame to write, `FRAME=FILE`, as `--dump` takes it.
fn dump(text: &str, dir: &Path) -> Result<(u64, PathBuf)> {
    let Some((frame, file)) = text.split_once('=') else {
        bail!("`{text}` is not FRAME=FILE");
    };
    let frame = frame
        .trim()
        .parse()
        .map_err(|_| anyhow!("`{text}` does not start with a frame number"))?;
    if file.is_empty() {
        bail!("`{text}` names no file");
    }
    Ok((frame, dir.join(file)))
}

#[cfg(test)]
mod tests {
    use vkrgss_proto::{Button, InputSnapshot};

    use super::*;

    #[test]
    fn a_game_without_the_file_has_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Options::read(dir.path()).unwrap(), Options::default());
    }

    #[test]
    fn the_keys_of_a_run_are_read_with_paths_relative_to_the_file() {
        let dir = Path::new("/files/game");
        let text = r#"
            builtin_scripts = "../rpg_builtin.rb"
            font_dirs = ["fonts"]
            font_own_metrics = true
            audio = false
            seed = 7
            frames = 91
            input = "60:RIGHT, 100:"
            dump = ["4=4.png", "90=90.png"]
            [rtp]
            Standard = "rtp"
        "#;
        let options = Options::parse(text, dir).unwrap();
        assert_eq!(
            options,
            Options {
                rtps: [("Standard".to_owned(), dir.join("rtp"))].into(),
                builtin_scripts: Some(dir.join("../rpg_builtin.rb")),
                font_dirs: vec![dir.join("fonts")],
                font_own_metrics: true,
                no_audio: true,
                seed: Some(7),
                frames: Some(91),
                input: Some(InputScript::parse("60:RIGHT, 100:").unwrap()),
                dumps: vec![(4, dir.join("4.png")), (90, dir.join("90.png"))],
                ..Options::default()
            }
        );
        let input = options.input.unwrap();
        assert_eq!(input.held_at(70), InputSnapshot::EMPTY.with(Button::Right));
    }

    #[test]
    fn an_input_script_can_be_a_file_beside_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("keys.txt"), "2:C\n4:\n").unwrap();
        std::fs::write(dir.path().join(FILE), "input = \"@keys.txt\"").unwrap();
        let options = Options::read(dir.path()).unwrap();
        assert_eq!(options.input, Some(InputScript::parse("2:C, 4:").unwrap()));
    }

    #[test]
    fn a_key_that_does_not_apply_on_android_is_an_error_naming_the_file_and_the_key() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE), "headless = true").unwrap();
        let message = format!("{:#}", Options::read(dir.path()).unwrap_err());
        assert!(message.contains(FILE), "{message}");
        assert!(message.contains("headless"), "{message}");
    }

    #[test]
    fn malformed_dumps_and_scripts_are_errors_naming_the_key() {
        let dir = Path::new("/game");
        for (text, key) in [
            (r#"dump = ["4"]"#, "dump"),
            (r#"dump = ["x=4.png"]"#, "dump"),
            (r#"dump = ["4="]"#, "dump"),
            (r#"input = "2:SIDEWAYS""#, "input"),
        ] {
            let message = format!("{:#}", Options::parse(text, dir).unwrap_err());
            assert!(message.starts_with(key), "{text}: {message}");
        }
    }
}
