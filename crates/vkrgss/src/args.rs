//! The command line of the program.

use std::collections::BTreeMap;
use std::path::PathBuf;

use vkrgss_host::{Filter, InputScript, StateSchedule};
use vkrgss_vfs::{Encoding, Runtime};

/// What `--help` prints, and what is printed with every command-line error.
pub(crate) const USAGE: &str = "\
usage: vkrgss [GAME_DIR] [OPTIONS]
       vkrgss [--runtime RUNTIME] [--sandbox on|off] [--allow-process-functions]
              [--verbose] --script FILE [ARG...]

  GAME_DIR                  the game directory; the current directory if absent
  --config FILE             the user's configuration file;
                            default: $XDG_CONFIG_HOME/vkrgss/config.toml
  --no-config               read neither that file nor vkrgss.toml in the game directory
  --runtime RUNTIME         the runtime of the game: rgss1 (RPG Maker XP), rgss2 (RPG
                            Maker VX) or rgss3 (RPG Maker VX Ace);
                            default: what Library and Scripts of Game.ini say
  --rtp NAME=DIR            the directory of the run-time package NAME; repeatable
  --builtin-scripts FILE    the built-in scripts file, made by `cargo xtask extract-rpg`;
                            default: $XDG_DATA_HOME/vkrgss/rpg_builtin.rb,
                            $XDG_DATA_HOME/vkrgss/rpg_builtin_rgss2.rb for rgss2 and
                            $XDG_DATA_HOME/vkrgss/rpg_builtin_rgss3.rb for rgss3
  --device INDEX|NAME       the Vulkan device, by index or by a part of its name
  --font-dir DIR            a directory of fonts, searched before the system's; repeatable
  --font-own-metrics        size every font by its own line box, a replacement included
  --encoding ENCODING       the text encoding of Game.ini: utf-8, gbk, shift-jis or big5
  --debug                   debug mode: sets $DEBUG to true for RGSS1; for RGSS2 sets
                            $TEST to true and leaves $DEBUG false
  --allow-process-functions let scripts start processes (system, exec and the like)
  --log-p                   p only writes to the error stream and shows no message panel
  --filter FILTER           how the frame is scaled into the window: auto, nearest,
                            linear or sharp; default: auto, which is nearest at a
                            whole-number scale and sharp otherwise
  --fullscreen              start in borderless fullscreen; Alt+Enter or F11 toggles
  --present-stats           write the intervals between the frames shown to the error
                            stream, every 600 frames and at exit
  --frame-stats             write where the frames spend their time to the error stream,
                            per stage: every 600 frames and at exit in a window, once
                            at the end of a headless run
  --verbose                 also write informational and debug messages to the error
                            stream; the environment variable VKRGSS_LOG sets a level
                            (off, error, warn, info, debug or trace) or a filter such
                            as vkrgss_host=trace, and comes before this option
  --no-audio                run without sound: no audio device is opened
  --soundfont FILE          the SoundFont that MIDI music is played with;
                            default: the first that exists of
                            /usr/share/soundfonts/default.sf2,
                            /usr/share/soundfonts/FluidR3_GM.sf2 and
                            /usr/share/sounds/sf2/FluidR3_GM.sf2
  --seed N                  the seed of the random number generator; default: 0 in a
                            headless run, the seed of the recording in a replay, and
                            one taken from the clock otherwise
  --preload FILE            a Ruby file evaluated before the game's scripts; repeatable
  --record FILE             write the input of every frame and the seed to FILE
  --replay FILE             play the input of a recording back instead of the keyboard
  --guest thread|process    where the game's scripts run: on a thread of the program or
                            in a process of their own, which save states need (F3
                            saves, F4 loads, with Shift they choose the slot);
                            default: process in a window, thread in a headless run;
                            a game of rgss3 runs in a process only; the process is
                            one of the guest executable beside the program,
                            vkrgss-guest-rgss12 for rgss1 and rgss2 and
                            vkrgss-guest-rgss3 for rgss3; the environment variable
                            VKRGSS_GUEST sets it too, and comes after this option
  --sandbox on|off          whether a guest process confines itself before the first
                            script runs: it then writes only below the game
                            directory, executes nothing and opens no TCP connection;
                            default: on; the environment variable VKRGSS_SANDBOX sets
                            it too, and comes after this option
  --script FILE [ARG...]    run the Ruby file FILE with the interpreter of the runtime,
                            without a game, a window or a device; the arguments after
                            FILE are ARGV of the script; default runtime: rgss3; only
                            --runtime, --sandbox, --allow-process-functions and
                            --verbose go before it
  --headless                run without a window, on a virtual clock
  --frames N                headless: stop after N frames
  --input SCRIPT|@FILE      headless: held buttons, for example \"2:RIGHT, 6:\"
  --dump FRAME=FILE         headless: write frame FRAME as a PNG file; repeatable
  --states SCHEDULE         headless, with --guest process: save and load states at
                            given frames, for example \"120:save 0, 200:load 0\"
  --version, -V             print the name and the version and exit
  --help, -h                print this text and exit";

/// What `--version` prints: the name of the program and the version of its package.
pub(crate) const VERSION: &str = concat!("vkrgss ", env!("CARGO_PKG_VERSION"));

