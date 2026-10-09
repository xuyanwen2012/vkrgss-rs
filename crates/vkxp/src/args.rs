//! The command line of the program.

use std::collections::BTreeMap;
use std::path::PathBuf;

use vkxp_host::{Filter, InputScript};
use vkxp_vfs::Encoding;

/// What is printed with every command-line error.
pub const USAGE: &str = "\
usage: vkxp [GAME_DIR] [OPTIONS]

  GAME_DIR                  the game directory; the current directory if absent
  --config FILE             the user's configuration file;
                            default: $XDG_CONFIG_HOME/vkxp/config.toml
  --no-config               read neither that file nor vkxp.toml in the game directory
  --rtp NAME=DIR            the directory of the run-time package NAME; repeatable
  --builtin-scripts FILE    the built-in scripts file, made by `cargo xtask extract-rpg`;
                            default: $XDG_DATA_HOME/vkxp/rpg_builtin.rb
  --device INDEX|NAME       the Vulkan device, by index or by a part of its name
  --font-dir DIR            a directory of fonts, searched before the system's; repeatable
  --font-own-metrics        size every font by its own line box, a replacement included
  --encoding ENCODING       the text encoding of Game.ini: utf-8, gbk, shift-jis or big5
  --debug                   debug mode: $DEBUG is true
  --allow-process-functions let scripts start processes (system, exec and the like)
  --log-p                   p only writes to the error stream and shows no message panel
  --filter FILTER           how the frame is scaled into the window: auto, nearest,
                            linear or sharp; default: auto, which is nearest at a
                            whole-number scale and sharp otherwise
  --fullscreen              start in borderless fullscreen; Alt+Enter or F11 toggles
  --no-audio                run without sound: no audio device is opened
  --soundfont FILE          the SoundFont that MIDI music is played with;
                            default: the first found in /usr/share/soundfonts
  --seed N                  the seed of the random number generator; default: 0 in a
                            headless run, the seed of the recording in a replay, and
                            one taken from the clock otherwise
  --preload FILE            a Ruby file evaluated before the game's scripts; repeatable
  --record FILE             write the input of every frame and the seed to FILE
  --replay FILE             play the input of a recording back instead of the keyboard
  --headless                run without a window, on a virtual clock
  --frames N                headless: stop after N frames
  --input SCRIPT|@FILE      headless: held buttons, for example \"2:RIGHT, 6:\"
  --dump FRAME=FILE         headless: write frame FRAME as a PNG file; repeatable";

/// Where the input script of a headless run comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// A script given on the command line.
    Script(InputScript),
    /// A file that holds the script.
    File(PathBuf),
}

/// The parsed command line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The game directory, if one was given.
    pub game_dir: Option<PathBuf>,
    /// The user's configuration file, instead of the default location.
    pub config: Option<PathBuf>,
    /// Whether no configuration file is read.
    pub no_config: bool,
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
    pub headless: bool,
    pub frames: Option<u64>,
    pub input: Option<Input>,
    /// Frames to write as PNG files, by frame number.
    pub dumps: Vec<(u64, PathBuf)>,
}

/// `NAME=VALUE` with neither part empty.
fn pair<'a>(text: &'a str, form: &str) -> Result<(&'a str, &'a str), String> {
    text.split_once('=')
        .filter(|(name, value)| !name.is_empty() && !value.is_empty())
        .ok_or_else(|| format!("`{text}` is not of the form {form}"))
}

/// An input script as it is given: the script itself, or `@FILE` for a file that holds it.
pub fn input(text: &str) -> Result<Input, String> {
    match text.strip_prefix('@') {
        Some(file) => Ok(Input::File(PathBuf::from(file))),
        None => InputScript::parse(text)
            .map(Input::Script)
            .map_err(|error| error.to_string()),
    }
}

/// A frame to dump as it is given: `FRAME=FILE`.
pub fn dump(text: &str) -> Result<(u64, PathBuf), String> {
    let (frame, file) = pair(text, "FRAME=FILE")?;
    let frame = frame
        .parse()
        .map_err(|_| format!("`{frame}` is not a frame number"))?;
    Ok((frame, PathBuf::from(file)))
}

/// The encoding with this name, in either case. The error says what the names are.
pub fn encoding(text: &str) -> Result<Encoding, String> {
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

/// The scaling filter with this name, in either case. The error says what the names are.
pub fn filter(text: &str) -> Result<Filter, String> {
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

/// Reads the arguments that follow the program name. The error is a message for the user,
/// to be printed with [`USAGE`]. Whether the options go together is for [`check`] to say,
/// once the configuration files have been put under them.
pub fn parse(arguments: &[String]) -> Result<Options, String> {
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
            "--debug" => options.debug = true,
            "--allow-process-functions" => options.process_functions = true,
            "--log-p" => options.log_p = true,
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
    Ok(options)
}

/// Whether the settings of a run go together, whichever of them came from the command line
/// and whichever from a configuration file. The error is a message for the user, to be
/// printed with [`USAGE`].
pub fn check(options: &Options) -> Result<(), String> {
    if !options.headless {
        let headless_only = [
            (options.frames.is_some(), "--frames"),
            (options.input.is_some(), "--input"),
            (!options.dumps.is_empty(), "--dump"),
        ];
        if let Some((_, option)) = headless_only.iter().find(|(given, _)| *given) {
            return Err(format!("{option} needs --headless"));
        }
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
}
