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
use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use vkxp_host::{GamepadMap, KeyMap};

use crate::args::{self, Input, Options};

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
    font_own_metrics: Option<bool>,
    encoding: Option<String>,
    debug: Option<bool>,
    allow_process_functions: Option<bool>,
    log_p: Option<bool>,
    /// The scaling filter, by its name.
    filter: Option<String>,
    fullscreen: Option<bool>,
    audio: Option<bool>,
    soundfont: Option<PathBuf>,
    seed: Option<u64>,
    preload: Option<Vec<PathBuf>>,
    record: Option<PathBuf>,
    replay: Option<PathBuf>,
    headless: Option<bool>,
    frames: Option<u64>,
    /// An input script or `@FILE`, as `--input` takes it.
    #[serde(default, deserialize_with = "input")]
    input: Option<Input>,
    /// Frames to write, each as `FRAME=FILE`, as `--dump` takes it.
    #[serde(default, deserialize_with = "dumps")]
    dump: Option<Vec<(u64, PathBuf)>>,
    /// The keys of buttons, by the button's name.
    #[serde(default)]
    keys: BTreeMap<String, Vec<String>>,
    /// The gamepad buttons of buttons, by the button's name.
    #[serde(default)]
    gamepad: BTreeMap<String, Vec<String>>,
}

fn input<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Input>, D::Error> {
    let text = String::deserialize(deserializer)?;
    args::input(&text).map(Some).map_err(D::Error::custom)
}

fn dumps<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<(u64, PathBuf)>>, D::Error> {
    let texts = Vec::<String>::deserialize(deserializer)?;
    let dumps: Result<Vec<_>, String> = texts.iter().map(|text| args::dump(text)).collect();
    dumps.map(Some).map_err(D::Error::custom)
}

