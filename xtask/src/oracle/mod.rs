//! `cargo xtask oracle`: runs one game in the original runtime and in vkxp under the same
//! conditions and reports how chosen frames differ.
//!
//! Each side runs a copy of the game below the output directory, with a probe script in
//! front of its scripts that fixes the random seed, answers `Input` from a schedule and
//! counts frames. The original runs under Wine in a virtual display, where the probe holds
//! it at the frames asked for while its window is captured; vkxp runs headless and dumps the
//! same frames. Nothing is written outside the output directory.

mod archive;
mod fonts;
mod game;
mod original;
mod probe;
mod report;
mod runtime;
mod vkxp;

use std::fs;
use std::path::{Path, PathBuf, absolute};
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};

use probe::{Schedule, Side};
use report::Sides;
use runtime::Runtime;

/// The usage of the command, printed with every error in its arguments.
pub(crate) const USAGE: &str = "\
usage: cargo xtask oracle <GAME_DIR> --frames N[,N...] [OPTIONS]

  GAME_DIR                the game directory; it is copied, never changed
  --frames N[,N...]       the frames to capture, numbered from 0 as `vkxp --dump` numbers them
  --dll FILE              the RGSS DLL the game's Game.ini names; needed without --vkxp-only,
                          except for a game of rgss3, which runs with the DLL it has at the
                          path its Library= names when the option is left out
  --runtime rgss1|rgss2|rgss3
                          the runtime of the game, when the file its Scripts= names is
                          none of .rxdata (rgss1), .rvdata (rgss2) and .rvdata2 (rgss3)
  --rtp NAME=DIR          the directory of the run-time package NAME; repeatable
  --input SCHEDULE|@FILE  held buttons in the syntax of `vkxp --input`, for example
                          \"60:RIGHT, 100:\", or a file that holds such a schedule
  --font FILE             a font file that both runtimes draw text with; repeatable
  --builtin-scripts FILE  the built-in scripts file for vkxp, instead of its default location
  --out DIR               where everything is written; default: target/oracle
  --vkxp-only             run only vkxp; needs neither Wine nor Xvfb nor the DLL
  --original-only         run only the original; its pictures are listed, not compared
  --no-threads            make Thread.new run its block at once in the caller on both sides,
                          for a game whose Ruby threads end the original under Wine
  --locale LOCALE         LC_ALL of the original, for the code page of the game's file
                          names; default: zh_CN.UTF-8
  --timeout SECONDS       how long the original may take to reach one frame; default: 120";

/// The parsed arguments of the command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Options {
    pub game: PathBuf,
    /// The frames to capture, in ascending order.
    pub frames: Vec<u64>,
    pub dll: Option<PathBuf>,
    /// The runtime of the game, if the command line names it.
    pub runtime: Option<Runtime>,
    pub rtps: Vec<(String, PathBuf)>,
    pub schedule: Schedule,
    /// The font files that both runtimes draw text with.
    pub fonts: Vec<PathBuf>,
    pub builtin_scripts: Option<PathBuf>,
    pub out: PathBuf,
    pub vkxp_only: bool,
    pub original_only: bool,
    pub no_threads: bool,
    pub locale: String,
    pub timeout: Duration,
}

/// Reads the arguments that follow `oracle`. Paths are taken relative to the directory the
/// command was started in, except the default output directory, which is below `root`.
pub(crate) fn parse(arguments: &[&str], root: &Path) -> Result<Options> {
    let mut game = None;
    let mut frames = None;
    let mut options = Options {
        game: PathBuf::new(),
        frames: Vec::new(),
        dll: None,
        runtime: None,
        rtps: Vec::new(),
        schedule: Schedule::default(),
        fonts: Vec::new(),
        builtin_scripts: None,
        out: root.join("target").join("oracle"),
        vkxp_only: false,
        original_only: false,
        no_threads: false,
        locale: "zh_CN.UTF-8".to_owned(),
        timeout: Duration::from_secs(120),
    };
    let path = |text: &str| absolute(text).with_context(|| text.to_owned());
    let mut arguments = arguments.iter().copied();
    while let Some(argument) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .with_context(|| format!("{argument} needs a value"))
        };
        match argument {
            "--frames" => frames = Some(probe::parse_frames(value()?).context("--frames")?),
            "--dll" => options.dll = Some(path(value()?)?),
            "--runtime" => options.runtime = Some(Runtime::parse(value()?)?),
            "--rtp" => {
                let text = value()?;
                let (name, dir) = text
                    .split_once('=')
                    .filter(|(name, dir)| !name.is_empty() && !dir.is_empty())
                    .with_context(|| format!("--rtp: `{text}` is not of the form NAME=DIR"))?;
                options.rtps.push((name.to_owned(), path(dir)?));
            }
            "--input" => {
                let text = value()?;
                let text = match text.strip_prefix('@') {
                    Some(file) => fs::read_to_string(file).with_context(|| file.to_owned())?,
                    None => text.to_owned(),
                };
                options.schedule = Schedule::parse(&text).context("--input")?;
            }
            "--font" => options.fonts.push(path(value()?)?),
            "--builtin-scripts" => options.builtin_scripts = Some(path(value()?)?),
            "--out" => options.out = path(value()?)?,
            "--vkxp-only" => options.vkxp_only = true,
            "--original-only" => options.original_only = true,
            "--no-threads" => options.no_threads = true,
            "--locale" => options.locale = value()?.to_owned(),
            "--timeout" => {
                let text = value()?;
                let seconds = text
                    .parse()
                    .ok()
                    .with_context(|| format!("--timeout: `{text}` is not a number of seconds"))?;
                options.timeout = Duration::from_secs(seconds);
            }
            _ if argument.starts_with("--") => bail!("unknown option {argument}"),
            _ if game.is_none() => game = Some(path(argument)?),
            _ => bail!("more than one game directory"),
        }
    }
    ensure!(
        !(options.vkxp_only && options.original_only),
        "--vkxp-only and --original-only exclude each other"
    );
    options.game = game.context("no game directory")?;
    options.frames = frames.context("--frames is missing")?;
    Ok(options)
}

/// Makes the copy of the game at `copy` that one side runs, with the probe of that side.
fn prepare_copy(options: &Options, runtime: Runtime, side: Side, copy: &Path) -> Result<()> {
    let source = probe::generate(
        runtime,
        side,
        &options.schedule,
        &options.frames,
        options.no_threads,
    );
    game::prepare(&options.game, copy, runtime, &source)
}

/// Runs a copy of the game with the probe in the original runtime and writes its frames.
/// Returns the frames that were captured late.
fn capture_original(
    options: &Options,
    runtime: Runtime,
    dll: Option<&Path>,
    fonts: Option<&fonts::Shared>,
) -> Result<Vec<u64>> {
    let Options { out, frames, .. } = options;
    let copy = out.join("original").join("game");
    prepare_copy(options, runtime, Side::Original, &copy)?;
    let system = original::GSTREAMER_SYSTEM_DIRS.map(Path::new);
    let gstreamer = original::GStreamer::prepare(out, &system)?;
    original::Run {
        runtime,
        dll,
        rtps: &options.rtps,
        fonts,
        prefix: &out.join("prefix"),
        gstreamer: &gstreamer,
        locale: &options.locale,
        timeout: options.timeout,
    }
    .capture(&copy, frames, out, &out.join("original").join("wine.log"))
}

