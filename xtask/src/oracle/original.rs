//! The original side of a comparison: `Game.exe` under Wine in a virtual display of its own.
//!
//! The probe holds the game at each capture frame by writing a request file into the game
//! copy; the picture of the game window is then read from the display and the request is
//! removed.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, ImageFormat, ImageOrder, MapState, Window};

use super::fonts;
use super::game::{self, Library};
use super::report::{self, Picture};
use super::runtime::Runtime;

/// How long the picture of a held frame is given to reach the display.
const SETTLE: Duration = Duration::from_millis(200);

/// The program every Wine command runs inside.
const SANDBOX: &str = "bwrap";

/// The sandbox of a Wine command: the file system as it is, except that `/dev/dri` is an
/// empty directory, so that no library of the run can open a GPU device node, and no network.
/// No process-id namespace of its own: an `Xvfb` inside one takes the display lock files of
/// other processes for stale.
const SANDBOX_ARGUMENTS: [&str; 8] = [
    "--dev-bind",
    "/",
    "/",
    "--tmpfs",
    "/dev/dri",
    "--unshare-net",
    "--die-with-parent",
    "--",
];

/// A directory that does not exist, where VA-API is sent to look for its drivers.
const NO_VA_DRIVERS: &str = "/nonexistent/vkrgss-oracle-no-va-drivers";

/// The directories where a system keeps the plugins of GStreamer 1.0.
pub(super) const GSTREAMER_SYSTEM_DIRS: [&str; 4] = [
    "/usr/lib64/gstreamer-1.0",
    "/usr/lib/gstreamer-1.0",
    "/usr/lib/x86_64-linux-gnu/gstreamer-1.0",
    "/usr/lib/i386-linux-gnu/gstreamer-1.0",
];

/// The plugins of GStreamer that open a GPU or a video device, the list of the measurements
/// of VX Ace (`ace-v0-env.sh`, kept outside the repository).
fn is_hardware_plugin(name: &str) -> bool {
    matches!(
        name,
        "libgstva.so"
            | "libgstqsv.so"
            | "libgstmsdk.so"
            | "libgstvulkan.so"
            | "libgstkms.so"
            | "libgstopengl.so"
            | "libgstqmlgl.so"
            | "libgstnvcodec.so"
            | "libgstvideo4linux2.so"
            | "libgstv4l2codecs.so"
    ) || (name.starts_with("libgstvaapi") && name.ends_with(".so"))
}

/// The plugins of GStreamer, which plays Ogg and MP3 for Wine, that the Wine processes of a
/// run may load: links to those of the system without its hardware plugins, which probe
/// VA-API and other devices, in directories below the output directory, with a registry of
/// their own so that the user's registry is not rewritten with this view. A third line
/// behind the sandbox and the variables of VA-API.
pub(super) struct GStreamer {
    /// The value of `GST_PLUGIN_SYSTEM_PATH_1_0`: the directories of the links.
    system_path: OsString,
    registry: PathBuf,
}

impl GStreamer {
    /// Makes the directories of the links below `out/gstreamer` afresh, one for each of
    /// `system`, the directories of the system's plugins, that exists; an empty one when
    /// none does.
    pub(super) fn prepare(out: &Path, system: &[&Path]) -> Result<GStreamer> {
        let base = out.join("gstreamer");
        fs::create_dir_all(&base).with_context(|| base.display().to_string())?;
        // The links of an earlier run may name plugins that are gone.
        for entry in fs::read_dir(&base).with_context(|| base.display().to_string())? {
            let entry = entry.with_context(|| base.display().to_string())?;
            if entry.file_type()?.is_dir() {
                let path = entry.path();
                fs::remove_dir_all(&path).with_context(|| path.display().to_string())?;
            }
        }
        let mut dirs = Vec::new();
        for dir in system.iter().filter(|dir| dir.is_dir()) {
            let made = base.join(format!("plugins-{}", dirs.len()));
            fs::create_dir(&made).with_context(|| made.display().to_string())?;
            for entry in fs::read_dir(dir).with_context(|| dir.display().to_string())? {
                let name = entry
                    .with_context(|| dir.display().to_string())?
                    .file_name();
                let Some(text) = name.to_str() else {
                    continue;
                };
                if text.ends_with(".so") && !is_hardware_plugin(text) {
                    std::os::unix::fs::symlink(dir.join(&name), made.join(&name))
                        .with_context(|| made.join(&name).display().to_string())?;
                }
            }
            dirs.push(made);
        }
        if dirs.is_empty() {
            let empty = base.join("no-plugins");
            fs::create_dir(&empty).with_context(|| empty.display().to_string())?;
            dirs.push(empty);
        }
        Ok(GStreamer {
            system_path: std::env::join_paths(&dirs).context("the GStreamer plugin directories")?,
            registry: base.join("registry.bin"),
        })
    }
}