/// Where the input script of a headless run comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Input {
    /// A script given on the command line.
    Script(InputScript),
    /// A file that holds the script.
    File(PathBuf),
}

/// Where the guest runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuestForm {
    /// On a thread of the program.
    Thread,
    /// In a child process.
    Process,
}

/// The environment variable that says where the guest runs.
pub(crate) const GUEST_VARIABLE: &str = "VKRGSS_GUEST";

/// The environment variable that says whether a guest process confines itself.
pub(crate) const SANDBOX_VARIABLE: &str = "VKRGSS_SANDBOX";

/// An environment variable of the program as it was read: its value, if it is set and not
/// empty, and the name it was read under.
///
/// Up to version 0.3.0 the program was named vkxp. For one version, a variable a player sets
/// that is not set is read under its former name, `VKXP_` in the place of `VKRGSS_`, if that
/// one is set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Variable {
    /// The name of the variable.
    pub name: &'static str,
    /// The value, if the variable is set and not empty under either name.
    pub value: Option<String>,
    /// The former name, if the value was read under it.
    pub former: Option<String>,
}

impl Variable {
    /// Reads the variable `name` of the environment of the program.
    pub(crate) fn read(name: &'static str) -> Variable {
        Variable::of(name, |name| std::env::var(name).ok())
    }

    /// Reads the variable `name` with `lookup`, which gives the value of a variable by its
    /// name.
    pub(crate) fn of(name: &'static str, lookup: impl Fn(&str) -> Option<String>) -> Variable {
        let set = |name: &str| lookup(name).filter(|value| !value.is_empty());
        if let Some(value) = set(name) {
            return Variable {
                name,
                value: Some(value),
                former: None,
            };
        }
        let former = name.replacen("VKRGSS_", "VKXP_", 1);
        match set(&former) {
            Some(value) => Variable {
                name,
                value: Some(value),
                former: Some(former),
            },
            None => Variable {
                name,
                value: None,
                former: None,
            },
        }
    }

    /// The name the value was read under.
    pub(crate) fn read_as(&self) -> &str {
        self.former.as_deref().unwrap_or(self.name)
    }

    /// Writes a warning event if the value was read under the former name. Called once the
    /// subscriber is installed.
    pub(crate) fn warn_if_former(&self) {
        if let Some(former) = &self.former {
            tracing::warn!(
                "read {former} under the former name of the program; the variable is named {} \
                 now",
                self.name
            );
        }
    }
}

/// A Ruby file that runs without a game, and its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScriptFile {
    /// The file as it was given.
    pub file: PathBuf,
    /// The arguments after it, which are `ARGV` of the script.
    pub arguments: Vec<String>,
}

/// The parsed command line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Options {
    /// The game directory, if one was given.
    pub game_dir: Option<PathBuf>,
    /// The user's configuration file, instead of the default location.
    pub config: Option<PathBuf>,
    /// Whether no configuration file is read.
    pub no_config: bool,
    /// The runtime of the game, if one was given; otherwise `Game.ini` tells it.
    pub runtime: Option<Runtime>,
    /// The directory of each run-time package, by its name.
    pub rtps: BTreeMap<String, PathBuf>,
    pub builtin_scripts: Option<PathBuf>,
    /// The device as given: an index or a part of a name.
    pub device: Option<String>,
    /// Font directories, in the order given.
    pub font_dirs: Vec<PathBuf>,
    /// Whether a font that replaces another keeps its own line box.
    pub font_own_metrics: bool,
    pub encoding: Option<Encoding>,
    pub debug: bool,
    pub process_functions: bool,
    /// Whether `p` is log-only.
    pub log_p: bool,
    /// The filter that scales the frame into the window, if one was given.
    pub filter: Option<Filter>,
    /// Whether the window starts in borderless fullscreen.
    pub fullscreen: bool,
    /// Whether the intervals between the frames shown are reported.
    pub present_stats: bool,
    /// Whether the frame time statistics are reported.
    pub frame_stats: bool,
    /// Whether informational and debug messages are written too.
    pub verbose: bool,
    /// Whether the audio output stays closed.
    pub no_audio: bool,
    /// The SoundFont for MIDI, instead of the system's.
    pub soundfont: Option<PathBuf>,
    /// The seed of the random number generator, if one was given.
    pub seed: Option<u64>,
    /// Preload scripts, in the order given.
    pub preload: Vec<PathBuf>,
    /// The file the input is recorded into.
    pub record: Option<PathBuf>,
    /// The recording that is played back.
    pub replay: Option<PathBuf>,
    /// Where the guest runs, if that was given.
    pub guest: Option<GuestForm>,
    /// Whether a guest process confines itself, if that was given.
    pub sandbox: Option<bool>,
    pub headless: bool,
    pub frames: Option<u64>,
    pub input: Option<Input>,
    /// Frames to write as PNG files, by frame number.
    pub dumps: Vec<(u64, PathBuf)>,
    /// The states to save and to load at given frames of a headless run.
    pub states: StateSchedule,
    /// The script that runs instead of a game, if `--script` was given.
    pub script: Option<ScriptFile>,
    /// Whether only the version is printed.
    pub version: bool,
    /// Whether only the usage text is printed.
    pub help: bool,
}

