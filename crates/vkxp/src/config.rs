//! The configuration files of the program.
//!
//! Two files hold what the command line holds, in TOML: the user's, at
//! `$XDG_CONFIG_HOME/vkxp/config.toml` unless `--config` names another, and `vkxp.toml` in the
//! game directory. Both are optional. The game's file overrides the user's and the command
//! line overrides both; `--no-config` reads neither. Paths in a file are relative to the
//! directory of that file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf, absolute};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use vkxp_host::{GamepadMap, KeyMap};

use crate::args::{self, Options};

/// The name of the configuration file in a game directory.
pub const GAME_FILE: &str = "vkxp.toml";

/// What one configuration file holds. Everything is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    /// The directory of each run-time package, by its name.
    #[serde(default)]
    rtp: BTreeMap<String, PathBuf>,
    builtin_scripts: Option<PathBuf>,
    /// The device: an index or a part of a name.
    device: Option<String>,
    font_dirs: Option<Vec<PathBuf>>,
    encoding: Option<String>,
    debug: Option<bool>,
    allow_process_functions: Option<bool>,
    audio: Option<bool>,
    soundfont: Option<PathBuf>,
    seed: Option<u64>,
    preload: Option<Vec<PathBuf>>,
    /// The keys of buttons, by the button's name.
    #[serde(default)]
    keys: BTreeMap<String, Vec<String>>,
    /// The gamepad buttons of buttons, by the button's name.
    #[serde(default)]
    gamepad: BTreeMap<String, Vec<String>>,
}

impl File {
    /// Reads and checks the file at `path`. Every error names the file.
    pub fn read(path: &Path) -> Result<File> {
        let named = || format!("in the configuration file {}", path.display());
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read the configuration file {}", path.display()))?;
        let mut file: File = toml::from_str(&text).with_context(named)?;
        if let Some(name) = &file.encoding {
            args::encoding(name)
                .map_err(|error| anyhow!("encoding: {error}"))
                .with_context(named)?;
        }
        KeyMap::with_overrides(&file.keys)
            .map_err(|error| anyhow!("[keys]: {error}"))
            .with_context(named)?;
        GamepadMap::with_overrides(&file.gamepad)
            .map_err(|error| anyhow!("[gamepad]: {error}"))
            .with_context(named)?;

        // Paths are relative to the file.
        let dir = absolute(path)?
            .parent()
            .map(Path::to_owned)
            .unwrap_or_default();
        let paths = file
            .rtp
            .values_mut()
            .chain(&mut file.builtin_scripts)
            .chain(file.font_dirs.iter_mut().flatten())
            .chain(&mut file.soundfont)
            .chain(file.preload.iter_mut().flatten());
        for path in paths {
            *path = dir.join(&*path);
        }
        Ok(file)
    }

    /// This file with everything `over` sets taking the place of its own: whole values, and
    /// single entries of the three tables.
    fn overridden_by(mut self, over: File) -> File {
        self.rtp.extend(over.rtp);
        self.keys.extend(over.keys);
        self.gamepad.extend(over.gamepad);
        File {
            rtp: self.rtp,
            builtin_scripts: over.builtin_scripts.or(self.builtin_scripts),
            device: over.device.or(self.device),
            font_dirs: over.font_dirs.or(self.font_dirs),
            encoding: over.encoding.or(self.encoding),
            debug: over.debug.or(self.debug),
            allow_process_functions: over
                .allow_process_functions
                .or(self.allow_process_functions),
            audio: over.audio.or(self.audio),
            soundfont: over.soundfont.or(self.soundfont),
            seed: over.seed.or(self.seed),
            preload: over.preload.or(self.preload),
            keys: self.keys,
            gamepad: self.gamepad,
        }
    }
}

/// The user's file at its default location, if that can be told.
fn default_user_file() -> Option<PathBuf> {
    let variable = |name| std::env::var_os(name).filter(|value| !value.is_empty());
    let config = match (variable("XDG_CONFIG_HOME"), variable("HOME")) {
        (Some(config), _) => PathBuf::from(config),
        (None, Some(home)) => Path::new(&home).join(".config"),
        (None, None) => return None,
    };
    Some(config.join("vkxp").join("config.toml"))
}