/// What a run of the original needs besides the game.
pub(super) struct Run<'a> {
    /// The runtime of the game, for the size of its window and the registry key of its
    /// run-time packages.
    pub runtime: Runtime,
    /// The RGSS DLL, which is copied into the game copy. A game of RGSS3 may run with the
    /// DLL it has.
    pub dll: Option<&'a Path>,
    /// The directory of each run-time package, by its name.
    pub rtps: &'a [(String, PathBuf)],
    /// The fonts that the game font names are replaced by, if any are shared with vkrgss.
    pub fonts: Option<&'a fonts::Shared>,
    /// The Wine prefix, made on first use.
    pub prefix: &'a Path,
    /// The plugins of GStreamer that Wine may load.
    pub gstreamer: &'a GStreamer,
    /// `LC_ALL` of the run, which decides the code page of the game's file names.
    pub locale: &'a str,
    /// How long one capture frame may take to be reached.
    pub timeout: Duration,
}

/// What must be there before the original side can run, checked before anything is started.
pub(super) fn check_prerequisites(game: &Path, runtime: Runtime, dll: Option<&Path>) -> Result<()> {
    check_dll(game, runtime, dll)?;
    ensure!(
        game.join("Game.exe").is_file(),
        "{}: the game has no Game.exe to run in the original runtime",
        game.display()
    );
    check_programs(std::env::var_os("PATH").as_deref())
}

/// Checks that the original of the game in `game` will have its DLL: `dll`, which is needed
/// for RGSS1 and RGSS2, or for RGSS3 the file the game has at the path of its `Library=`.
pub(super) fn check_dll(game: &Path, runtime: Runtime, dll: Option<&Path>) -> Result<()> {
    if let Some(dll) = dll {
        ensure!(
            dll.is_file(),
            "{}: the RGSS DLL does not exist",
            dll.display()
        );
    }
    if runtime != Runtime::Rgss3 {
        ensure!(
            dll.is_some(),
            "--dll is missing; without the original, use --vkrgss-only"
        );
        return Ok(());
    }
    let library = Library::of(game)?;
    ensure!(
        dll.is_some() || library.find(game).is_some(),
        "{}: the game has no DLL at `{}`, which `Library=` of its Game.ini names; give one \
         with --dll",
        game.display(),
        library.written
    );
    Ok(())
}

/// Puts the DLL into the game copy `game` where the original of `runtime` loads it from: for
/// RGSS3 at the path of `Library=`, replacing the file the game has there, and nothing when
/// no DLL is given; for RGSS1 and RGSS2 beside `Game.exe` under its own name.
pub(super) fn place_dll(game: &Path, runtime: Runtime, dll: Option<&Path>) -> Result<()> {
    let Some(dll) = dll else {
        return Ok(());
    };
    let target = match runtime {
        Runtime::Rgss3 => Library::of(game)?.place(game)?,
        Runtime::Rgss1 | Runtime::Rgss2 => {
            game.join(dll.file_name().context("the DLL path has no name")?)
        }
    };
    fs::copy(dll, &target).with_context(|| format!("{} to {}", dll.display(), target.display()))?;
    game::set_fixed_time(&target)
}

