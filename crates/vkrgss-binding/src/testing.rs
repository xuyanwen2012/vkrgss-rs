//! The test harness of the binding: an interpreter with a guest installed, and `eval`.
//!
//! Only one interpreter can exist per process, so [`start_as`] starts one the first time it
//! is called and keeps it; the tests of one test binary run on it one after another. State
//! that Ruby code changes survives from test to test: tests use fresh objects and put back
//! the font defaults they change. The guest is that of a game of the runtime that
//! [`start_as`] is given, and the interpreter is reached only through the [`Harness`] it
//! returns, so every test declares its runtime; the harness chooses none of its own.
//!
//! The guest is installed over a generated game directory, below the directory of the test
//! executable and of its own for each process, with these files:
//!
//! | File | Contents |
//! |---|---|
//! | `game/Graphics/Pictures/Dot.png` | 2 by 2 pixels, [`DOT_PIXELS`] |
//! | `game/Graphics/Pictures/Broken.png` | text, not an image |
//! | `rtp/Graphics/Pictures/RtpOnly.png` | 3 by 1 pixels, [`RTP_ONLY_PIXELS`] |
//!
//! `rtp` is the one run-time package directory. Text comes from [`CellBackend`], which knows
//! the families of [`FONT_FAMILIES`]; the default font name is the first of them.
//!
//! The same tests run over Ruby 1.8.7 and over Ruby 1.9.2. What cannot pass over the latter
//! yet is named in the allow-list [`ALLOW_LIST`]: a test through the marker
//! [`pending_over_ruby_1_9_2!`](crate::pending_over_ruby_1_9_2) and [`pending`], a snippet
//! case through [`pending_case`]. The private module `pending` describes the list.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use vkrgss_core::test_support::CellRasterizer;
use vkrgss_core::{Font, Mask, TextMetrics, TextRasterizer};
use vkrgss_ruby_sys::{Interpreter, Options, Ruby, RubyError};
use vkrgss_vfs::{GameFiles, Runtime};

use crate::{Config, Guest, TextBackend};

pub mod host;
mod pending;

pub use pending::{ALLOW_LIST, pending, pending_case};

/// The font families the harness's text backend has.
pub const FONT_FAMILIES: &[&str] = &["Test Sans", "Liberation Sans"];

/// The pixels of `Graphics/Pictures/Dot.png` in the game directory, row by row, as red,
/// green, blue and alpha.
pub const DOT_PIXELS: [[u8; 4]; 4] = [
    [255, 0, 0, 255],
    [0, 255, 0, 128],
    [0, 0, 255, 255],
    [10, 20, 30, 0],
];

/// The pixels of `Graphics/Pictures/RtpOnly.png` in the run-time package directory.
pub const RTP_ONLY_PIXELS: [[u8; 4]; 3] = [[1, 2, 3, 255], [4, 5, 6, 255], [7, 8, 9, 255]];

/// A text backend over core's deterministic [`CellRasterizer`] that has a fixed list of
/// font families.
#[derive(Debug, Clone)]
pub struct CellBackend {
    families: Vec<String>,
}

impl CellBackend {
    /// A backend in which exactly `families` exist.
    pub fn new(families: impl IntoIterator<Item = impl Into<String>>) -> CellBackend {
        CellBackend {
            families: families.into_iter().map(Into::into).collect(),
        }
    }
}

impl TextRasterizer for CellBackend {
    fn metrics(&self, font: &Font, text: &str) -> TextMetrics {
        CellRasterizer.metrics(font, text)
    }

    fn rasterize(
        &self,
        font: &Font,
        text: &str,
        advances: &[u32],
        condense: Option<(u32, u32)>,
    ) -> Mask {
        CellRasterizer.rasterize(font, text, advances, condense)
    }
}

impl TextBackend for CellBackend {
    fn font_exists(&self, name: &str) -> bool {
        self.families.iter().any(|family| family == name)
    }

    fn font_exists_exactly(&self, name: &str) -> bool {
        self.families.iter().any(|family| family == name)
    }
}