/// `NAME=VALUE` with neither part empty.
fn pair<'a>(text: &'a str, form: &str) -> Result<(&'a str, &'a str), String> {
    text.split_once('=')
        .filter(|(name, value)| !name.is_empty() && !value.is_empty())
        .ok_or_else(|| format!("`{text}` is not of the form {form}"))
}

/// An input script as it is given: the script itself, or `@FILE` for a file that holds it.
pub(crate) fn input(text: &str) -> Result<Input, String> {
    match text.strip_prefix('@') {
        Some(file) => Ok(Input::File(PathBuf::from(file))),
        None => InputScript::parse(text)
            .map(Input::Script)
            .map_err(|error| error.to_string()),
    }
}

/// A frame to dump as it is given: `FRAME=FILE`.
pub(crate) fn dump(text: &str) -> Result<(u64, PathBuf), String> {
    let (frame, file) = pair(text, "FRAME=FILE")?;
    let frame = frame
        .parse()
        .map_err(|_| format!("`{frame}` is not a frame number"))?;
    Ok((frame, PathBuf::from(file)))
}

/// The encoding with this name, in either case. The error says what the names are.
pub(crate) fn encoding(text: &str) -> Result<Encoding, String> {
    match text.to_ascii_lowercase().as_str() {
        "utf-8" => Ok(Encoding::Utf8),
        "gbk" => Ok(Encoding::Gbk),
        "shift-jis" => Ok(Encoding::ShiftJis),
        "big5" => Ok(Encoding::Big5),
        _ => Err(format!(
            "`{text}` is not one of utf-8, gbk, shift-jis, big5"
        )),
    }
}

/// The runtime with this name, in either case. The error says what the names are.
pub(crate) fn runtime(text: &str) -> Result<Runtime, String> {
    Runtime::ALL
        .into_iter()
        .find(|runtime| text.eq_ignore_ascii_case(runtime.name()))
        .ok_or_else(|| format!("`{text}` is not one of rgss1, rgss2, rgss3"))
}

/// The scaling filter with this name, in either case. The error says what the names are.
pub(crate) fn filter(text: &str) -> Result<Filter, String> {
    match text.to_ascii_lowercase().as_str() {
        "auto" => Ok(Filter::Auto),
        "nearest" => Ok(Filter::Nearest),
        "linear" => Ok(Filter::Linear),
        "sharp" => Ok(Filter::Sharp),
        _ => Err(format!(
            "`{text}` is not one of auto, nearest, linear, sharp"
        )),
    }
}

/// The form of the guest with this name, in either case. The error says what the names are.
pub(crate) fn guest_form(text: &str) -> Result<GuestForm, String> {
    match text.to_ascii_lowercase().as_str() {
        "thread" => Ok(GuestForm::Thread),
        "process" => Ok(GuestForm::Process),
        _ => Err(format!("`{text}` is not one of thread, process")),
    }
}

/// The form of the guest that the value of [`GUEST_VARIABLE`] asks for; none when the
/// variable is not set. The error is a message for the user that names the variable.
pub(crate) fn guest_of_environment(variable: &Variable) -> Result<Option<GuestForm>, String> {
    variable
        .value
        .as_deref()
        .map(guest_form)
        .transpose()
        .map_err(|error| format!("{}: {error}", variable.read_as()))
}

/// Whether a guest process confines itself, by the word for it, in either case. The error
/// says what the words are.
pub(crate) fn sandbox(text: &str) -> Result<bool, String> {
    match text.to_ascii_lowercase().as_str() {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!("`{text}` is not one of on, off")),
    }
}

/// What the value of [`SANDBOX_VARIABLE`] asks for; nothing when the variable is not set.
/// The error is a message for the user that names the variable.
pub(crate) fn sandbox_of_environment(variable: &Variable) -> Result<Option<bool>, String> {
    variable
        .value
        .as_deref()
        .map(sandbox)
        .transpose()
        .map_err(|error| format!("{}: {error}", variable.read_as()))
}