/// Runs the comparison. `root` is the workspace, whose `vkxp` is built and run.
pub(crate) fn run(options: &Options, root: &Path) -> Result<()> {
    let Options {
        game, out, frames, ..
    } = options;
    ensure!(
        game.is_dir(),
        "{}: the game directory does not exist",
        game.display()
    );
    // The DLL of the original is checked by the runtime of the game.
    let runtime = game::runtime(game, options.runtime)?;
    let dll = options.dll.as_deref();
    if !options.vkxp_only {
        original::check_prerequisites(game, runtime, dll)?;
    }
    // A copy below the game would be copied into itself, and would change the game.
    let inside = |inner: &Path, outer: &Path| {
        let outer = outer.canonicalize().unwrap_or_else(|_| outer.to_path_buf());
        inner.ancestors().any(|ancestor| {
            ancestor
                .canonicalize()
                .is_ok_and(|ancestor| ancestor == outer)
        })
    };
    ensure!(
        !inside(out, game),
        "the output directory {} is inside the game directory",
        out.display()
    );
    // Before either runtime is started: a file that is no font ends the run here.
    let shared = fonts::share(&options.fonts, out)?;
    let executable = match options.original_only {
        true => None,
        false => Some(vkxp::build(root, runtime)?),
    };

    let frames_dir = out.join("frames");
    if frames_dir.exists() {
        // Pictures of an earlier run with other frames would look like results of this one.
        fs::remove_dir_all(&frames_dir).with_context(|| frames_dir.display().to_string())?;
    }
    fs::create_dir_all(&frames_dir).with_context(|| frames_dir.display().to_string())?;

    let mut late = Vec::new();
    if !options.vkxp_only {
        eprintln!("oracle: running the original");
        late = capture_original(options, runtime, dll, shared.as_ref())?;
    }

    if let Some(executable) = &executable {
        let copy = out.join("vkxp").join("game");
        prepare_copy(options, runtime, Side::Vkxp, &copy)?;
        eprintln!("oracle: running vkxp");
        vkxp::Run {
            executable,
            rtps: &options.rtps,
            font_dir: shared.as_ref().map(|shared| shared.dir.as_path()),
            builtin_scripts: options.builtin_scripts.as_deref(),
        }
        .capture(&copy, frames, out)?;
        println!("{}", fonts::notice(shared.as_ref()));
    }

    let sides = match (options.vkxp_only, options.original_only) {
        (true, _) => Sides::VkxpOnly,
        (_, true) => Sides::OriginalOnly,
        _ => Sides::Both,
    };
    let run = report::Run {
        sides,
        late: &late,
        no_threads: options.no_threads,
    };
    report::write(out, frames, &run)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::process::Command;

    use tempfile::TempDir;

    use super::*;
    use crate::rgss3a;

    /// A stand-in for the built-in scripts of the original, which are not in the repository.
    const RPG_STAND_IN: &str = "module RPG\n  module Cache\n  end\nend\n";

    /// A stand-in for the built-in scripts of the original of RGSS3, which are not in the
    /// repository either: the module `RPG` and an `rgss_main` that yields.
    const RGSS3_STAND_IN: &str = "module RPG\nend\n\ndef rgss_main\n  yield\nend\n";

    /// A red square that moves to the right by four pixels in every frame, for ever.
    const MOVING_SQUARE: &str = "\
sprite = Sprite.new
sprite.bitmap = Bitmap.new(32, 32)
sprite.bitmap.fill_rect(0, 0, 32, 32, Color.new(255, 0, 0))
sprite.y = 20
loop do
  Graphics.update
  sprite.x += 4
end
";

    /// A game made for one test, with what running it needs, in a directory of its own.
    struct Generated {
        dir: TempDir,
    }

    impl Generated {
        /// Runs the game, whose probe is that of the original's side, in vkxp, and answers
        /// the probe's requests to hold a frame in place of the harness. Returns their text.
        fn answered_requests(&self) -> Vec<String> {
            let game = self.game();
            let deadline = std::time::Instant::now() + Duration::from_secs(120);
            let mut requests = Vec::new();
            std::thread::scope(|scope| {
                let runner = scope.spawn(|| self.plain_run(&[]));
                while !runner.is_finished() {
                    assert!(std::time::Instant::now() < deadline, "{requests:?}");
                    std::thread::sleep(Duration::from_millis(2));
                    for entry in fs::read_dir(&game).unwrap() {
                        let path = entry.unwrap().path();
                        if path
                            .extension()
                            .is_none_or(|extension| extension != "request")
                        {
                            continue;
                        }
                        let record = fs::read_to_string(&path).unwrap();
                        // An empty file is one that the probe has not written yet.
                        if !record.is_empty() {
                            requests.push(record);
                            fs::remove_file(&path).unwrap();
                        }
                    }
                }
            });
            requests
        }

        /// A game of RGSS1 whose archive, written by the harness's writer, holds `scripts`
        /// as pairs of name and source.
        fn new(scripts: &[(&str, &str)]) -> Generated {
            Generated::named("Scripts.rxdata", "", scripts)
        }

        /// A game whose archive is the file `archive` of its `Data` folder. `lines` are
        /// further lines of its `Game.ini`.
        fn named(archive: &str, lines: &str, scripts: &[(&str, &str)]) -> Generated {
            let ini = format!(
                "[Game]\r\nLibrary=RGSS104E.dll\r\nScripts=Data\\{archive}\r\n\
                 Title=Generated\r\n{lines}"
            );
            Generated::written(archive, &ini, RPG_STAND_IN, scripts)
        }

        /// A game of RGSS3 without a run-time package, whose `Library=` names
        /// `System\RGSS300.dll`, which it does not have.
        fn ace(scripts: &[(&str, &str)]) -> Generated {
            let ini = "[Game]\r\nLibrary=System\\RGSS300.dll\r\n\
                       Scripts=Data\\Scripts.rvdata2\r\nRTP=\r\nTitle=Generated\r\n";
            Generated::written("Scripts.rvdata2", ini, RGSS3_STAND_IN, scripts)
        }

        /// A game with this `Game.ini` and these built-in scripts, whose archive is the file
        /// `archive` of its `Data` folder.
        fn written(archive: &str, ini: &str, builtin: &str, scripts: &[(&str, &str)]) -> Generated {
            let dir = tempfile::tempdir().expect("a temporary directory");
            let generated = Generated { dir };
            let game = generated.game();
            fs::create_dir_all(game.join("Data")).unwrap();
            fs::write(game.join("Game.ini"), ini).unwrap();
            let entries: Vec<_> = (1..)
                .zip(scripts)
                .map(|(id, (name, source))| archive::Entry::new(id, name, source))
                .collect();
            fs::write(game.join("Data").join(archive), archive::write(&entries)).unwrap();
            fs::write(generated.builtin_scripts(), builtin).unwrap();
            generated
        }

        /// Packs the game as `cargo xtask pack-rgssad` does.
        fn pack(&self) {
            crate::rgssad::pack(&self.game()).unwrap();
        }

        /// Packs the files of the `Data` folder of a game of RGSS3 into `Game.rgss3a`, under
        /// keys of its own, and removes the folder.
        fn pack_rgss3a(&self) {
            let data = self.game().join("Data");
            let mut names: Vec<_> = fs::read_dir(&data)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .collect();
            names.sort();
            let entries = names
                .iter()
                .zip([41, 18467, 6334, 26500])
                .map(|(name, key)| rgss3a::Entry {
                    name: format!("Data\\{name}").into_bytes(),
                    key,
                    data: fs::read(data.join(name)).unwrap(),
                })
                .collect();
            let archive = rgss3a::Archive::new(0x2a38, entries);
            fs::write(self.game().join(rgss3a::FILE_NAME), rgss3a::write(&archive)).unwrap();
            fs::remove_dir_all(data).unwrap();
        }

        fn game(&self) -> PathBuf {
            self.dir.path().join("game")
        }

        fn builtin_scripts(&self) -> PathBuf {
            self.dir.path().join("rpg_builtin.rb")
        }

        fn out(&self) -> PathBuf {
            self.dir.path().join("out")
        }

        /// The options of a run of the harness on this game without the original.
        fn options(&self, frames: &str, input: &str) -> Options {
            let game = self.game();
            let (builtin, out) = (self.builtin_scripts(), self.out());
            let arguments = [
                game.to_str().unwrap(),
                "--vkxp-only",
                "--frames",
                frames,
                "--input",
                input,
                "--builtin-scripts",
                builtin.to_str().unwrap(),
                "--out",
                out.to_str().unwrap(),
            ];
            parse(&arguments, &crate::workspace_root()).expect("valid arguments")
        }

        /// Every file of the game directory with its contents.
        fn files(&self) -> BTreeMap<PathBuf, Vec<u8>> {
            fn collect(dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
                for entry in fs::read_dir(dir).unwrap() {
                    let path = entry.unwrap().path();
                    if path.is_dir() {
                        collect(&path, files);
                    } else {
                        files.insert(path.clone(), fs::read(&path).unwrap());
                    }
                }
            }
            let mut files = BTreeMap::new();
            collect(&self.game(), &mut files);
            files
        }

        /// A run of vkxp on the game itself, without the harness: headless, with these
        /// further arguments, started in the game's parent directory. That directory also
        /// stands in for the user's run-time directory, where vkxp makes the socket of a
        /// guest process, as in the tests of `vkxp`.
        fn plain_run(&self, arguments: &[&str]) {
            let runtime = game::runtime(&self.game(), None).unwrap_or(Runtime::Rgss1);
            let executable = vkxp::build(&crate::workspace_root(), runtime).unwrap();
            let mut command = Command::new(executable);
            command
                .arg(self.game())
                .arg("--headless")
                .arg("--builtin-scripts")
                .arg(self.builtin_scripts())
                .args(arguments)
                .current_dir(self.dir.path())
                .env("XDG_RUNTIME_DIR", self.dir.path());
            if !vkxp::loader_is_restricted() {
                command.env("VK_LOADER_DRIVERS_SELECT", "*lvp*");
            }
            let status = command.status().expect("vkxp starts");
            assert!(status.success(), "{status}");
        }

        /// What [`run`] does with `options` on the vkxp side alone: the copy with the probe
        /// and the command of vkxp, which it runs with the run-time directory of
        /// [`Generated::plain_run`]. A game of RGSS3 runs its scripts in a guest process.
        fn vkxp_side(&self, options: &Options) {
            let runtime = game::runtime(&options.game, options.runtime).unwrap();
            let executable = vkxp::build(&crate::workspace_root(), runtime).unwrap();
            let copy = options.out.join("vkxp").join("game");
            prepare_copy(options, runtime, Side::Vkxp, &copy).unwrap();
            fs::create_dir_all(options.out.join("frames")).unwrap();
            let mut command = vkxp::Run {
                executable: &executable,
                rtps: &options.rtps,
                font_dir: None,
                builtin_scripts: options.builtin_scripts.as_deref(),
            }
            .command(&copy, &options.frames, &options.out)
            .unwrap();
            let status = command
                .env("XDG_RUNTIME_DIR", self.dir.path())
                .status()
                .expect("vkxp starts");
            assert!(status.success(), "{status}");
        }
    }

    #[test]
    fn the_arguments_are_read() {
        let root = Path::new("/workspace");
        let options = parse(
            &[
                "/games/one",
                "--frames",
                "20,10",
                "--dll",
                "/sys/RGSS103J.dll",
                "--rtp",
                "Standard=/rtp",
                "--input",
                "60:RIGHT, 100:",
                "--locale",
                "ja_JP.UTF-8",
                "--timeout",
                "30",
                "--font",
                "/fonts/one.otf",
                "--font",
                "/fonts/two.ttf",
            ],
            root,
        )
        .unwrap();
        assert_eq!(options.game, Path::new("/games/one"));
        assert_eq!(options.frames, [10, 20]);
        assert_eq!(options.dll.as_deref(), Some(Path::new("/sys/RGSS103J.dll")));
        assert_eq!(
            options.rtps,
            [("Standard".to_owned(), PathBuf::from("/rtp"))]
        );
        assert_eq!(options.schedule, Schedule::parse("60:RIGHT, 100:").unwrap());
        assert_eq!(
            options.fonts,
            [Path::new("/fonts/one.otf"), Path::new("/fonts/two.ttf")]
        );
        assert_eq!(options.out, Path::new("/workspace/target/oracle"));
        assert_eq!(options.locale, "ja_JP.UTF-8");
        assert_eq!(options.timeout, Duration::from_secs(30));
        assert!(!options.vkxp_only && !options.original_only && !options.no_threads);
        assert_eq!(options.runtime, None);

        let more = ["/games/one", "--frames", "1", "--runtime", "rgss2"];
        let options = parse(
            &[&more[..], &["--original-only", "--no-threads"]].concat(),
            root,
        );
        let options = options.unwrap();
        assert_eq!(options.runtime, Some(Runtime::Rgss2));
        assert!(options.original_only && options.no_threads && !options.vkxp_only);

        for wrong in [
            // The both-options scenario of `oracle-harness`: the command then prints the usage.
            &[
                "/games/one",
                "--frames",
                "1",
                "--original-only",
                "--vkxp-only",
            ][..],
            &["/games/one", "--frames", "1", "--runtime", "rgss4"],
            &["--frames", "1"],
            &["/games/one"],
            &["/games/one", "--frames"],
            &["/games/one", "--frames", "1", "--rtp", "Standard"],
            &["/games/one", "--frames", "1", "--colour"],
            &["/games/one", "/games/two", "--frames", "1"],
        ] {
            assert!(parse(wrong, root).is_err(), "{wrong:?}");
        }
    }

    #[test]
    fn vkxp_runs_a_game_whose_archive_the_writer_produced() {
        let generated = Generated::new(&[
            ("First", "$drawn = 0"),
            ("Main", "3.times { Graphics.update; $drawn += 1 }"),
        ]);
        generated.plain_run(&["--dump", "2=last.png"]);
        assert!(generated.dir.path().join("last.png").is_file());
    }

    #[test]
    fn vkxp_runs_a_game_that_the_command_packed() {
        let generated = Generated::new(&[
            ("First", "$drawn = 0"),
            ("Main", "3.times { Graphics.update; $drawn += 1 }"),
        ]);
        generated.pack();
        assert!(!generated.game().join("Data").exists());
        generated.plain_run(&["--dump", "2=last.png"]);
        assert!(generated.dir.path().join("last.png").is_file());
    }

    /// The capture-in-vkxp scenario of `oracle-harness`, and the source-untouched one.
    #[test]
    fn the_frames_of_vkxp_equal_those_of_a_plain_run_and_the_game_is_untouched() {
        let generated = Generated::new(&[("Main", MOVING_SQUARE)]);
        let before = generated.files();

        run(&generated.options("10,20", ""), &crate::workspace_root()).unwrap();

        assert_eq!(generated.files(), before);
        generated.plain_run(&[
            "--frames",
            "21",
            "--dump",
            "10=10.png",
            "--dump",
            "20=20.png",
        ]);
        let mut pictures = Vec::new();
        for frame in [10, 20] {
            let captured =
                report::Picture::read(&generated.out().join(report::vkxp_file(frame))).unwrap();
            assert_eq!((captured.width, captured.height), (640, 480));
            let plain =
                report::Picture::read(&generated.dir.path().join(format!("{frame}.png"))).unwrap();
            assert!(
                captured == plain,
                "frame {frame} differs from the plain run"
            );
            pictures.push(captured);
        }
        assert!(pictures[0] != pictures[1], "the square did not move");
        let summary = fs::read_to_string(generated.out().join("summary.json")).unwrap();
        assert!(summary.contains("frames/20-vkxp.png"), "{summary}");
    }

    /// The probe-inside-the-archive scenario of `oracle-harness`.
    #[test]
    fn the_probe_is_added_inside_the_archive_of_a_packed_game() {
        use crate::rgssad;

        let generated = Generated::new(&[("Main", MOVING_SQUARE)]);
        let game = generated.game();
        fs::write(game.join("Data/Map001.rxdata"), [4, 8, b'0']).unwrap();
        fs::create_dir_all(game.join("Graphics/Titles")).unwrap();
        fs::write(game.join("Graphics/Titles/one.png"), b"not read").unwrap();
        generated.pack();
        let before = generated.files();
        let packed = rgssad::read(&before[&game.join(rgssad::FILE_NAME)]).unwrap();
        let copy = generated.out().join("game");

        game::prepare(&game, &copy, Runtime::Rgss1, "$probed = true").unwrap();

        assert_eq!(generated.files(), before);
        assert!(!copy.join("Data").exists());
        let probed = rgssad::read(&fs::read(copy.join(rgssad::FILE_NAME)).unwrap()).unwrap();
        assert_eq!(probed.len(), 3);
        for (probed, packed) in probed.iter().zip(&packed) {
            assert_eq!(probed.name, packed.name);
            if probed.name != b"Data\\Scripts.rxdata" {
                assert_eq!(probed, packed);
                continue;
            }
            let scripts = archive::read(&probed.data).unwrap();
            assert_eq!(scripts[0].name, probe::NAME.as_bytes());
            assert_eq!(scripts[1..], archive::read(&packed.data).unwrap());
        }
    }

    /// The packed-game-runs-in-vkxp scenario of `oracle-harness`.
    #[test]
    fn the_frames_of_a_packed_game_equal_those_of_the_game_unpacked() {
        let pictures = |packed: bool| {
            let generated = Generated::new(&[("Main", MOVING_SQUARE)]);
            if packed {
                generated.pack();
            }
            run(&generated.options("10,20", ""), &crate::workspace_root()).unwrap();
            [10, 20].map(|frame| {
                report::Picture::read(&generated.out().join(report::vkxp_file(frame))).unwrap()
            })
        };
        let (packed, unpacked) = (pictures(true), pictures(false));
        assert!(packed == unpacked, "the packed game draws other frames");
        assert!(packed[0] != packed[1], "the square did not move");
    }

    /// The scheduled-input scenario of `oracle-harness`: the probe runs in vkxp.
    #[test]
    fn the_probe_answers_input_from_the_schedule_by_frame() {
        // The transition comes first, so that the log also shows how its frames are counted.
        let main = "\
Graphics.freeze
Graphics.transition(4)
log = [OracleProbe.frame.to_s]
(4...130).each do
  Graphics.update
  Input.update
  right = Input::RIGHT
  answers = [Input.press?(right), Input.trigger?(right), Input.repeat?(right)]
  answers << Input.repeat?(Input::DOWN)
  marks = answers.map { |answer| answer ? '1' : '0' }.join
  log << \"#{OracleProbe.frame} #{marks} #{Input.dir4} #{Input.dir8} #{Input.press?(Input::C)}\"
end
File.open('input.log', 'w') { |file| file.write(log.join(\"\\n\")) }
loop { Graphics.update }
";
        let generated = Generated::new(&[("Main", main)]);
        let options = generated.options("135", "60:RIGHT, 80:RIGHT+DOWN, 90:RIGHT, 100:");

        run(&options, &crate::workspace_root()).unwrap();

        let log = fs::read_to_string(generated.out().join("vkxp/game/input.log")).unwrap();
        let mut lines = log.lines();
        assert_eq!(lines.next(), Some("4"), "a transition of 4 is 4 frames");
        let mut frames = 5..;
        for (line, frame) in lines.zip(&mut frames) {
            let pressed = (60..100).contains(&frame);
            // RIGHT repeats at its 1st, 16th and 20th update. Then DOWN takes the repeat for
            // the ten frames it is held, and RIGHT does not get it back.
            let right_repeated = matches!(frame, 60 | 75 | 79);
            let marks: String = [pressed, frame == 60, right_repeated, frame == 80]
                .map(|answer| if answer { '1' } else { '0' })
                .into_iter()
                .collect();
            // RIGHT was held alone last, so DOWN is the direction while both are held.
            let (dir4, dir8) = match frame {
                80..90 => (2, 3),
                60..100 => (6, 6),
                _ => (0, 0),
            };
            assert_eq!(line, format!("{frame} {marks} {dir4} {dir8} false"));
        }
        assert_eq!(frames.next(), Some(131), "every frame was logged");
    }

    /// The VX-game, VX-Ace-game and XP-game scenarios of `oracle-harness`; the registry
    /// script itself is covered in `original`.
    #[test]
    fn the_runtime_of_the_game_chooses_the_probe_and_the_registry_key() {
        let first_script = |generated: &Generated, runtime, rtp: &str| {
            let mut options = generated.options("10", "");
            options.rtps = vec![(rtp.to_owned(), PathBuf::from("/rtp"))];
            let copy = generated.out().join("game");
            prepare_copy(&options, runtime, Side::Original, &copy).unwrap();
            let name = game::runtime(&options.game, None).unwrap();
            assert_eq!(name, runtime);
            let file = match runtime {
                Runtime::Rgss1 => "Data/Scripts.rxdata",
                Runtime::Rgss2 => "Data/Scripts.rvdata",
                Runtime::Rgss3 => "Data/Scripts.rvdata2",
            };
            let scripts = archive::read(&fs::read(copy.join(file)).unwrap()).unwrap();
            assert_eq!(scripts.len(), 2);
            (scripts[0].clone(), options)
        };
        let probe = |runtime, options: &Options| {
            let source = probe::generate(runtime, Side::Original, &options.schedule, &[10], false);
            archive::Entry::new(0, probe::NAME, &source)
        };

        let vx = Generated::named("Scripts.rvdata", "RTP=RPGVX\r\n", &[("Main", "")]);
        let (script, options) = first_script(&vx, Runtime::Rgss2, "RPGVX");
        assert!(script == probe(Runtime::Rgss2, &options));
        assert!(script != probe(Runtime::Rgss1, &options));
        let registry = original::rtp_registry_script(Runtime::Rgss2, &options.rtps, true);
        assert!(registry.contains("\\Enterbrain\\RGSS2\\RTP]\r\n\"RPGVX\"=\"Z:\\\\rtp\"\r\n"));

        let ace = Generated::named("Scripts.rvdata2", "RTP=RPGVXAce\r\n", &[("Main", "")]);
        let (script, options) = first_script(&ace, Runtime::Rgss3, "RPGVXAce");
        assert!(script == probe(Runtime::Rgss3, &options));
        assert!(script != probe(Runtime::Rgss2, &options));
        let registry = original::rtp_registry_script(Runtime::Rgss3, &options.rtps, true);
        assert!(
            registry.contains(
                "[HKEY_LOCAL_MACHINE\\Software\\Wow6432Node\\Enterbrain\\RGSS3\\RTP]\r\n\
                 \"RPGVXAce\"=\"Z:\\\\rtp\"\r\n"
            ),
            "{registry}"
        );

        let xp = Generated::new(&[("Main", "")]);
        let (script, options) = first_script(&xp, Runtime::Rgss1, "RPGVX");
        assert!(script == probe(Runtime::Rgss1, &options));
        let registry = original::rtp_registry_script(Runtime::Rgss1, &options.rtps, true);
        assert!(
            registry.contains("\\Enterbrain\\RGSS\\RTP]\r\n"),
            "{registry}"
        );
    }

    /// The unknown-extension scenario of `oracle-harness`.
    #[test]
    fn a_script_archive_of_another_extension_needs_the_option() {
        let generated = Generated::named("Code.bin", "", &[("Main", MOVING_SQUARE)]);
        let mut options = generated.options("1", "");
        let error = run(&options, &crate::workspace_root()).unwrap_err();
        assert!(format!("{error:#}").contains("--runtime"), "{error:#}");
        assert!(!generated.out().exists(), "nothing was started");

        options.runtime = Some(Runtime::Rgss1);
        run(&options, &crate::workspace_root()).unwrap();
        assert!(generated.out().join(report::vkxp_file(1)).is_file());
    }

    /// The archive-of-VX scenario of `oracle-harness`.
    #[test]
    fn the_probe_is_added_inside_the_archive_of_a_packed_game_of_rgss2() {
        use crate::rgssad;

        let generated = Generated::named("Scripts.rvdata", "RTP=\r\n", &[("Main", "")]);
        let game = generated.game();
        fs::write(game.join("Data/Map001.rvdata"), [4, 8, b'0']).unwrap();
        generated.pack();
        fs::rename(
            game.join(rgssad::FILE_NAME),
            game.join(rgssad::FILE_NAME_RGSS2),
        )
        .unwrap();
        let before = generated.files();
        let packed = rgssad::read(&before[&game.join(rgssad::FILE_NAME_RGSS2)]).unwrap();
        let copy = generated.out().join("game");

        let runtime = game::runtime(&game, None).unwrap();
        game::prepare(&game, &copy, runtime, "$probed = true").unwrap();

        assert_eq!(runtime, Runtime::Rgss2);
        assert_eq!(generated.files(), before);
        assert!(!copy.join("Data").exists());
        let probed = fs::read(copy.join(rgssad::FILE_NAME_RGSS2)).unwrap();
        let probed = rgssad::read(&probed).unwrap();
        assert_eq!(probed.len(), 2);
        for (probed, packed) in probed.iter().zip(&packed) {
            assert_eq!(probed.name, packed.name);
            if probed.name != b"Data\\Scripts.rvdata" {
                assert_eq!(probed, packed);
                continue;
            }
            let scripts = archive::read(&probed.data).unwrap();
            assert_eq!(scripts[0].name, probe::NAME.as_bytes());
            assert_eq!(scripts[1..], archive::read(&packed.data).unwrap());
        }
    }

    /// The archive-of-VX-Ace scenario of `oracle-harness`.
    #[test]
    fn the_probe_is_added_inside_the_archive_of_a_packed_game_of_rgss3() {
        let generated = Generated::ace(&[("Main", "")]);
        let game = generated.game();
        fs::write(game.join("Data/Map001.rvdata2"), [4, 8, b'0']).unwrap();
        fs::write(game.join("Data/Actors.rvdata2"), b"\x04\x08[\x00").unwrap();
        generated.pack_rgss3a();
        let before = generated.files();
        let packed = rgss3a::read(&before[&game.join(rgss3a::FILE_NAME)]).unwrap();
        let copy = generated.out().join("game");

        let runtime = game::runtime(&game, None).unwrap();
        game::prepare(&game, &copy, runtime, "$probed = true").unwrap();

        assert_eq!(runtime, Runtime::Rgss3);
        assert_eq!(generated.files(), before);
        assert!(!copy.join("Data").exists());
        let probed = rgss3a::read(&fs::read(copy.join(rgss3a::FILE_NAME)).unwrap()).unwrap();
        assert_eq!(probed.seed, 0x2a38);
        assert_eq!(probed.end, packed.end);
        assert_eq!(probed.entries.len(), 3);
        for (probed, packed) in probed.entries.iter().zip(&packed.entries) {
            assert_eq!((&probed.name, probed.key), (&packed.name, packed.key));
            if probed.name != b"Data\\Scripts.rvdata2" {
                assert_eq!(probed.data, packed.data);
                continue;
            }
            let scripts = archive::read(&probed.data).unwrap();
            assert_eq!(scripts[0].name, probe::NAME.as_bytes());
            assert_eq!(scripts[1..], archive::read(&packed.data).unwrap());
        }
    }

    /// The DLL-given and DLL-of-the-game scenarios of `oracle-harness`.
    #[test]
    fn the_dll_of_a_game_of_rgss3_is_at_the_path_of_its_library() {
        let generated = Generated::ace(&[("Main", "")]);
        let game = generated.game();
        fs::create_dir_all(game.join("system")).unwrap();
        fs::write(game.join("system/readme.txt"), b"a file").unwrap();
        let dll = generated.dir.path().join("Given.dll");
        fs::write(&dll, b"the given DLL").unwrap();
        let options = generated.options("10", "");
        let copy = generated.out().join("given");

        original::check_dll(&game, Runtime::Rgss3, Some(&dll)).unwrap();
        prepare_copy(&options, Runtime::Rgss3, Side::Original, &copy).unwrap();
        original::place_dll(&copy, Runtime::Rgss3, Some(&dll)).unwrap();

        let placed = fs::read(copy.join("system/RGSS300.dll")).unwrap();
        assert_eq!(placed, b"the given DLL");
        assert!(!copy.join("System").exists());
        assert!(!copy.join("Given.dll").exists());
        let dlls: Vec<_> = fs::read_dir(&copy)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name.to_string_lossy().to_lowercase().ends_with(".dll"))
            .collect();
        assert!(dlls.is_empty(), "{dlls:?}");

        // The game's own file, used without --dll.
        fs::write(game.join("system/RGSS300.dll"), b"the game's DLL").unwrap();
        let copy = generated.out().join("own");
        original::check_dll(&game, Runtime::Rgss3, None).unwrap();
        prepare_copy(&options, Runtime::Rgss3, Side::Original, &copy).unwrap();
        original::place_dll(&copy, Runtime::Rgss3, None).unwrap();
        let placed = fs::read(copy.join("system/RGSS300.dll")).unwrap();
        assert_eq!(placed, b"the game's DLL");

        // A folder that the game lacks is made.
        let other = Generated::ace(&[("Main", "")]);
        let copy = other.out().join("made");
        prepare_copy(
            &other.options("10", ""),
            Runtime::Rgss3,
            Side::Original,
            &copy,
        )
        .unwrap();
        original::place_dll(&copy, Runtime::Rgss3, Some(&dll)).unwrap();
        assert_eq!(
            fs::read(copy.join("System/RGSS300.dll")).unwrap(),
            b"the given DLL"
        );
    }

    /// The no-DLL and path-outside-the-game scenarios of `oracle-harness`, and the other
    /// paths of `Library=` that are refused.
    #[test]
    fn a_game_of_rgss3_without_its_dll_or_with_a_path_outside_it_is_an_error() {
        let generated = Generated::ace(&[("Main", "")]);
        let mut options = generated.options("10", "");
        options.vkxp_only = false;
        let error = run(&options, &crate::workspace_root()).unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("`System\\RGSS300.dll`"), "{message}");
        assert!(message.contains("--dll"), "{message}");
        assert!(!generated.out().exists(), "nothing was started");

        let ini = |library: &str| {
            let text = format!("[Game]\r\n{library}Scripts=Data\\Scripts.rvdata2\r\nRTP=\r\n");
            fs::write(generated.game().join("Game.ini"), text).unwrap();
        };
        let dll = generated.dir.path().join("Given.dll");
        fs::write(&dll, b"the given DLL").unwrap();
        for (library, named) in [
            ("..\\RGSS300.dll", "..\\RGSS300.dll"),
            ("System/../../RGSS300.dll", "System/../../RGSS300.dll"),
            ("\\RGSS300.dll", "\\RGSS300.dll"),
            ("C:\\RGSS300.dll", "C:\\RGSS300.dll"),
            ("", "Library="),
        ] {
            let line = match library {
                "" => String::new(),
                _ => format!("Library={library}\r\n"),
            };
            ini(&line);
            for given in [None, Some(dll.clone())] {
                options.dll = given;
                let error = run(&options, &crate::workspace_root()).unwrap_err();
                let message = format!("{error:#}");
                assert!(message.contains(named), "{library}: {message}");
                assert!(!generated.out().exists(), "nothing was started");
            }
        }
    }

    /// What RGSS2 has and the `Graphics` of vkxp, which runs these games as RGSS1, has not,
    /// so that the probe of RGSS2 can be run: the functions show nothing here.
    const RGSS2_STAND_IN: &str = "\
class << Graphics
  def wait(duration)
  end

  def fadeout(duration)
  end

  def fadein(duration)
  end

  def brightness
    @oracle_test_brightness || 255
  end

  def brightness=(value)
    @oracle_test_brightness = value
  end
end
";

    /// The scenarios of `oracle-harness` that are stated on the model of the probe of RGSS2:
    /// its repeat rule and its frame numbers.
    #[test]
    fn the_probe_of_rgss2_repeats_and_counts_frames_as_measured() {
        let main = "\
repeats = []
(1..40).each do |update|
  Input.update
  repeats << update if Input.repeat?(Input::DOWN)
  Graphics.update
end
counts = []
mark = OracleProbe.frame
[proc { Graphics.wait(5) }, proc { Graphics.fadeout(3) }, proc { Graphics.transition(20) },
 proc { Graphics.freeze }, proc { Graphics.transition(20) }].each do |call|
  call.call
  counts << OracleProbe.frame - mark
  mark = OracleProbe.frame
end
File.open('model.log', 'w') { |file| file.write(repeats.join(' ') + '|' + counts.join(' ')) }
";
        let schedule = Schedule::parse("0:DOWN").unwrap();
        let probe = probe::generate(Runtime::Rgss2, Side::Vkxp, &schedule, &[], false);
        let generated = Generated::new(&[
            ("Stand-in", RGSS2_STAND_IN),
            (probe::NAME, &probe),
            ("Main", main),
        ]);

        generated.plain_run(&[]);

        let log = fs::read_to_string(generated.game().join("model.log")).unwrap();
        assert_eq!(log, "1 24 30 36|5 3 0 0 20");
    }

    /// The probe of RGSS2 on the original's side, run in vkxp with this test in the place of
    /// the harness: frames inside `wait` and a fade are held at their frame, and a frame
    /// inside a transition is held after it and marked as late, unless it is its last.
    #[test]
    fn the_probe_of_rgss2_holds_inside_waits_and_marks_frames_in_a_transition_as_late() {
        // Frame 0, then 1 to 4, 5 to 8, 9 to 18, and updates from 19 on.
        let main = "\
Graphics.update
Graphics.wait(4)
Graphics.fadeout(4)
Graphics.freeze
Graphics.transition(10)
loop { Graphics.update }
";
        let captures = [2, 7, 12, 18, 20];
        let schedule = Schedule::default();
        let probe = probe::generate(Runtime::Rgss2, Side::Original, &schedule, &captures, false);
        let generated = Generated::new(&[
            ("Stand-in", RGSS2_STAND_IN),
            (probe::NAME, &probe),
            ("Main", main),
        ]);

        assert_eq!(
            generated.answered_requests(),
            ["2", "7", "12 late", "18", "20"]
        );
    }

    /// The block-runs-at-once scenario of `oracle-harness`, and the report of the option.
    #[test]
    fn with_the_option_the_block_of_a_thread_runs_at_once_in_the_caller() {
        let main = "\
started = Time.now
x = 0
Thread.new { sleep(5); x = 1 }
File.open('threads.log', 'w') { |file| file.write(\"#{x} #{Time.now - started < 4}\") }
loop { Graphics.update }
";
        let generated = Generated::new(&[("Main", main)]);
        let mut options = generated.options("1", "");
        options.no_threads = true;

        run(&options, &crate::workspace_root()).unwrap();

        let log = fs::read_to_string(generated.out().join("vkxp/game/threads.log")).unwrap();
        assert_eq!(log, "1 true");
        let summary = fs::read_to_string(generated.out().join("summary.json")).unwrap();
        assert!(summary.contains("\"no_threads\": true"), "{summary}");
    }

    /// The family-name scenario of `oracle-harness`, on the side of vkxp; the registry script
    /// is covered in `fonts`.
    #[test]
    fn the_command_line_of_vkxp_names_a_directory_with_a_copy_of_the_font() {
        let dir = tempfile::tempdir().unwrap();
        let font = dir.path().join("TestSans.ttf");
        let data = fonts::minimal_font(&[(1, 0x409, "Test Sans")]);
        fs::write(&font, &data).unwrap();
        let out = dir.path().join("out");

        let shared = fonts::share(&[font], &out).unwrap().unwrap();
        let command = vkxp::Run {
            executable: Path::new("vkxp"),
            rtps: &[],
            font_dir: Some(&shared.dir),
            builtin_scripts: None,
        }
        .command(&dir.path().join("game"), &[3], &out)
        .unwrap();

        assert_eq!(shared.family, "Test Sans");
        let arguments: Vec<_> = command.get_args().collect();
        let option = arguments
            .iter()
            .position(|argument| *argument == "--font-dir")
            .expect("a font directory");
        let font_dir = Path::new(arguments[option + 1]);
        assert_eq!(fs::read(font_dir.join("TestSans.ttf")).unwrap(), data);
    }

    /// The command-line scenario of `oracle-harness`.
    #[test]
    fn the_command_line_of_vkxp_asks_for_own_metrics_exactly_with_shared_fonts() {
        let own_metrics = |font_dir| {
            let command = vkxp::Run {
                executable: Path::new("vkxp"),
                rtps: &[],
                font_dir,
                builtin_scripts: None,
            }
            .command(Path::new("game"), &[3], Path::new("out"))
            .unwrap();
            command
                .get_args()
                .any(|argument| argument == "--font-own-metrics")
        };
        assert!(own_metrics(Some(Path::new("out/fonts"))));
        assert!(!own_metrics(None));
    }

    /// The not-a-font scenario of `oracle-harness`.
    #[test]
    fn a_font_that_is_a_text_file_is_an_error_naming_it() {
        let generated = Generated::new(&[("Main", MOVING_SQUARE)]);
        let mut options = generated.options("1", "");
        let text = generated.dir.path().join("notes.txt");
        fs::write(&text, "These are notes, not glyphs.\n").unwrap();
        options.fonts = vec![text];
        let error = run(&options, &crate::workspace_root()).unwrap_err();
        assert!(format!("{error:#}").contains("notes.txt"), "{error:#}");
        assert!(!generated.out().exists(), "nothing was started");

        options.fonts = vec![generated.dir.path().join("Missing.ttf")];
        let error = run(&options, &crate::workspace_root()).unwrap_err();
        assert!(format!("{error:#}").contains("Missing.ttf"), "{error:#}");
        assert!(!generated.out().exists(), "nothing was started");
    }

    /// The no-font-given scenario of `oracle-harness`: this is the line that a run prints.
    #[test]
    fn without_a_font_the_output_says_that_text_is_not_comparable() {
        let line = fonts::notice(None);
        assert!(line.contains("text is not comparable"), "{line}");
        assert_eq!(line.lines().count(), 1);

        let shared = fonts::Shared {
            family: "Test Sans".to_owned(),
            dir: PathBuf::new(),
            files: Vec::new(),
        };
        let line = fonts::notice(Some(&shared));
        assert!(!line.contains("not comparable"), "{line}");
    }

    /// The capture-in-the-original scenario of `oracle-harness`. It needs the original
    /// runtime, which is not in the repository: `VKXP_ORACLE_DLL` names the RGSS DLL,
    /// `VKXP_ORACLE_RTP` the directory of the run-time package `Standard`, and
    /// `VKXP_ORACLE_GAME` a game with its `Game.exe` whose `Game.ini` names that DLL, all
    /// three as absolute paths.
    /// Everything is written below `target/oracle-test`.
    #[test]
    #[ignore = "needs Wine, Xvfb and the original runtime; see docs/development.md"]
    fn the_original_is_captured_at_the_frames_asked_for() {
        let variable =
            |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));
        let root = crate::workspace_root();
        let out = root.join("target").join("oracle-test");
        let (game, dll) = (variable("VKXP_ORACLE_GAME"), variable("VKXP_ORACLE_DLL"));
        let rtp = format!("Standard={}", variable("VKXP_ORACLE_RTP"));
        let arguments = [&game, "--frames", "10,20", "--dll", &dll, "--rtp", &rtp];
        let mut options = parse(&arguments, &root).unwrap();
        options.out = out.clone();
        let dll = options.dll.clone().unwrap();
        original::check_prerequisites(&options.game, Runtime::Rgss1, Some(&dll)).unwrap();
        fs::create_dir_all(out.join("frames")).unwrap();

        capture_original(&options, Runtime::Rgss1, Some(&dll), None).unwrap();

        for frame in [10, 20] {
            let picture = report::Picture::read(&out.join(report::original_file(frame))).unwrap();
            assert_eq!((picture.width, picture.height), (640, 480));
        }
    }

    /// The missing-prerequisites scenario of `oracle-harness`.
    #[test]
    fn a_dll_that_does_not_exist_is_an_error_naming_it() {
        let generated = Generated::new(&[("Main", MOVING_SQUARE)]);
        let mut options = generated.options("1", "");
        options.vkxp_only = false;
        options.dll = Some(generated.dir.path().join("RGSS999.dll"));
        let error = run(&options, &crate::workspace_root()).unwrap_err();
        assert!(format!("{error:#}").contains("RGSS999.dll"), "{error:#}");
        assert!(!generated.out().exists(), "nothing was started");

        options.dll = None;
        let error = run(&options, &crate::workspace_root()).unwrap_err();
        assert!(format!("{error:#}").contains("--dll"), "{error:#}");

        options.game = generated.dir.path().join("nowhere");
        let error = run(&options, &crate::workspace_root()).unwrap_err();
        assert!(format!("{error:#}").contains("nowhere"), "{error:#}");
    }

    /// Games of RGSS3 run in vkxp. They need the guest executable of RGSS3, which is built
    /// only where Ruby 1.9.2 is, as for the tests of `crates/vkxp/tests/rgss3.rs`.
    #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
    mod rgss3 {
        use super::*;

        /// The capture-in-vkxp-of-RGSS3 scenario of `oracle-harness`.
        #[test]
        fn the_frames_of_vkxp_of_rgss3_equal_those_of_a_plain_run() {
            let generated = Generated::ace(&[("Main", MOVING_SQUARE)]);

            generated.vkxp_side(&generated.options("10,20", ""));

            generated.plain_run(&[
                "--frames",
                "21",
                "--dump",
                "10=10.png",
                "--dump",
                "20=20.png",
            ]);
            let mut pictures = Vec::new();
            for frame in [10, 20] {
                let file = generated.out().join(report::vkxp_file(frame));
                let captured = report::Picture::read(&file).unwrap();
                assert_eq!((captured.width, captured.height), (544, 416));
                let file = generated.dir.path().join(format!("{frame}.png"));
                let plain = report::Picture::read(&file).unwrap();
                assert!(
                    captured == plain,
                    "frame {frame} differs from the plain run"
                );
                pictures.push(captured);
            }
            assert!(pictures[0] != pictures[1], "the square did not move");
        }

        /// The packed-game-of-RGSS3-runs-in-vkxp scenario of `oracle-harness`.
        #[test]
        fn the_frames_of_a_game_of_rgss3_in_its_archive_equal_those_of_the_game_unpacked() {
            let pictures = |packed: bool| {
                let generated = Generated::ace(&[("Main", MOVING_SQUARE)]);
                if packed {
                    generated.pack_rgss3a();
                }
                generated.vkxp_side(&generated.options("10,20", ""));
                [10, 20].map(|frame| {
                    let file = generated.out().join(report::vkxp_file(frame));
                    report::Picture::read(&file).unwrap()
                })
            };
            let (packed, unpacked) = (pictures(true), pictures(false));
            assert!(packed == unpacked, "the packed game draws other frames");
            assert!(packed[0] != packed[1], "the square did not move");
        }

        /// The buttons-of-RGSS3 scenario of `oracle-harness`: the probe runs in vkxp.
        #[test]
        fn the_probe_of_rgss3_takes_a_button_as_its_symbol_or_its_integer_of_rgss2() {
            let main = "\
answers = nil
loop do
  Input.update
  if OracleProbe.frame == 70 && answers.nil?
    asked = [:RIGHT, 6, :right, 'RIGHT', nil, :LEFT]
    answers = asked.map { |button| Input.press?(button) ? '1' : '0' }.join
    File.open('buttons.log', 'w') { |file| file.write(answers) }
  end
  Graphics.update
end
";
            let generated = Generated::ace(&[("Main", main)]);
            let options = generated.options("75", "60:RIGHT, 100:");

            generated.vkxp_side(&options);

            let log = generated.out().join("vkxp/game/buttons.log");
            assert_eq!(fs::read_to_string(log).unwrap(), "110000");
        }

        /// The scenarios of `oracle-harness` that are stated on the model of the probe of
        /// RGSS3: its repeat rule and its frame numbers.
        #[test]
        fn the_probe_of_rgss3_repeats_and_counts_frames_as_measured() {
            let main = "\
repeats = []
(1..40).each do |update|
  Input.update
  repeats << update if Input.repeat?(:DOWN)
  Graphics.update
end
counts = []
mark = OracleProbe.frame
[proc { Graphics.wait(5) }, proc { Graphics.fadeout(3) }, proc { Graphics.transition(20) },
 proc { Graphics.freeze }, proc { Graphics.transition(20) }].each do |call|
  call.call
  counts << OracleProbe.frame - mark
  mark = OracleProbe.frame
end
File.open('model.log', 'w') { |file| file.write(repeats.join(' ') + '|' + counts.join(' ')) }
";
            let schedule = Schedule::parse("0:DOWN").unwrap();
            let probe = probe::generate(Runtime::Rgss3, Side::Vkxp, &schedule, &[], false);
            let generated = Generated::ace(&[(probe::NAME, &probe), ("Main", main)]);

            generated.plain_run(&[]);

            let log = fs::read_to_string(generated.game().join("model.log")).unwrap();
            assert_eq!(log, "1 24 30 36|5 3 0 0 20");
        }

        /// The screen-resized-in-RGSS3 scenario of `oracle-harness`.
        #[test]
        fn the_probe_of_rgss3_asks_for_the_size_the_screen_has_at_the_frame() {
            let main = "\
3.times { Graphics.update }
Graphics.resize_screen(640, 480)
loop { Graphics.update }
";
            let schedule = Schedule::default();
            let probe = probe::generate(Runtime::Rgss3, Side::Original, &schedule, &[2, 5], false);
            let generated = Generated::ace(&[(probe::NAME, &probe), ("Main", main)]);

            assert_eq!(generated.answered_requests(), ["2 544x416", "5 640x480"]);
        }

        /// The late-frame-of-RGSS3 scenario of `oracle-harness`.
        #[test]
        fn the_probe_of_rgss3_marks_only_a_frame_inside_a_transition_as_late() {
            // Frame 0, then 1 to 4, 5 to 8, 9 to 18, and updates from 19 on.
            let main = "\
Graphics.update
Graphics.wait(4)
Graphics.fadeout(4)
Graphics.freeze
Graphics.transition(10)
loop { Graphics.update }
";
            let captures = [2, 7, 12, 18, 20];
            let schedule = Schedule::default();
            let probe =
                probe::generate(Runtime::Rgss3, Side::Original, &schedule, &captures, false);
            let generated = Generated::ace(&[(probe::NAME, &probe), ("Main", main)]);

            assert_eq!(
                generated.answered_requests(),
                [
                    "2 544x416",
                    "7 544x416",
                    "12 late 544x416",
                    "18 544x416",
                    "20 544x416"
                ]
            );
        }

        /// The block-runs-at-once scenario of `oracle-harness` with the probe of RGSS3.
        #[test]
        fn with_the_option_the_block_of_a_thread_of_rgss3_runs_at_once_in_the_caller() {
            let main = "\
started = Time.now
x = 0
Thread.new { sleep(5); x = 1 }
File.open('threads.log', 'w') { |file| file.write(\"#{x} #{Time.now - started < 4}\") }
loop { Graphics.update }
";
            let generated = Generated::ace(&[("Main", main)]);
            let mut options = generated.options("1", "");
            options.no_threads = true;

            generated.vkxp_side(&options);

            let log = fs::read_to_string(generated.out().join("vkxp/game/threads.log")).unwrap();
            assert_eq!(log, "1 true");
        }
    }
}