/// The started harness: the one interpreter of the process, with the guest of a game of one
/// runtime installed. [`start_as`] returns it.
pub struct Harness {
    interpreter: Interpreter,
    game_dir: PathBuf,
    runtime: Runtime,
}

/// A PNG file of these pixels.
fn png(width: u32, pixels: &[[u8; 4]]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, pixels.len() as u32 / width);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("a PNG header");
    writer
        .write_image_data(pixels.as_flattened())
        .expect("PNG data");
    writer.finish().expect("a complete PNG");
    out
}

fn write(path: &Path, contents: &[u8]) {
    let dir = path.parent().expect("a file inside a directory");
    fs::create_dir_all(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
    fs::write(path, contents).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
}

/// The name of the directory of generated files of the process `process` of the test
/// executable `executable`, which runs the test `alone` of the allow-list alone if it is one.
fn generated_dir_name(executable: &OsStr, process: u32, alone: Option<&str>) -> OsString {
    let mut dir_name = executable.to_os_string();
    dir_name.push(format!(".vkrgss-binding-testing.{process}"));
    if let Some(test) = alone {
        dir_name.push(".");
        dir_name.push(test);
    }
    dir_name
}

/// Creates the game and run-time package directories afresh and returns their parent. It is
/// named after the test executable and the process, so test binaries that run at the same
/// time, and processes of one test binary, do not share it, and lies next to the executable,
/// where `cargo clean` removes it. The process that runs a test of the allow-list alone adds
/// the name of that test.
fn generate_files() -> PathBuf {
    let executable = std::env::current_exe().expect("the path of the test executable");
    let name = executable
        .file_name()
        .expect("an executable with a file name");
    let dir_name = generated_dir_name(name, std::process::id(), pending::alone().as_deref());
    let root = executable.with_file_name(dir_name);
    if root.exists() {
        fs::remove_dir_all(&root).unwrap_or_else(|error| panic!("{}: {error}", root.display()));
    }
    let pictures = Path::new("Graphics").join("Pictures");
    let game = root.join("game").join(&pictures);
    write(&game.join("Dot.png"), &png(2, &DOT_PIXELS));
    write(&game.join("Broken.png"), b"this is not an image\n");
    let rtp = root.join("rtp").join(&pictures);
    write(&rtp.join("RtpOnly.png"), &png(3, &RTP_ONLY_PIXELS));
    root
}

/// Starts the harness with the guest of a game of `runtime`: its symbol table, its frame and
/// its functions, and returns it. Every test calls this before it uses the interpreter, with
/// the runtime of its test binary, because the one interpreter of the process is started
/// once.
///
/// # Panics
///
/// Panics when the harness was started with another runtime, and names both.
pub fn start_as(runtime: Runtime) -> &'static Harness {
    static SHARED: OnceLock<Harness> = OnceLock::new();
    let harness = SHARED.get_or_init(|| Harness::start(runtime));
    assert!(
        harness.runtime == runtime,
        "the harness was started as {} and cannot start as {} as well",
        harness.runtime.name(),
        runtime.name()
    );
    harness
}

impl Harness {
    fn start(runtime: Runtime) -> Harness {
        let root = generate_files();
        let game_dir = root.join("game");
        let config = Config {
            files: GameFiles::new(game_dir.clone(), [root.join("rtp")]),
            font_names: vec![FONT_FAMILIES[0].to_owned()],
            text: Box::new(CellBackend::new(FONT_FAMILIES.iter().copied())),
            run: crate::RunConfig {
                runtime,
                ..crate::RunConfig::default()
            },
        };
        let options = Options::for_rgss(match runtime {
            Runtime::Rgss1 => 1,
            Runtime::Rgss2 => 2,
            Runtime::Rgss3 => 3,
        });
        let interpreter = Interpreter::start(options).expect("the interpreter starts");
        interpreter
            .run(move |ruby| {
                Guest::install(ruby, config).map_err(|raised| {
                    // SAFETY: `raised` was produced just now on this thread and sat in a
                    // local since.
                    unsafe { ruby.describe(raised) }
                })
            })
            .expect("the binding installs");
        Harness {
            interpreter,
            game_dir,
            runtime,
        }
    }