/// The programs of the original side, which must all be in a directory of `path`, the value
/// of `PATH`.
fn check_programs(path: Option<&OsStr>) -> Result<()> {
    for program in ["wine", "wineboot", "wineserver", "Xvfb", SANDBOX] {
        ensure!(
            path.is_some_and(|path| {
                std::env::split_paths(path).any(|dir| dir.join(program).is_file())
            }),
            "`{program}` is not on PATH; the original side needs Wine, Xvfb and bwrap \
             (--vkrgss-only runs without them)"
        );
    }
    Ok(())
}

impl Run<'_> {
    /// Runs the game copy `game`, into which the probe of the original side was put, and
    /// writes each of `frames`, which are in ascending order, as the picture `out` holds
    /// for the original. `log` receives what Wine and the game print. Returns the frames
    /// that the probe marked as late: their picture is the one after the transition they
    /// fall inside.
    pub(super) fn capture(
        &self,
        game: &Path,
        frames: &[u64],
        out: &Path,
        log: &Path,
    ) -> Result<Vec<u64>> {
        place_dll(game, self.runtime, self.dll)?;

        let display = Xvfb::start()?;
        // From here on the Wine server is shut down however the run ends.
        let _server = Server(self, &display.name);
        self.prepare_prefix(&display.name)?;

        let log_file = fs::File::create(log).with_context(|| log.display().to_string())?;
        let mut wine = Killed(
            self.wine("wine", &display.name)
                .arg("Game.exe")
                .current_dir(game)
                .stdin(Stdio::null())
                .stdout(log_file.try_clone().context("the log file")?)
                .stderr(log_file)
                .spawn()
                .context("could not start `wine`")?,
        );
        let mut late = Vec::new();
        for &frame in frames {
            let request = game.join(format!("capture-{frame}.request"));
            let deadline = Instant::now() + self.timeout;
            while !request.exists() {
                let failure = match wine.0.try_wait().context("waiting for `wine`")? {
                    Some(status) => format!("the original ended ({status})"),
                    None if Instant::now() > deadline => {
                        format!("the original took more than {:?}", self.timeout)
                    }
                    None => {
                        sleep(Duration::from_millis(10));
                        continue;
                    }
                };
                bail!(
                    "{failure} before frame {frame} was reached; its messages are in {}",
                    log.display()
                );
            }
            sleep(SETTLE);
            let record = fs::read(&request).with_context(|| request.display().to_string())?;
            let size = capture_size(self.runtime, &record);
            let picture = grab(&display.name, size).with_context(|| format!("frame {frame}"))?;
            picture.write(&out.join(report::original_file(frame)))?;
            if request_says_late(&record) {
                late.push(frame);
            }
            fs::remove_file(&request).with_context(|| request.display().to_string())?;
        }
        Ok(late)
    }

    /// A Wine program inside the sandbox, with the environment of the run, which the
    /// sandbox hands on.
    fn wine(&self, program: &str, display: &str) -> Command {
        let mut command = Command::new(SANDBOX);
        command
            .args(SANDBOX_ARGUMENTS)
            .arg(program)
            .env("WINEPREFIX", self.prefix)
            .env("WINEDEBUG", "err+all,+msgbox")
            // No Mono or Gecko installer, and no menu entries in the user's home directory.
            .env("WINEDLLOVERRIDES", "mscoree,mshtml=;winemenubuilder.exe=d")
            .env("DISPLAY", display)
            .env_remove("WAYLAND_DISPLAY")
            // Software rendering only: Wine must not open a GPU of the machine.
            .env("LIBGL_ALWAYS_SOFTWARE", "1")
            .env("VK_LOADER_DRIVERS_SELECT", "*lvp*")
            // The hardware plugins of GStreamer, which plays Ogg and MP3 for Wine, probe
            // VA-API: it finds no driver even where the sandbox would fail.
            .env("LIBVA_DRIVERS_PATH", NO_VA_DRIVERS)
            .env("LIBVA_DRIVER_NAME", "vkrgss-oracle-none")
            .env("GST_PLUGIN_SYSTEM_PATH_1_0", &self.gstreamer.system_path)
            .env("GST_REGISTRY_1_0", &self.gstreamer.registry)
            .env_remove("GST_PLUGIN_PATH")
            .env_remove("GST_PLUGIN_PATH_1_0")
            .env_remove("GST_PLUGIN_SYSTEM_PATH")
            // The runtime uses the ANSI file functions, so the locale decides which file
            // names of the game resolve.
            .env("LC_ALL", self.locale);
        command
    }

    /// Makes the prefix if it is new and writes what the runtime looks for into it.
    fn prepare_prefix(&self, display: &str) -> Result<()> {
        fs::create_dir_all(self.prefix).with_context(|| self.prefix.display().to_string())?;
        let prepared = self.prefix.join("vkxp-oracle-prepared");
        if !prepared.exists() {
            eprintln!("oracle: creating the Wine prefix {}", self.prefix.display());
            run(self.wine("wineboot", display).arg("-i"), "wineboot -i")?;
            fs::write(&prepared, b"").with_context(|| prepared.display().to_string())?;
        }

        // Written on every run, because the fonts may differ from those of the last one.
        let windows = self.prefix.join("drive_c/windows");
        let installed = fonts::install(self.fonts, &windows.join("Fonts"))?;
        let family = self.fonts.map(|shared| shared.family.as_str());
        let script = self.prefix.join("drive_c/vkxp-oracle.reg");
        // A prefix of 64 bits keeps the 32-bit view of the registry below a node of its own.
        let wow64 = windows.join("syswow64").is_dir();
        let text = fonts::registry_script(family, &installed)
            + &rtp_registry_script(self.runtime, self.rtps, wow64);
        fs::write(&script, fonts::encode_script(&text))
            .with_context(|| script.display().to_string())?;
        let mut regedit = self.wine("wine", display);
        regedit.args(["regedit", "/S", "C:\\vkxp-oracle.reg"]);
        run(&mut regedit, "wine regedit")?;

        // The runtime refuses to start unless system32\D3D8.DLL exists, and a Wine without
        // 32-bit Direct3D 8 has none. Only the existence of the file is checked, so another
        // DLL of the prefix stands in for it.
        let system = ["syswow64", "system32"]
            .map(|name| windows.join(name))
            .into_iter()
            .find(|dir| dir.is_dir())
            .with_context(|| format!("{}: no system directory", windows.display()))?;
        let d3d8 = system.join("d3d8.dll");
        if !d3d8.exists() {
            let stand_in = system.join("d3d8thk.dll");
            if stand_in.is_file() {
                fs::copy(&stand_in, &d3d8).with_context(|| d3d8.display().to_string())?;
            } else {
                fs::write(&d3d8, b"").with_context(|| d3d8.display().to_string())?;
            }
        }

        // Wine lists the fonts once per server, so the game gets a server that starts after
        // the fonts were installed and registered.
        self.stop_server(display);
        Ok(())
    }

    /// Shuts the Wine server of the prefix down, which ends every Wine process in it.
    fn stop_server(&self, display: &str) {
        let _ = self.wine("wineserver", display).arg("-k").status();
        let _ = self.wine("wineserver", display).arg("-w").status();
    }
}