/// Reads the arguments that follow the program name. The error is a message for the user,
/// to be printed with [`USAGE`]. Whether the options go together is for [`check`] to say,
/// once the configuration files have been put under them.
pub(crate) fn parse(arguments: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .ok_or_else(|| format!("{argument} needs a value"))
        };
        match argument.as_str() {
            "--rtp" => {
                let (name, dir) =
                    pair(value()?, "NAME=DIR").map_err(|error| format!("--rtp: {error}"))?;
                options.rtps.insert(name.to_owned(), PathBuf::from(dir));
            }
            "--runtime" => {
                let runtime = runtime(value()?).map_err(|error| format!("--runtime: {error}"))?;
                options.runtime = Some(runtime);
            }
            "--builtin-scripts" => options.builtin_scripts = Some(PathBuf::from(value()?)),
            "--device" => options.device = Some(value()?.clone()),
            "--font-dir" => options.font_dirs.push(PathBuf::from(value()?)),
            "--font-own-metrics" => options.font_own_metrics = true,
            "--config" => options.config = Some(PathBuf::from(value()?)),
            "--no-config" => options.no_config = true,
            "--encoding" => {
                let encoding =
                    encoding(value()?).map_err(|error| format!("--encoding: {error}"))?;
                options.encoding = Some(encoding);
            }
            "--filter" => {
                let filter = filter(value()?).map_err(|error| format!("--filter: {error}"))?;
                options.filter = Some(filter);
            }
            "--fullscreen" => options.fullscreen = true,
            "--present-stats" => options.present_stats = true,
            "--frame-stats" => options.frame_stats = true,
            "--debug" => options.debug = true,
            "--allow-process-functions" => options.process_functions = true,
            "--log-p" => options.log_p = true,
            "--verbose" => options.verbose = true,
            "--no-audio" => options.no_audio = true,
            "--soundfont" => options.soundfont = Some(PathBuf::from(value()?)),
            "--seed" => {
                let text = value()?;
                let seed = text
                    .parse()
                    .map_err(|_| format!("--seed: `{text}` is not a seed, a number from 0 on"))?;
                options.seed = Some(seed);
            }
            "--preload" => options.preload.push(PathBuf::from(value()?)),
            "--record" => options.record = Some(PathBuf::from(value()?)),
            "--replay" => options.replay = Some(PathBuf::from(value()?)),
            "--guest" => {
                let form = guest_form(value()?).map_err(|error| format!("--guest: {error}"))?;
                options.guest = Some(form);
            }
            "--sandbox" => {
                let on = sandbox(value()?).map_err(|error| format!("--sandbox: {error}"))?;
                options.sandbox = Some(on);
            }
            "--headless" => options.headless = true,
            "--frames" => {
                let text = value()?;
                let frames = text
                    .parse()
                    .map_err(|_| format!("--frames: `{text}` is not a number of frames"))?;
                options.frames = Some(frames);
            }
            "--input" => {
                let input = input(value()?).map_err(|error| format!("--input: {error}"))?;
                options.input = Some(input);
            }
            "--dump" => {
                let dump = dump(value()?).map_err(|error| format!("--dump: {error}"))?;
                options.dumps.push(dump);
            }
            "--states" => {
                options.states =
                    StateSchedule::parse(value()?).map_err(|error| format!("--states: {error}"))?;
            }
            "--script" => {
                let file = PathBuf::from(value()?);
                // Every argument after the file is the script's.
                let arguments = arguments.by_ref().cloned().collect();
                options.script = Some(ScriptFile { file, arguments });
            }
            "--version" | "-V" => options.version = true,
            "--help" | "-h" => options.help = true,
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            _ if options.game_dir.is_some() => {
                return Err(format!("unexpected argument `{argument}`"));
            }
            dir => options.game_dir = Some(PathBuf::from(dir)),
        }
    }
    if options.no_config && options.config.is_some() {
        return Err("--config cannot be used together with --no-config".to_owned());
    }
    let of_games = options_of_games(&options);
    if options.script.is_some() && !of_games.is_empty() {
        return Err(format!(
            "--script runs a Ruby file without a game and does not go together with {}",
            of_games.join(", ")
        ));
    }
    Ok(options)
}

/// The options among `options` that concern only games: all but `--runtime`, `--sandbox`,
/// `--allow-process-functions`, `--verbose`, `--script`, `--help` and `--version`.
fn options_of_games(options: &Options) -> Vec<&'static str> {
    let given = [
        (options.game_dir.is_some(), "a game directory"),
        (options.config.is_some(), "--config"),
        (options.no_config, "--no-config"),
        (!options.rtps.is_empty(), "--rtp"),
        (options.builtin_scripts.is_some(), "--builtin-scripts"),
        (options.device.is_some(), "--device"),
        (!options.font_dirs.is_empty(), "--font-dir"),
        (options.font_own_metrics, "--font-own-metrics"),
        (options.encoding.is_some(), "--encoding"),
        (options.debug, "--debug"),
        (options.log_p, "--log-p"),
        (options.filter.is_some(), "--filter"),
        (options.fullscreen, "--fullscreen"),
        (options.present_stats, "--present-stats"),
        (options.frame_stats, "--frame-stats"),
        (options.no_audio, "--no-audio"),
        (options.soundfont.is_some(), "--soundfont"),
        (options.seed.is_some(), "--seed"),
        (!options.preload.is_empty(), "--preload"),
        (options.record.is_some(), "--record"),
        (options.replay.is_some(), "--replay"),
        (options.guest.is_some(), "--guest"),
        (options.headless, "--headless"),
        (options.frames.is_some(), "--frames"),
        (options.input.is_some(), "--input"),
        (!options.dumps.is_empty(), "--dump"),
        (!options.states.is_empty(), "--states"),
    ];
    given
        .into_iter()
        .filter_map(|(given, option)| given.then_some(option))
        .collect()
}