    /// The runtime the harness was started with.
    pub fn runtime(&self) -> Runtime {
        self.runtime
    }

    /// The game directory the guest reads from.
    pub fn game_dir(&self) -> &Path {
        &self.game_dir
    }

    /// Runs a closure on the interpreter thread and returns its result.
    pub fn run<F, R>(&self, work: F) -> R
    where
        F: FnOnce(&Ruby) -> R + Send + 'static,
        R: Send + 'static,
    {
        self.interpreter.run(work)
    }

    /// Evaluates Ruby source and returns `inspect` of its value, or the exception that left
    /// it as its class name, message and backtrace. The source is reported as `test`, line 1.
    pub fn eval(&self, source: &str) -> Result<String, RubyError> {
        let source = source.to_owned();
        self.run(move |ruby| {
            let value = ruby.eval_protected(&source, "test", 1)?;
            // SAFETY: `value` was just returned by the interpreter and is held in a local; so
            // is the exception of a failed `inspect`.
            unsafe { ruby.inspect(value).map_err(|raised| ruby.describe(raised)) }
        })
    }

    /// The number of live bitmaps in the guest's arena.
    pub fn bitmap_count(&self) -> usize {
        self.run(|_ruby| Guest::with(|guest| guest.bitmaps().len()))
    }
}

impl std::fmt::Debug for Harness {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Harness")
            .field("game_dir", &self.game_dir)
            .field("runtime", &self.runtime)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn harness() -> &'static Harness {
        start_as(Runtime::Rgss1)
    }

    #[test]
    fn evaluation_returns_inspect_of_the_value() {
        assert_eq!(harness().eval("1 + 1").as_deref(), Ok("2"));
        assert_eq!(harness().eval("'a' + 'b'").as_deref(), Ok("\"ab\""));
    }

    #[test]
    fn a_raising_snippet_returns_class_and_message() {
        let error = harness()
            .eval("raise ArgumentError, 'boom'")
            .expect_err("the snippet raises");
        assert_eq!(error.class_name, "ArgumentError");
        assert_eq!(error.message, "boom");
        // The interpreter goes on.
        assert_eq!(harness().eval("1 + 1").as_deref(), Ok("2"));
    }

    #[test]
    fn the_game_directory_holds_the_generated_files() {
        let game_dir = harness().game_dir();
        let picture = game_dir.join("Graphics/Pictures/Dot.png");
        let decoded = vkrgss_image::decode(&fs::read(picture).unwrap()).unwrap();
        assert_eq!((decoded.width, decoded.height), (2, 2));
        assert_eq!(decoded.rgba, DOT_PIXELS.as_flattened());
        assert!(
            game_dir
                .with_file_name("rtp")
                .join("Graphics/Pictures/RtpOnly.png")
                .is_file()
        );
    }

    #[test]
    fn a_second_runtime_panics_and_names_both() {
        harness();
        let panic = std::panic::catch_unwind(|| start_as(Runtime::Rgss2))
            .expect_err("the second runtime is refused");
        let message = panic.downcast_ref::<String>().expect("a formatted message");
        assert!(
            message.contains("rgss1") && message.contains("rgss2"),
            "{message}"
        );
    }

    #[test]
    fn processes_of_one_executable_have_directories_of_their_own() {
        let executable = OsStr::new("ruby-1a2b");
        let first = generated_dir_name(executable, 100, None);
        let second = generated_dir_name(executable, 101, None);
        assert_ne!(first, second);
        assert_eq!(first, "ruby-1a2b.vkrgss-binding-testing.100");
        assert_eq!(
            generated_dir_name(executable, 100, Some("t.a_test")),
            "ruby-1a2b.vkrgss-binding-testing.100.t.a_test"
        );
    }
}