/// Reads the configuration files of a run of the game in `game_dir`: the user's and the
/// game's, as far as they exist, as one. A file named with `--config` has to exist.
pub fn load(options: &Options, game_dir: &Path) -> Result<File> {
    if options.no_config {
        return Ok(File::default());
    }
    let user = match &options.config {
        Some(path) => Some(path.clone()),
        None => default_user_file().filter(|path| path.is_file()),
    };
    let game = Some(game_dir.join(GAME_FILE)).filter(|path| path.is_file());
    let mut settled = File::default();
    for path in user.iter().chain(&game) {
        settled = settled.overridden_by(File::read(path)?);
    }
    Ok(settled)
}

/// What a run is set up with: the command line over the configuration files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub options: Options,
    pub keys: KeyMap,
    pub pads: GamepadMap,
}

/// Puts the command line over what the configuration files hold. A list given on the command
/// line replaces the list of the files, and a run-time package given there the one of that
/// name.
pub fn settle(file: File, mut options: Options) -> Result<Settings> {
    for (name, dir) in file.rtp {
        options.rtps.entry(name).or_insert(dir);
    }
    options.builtin_scripts = options.builtin_scripts.or(file.builtin_scripts);
    options.device = options.device.or(file.device);
    if options.font_dirs.is_empty() {
        options.font_dirs = file.font_dirs.unwrap_or_default();
    }
    if options.encoding.is_none() {
        let encoding = file.encoding.as_deref().map(args::encoding).transpose();
        options.encoding = encoding.map_err(|error| anyhow!("encoding: {error}"))?;
    }
    options.debug |= file.debug == Some(true);
    options.process_functions |= file.allow_process_functions == Some(true);
    options.no_audio |= file.audio == Some(false);
    options.soundfont = options.soundfont.or(file.soundfont);
    options.seed = options.seed.or(file.seed);
    if options.preload.is_empty() {
        options.preload = file.preload.unwrap_or_default();
    }
    Ok(Settings {
        options,
        keys: KeyMap::with_overrides(&file.keys)?,
        pads: GamepadMap::with_overrides(&file.gamepad)?,
    })
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use vkxp_vfs::Encoding;

    use super::*;

    /// A directory with a user file, empty until it is written, and a game directory, whose
    /// file is written on demand. The user file is always named with `--config`, so that the
    /// environment of the test decides nothing.
    struct Files {
        dir: TempDir,
    }

    impl Files {
        fn new() -> Files {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join("game")).unwrap();
            Files { dir }.user("")
        }

        fn user_file(&self) -> PathBuf {
            self.dir.path().join("user.toml")
        }

        fn game_dir(&self) -> PathBuf {
            self.dir.path().join("game")
        }

        fn user(self, text: &str) -> Files {
            std::fs::write(self.user_file(), text).unwrap();
            self
        }

        fn game(self, text: &str) -> Files {
            std::fs::write(self.game_dir().join(GAME_FILE), text).unwrap();
            self
        }

        /// The settings of a run with these command-line options besides `--config`.
        fn settle(&self, options: Options) -> Result<Settings> {
            let options = Options {
                config: Some(self.user_file()),
                ..options
            };
            settle(load(&options, &self.game_dir())?, options)
        }
    }

    /// Scenario "Precedence".
    #[test]
    fn the_game_file_overrides_the_user_file_and_the_command_line_both() {
        let files = Files::new().user("seed = 1").game("seed = 2");
        let settings = files.settle(Options::default()).unwrap();
        assert_eq!(settings.options.seed, Some(2));

        let with_seed = Options {
            seed: Some(3),
            ..Options::default()
        };
        assert_eq!(files.settle(with_seed).unwrap().options.seed, Some(3));

        let files = Files::new().user("seed = 1");
        let settings = files.settle(Options::default()).unwrap();
        assert_eq!(settings.options.seed, Some(1));
    }

    /// Scenario "Ignored", as far as the files go.
    #[test]
    fn no_config_reads_neither_file() {
        let files = Files::new().user("seed = ").game("no_such_key = 1");
        let options = Options {
            no_config: true,
            ..Options::default()
        };
        let file = load(&options, &files.game_dir()).unwrap();
        assert_eq!(file, File::default());
        assert_eq!(
            settle(file, options.clone()).unwrap(),
            Settings {
                options,
                keys: KeyMap::default(),
                pads: GamepadMap::default(),
            }
        );
    }

    #[test]
    fn without_files_the_command_line_stands() {
        let dir = tempfile::tempdir().unwrap();
        let options = Options {
            // A default location without a file, whatever the environment of the test is.
            config: None,
            no_config: false,
            seed: Some(5),
            ..Options::default()
        };
        let file = File::default();
        let settings = settle(file, options.clone()).unwrap();
        assert_eq!(settings.options, options);
        // A game directory without a file is read as nothing.
        let named = Options {
            config: Some(dir.path().join("absent.toml")),
            ..Options::default()
        };
        let error = load(&named, dir.path()).unwrap_err();
        assert!(format!("{error:#}").contains("absent.toml"), "{error:#}");
    }

    /// Scenario "Unknown key".
    #[test]
    fn an_unknown_key_is_an_error_naming_the_file_and_the_key() {
        for text in ["sead = 1", "[keys]\nC = [\"KeyJ\"]\n[mouse]\nC = []"] {
            let files = Files::new().user(text);
            let error = format!("{:#}", files.settle(Options::default()).unwrap_err());
            assert!(error.contains("user.toml"), "{error}");
            let key = if text.starts_with("sead") {
                "sead"
            } else {
                "mouse"
            };
            assert!(error.contains(key), "{error}");
        }

        let files = Files::new().game("seed = 1\nvolume = 3");
        let error = format!("{:#}", files.settle(Options::default()).unwrap_err());
        assert!(error.contains(GAME_FILE), "{error}");
        assert!(error.contains("volume"), "{error}");
    }

    #[test]
    fn a_value_of_the_wrong_type_is_an_error_naming_the_file_and_the_key() {
        for (text, key) in [
            ("seed = \"seven\"", "seed"),
            ("seed = -1", "seed"),
            ("audio = 1", "audio"),
            ("preload = \"one.rb\"", "preload"),
            ("[rtp]\nStandard = 3", "Standard"),
            ("[keys]\nC = \"KeyJ\"", "C"),
        ] {
            let files = Files::new().user(text);
            let error = format!("{:#}", files.settle(Options::default()).unwrap_err());
            assert!(error.contains("user.toml"), "{text}: {error}");
            assert!(error.contains(key), "{text}: {error}");
        }
    }

    #[test]
    fn a_file_that_does_not_parse_is_an_error_naming_it() {
        let files = Files::new().user("seed = = 1");
        let error = format!("{:#}", files.settle(Options::default()).unwrap_err());
        assert!(error.contains("user.toml"), "{error}");
    }

    #[test]
    fn unknown_names_in_a_file_are_errors_naming_the_file_and_the_name() {
        for (text, name) in [
            ("[keys]\nC = [\"Jay\"]", "Jay"),
            ("[keys]\nJUMP = [\"KeyJ\"]", "JUMP"),
            ("[gamepad]\nC = [\"Middle\"]", "Middle"),
            ("encoding = \"latin1\"", "latin1"),
        ] {
            let files = Files::new().game(text);
            let error = format!("{:#}", files.settle(Options::default()).unwrap_err());
            assert!(error.contains(GAME_FILE), "{text}: {error}");
            assert!(error.contains(name), "{text}: {error}");
        }
    }

    #[test]
    fn every_setting_is_read_and_paths_are_relative_to_the_file() {
        let files = Files::new().user(
            "builtin_scripts = \"rpg.rb\"\n\
             device = \"RADV\"\n\
             font_dirs = [\"fonts\", \"/abs/fonts\"]\n\
             encoding = \"Shift-JIS\"\n\
             debug = true\n\
             allow_process_functions = true\n\
             audio = false\n\
             soundfont = \"sf/gm.sf2\"\n\
             seed = 7\n\
             preload = [\"one.rb\"]\n\
             [rtp]\n\
             Standard = \"rtp/standard\"\n",
        );
        let base = files.dir.path();
        let settings = files.settle(Options::default()).unwrap();
        assert_eq!(
            settings.options,
            Options {
                config: Some(files.user_file()),
                rtps: BTreeMap::from([("Standard".to_owned(), base.join("rtp/standard"))]),
                builtin_scripts: Some(base.join("rpg.rb")),
                device: Some("RADV".to_owned()),
                font_dirs: vec![base.join("fonts"), "/abs/fonts".into()],
                encoding: Some(Encoding::ShiftJis),
                debug: true,
                process_functions: true,
                no_audio: true,
                soundfont: Some(base.join("sf/gm.sf2")),
                seed: Some(7),
                preload: vec![base.join("one.rb")],
                ..Options::default()
            }
        );
    }

    #[test]
    fn the_command_line_overrides_every_setting() {
        let files = Files::new().user(
            "builtin_scripts = \"rpg.rb\"\n\
             device = \"RADV\"\n\
             font_dirs = [\"fonts\"]\n\
             encoding = \"gbk\"\n\
             soundfont = \"gm.sf2\"\n\
             preload = [\"one.rb\"]\n\
             [rtp]\n\
             Standard = \"standard\"\n\
             Extra = \"extra\"\n",
        );
        let options = Options {
            config: Some(files.user_file()),
            rtps: BTreeMap::from([("Standard".to_owned(), "cli/standard".into())]),
            builtin_scripts: Some("cli.rb".into()),
            device: Some("0".to_owned()),
            font_dirs: vec!["cli-fonts".into()],
            encoding: Some(Encoding::Big5),
            soundfont: Some("cli.sf2".into()),
            preload: vec!["cli-preload.rb".into()],
            ..Options::default()
        };
        let mut expected = options.clone();
        expected
            .rtps
            .insert("Extra".to_owned(), files.dir.path().join("extra"));
        assert_eq!(files.settle(options).unwrap().options, expected);
    }

    #[test]
    fn the_tables_are_layered_per_entry() {
        let files = Files::new()
            .user(
                "debug = true\n\
                 [rtp]\nStandard = \"/user/standard\"\nExtra = \"/user/extra\"\n\
                 [keys]\nC = [\"KeyJ\"]\nB = [\"KeyK\"]\n\
                 [gamepad]\nC = [\"East\"]\n",
            )
            .game(
                "debug = false\n\
                 [rtp]\nStandard = \"/game/standard\"\n\
                 [keys]\nC = [\"KeyL\"]\n\
                 [gamepad]\nB = [\"South\"]\n",
            );
        let settings = files.settle(Options::default()).unwrap();
        assert!(!settings.options.debug);
        assert_eq!(
            settings.options.rtps,
            BTreeMap::from([
                ("Standard".to_owned(), "/game/standard".into()),
                ("Extra".to_owned(), "/user/extra".into()),
            ])
        );
        let table = |entries: &[(&str, &str)]| -> BTreeMap<String, Vec<String>> {
            entries
                .iter()
                .map(|(button, name)| ((*button).to_owned(), vec![(*name).to_owned()]))
                .collect()
        };
        assert_eq!(
            settings.keys,
            KeyMap::with_overrides(&table(&[("C", "KeyL"), ("B", "KeyK")])).unwrap()
        );
        assert_eq!(
            settings.pads,
            GamepadMap::with_overrides(&table(&[("C", "East"), ("B", "South")])).unwrap()
        );
    }

    /// The example of the README is a complete file, and its two maps are the defaults.
    #[test]
    fn the_example_of_the_readme_loads() {
        let readme = concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md");
        let readme = std::fs::read_to_string(readme).unwrap();
        let (_, rest) = readme
            .split_once("```toml\n")
            .expect("the README has an example");
        let (example, _) = rest.split_once("```").unwrap();

        let files = Files::new().user(example);
        let settings = files.settle(Options::default()).unwrap();
        let options = &settings.options;
        let base = files.dir.path();
        assert_eq!(
            options.rtps,
            BTreeMap::from([("Standard".to_owned(), "/home/me/rtp/Standard".into())])
        );
        assert_eq!(options.builtin_scripts, Some(base.join("rpg_builtin.rb")));
        assert_eq!(options.device.as_deref(), Some("RADV"));
        assert_eq!(options.font_dirs, [base.join("fonts")]);
        assert_eq!(options.encoding, Some(Encoding::Gbk));
        assert!(!options.debug);
        assert!(!options.process_functions);
        assert!(!options.no_audio);
        assert!(options.soundfont.is_some());
        assert_eq!(options.seed, Some(7));
        assert_eq!(options.preload, [base.join("patches/fonts.rb")]);
        assert_eq!(settings.keys, KeyMap::default());
        assert_eq!(settings.pads, GamepadMap::default());

        // Every setting and every button is in the example.
        let file = File::read(&files.user_file()).unwrap();
        assert_eq!(file.keys.len(), vkxp_proto::Button::ALL.len());
        assert!(file.audio.is_some() && file.debug.is_some());
        assert!(file.allow_process_functions.is_some());
        assert!(!file.gamepad.is_empty());
    }
}