/// Whether the request file of a capture, as the probe wrote it, marks the frame as late.
fn request_says_late(record: &[u8]) -> bool {
    String::from_utf8_lossy(record)
        .split_whitespace()
        .any(|word| word == "late")
}

/// The size of the window to capture for a request file as the probe wrote it: the word of
/// the form `<width>x<height>` that the probe of RGSS3 writes, or the frame size of `runtime`
/// for a request without one.
fn capture_size(runtime: Runtime, record: &[u8]) -> (u32, u32) {
    String::from_utf8_lossy(record)
        .split_whitespace()
        .find_map(|word| {
            let (width, height) = word.split_once('x')?;
            let number = |text: &str| {
                text.bytes()
                    .all(|byte| byte.is_ascii_digit())
                    .then(|| text.parse().ok())
                    .flatten()
            };
            Some((number(width)?, number(height)?))
        })
        .unwrap_or_else(|| runtime.frame_size())
}

/// A top-level window of the display, as the choice of the game window sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TopLevel {
    window: Window,
    size: (u32, u32),
    viewable: bool,
}

/// The game window among `windows`, which are in stacking order with the topmost last: the
/// topmost viewable one of `size`.
fn choose_window(windows: &[TopLevel], size: (u32, u32)) -> Option<Window> {
    windows
        .iter()
        .rev()
        .find(|top| top.viewable && top.size == size)
        .map(|top| top.window)
}

