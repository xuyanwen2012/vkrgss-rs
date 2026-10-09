//! The command line of the program.

use std::collections::BTreeMap;
use std::path::PathBuf;

use vkxp_host::InputScript;
use vkxp_vfs::Encoding;

/// What is printed with every command-line error.
pub const USAGE: &str = "\
usage: vkxp [GAME_DIR] [OPTIONS]

  GAME_DIR                  the game directory; the current directory if absent
  --rtp NAME=DIR            the directory of the run-time package NAME; repeatable
  --builtin-scripts FILE    the built-in scripts file, made by `cargo xtask extract-rpg`;
                            default: $XDG_DATA_HOME/vkxp/rpg_builtin.rb
  --device INDEX|NAME       the Vulkan device, by index or by a part of its name
  --font-dir DIR            a directory of fonts, searched before the system's; repeatable
  --encoding ENCODING       the text encoding of Game.ini: utf-8, gbk, shift-jis or big5
  --debug                   debug mode: $DEBUG is true
  --allow-process-functions let scripts start processes (system, exec and the like)
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
    /// The directory of each run-time package, by its name.
    pub rtps: BTreeMap<String, PathBuf>,
    pub builtin_scripts: Option<PathBuf>,
    /// The device as given: an index or a part of a name.
    pub device: Option<String>,
    /// Font directories, in the order given.
    pub font_dirs: Vec<PathBuf>,
    pub encoding: Option<Encoding>,
    pub debug: bool,
    pub process_functions: bool,
    pub headless: bool,
    pub frames: Option<u64>,
    pub input: Option<Input>,
    /// Frames to write as PNG files, by frame number.
    pub dumps: Vec<(u64, PathBuf)>,
}

/// `NAME=VALUE` with neither part empty.
fn pair<'a>(option: &str, text: &'a str, form: &str) -> Result<(&'a str, &'a str), String> {
    text.split_once('=')
        .filter(|(name, value)| !name.is_empty() && !value.is_empty())
        .ok_or_else(|| format!("{option}: `{text}` is not of the form {form}"))
}

fn encoding(text: &str) -> Result<Encoding, String> {
    match text.to_ascii_lowercase().as_str() {
        "utf-8" => Ok(Encoding::Utf8),
        "gbk" => Ok(Encoding::Gbk),
        "shift-jis" => Ok(Encoding::ShiftJis),
        "big5" => Ok(Encoding::Big5),
        _ => Err(format!(
            "--encoding: `{text}` is not one of utf-8, gbk, shift-jis, big5"
        )),
    }
}

/// Reads the arguments that follow the program name. The error is a message for the user,
/// to be printed with [`USAGE`].
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
                let (name, dir) = pair(argument, value()?, "NAME=DIR")?;
                options.rtps.insert(name.to_owned(), PathBuf::from(dir));
            }
            "--builtin-scripts" => options.builtin_scripts = Some(PathBuf::from(value()?)),
            "--device" => options.device = Some(value()?.clone()),
            "--font-dir" => options.font_dirs.push(PathBuf::from(value()?)),
            "--encoding" => options.encoding = Some(encoding(value()?)?),
            "--debug" => options.debug = true,
            "--allow-process-functions" => options.process_functions = true,
            "--headless" => options.headless = true,
            "--frames" => {
                let text = value()?;
                let frames = text
                    .parse()
                    .map_err(|_| format!("--frames: `{text}` is not a number of frames"))?;
                options.frames = Some(frames);
            }
            "--input" => {
                let text = value()?;
                options.input = Some(match text.strip_prefix('@') {
                    Some(file) => Input::File(PathBuf::from(file)),
                    None => Input::Script(
                        InputScript::parse(text).map_err(|error| format!("--input: {error}"))?,
                    ),
                });
            }
            "--dump" => {
                let text = value()?;
                let (frame, file) = pair(argument, text, "FRAME=FILE")?;
                let frame = frame
                    .parse()
                    .map_err(|_| format!("--dump: `{frame}` is not a frame number"))?;
                options.dumps.push((frame, PathBuf::from(file)));
            }
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            _ if options.game_dir.is_some() => {
                return Err(format!("unexpected argument `{argument}`"));
            }
            dir => options.game_dir = Some(PathBuf::from(dir)),
        }
    }
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
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_words(words: &[&str]) -> Result<Options, String> {
        let arguments: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
        parse(&arguments)
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
            "--encoding",
            "Shift-JIS",
            "--debug",
            "--allow-process-functions",
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
                encoding: Some(Encoding::ShiftJis),
                debug: true,
                process_functions: true,
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
    fn an_input_script_can_be_a_file() {
        let options = parse_words(&["--headless", "--input", "@keys.txt"]).unwrap();
        assert_eq!(options.input, Some(Input::File("keys.txt".into())));
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