/// A table of buttons, whose names were accepted in either case, with every button under its
/// one name, so that an entry of one file takes the place of the same button's entry in
/// another however each spells it.
fn by_button(table: BTreeMap<String, Vec<String>>) -> Result<BTreeMap<String, Vec<String>>> {
    let mut buttons = BTreeMap::new();
    for (name, sources) in table {
        // The names of the buttons are upper case.
        if buttons.insert(name.to_ascii_uppercase(), sources).is_some() {
            return Err(anyhow!("the button `{name}` is listed more than once"));
        }
    }
    Ok(buttons)
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
        if let Some(name) = &file.filter {
            args::filter(name)
                .map_err(|error| anyhow!("filter: {error}"))
                .with_context(named)?;
        }
        KeyMap::with_overrides(&file.keys)
            .map_err(|error| anyhow!("[keys]: {error}"))
            .with_context(named)?;
        GamepadMap::with_overrides(&file.gamepad)
            .map_err(|error| anyhow!("[gamepad]: {error}"))
            .with_context(named)?;
        file.keys = by_button(file.keys)
            .map_err(|error| anyhow!("[keys]: {error}"))
            .with_context(named)?;
        file.gamepad = by_button(file.gamepad)
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
            .chain(file.preload.iter_mut().flatten())
            .chain(&mut file.record)
            .chain(&mut file.replay)
            .chain(file.input.iter_mut().filter_map(|input| match input {
                Input::File(path) => Some(path),
                Input::Script(_) => None,
            }))
            .chain(file.dump.iter_mut().flatten().map(|(_, path)| path));
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
            font_own_metrics: over.font_own_metrics.or(self.font_own_metrics),
            encoding: over.encoding.or(self.encoding),
            debug: over.debug.or(self.debug),
            allow_process_functions: over
                .allow_process_functions
                .or(self.allow_process_functions),
            log_p: over.log_p.or(self.log_p),
            filter: over.filter.or(self.filter),
            fullscreen: over.fullscreen.or(self.fullscreen),
            audio: over.audio.or(self.audio),
            soundfont: over.soundfont.or(self.soundfont),
            seed: over.seed.or(self.seed),
            preload: over.preload.or(self.preload),
            record: over.record.or(self.record),
            replay: over.replay.or(self.replay),
            headless: over.headless.or(self.headless),
            frames: over.frames.or(self.frames),
            input: over.input.or(self.input),
            dump: over.dump.or(self.dump),
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
    options.font_own_metrics |= file.font_own_metrics == Some(true);
    if options.encoding.is_none() {
        let encoding = file.encoding.as_deref().map(args::encoding).transpose();
        options.encoding = encoding.map_err(|error| anyhow!("encoding: {error}"))?;
    }
    options.debug |= file.debug == Some(true);
    options.process_functions |= file.allow_process_functions == Some(true);
    options.log_p |= file.log_p == Some(true);
    if options.filter.is_none() {
        let filter = file.filter.as_deref().map(args::filter).transpose();
        options.filter = filter.map_err(|error| anyhow!("filter: {error}"))?;
    }
    options.fullscreen |= file.fullscreen == Some(true);
    options.no_audio |= file.audio == Some(false);
    options.soundfont = options.soundfont.or(file.soundfont);
    options.seed = options.seed.or(file.seed);
    if options.preload.is_empty() {
        options.preload = file.preload.unwrap_or_default();
    }
    options.record = options.record.or(file.record);
    options.replay = options.replay.or(file.replay);
    options.headless |= file.headless == Some(true);
    options.frames = options.frames.or(file.frames);
    options.input = options.input.or(file.input);
    if options.dumps.is_empty() {
        options.dumps = file.dump.unwrap_or_default();
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
    use vkxp_host::Filter;
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
            ("headless = \"yes\"", "headless"),
            ("frames = -3", "frames"),
            ("record = 1", "record"),
            ("replay = []", "replay"),
            ("input = 2", "input"),
            ("dump = \"1=a.png\"", "dump"),
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
            ("filter = \"cubic\"", "cubic"),
            ("input = \"2:SIDEWAYS\"", "SIDEWAYS"),
            ("dump = [\"1=a.png\", \"three=b.png\"]", "three"),
            ("dump = [\"7\"]", "FRAME=FILE"),
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
             font_own_metrics = true\n\
             encoding = \"Shift-JIS\"\n\
             debug = true\n\
             allow_process_functions = true\n\
             log_p = true\n\
             filter = \"sharp\"\n\
             fullscreen = true\n\
             audio = false\n\
             soundfont = \"sf/gm.sf2\"\n\
             seed = 7\n\
             preload = [\"one.rb\"]\n\
             record = \"session.txt\"\n\
             headless = true\n\
             frames = 10\n\
             input = \"@keys.txt\"\n\
             dump = [\"1=a.png\", \"8=/abs/b=c.png\"]\n\
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
                font_own_metrics: true,
                encoding: Some(Encoding::ShiftJis),
                debug: true,
                process_functions: true,
                log_p: true,
                filter: Some(Filter::Sharp),
                fullscreen: true,
                no_audio: true,
                soundfont: Some(base.join("sf/gm.sf2")),
                seed: Some(7),
                preload: vec![base.join("one.rb")],
                record: Some(base.join("session.txt")),
                headless: true,
                frames: Some(10),
                input: Some(Input::File(base.join("keys.txt"))),
                dumps: vec![(1, base.join("a.png")), (8, "/abs/b=c.png".into())],
                ..Options::default()
            }
        );
        assert_eq!(args::check(&settings.options), Ok(()));

        let files = Files::new().user("replay = \"session.txt\"\ninput = \"2:RIGHT, 6:\"");
        let options = files.settle(Options::default()).unwrap().options;
        assert_eq!(options.replay, Some(files.dir.path().join("session.txt")));
        let script = vkxp_host::InputScript::parse("2:RIGHT, 6:").unwrap();
        assert_eq!(options.input, Some(Input::Script(script)));
    }

    #[test]
    fn the_run_settings_are_layered_like_the_others() {
        let files = Files::new()
            .user(
                "headless = true\nframes = 1\ninput = \"1:UP\"\nrecord = \"user.txt\"\n\
                 dump = [\"1=user.png\"]\n",
            )
            .game("frames = 2\ninput = \"@keys.txt\"\ndump = [\"2=game.png\"]\n");
        let game = files.game_dir();
        let options = files.settle(Options::default()).unwrap().options;
        assert!(options.headless);
        assert_eq!(options.frames, Some(2));
        assert_eq!(options.input, Some(Input::File(game.join("keys.txt"))));
        assert_eq!(options.dumps, [(2, game.join("game.png"))]);
        assert_eq!(options.record, Some(files.dir.path().join("user.txt")));

        let script = vkxp_host::InputScript::parse("3:C").unwrap();
        let command_line = Options {
            frames: Some(3),
            input: Some(Input::Script(script)),
            dumps: vec![(3, "cli.png".into())],
            record: Some("cli.txt".into()),
            ..Options::default()
        };
        let options = files.settle(command_line.clone()).unwrap().options;
        assert!(options.headless);
        assert_eq!(options.frames, Some(3));
        assert_eq!(options.input, command_line.input);
        assert_eq!(options.dumps, command_line.dumps);
        assert_eq!(options.record, command_line.record);
    }

    /// Whether settings go together is asked of the final ones.
    #[test]
    fn settings_that_do_not_go_together_are_refused_wherever_they_come_from() {
        let check =
            |files: &Files, options: Options| args::check(&files.settle(options).unwrap().options);
        for text in ["frames = 3", "input = \"1:UP\"", "dump = [\"1=a.png\"]"] {
            let files = Files::new().user(text);
            let error = check(&files, Options::default()).unwrap_err();
            assert!(error.contains("--headless"), "{text}: {error}");
            // The command line may supply what the file leaves out, and the game's file too.
            let headless = Options {
                headless: true,
                ..Options::default()
            };
            assert_eq!(check(&files, headless), Ok(()), "{text}");
            let files = files.game("headless = true");
            assert_eq!(check(&files, Options::default()), Ok(()), "{text}");
        }

        // A file that sets headless lets the command line give the headless options.
        let files = Files::new().user("headless = true");
        let frames = Options {
            frames: Some(3),
            ..Options::default()
        };
        assert_eq!(check(&files, frames), Ok(()));

        let files = Files::new()
            .user("replay = \"a.txt\"")
            .game("record = \"b.txt\"");
        let error = check(&files, Options::default()).unwrap_err();
        assert!(error.contains("--record"), "{error}");
        assert!(error.contains("--replay"), "{error}");

        let files = Files::new().user("record = \"b.txt\"");
        let replay = Options {
            replay: Some("a.txt".into()),
            ..Options::default()
        };
        let error = check(&files, replay).unwrap_err();
        assert!(error.contains("--replay"), "{error}");

        let files = Files::new().user("headless = true\ninput = \"1:UP\"");
        let replay = Options {
            replay: Some("a.txt".into()),
            ..Options::default()
        };
        let error = check(&files, replay).unwrap_err();
        assert!(error.contains("--input"), "{error}");
    }

    /// Scenario "From the file".
    #[test]
    fn the_filter_of_a_file_is_used_unless_the_command_line_gives_one() {
        let files = Files::new().user("filter = \"linear\"");
        let settings = files.settle(Options::default()).unwrap();
        assert_eq!(settings.options.filter, Some(Filter::Linear));

        let nearest = Options {
            filter: Some(Filter::Nearest),
            ..Options::default()
        };
        let settings = files.settle(nearest).unwrap();
        assert_eq!(settings.options.filter, Some(Filter::Nearest));

        let files = Files::new();
        assert_eq!(
            files.settle(Options::default()).unwrap().options.filter,
            None
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

    /// A button is the same button however a file spells its name.
    #[test]
    fn the_game_file_overrides_a_button_spelled_in_another_case() {
        let table = |entries: &[(&str, &str)]| -> BTreeMap<String, Vec<String>> {
            entries
                .iter()
                .map(|(button, name)| ((*button).to_owned(), vec![(*name).to_owned()]))
                .collect()
        };
        for (user, game) in [("c", "C"), ("C", "c"), ("Shift", "sHIFT")] {
            let canonical = user.to_ascii_uppercase();
            let files = Files::new()
                .user(&format!(
                    "[keys]\n{user} = ['KeyJ']\n[gamepad]\n{user} = ['East']\n"
                ))
                .game(&format!(
                    "[keys]\n{game} = ['KeyK']\n[gamepad]\n{game} = ['North']\n"
                ));
            let settings = files.settle(Options::default()).unwrap();
            // Exactly the game's entry on top of the defaults: the user's is gone and the
            // buttons neither file mentions keep their default bindings.
            assert_eq!(
                settings.keys,
                KeyMap::with_overrides(&table(&[(&canonical, "KeyK")])).unwrap(),
                "{user} then {game}"
            );
            assert_ne!(settings.keys, KeyMap::default());
            assert_eq!(
                settings.pads,
                GamepadMap::with_overrides(&table(&[(&canonical, "North")])).unwrap(),
                "{user} then {game}"
            );
            assert_ne!(settings.pads, GamepadMap::default());
        }

        // Without a game file the user's entry stands, in either case.
        let files = Files::new().user("[keys]\nc = ['KeyJ']\n[gamepad]\nb = ['North']\n");
        let settings = files.settle(Options::default()).unwrap();
        assert_eq!(
            settings.keys,
            KeyMap::with_overrides(&table(&[("C", "KeyJ")])).unwrap()
        );
        assert_eq!(
            settings.pads,
            GamepadMap::with_overrides(&table(&[("B", "North")])).unwrap()
        );
    }

    #[test]
    fn a_button_listed_twice_in_one_file_is_an_error_naming_it() {
        for text in [
            "[keys]\nc = ['KeyJ']\nC = ['KeyK']\n",
            "[gamepad]\nc = ['East']\nC = ['North']\n",
        ] {
            let files = Files::new().user(text);
            let error = format!("{:#}", files.settle(Options::default()).unwrap_err());
            assert!(error.contains("user.toml"), "{error}");
            assert!(error.contains("`c`"), "{error}");
        }
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
        assert!(!options.font_own_metrics);
        assert_eq!(options.encoding, Some(Encoding::Gbk));
        assert!(!options.debug);
        assert!(!options.process_functions);
        assert!(!options.log_p);
        assert!(!options.no_audio);
        assert!(options.soundfont.is_some());
        assert_eq!(options.seed, Some(7));
        assert_eq!(options.preload, [base.join("patches/fonts.rb")]);
        assert!(options.headless);
        assert_eq!(options.frames, Some(600));
        let script = vkxp_host::InputScript::parse("40:C, 42:").unwrap();
        assert_eq!(options.input, Some(Input::Script(script)));
        assert_eq!(options.dumps, [(100, base.join("frame.png"))]);
        assert_eq!(options.record, Some(base.join("session.txt")));
        assert_eq!(options.replay, None);
        assert_eq!(args::check(options), Ok(()));
        assert_eq!(settings.keys, KeyMap::default());
        assert_eq!(settings.pads, GamepadMap::default());

        // Every setting and every button is in the example.
        let file = File::read(&files.user_file()).unwrap();
        assert_eq!(file.keys.len(), vkxp_proto::Button::ALL.len());
        assert!(file.audio.is_some() && file.debug.is_some());
        assert!(file.allow_process_functions.is_some());
        assert!(file.log_p.is_some());
        assert!(file.font_own_metrics.is_some());
        assert!(!file.gamepad.is_empty());

        // The one setting the example can only show as a comment, because it does not go
        // together with two of the others.
        let with_replay: String = example
            .lines()
            .filter(|line| !line.starts_with("record =") && !line.starts_with("input ="))
            .map(|line| line.strip_prefix("# ").unwrap_or(line).to_owned() + "\n")
            .collect();
        assert_ne!(with_replay, example);
        let files = Files::new().user(&with_replay);
        let options = files.settle(Options::default()).unwrap().options;
        assert_eq!(options.replay, Some(files.dir.path().join("session.txt")));
        assert_eq!(options.record, None);
        assert_eq!(options.input, None);
        assert_eq!(args::check(&options), Ok(()));
    }
}