/// The part of the registry script of a prefix that names the directory of each run-time
/// package, which the runtime reads from the 32-bit view of the registry. `wow64` says that
/// the prefix is one of 64 bits.
pub(super) fn rtp_registry_script(
    runtime: Runtime,
    rtps: &[(String, PathBuf)],
    wow64: bool,
) -> String {
    let quoted = |text: &str| format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""));
    let view = if wow64 { "Wow6432Node\\" } else { "" };
    let key = runtime.rtp_key();
    let mut script = format!("\r\n[HKEY_LOCAL_MACHINE\\Software\\{view}{key}]\r\n");
    for (name, dir) in rtps {
        let dir = format!("Z:{}", dir.display()).replace('/', "\\");
        script += &format!("{}={}\r\n", quoted(name), quoted(&dir));
    }
    script
}

fn run(command: &mut Command, name: &str) -> Result<()> {
    let output = command
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("could not start `{name}`"))?;
    ensure!(
        output.status.success(),
        "`{name}` ended with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// A child process that is killed when the run is over.
struct Killed(Child);

impl Drop for Killed {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Shuts the Wine server of the prefix down when the run is over, which ends every Wine
/// process in it.
struct Server<'a>(&'a Run<'a>, &'a str);

impl Drop for Server<'_> {
    fn drop(&mut self) {
        let Server(run, display) = self;
        run.stop_server(display);
    }
}

/// How long the display server is given to announce its display.
const DISPLAY_START: Duration = Duration::from_secs(10);

/// A virtual display started for one run.
struct Xvfb {
    /// The display as `DISPLAY` names it.
    name: String,
    _process: Killed,
}

impl Xvfb {
    /// Starts `Xvfb` on a display number that it chooses itself.
    fn start() -> Result<Xvfb> {
        Xvfb::start_server(Command::new("Xvfb"), DISPLAY_START)
    }

    /// Starts `server`, which takes the arguments of `Xvfb`, and waits up to `timeout` for
    /// the display number it announces. The server writes the number to its standard output,
    /// as `-displayfd` asks, once it accepts connections; a number picked here could be taken
    /// by another run before the server binds it.
    fn start_server(mut server: Command, timeout: Duration) -> Result<Xvfb> {
        let mut process = Killed(
            server
                .args(["-displayfd", "1"])
                .args(["-screen", "0", "1024x768x24", "-nolisten", "tcp"])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .context("could not start `Xvfb`")?,
        );
        let mut output = BufReader::new(process.0.stdout.take().context("no output of `Xvfb`")?);
        let (sender, announced) = mpsc::channel();
        // The read ends when the server has written its line or is gone, at the latest when
        // it is killed after the timeout.
        thread::spawn(move || {
            let mut line = String::new();
            let _ = sender.send(output.read_line(&mut line).map(|_| line));
        });
        let line = match announced.recv_timeout(timeout) {
            Ok(line) => line.context("reading the display number of `Xvfb`")?,
            Err(_) => bail!(
                "`Xvfb` did not announce a display within {} seconds",
                timeout.as_secs_f32()
            ),
        };
        ensure!(
            !line.is_empty(),
            "`Xvfb` exited without announcing a display; it may not be allowed to create its \
             socket in /tmp/.X11-unix"
        );
        let number: u32 = line
            .trim()
            .parse()
            .with_context(|| format!("`Xvfb` announced {line:?}, which is no display number"))?;
        Ok(Xvfb {
            name: format!(":{number}"),
            _process: process,
        })
    }
}