/// Whether the settings of a run go together, whichever of them came from the command line
/// and whichever from a configuration file. The error is a message for the user, to be
/// printed with [`USAGE`].
pub(crate) fn check(options: &Options) -> Result<(), String> {
    if !options.headless {
        let headless_only = [
            (options.frames.is_some(), "--frames"),
            (options.input.is_some(), "--input"),
            (!options.dumps.is_empty(), "--dump"),
            (!options.states.is_empty(), "--states"),
        ];
        if let Some((_, option)) = headless_only.iter().find(|(given, _)| *given) {
            return Err(format!("{option} needs --headless"));
        }
    }
    // States are copies of the guest process: a guest on a thread has none.
    if !options.states.is_empty() && options.guest != Some(GuestForm::Process) {
        return Err("--states needs the guest in a process: add --guest process".to_owned());
    }
    if options.replay.is_some() {
        // A replay stands in for every other input.
        let other_input = [
            (options.record.is_some(), "--record"),
            (options.input.is_some(), "--input"),
        ];
        if let Some((_, option)) = other_input.iter().find(|(given, _)| *given) {
            return Err(format!("{option} cannot be used together with --replay"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_words(words: &[&str]) -> Result<Options, String> {
        let arguments: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
        let options = parse(&arguments)?;
        check(&options)?;
        Ok(options)
    }

    #[test]
    fn no_arguments_is_a_windowed_run_of_the_current_directory() {
        assert_eq!(parse_words(&[]), Ok(Options::default()));
    }

    #[test]
    fn a_windowed_run_with_every_option() {
        let options = parse_words(&[
            "--rtp",
            "Standard=/rtp/standard",
            "games/demo",
            "--rtp",
            "Extra=rtp=extra",
            "--builtin-scripts",
            "rpg.rb",
            "--device",
            "RADV",
            "--font-dir",
            "first",
            "--font-dir",
            "second",
            "--font-own-metrics",
            "--encoding",
            "Shift-JIS",
            "--debug",
            "--allow-process-functions",
            "--log-p",
            "--no-audio",
            "--soundfont",
            "sf/gm.sf2",
            "--filter",
            "linear",
            "--fullscreen",
        ])
        .unwrap();
        assert_eq!(
            options,
            Options {
                game_dir: Some("games/demo".into()),
                rtps: BTreeMap::from([
                    ("Standard".to_owned(), "/rtp/standard".into()),
                    ("Extra".to_owned(), "rtp=extra".into()),
                ]),
                builtin_scripts: Some("rpg.rb".into()),
                device: Some("RADV".to_owned()),
                font_dirs: vec!["first".into(), "second".into()],
                font_own_metrics: true,
                encoding: Some(Encoding::ShiftJis),
                debug: true,
                process_functions: true,
                log_p: true,
                no_audio: true,
                soundfont: Some("sf/gm.sf2".into()),
                filter: Some(Filter::Linear),
                fullscreen: true,
                ..Options::default()
            }
        );
    }

    #[test]
    fn present_statistics_are_asked_for_with_a_flag() {
        assert!(!parse_words(&[]).unwrap().present_stats);
        let options = parse_words(&["games/demo", "--present-stats"]).unwrap();
        assert!(options.present_stats);
        assert!(USAGE.contains("--present-stats"));
        // The flag takes no value.
        let options = parse_words(&["--present-stats", "games/demo"]).unwrap();
        assert_eq!(options.game_dir, Some("games/demo".into()));
    }

    #[test]
    fn frame_statistics_are_asked_for_with_a_flag() {
        assert!(!parse_words(&[]).unwrap().frame_stats);
        // The flag takes no value, and goes with a window and with a headless run.
        let options = parse_words(&["--frame-stats", "games/demo"]).unwrap();
        assert!(options.frame_stats);
        assert_eq!(options.game_dir, Some("games/demo".into()));
        let options = parse_words(&["games/demo", "--headless", "--frame-stats"]).unwrap();
        assert!(options.frame_stats && options.headless);
        assert!(USAGE.contains("--frame-stats"));
    }

    /// Scenario "Unknown value", and the option itself.
    #[test]
    fn the_form_of_the_guest_is_thread_or_process() {
        assert_eq!(parse_words(&[]).unwrap().guest, None);
        let options = parse_words(&["--guest", "process", "games/demo"]).unwrap();
        assert_eq!(options.guest, Some(GuestForm::Process));
        assert_eq!(options.game_dir, Some("games/demo".into()));
        let options = parse_words(&["--guest", "Thread"]).unwrap();
        assert_eq!(options.guest, Some(GuestForm::Thread));
        assert!(USAGE.contains("--guest thread|process"));
        assert!(USAGE.contains(GUEST_VARIABLE));

        let error = parse_words(&["--guest", "fiber"]).unwrap_err();
        assert!(error.contains("--guest"), "{error}");
        assert!(error.contains("fiber"), "{error}");
        assert!(
            error.contains("thread") && error.contains("process"),
            "{error}"
        );
        assert!(parse_words(&["--guest"]).is_err());
    }

    /// Scenarios "Thread form" and "Malformed schedule".
    #[test]
    fn a_states_schedule_is_for_a_headless_run_with_the_guest_in_a_process() {
        let words = ["--headless", "--guest", "process", "--states"];
        let with = |schedule: &'static str| parse_words(&[&words[..], &[schedule]].concat());
        let options = with("120:save 0, 200:load 0").unwrap();
        assert_eq!(
            options.states,
            StateSchedule::parse("120:save 0, 200:load 0").unwrap()
        );
        assert!(parse_words(&[]).unwrap().states.is_empty());
        assert!(USAGE.contains("--states SCHEDULE"));

        for entry in ["5:keep 0", "5:save 10"] {
            let error = with(entry).unwrap_err();
            assert!(error.contains("--states"), "{error}");
            assert!(error.contains(&format!("`{entry}`")), "{error}");
        }
        // The guest on a thread, by default or by choice, has no states.
        for form in [&[][..], &["--guest", "thread"]] {
            let error =
                parse_words(&[&["--headless", "--states", "5:save 0"], form].concat()).unwrap_err();
            assert!(error.contains("--guest process"), "{error}");
        }
        let error = parse_words(&["--guest", "process", "--states", "5:save 0"]).unwrap_err();
        assert_eq!(error, "--states needs --headless");
    }

    /// Scenario "Unknown value" of the sandbox option, and the option itself.
    #[test]
    fn the_sandbox_is_on_or_off() {
        assert_eq!(parse_words(&[]).unwrap().sandbox, None);
        let options = parse_words(&["--sandbox", "off", "games/demo"]).unwrap();
        assert_eq!(options.sandbox, Some(false));
        assert_eq!(options.game_dir, Some("games/demo".into()));
        assert_eq!(
            parse_words(&["--sandbox", "On"]).unwrap().sandbox,
            Some(true)
        );
        assert!(USAGE.contains("--sandbox on|off"));
        assert!(USAGE.contains(SANDBOX_VARIABLE));

        let error = parse_words(&["--sandbox", "maybe"]).unwrap_err();
        assert!(error.contains("--sandbox"), "{error}");
        assert!(error.contains("maybe"), "{error}");
        assert!(error.contains("on") && error.contains("off"), "{error}");
        assert!(parse_words(&["--sandbox"]).is_err());
    }

    /// The variable `name` read from an environment that holds `set`.
    fn variable(name: &'static str, set: &[(&str, &str)]) -> Variable {
        Variable::of(name, |wanted| {
            set.iter()
                .find(|(name, _)| *name == wanted)
                .map(|(_, value)| (*value).to_owned())
        })
    }

    #[test]
    fn the_environment_says_whether_the_guest_is_confined() {
        let sandbox = |value| sandbox_of_environment(&variable(SANDBOX_VARIABLE, value));
        assert_eq!(sandbox(&[]), Ok(None));
        assert_eq!(sandbox(&[(SANDBOX_VARIABLE, "")]), Ok(None));
        assert_eq!(sandbox(&[(SANDBOX_VARIABLE, "off")]), Ok(Some(false)));
        assert_eq!(sandbox(&[(SANDBOX_VARIABLE, "on")]), Ok(Some(true)));
        let error = sandbox(&[(SANDBOX_VARIABLE, "maybe")]).unwrap_err();
        assert!(error.contains(SANDBOX_VARIABLE), "{error}");
        assert!(error.contains("on, off"), "{error}");
    }

    #[test]
    fn the_environment_names_the_form_of_the_guest() {
        let guest = |value| guest_of_environment(&variable(GUEST_VARIABLE, value));
        assert_eq!(guest(&[]), Ok(None));
        assert_eq!(guest(&[(GUEST_VARIABLE, "")]), Ok(None));
        assert_eq!(
            guest(&[(GUEST_VARIABLE, "process")]),
            Ok(Some(GuestForm::Process))
        );
        let error = guest(&[(GUEST_VARIABLE, "fiber")]).unwrap_err();
        assert!(error.contains(GUEST_VARIABLE), "{error}");
        assert!(
            error.contains("thread") && error.contains("process"),
            "{error}"
        );
    }

    /// Requirement "Environment variables under the former name": the former name is read
    /// when the variable is not set, and names the value it gave in an error.
    #[test]
    fn a_variable_that_is_not_set_is_read_under_its_former_name() {
        let former = "VKXP_GUEST";
        let read = variable(GUEST_VARIABLE, &[(former, "process")]);
        assert_eq!(read.value.as_deref(), Some("process"));
        assert_eq!(read.read_as(), former);
        let read = variable(GUEST_VARIABLE, &[(GUEST_VARIABLE, ""), (former, "process")]);
        assert_eq!(read.value.as_deref(), Some("process"));
        let error = guest_of_environment(&variable(GUEST_VARIABLE, &[(former, "fiber")]));
        let error = error.unwrap_err();
        assert!(error.starts_with("VKXP_GUEST: "), "{error}");

        let both = [(GUEST_VARIABLE, "thread"), (former, "process")];
        let read = variable(GUEST_VARIABLE, &both);
        assert_eq!(read.value.as_deref(), Some("thread"));
        assert_eq!(read.former, None);
        assert_eq!(variable(GUEST_VARIABLE, &[(former, "")]).value, None);
    }

    #[test]
    fn the_version_is_asked_for_with_either_spelling() {
        assert!(!parse_words(&[]).unwrap().version);
        assert!(parse_words(&["--version"]).unwrap().version);
        assert!(parse_words(&["-V"]).unwrap().version);
        assert_eq!(VERSION, concat!("vkrgss ", env!("CARGO_PKG_VERSION")));
        assert!(USAGE.contains("--version, -V"));
    }

    #[test]
    fn help_is_asked_for_with_either_spelling() {
        assert!(!parse_words(&[]).unwrap().help);
        assert!(parse_words(&["--help"]).unwrap().help);
        assert!(parse_words(&["-h"]).unwrap().help);
        assert!(USAGE.contains("--help, -h"));
    }

    #[test]
    fn verbose_is_a_flag() {
        assert!(!parse_words(&[]).unwrap().verbose);
        let options = parse_words(&["--verbose", "games/demo"]).unwrap();
        assert!(options.verbose);
        assert_eq!(options.game_dir, Some("games/demo".into()));
        assert!(USAGE.contains("--verbose"));
        assert!(USAGE.contains("VKRGSS_LOG"));
    }

    #[test]
    fn a_headless_run() {
        let options = parse_words(&[
            "game",
            "--headless",
            "--frames",
            "10",
            "--input",
            "2:RIGHT, 6:",
            "--dump",
            "1=a.png",
            "--dump",
            "8=b=c.png",
            "--device",
            "0",
        ])
        .unwrap();
        assert!(options.headless);
        assert_eq!(options.frames, Some(10));
        assert_eq!(
            options.input,
            Some(Input::Script(InputScript::parse("2:RIGHT, 6:").unwrap()))
        );
        assert_eq!(options.dumps, [(1, "a.png".into()), (8, "b=c.png".into())]);
        assert_eq!(options.device.as_deref(), Some("0"));
    }

    #[test]
    fn seed_preload_record_and_replay() {
        let options = parse_words(&[
            "game",
            "--seed",
            "18446744073709551615",
            "--preload",
            "one.rb",
            "--preload",
            "patches/two.rb",
            "--record",
            "input.txt",
        ])
        .unwrap();
        assert_eq!(options.seed, Some(u64::MAX));
        assert_eq!(
            options.preload,
            [PathBuf::from("one.rb"), "patches/two.rb".into()]
        );
        assert_eq!(options.record, Some("input.txt".into()));
        assert_eq!(options.replay, None);

        let options = parse_words(&["--headless", "--replay", "input.txt"]).unwrap();
        assert_eq!(options.replay, Some("input.txt".into()));
        assert_eq!(options.seed, None);
        assert!(options.preload.is_empty());

        for option in ["--seed", "--preload", "--record", "--replay"] {
            let error = parse_words(&["game", option]).unwrap_err();
            assert!(error.contains(option), "{error}");
        }
    }

    #[test]
    fn a_malformed_seed_is_an_error() {
        for value in ["seven", "-1", "1.5", "", "18446744073709551616"] {
            let error = parse_words(&["--seed", value]).unwrap_err();
            assert!(error.contains("--seed"), "{value}: {error}");
        }
    }

    /// Scenario "Record and replay together".
    #[test]
    fn record_and_replay_together_are_refused() {
        for words in [
            &["--record", "a.txt", "--replay", "b.txt"][..],
            &["--replay", "b.txt", "--record", "a.txt", "--headless"],
        ] {
            let error = parse_words(words).unwrap_err();
            assert!(error.contains("--record"), "{error}");
            assert!(error.contains("--replay"), "{error}");
        }
        let error =
            parse_words(&["--headless", "--input", "1:UP", "--replay", "b.txt"]).unwrap_err();
        assert!(error.contains("--input"), "{error}");
    }

    #[test]
    fn the_configuration_file_is_named_or_left_out() {
        let options = parse_words(&["game"]).unwrap();
        assert_eq!(options.config, None);
        assert!(!options.no_config);

        let options = parse_words(&["game", "--config", "my.toml"]).unwrap();
        assert_eq!(options.config, Some("my.toml".into()));

        let options = parse_words(&["--no-config", "game"]).unwrap();
        assert!(options.no_config);

        let error = parse_words(&["--no-config", "--config", "my.toml"]).unwrap_err();
        assert!(error.contains("--no-config"), "{error}");
        assert!(parse_words(&["--config"]).is_err());
    }

    #[test]
    fn an_input_script_can_be_a_file() {
        let options = parse_words(&["--headless", "--input", "@keys.txt"]).unwrap();
        assert_eq!(options.input, Some(Input::File("keys.txt".into())));
    }

    /// Scenario "Unknown filter", as far as the parser goes.
    #[test]
    fn every_filter_has_a_name_and_another_name_is_refused() {
        for (name, filter) in [
            ("auto", Filter::Auto),
            ("nearest", Filter::Nearest),
            ("linear", Filter::Linear),
            ("sharp", Filter::Sharp),
        ] {
            let options = parse_words(&["--filter", name]).unwrap();
            assert_eq!(options.filter, Some(filter));
        }
        let error = parse_words(&["--filter", "cubic"]).unwrap_err();
        assert!(
            error.contains("--filter") && error.contains("cubic"),
            "{error}"
        );
        assert!(parse_words(&["--filter"]).is_err());
    }

    /// Requirement "Supported runtimes say nothing about their state", scenario "Usage".
    #[test]
    fn the_usage_text_names_the_runtimes_without_their_state() {
        let lines: Vec<&str> = USAGE
            .lines()
            .skip_while(|line| !line.trim_start().starts_with("--runtime"))
            .take(3)
            .map(str::trim)
            .collect();
        let runtime = lines.join(" ");
        assert!(runtime.contains("rgss2 (RPG Maker VX)"), "{runtime}");
        assert!(runtime.contains("rgss3 (RPG Maker VX Ace);"), "{runtime}");
        assert!(!USAGE.contains("experimental"));
        assert!(USAGE.contains("rgss3 runs in a process"));
    }

    #[test]
    fn every_runtime_has_a_name() {
        assert_eq!(parse_words(&[]).unwrap().runtime, None);
        for (name, runtime) in [
            ("rgss1", Runtime::Rgss1),
            ("rgss2", Runtime::Rgss2),
            ("RGSS3", Runtime::Rgss3),
        ] {
            let options = parse_words(&["--runtime", name]).unwrap();
            assert_eq!(options.runtime, Some(runtime));
            assert!(USAGE.contains(runtime.name()));
        }
        let error = parse_words(&["--runtime", "vx"]).unwrap_err();
        assert!(
            error.contains("--runtime") && error.contains("vx"),
            "{error}"
        );
        assert!(parse_words(&["--runtime"]).is_err());
    }

    #[test]
    fn every_encoding_has_a_name() {
        for (name, encoding) in [
            ("utf-8", Encoding::Utf8),
            ("gbk", Encoding::Gbk),
            ("shift-jis", Encoding::ShiftJis),
            ("big5", Encoding::Big5),
        ] {
            let options = parse_words(&["--encoding", name]).unwrap();
            assert_eq!(options.encoding, Some(encoding));
        }
        assert!(parse_words(&["--encoding", "latin1"]).is_err());
    }

    #[test]
    fn audio_is_on_unless_disabled() {
        let options = parse_words(&["game"]).unwrap();
        assert!(!options.no_audio);
        assert_eq!(options.soundfont, None);

        let options = parse_words(&["game", "--no-audio"]).unwrap();
        assert!(options.no_audio);
    }

    #[test]
    fn a_soundfont_is_a_file_name() {
        let options = parse_words(&["--soundfont", "FluidR3_GM.sf2", "game"]).unwrap();
        assert_eq!(options.soundfont, Some("FluidR3_GM.sf2".into()));
        assert!(!options.no_audio);

        let error = parse_words(&["game", "--soundfont"]).unwrap_err();
        assert!(error.contains("--soundfont"), "{error}");
    }

    #[test]
    fn an_unknown_option_is_an_error() {
        let error = parse_words(&["game", "--no-such-option"]).unwrap_err();
        assert!(error.contains("--no-such-option"), "{error}");
    }

    #[test]
    fn a_second_directory_is_an_error() {
        assert!(parse_words(&["one", "two"]).is_err());
    }

    #[test]
    fn a_malformed_rtp_is_an_error() {
        for value in ["Standard", "=dir", "Standard="] {
            let error = parse_words(&["--rtp", value]).unwrap_err();
            assert!(error.contains("NAME=DIR"), "{error}");
        }
        assert!(parse_words(&["--rtp"]).is_err());
    }

    #[test]
    fn a_malformed_dump_is_an_error() {
        for value in ["3", "=out.png", "3=", "three=out.png", "-1=out.png"] {
            assert!(
                parse_words(&["--headless", "--dump", value]).is_err(),
                "{value}"
            );
        }
    }

    #[test]
    fn a_malformed_frame_count_is_an_error() {
        for value in ["ten", "-1", "1.5", ""] {
            assert!(
                parse_words(&["--headless", "--frames", value]).is_err(),
                "{value}"
            );
        }
        assert!(parse_words(&["--headless", "--frames"]).is_err());
    }

    #[test]
    fn a_malformed_input_script_is_an_error() {
        let error = parse_words(&["--headless", "--input", "2:SIDEWAYS"]).unwrap_err();
        assert!(error.contains("SIDEWAYS"), "{error}");
    }

    #[test]
    fn headless_options_need_headless() {
        for words in [
            &["--frames", "3"][..],
            &["--input", "1:UP"],
            &["--dump", "1=a.png"],
        ] {
            let error = parse_words(words).unwrap_err();
            assert!(error.contains("--headless"), "{error}");
        }
    }

    /// Requirement "Run a Ruby file without a game": the options of the interpreter go
    /// before `--script`, and everything after the file is the script's.
    #[test]
    fn a_script_takes_every_argument_after_its_file() {
        let options = parse_words(&[
            "--runtime",
            "rgss1",
            "--sandbox",
            "off",
            "--allow-process-functions",
            "--verbose",
            "--script",
            "s.rb",
            "a",
            "--headless",
            "--script",
        ])
        .unwrap();
        assert_eq!(
            options,
            Options {
                runtime: Some(Runtime::Rgss1),
                sandbox: Some(false),
                process_functions: true,
                verbose: true,
                script: Some(ScriptFile {
                    file: "s.rb".into(),
                    arguments: ["a", "--headless", "--script"].map(String::from).into(),
                }),
                ..Options::default()
            }
        );
        let options = parse_words(&["--script", "s.rb"]).unwrap();
        assert_eq!(options.script.unwrap().arguments, Vec::<String>::new());
        assert!(
            parse_words(&["--script"])
                .unwrap_err()
                .contains("--script needs a value")
        );
        assert!(USAGE.contains("--script FILE [ARG...]"));
    }

    /// The options of games before `--script` are an error that names each of them.
    #[test]
    fn a_script_refuses_the_options_of_games() {
        let error =
            parse_words(&["game", "--headless", "--frames", "3", "--script", "s.rb"]).unwrap_err();
        assert!(
            error.ends_with("a game directory, --headless, --frames"),
            "{error}"
        );
        for words in [
            &["--rtp", "Standard=rtp"][..],
            &["--preload", "p.rb"],
            &["--guest", "process"],
            &["--debug"],
            &["--seed", "1"],
        ] {
            let mut words = words.to_vec();
            words.extend(["--script", "s.rb"]);
            let error = parse_words(&words).unwrap_err();
            assert!(error.contains(words[0]), "{error}");
        }
    }
}