/// Reads the picture of the game window, the top-level window of `width` by `height`, from
/// `display`.
fn grab(display: &str, (width, height): (u32, u32)) -> Result<Picture> {
    let (connection, screen) =
        x11rb::connect(Some(display)).with_context(|| format!("the display {display}"))?;
    let setup = connection.setup();
    let root = setup.roots[screen].root;
    let mut windows = Vec::new();
    // The children of the root are in stacking order, the topmost last.
    for &child in &connection.query_tree(root)?.reply()?.children {
        // A window may be gone by the time it is asked about.
        let Ok(geometry) = connection.get_geometry(child)?.reply() else {
            continue;
        };
        let Ok(attributes) = connection.get_window_attributes(child)?.reply() else {
            continue;
        };
        windows.push(TopLevel {
            window: child,
            size: (u32::from(geometry.width), u32::from(geometry.height)),
            viewable: attributes.map_state == MapState::VIEWABLE,
        });
    }
    let window = choose_window(&windows, (width, height))
        .with_context(|| format!("no window of {width} by {height} on the display"))?;
    let image = connection
        .get_image(
            ImageFormat::Z_PIXMAP,
            window,
            0,
            0,
            width as u16,
            height as u16,
            !0,
        )?
        .reply()
        .context("reading the game window")?;
    let bits_per_pixel = setup
        .pixmap_formats
        .iter()
        .find(|format| format.depth == image.depth)
        .map(|format| format.bits_per_pixel);
    ensure!(
        bits_per_pixel == Some(32) && setup.image_byte_order == ImageOrder::LSB_FIRST,
        "the display does not store pixels as 32 bits with the least significant byte first"
    );
    let rgba: Vec<u8> = image
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|bgrx| [bgrx[2], bgrx[1], bgrx[0], 255])
        .collect();
    ensure!(
        rgba.len() == (width * height * 4) as usize,
        "the picture of the game window is incomplete"
    );
    Ok(Picture {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A program in place of `Xvfb`: a shell that runs `script` with the arguments of the
    /// server and the file descriptor of `-displayfd` in `$fd`.
    fn stand_in(script: &str) -> Command {
        let find = "while [ \"$1\" != -displayfd ]; do shift; done; fd=$2; ";
        let mut shell = Command::new("sh");
        shell.args(["-c", &format!("{find}{script}"), "sh"]);
        shell
    }

    /// The registry part of the VX-game scenario of `oracle-harness`, and the key of XP.
    #[test]
    fn the_registry_script_names_the_packages_under_the_key_of_the_runtime() {
        let rtps = [("RPGVX".to_owned(), PathBuf::from("/ref/vx/rtp"))];
        let script = rtp_registry_script(Runtime::Rgss2, &rtps, true);
        assert_eq!(
            script,
            "\r\n[HKEY_LOCAL_MACHINE\\Software\\Wow6432Node\\Enterbrain\\RGSS2\\RTP]\r\n\
             \"RPGVX\"=\"Z:\\\\ref\\\\vx\\\\rtp\"\r\n"
        );

        let rtps = [("Standard".to_owned(), PathBuf::from("/rtp"))];
        let script = rtp_registry_script(Runtime::Rgss1, &rtps, false);
        assert_eq!(
            script,
            "\r\n[HKEY_LOCAL_MACHINE\\Software\\Enterbrain\\RGSS\\RTP]\r\n\
             \"Standard\"=\"Z:\\\\rtp\"\r\n"
        );
    }

    /// The command-that-is-built and VA-API scenarios of `oracle-harness`.
    #[test]
    fn every_wine_command_runs_in_the_sandbox_without_a_driver_of_va_api() {
        let out = tempfile::tempdir().unwrap();
        let gstreamer = GStreamer::prepare(out.path(), &[]).unwrap();
        let run = Run {
            runtime: Runtime::Rgss1,
            dll: Some(Path::new("RGSS104E.dll")),
            rtps: &[],
            fonts: None,
            prefix: Path::new("prefix"),
            gstreamer: &gstreamer,
            locale: "zh_CN.UTF-8",
            timeout: Duration::from_secs(1),
        };
        for program in ["wine", "wineboot", "wineserver"] {
            let command = run.wine(program, ":104");
            assert_eq!(command.get_program(), "bwrap");
            let arguments: Vec<_> = command.get_args().map(|a| a.to_str().unwrap()).collect();
            assert_eq!(
                arguments,
                [
                    "--dev-bind",
                    "/",
                    "/",
                    "--tmpfs",
                    "/dev/dri",
                    "--unshare-net",
                    "--die-with-parent",
                    "--",
                    program
                ]
            );
            assert!(!arguments.contains(&"--unshare-pid"));

            let variable = |name: &str| {
                let (_, value) = command.get_envs().find(|(key, _)| *key == name).unwrap();
                value.unwrap().to_str().unwrap()
            };
            assert!(!Path::new(variable("LIBVA_DRIVERS_PATH")).exists());
            assert!(!variable("LIBVA_DRIVER_NAME").is_empty());
        }
    }

    /// The GStreamer scenario of `oracle-harness`, on a stand-in for the system's directory.
    #[test]
    fn wine_gets_the_plugins_of_gstreamer_without_the_hardware_ones() {
        let dir = tempfile::tempdir().unwrap();
        let system = dir.path().join("system");
        fs::create_dir(&system).unwrap();
        let plugins = [
            "libgstogg.so",
            "libgstva.so",
            "libgstvaapi.so",
            "libgstvulkan.so",
            "libgstmsdk.so",
        ];
        for name in plugins {
            fs::write(system.join(name), name).unwrap();
        }
        let out = dir.path().join("out");
        let missing = dir.path().join("missing");

        let gstreamer = GStreamer::prepare(&out, &[&missing, &system]).unwrap();
        let run = Run {
            runtime: Runtime::Rgss3,
            dll: None,
            rtps: &[],
            fonts: None,
            prefix: Path::new("prefix"),
            gstreamer: &gstreamer,
            locale: "en_US.UTF-8",
            timeout: Duration::from_secs(1),
        };
        let command = run.wine("wine", ":104");

        let variable = |name: &str| {
            let (_, value) = command.get_envs().find(|(key, _)| *key == name).unwrap();
            value.map(Path::new)
        };
        let made = variable("GST_PLUGIN_SYSTEM_PATH_1_0").unwrap();
        assert_eq!(made, out.join("gstreamer/plugins-0"));
        let linked: Vec<_> = fs::read_dir(made)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(linked, ["libgstogg.so"]);
        assert_eq!(
            fs::read(made.join("libgstogg.so")).unwrap(),
            b"libgstogg.so"
        );
        let registry = variable("GST_REGISTRY_1_0").unwrap();
        assert!(registry.starts_with(&out), "{}", registry.display());
        for removed in [
            "GST_PLUGIN_PATH",
            "GST_PLUGIN_PATH_1_0",
            "GST_PLUGIN_SYSTEM_PATH",
        ] {
            assert_eq!(variable(removed), None, "{removed}");
        }

        // Made afresh: what an earlier run left there is gone, and without a directory of
        // the system the variable names an empty one.
        fs::write(made.join("libgstva.so"), b"left").unwrap();
        GStreamer::prepare(&out, &[&system]).unwrap();
        assert!(!out.join("gstreamer/plugins-0/libgstva.so").exists());
        let none = GStreamer::prepare(&out, &[&missing]).unwrap();
        let empty = Path::new(&none.system_path);
        assert!(empty.starts_with(out.join("gstreamer")));
        assert_eq!(fs::read_dir(empty).unwrap().count(), 0);
        assert!(!out.join("gstreamer/plugins-0").exists());
    }

    /// The bwrap-is-missing scenario of `oracle-harness`.
    #[test]
    fn a_path_without_bwrap_is_an_error_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        for program in ["wine", "wineboot", "wineserver", "Xvfb"] {
            fs::write(dir.path().join(program), b"").unwrap();
        }
        let error = check_programs(Some(dir.path().as_os_str())).unwrap_err();
        assert!(error.to_string().contains("`bwrap`"), "{error}");

        fs::write(dir.path().join("bwrap"), b"").unwrap();
        check_programs(Some(dir.path().as_os_str())).unwrap();
    }

    #[test]
    fn a_request_marks_its_frame_as_late_with_a_word() {
        assert!(request_says_late(b"10 late"));
        assert!(!request_says_late(b"10"));
        assert!(!request_says_late(b""));
    }

    /// The size-from-the-request scenario of `oracle-harness`, and the requests of the probes
    /// of RGSS1 and RGSS2, which name no size.
    #[test]
    fn the_size_of_a_capture_is_that_of_the_request_or_that_of_the_runtime() {
        assert_eq!(capture_size(Runtime::Rgss3, b"5 640x480"), (640, 480));
        assert_eq!(capture_size(Runtime::Rgss3, b"12 late 544x416"), (544, 416));
        assert_eq!(capture_size(Runtime::Rgss2, b"5"), (544, 416));
        assert_eq!(capture_size(Runtime::Rgss2, b"12 late"), (544, 416));
        assert_eq!(capture_size(Runtime::Rgss1, b"5"), (640, 480));
        assert_eq!(capture_size(Runtime::Rgss3, b"5"), (544, 416));
        assert_eq!(capture_size(Runtime::Rgss3, b"5 x480 +1x2"), (544, 416));
    }

    #[test]
    fn the_game_window_is_the_topmost_viewable_one_of_the_size() {
        let top = |window, size, viewable| TopLevel {
            window,
            size,
            viewable,
        };
        let windows = [
            top(1, (640, 480), true),
            top(2, (544, 416), true),
            top(3, (640, 480), true),
            top(4, (640, 480), false),
            top(5, (1024, 768), true),
        ];
        assert_eq!(choose_window(&windows, (640, 480)), Some(3));
        assert_eq!(choose_window(&windows, (544, 416)), Some(2));
        assert_eq!(choose_window(&windows, (800, 600)), None);
        assert_eq!(choose_window(&windows[3..4], (640, 480)), None);
    }

    #[test]
    fn the_display_is_the_one_the_server_announces() {
        // The server found the first numbers taken, 99 among them, and chose another.
        let display = Xvfb::start_server(
            stand_in("echo 104 >&$fd; exec sleep 60"),
            Duration::from_secs(30),
        )
        .unwrap();
        assert_eq!(display.name, ":104");
    }

    #[test]
    fn a_server_that_exits_without_a_display_is_an_error() {
        let started = Instant::now();
        let error = Xvfb::start_server(stand_in("exit 1"), Duration::from_secs(30))
            .err()
            .unwrap();
        assert!(
            error
                .to_string()
                .contains("exited without announcing a display"),
            "{error}"
        );
        assert!(started.elapsed() < Duration::from_secs(30));
    }

    #[test]
    fn a_server_that_announces_nothing_is_an_error_after_the_timeout() {
        let error = Xvfb::start_server(stand_in("exec sleep 60"), Duration::from_millis(200))
            .err()
            .unwrap();
        assert!(
            error.to_string().contains("did not announce a display"),
            "{error}"
        );
    }

    #[test]
    #[ignore = "needs Xvfb"]
    fn two_displays_started_at_once_are_distinct_and_accept_connections() {
        let starts: Vec<_> = (0..2).map(|_| thread::spawn(Xvfb::start)).collect();
        let displays: Vec<Xvfb> = starts
            .into_iter()
            .map(|start| start.join().unwrap().unwrap())
            .collect();
        assert_ne!(displays[0].name, displays[1].name);
        for display in &displays {
            x11rb::connect(Some(&display.name)).unwrap();
        }
    }
}
