//! The test harness of the binding: an interpreter with a guest installed, and `eval`.
//!
//! Only one interpreter can exist per process, so the harness starts one the first time it
//! is used and keeps it; the tests of one test binary run on it one after another. State
//! that Ruby code changes survives from test to test: tests use fresh objects and put back
//! the font defaults they change.
//!
//! The guest is installed over a generated game directory, below the directory of the test
//! executable, with these files:
//!
//! | File | Contents |
//! |---|---|
//! | `game/Graphics/Pictures/Dot.png` | 2 by 2 pixels, [`DOT_PIXELS`] |
//! | `game/Graphics/Pictures/Broken.png` | text, not an image |
//! | `rtp/Graphics/Pictures/RtpOnly.png` | 3 by 1 pixels, [`RTP_ONLY_PIXELS`] |
//!
//! `rtp` is the one run-time package directory. Text comes from [`CellBackend`], which knows
//! the families of [`FONT_FAMILIES`]; the default font name is the first of them.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use vkxp_core::test_support::CellRasterizer;
use vkxp_core::{Font, Mask, TextRasterizer};
use vkxp_ruby_sys::{Interpreter, Options, Ruby, RubyError};
use vkxp_vfs::GameFiles;

use crate::{Config, Guest, TextBackend};

pub mod host;

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
    fn measure(&self, font: &Font, text: &str) -> (u32, u32) {
        CellRasterizer.measure(font, text)
    }

    fn rasterize(&self, font: &Font, text: &str) -> Mask {
        CellRasterizer.rasterize(font, text)
    }
}

impl TextBackend for CellBackend {
    fn font_exists(&self, name: &str) -> bool {
        self.families.iter().any(|family| family == name)
    }
}

struct Harness {
    interpreter: Interpreter,
    game_dir: PathBuf,
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

/// Creates the game and run-time package directories afresh and returns their parent. It is
/// named after the test executable, so test binaries that run at the same time do not share
/// it, and lies next to the executable, where `cargo clean` removes it.
fn generate_files() -> PathBuf {
    let executable = std::env::current_exe().expect("the path of the test executable");
    let name = executable
        .file_name()
        .expect("an executable with a file name");
    let mut dir_name = name.to_os_string();
    dir_name.push(".vkxp-binding-testing");
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

fn harness() -> &'static Harness {
    static SHARED: OnceLock<Harness> = OnceLock::new();
    SHARED.get_or_init(|| {
        let root = generate_files();
        let game_dir = root.join("game");
        let config = Config {
            files: GameFiles::new(game_dir.clone(), [root.join("rtp")]),
            font_names: vec![FONT_FAMILIES[0].to_owned()],
            text: Box::new(CellBackend::new(FONT_FAMILIES.iter().copied())),
            run: crate::RunConfig::default(),
        };
        let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
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
        }
    })
}

/// The game directory the guest reads from.
pub fn game_dir() -> &'static Path {
    &harness().game_dir
}

/// Runs a closure on the interpreter thread and returns its result.
pub fn run<F, R>(work: F) -> R
where
    F: FnOnce(&Ruby) -> R + Send + 'static,
    R: Send + 'static,
{
    harness().interpreter.run(work)
}

/// Evaluates Ruby source and returns `inspect` of its value, or the exception that left it
/// as its class name, message and backtrace. The source is reported as `test`, line 1.
pub fn eval(source: &str) -> Result<String, RubyError> {
    let source = source.to_owned();
    run(move |ruby| {
        let value = ruby.eval_protected(&source, "test", 1)?;
        // SAFETY: `value` was just returned by the interpreter and is held in a local; so
        // is the exception of a failed `inspect`.
        unsafe { ruby.inspect(value).map_err(|raised| ruby.describe(raised)) }
    })
}

/// The number of live bitmaps in the guest's arena.
pub fn bitmap_count() -> usize {
    run(|_ruby| Guest::with(|guest| guest.bitmaps().len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_returns_inspect_of_the_value() {
        assert_eq!(eval("1 + 1").as_deref(), Ok("2"));
        assert_eq!(eval("'a' + 'b'").as_deref(), Ok("\"ab\""));
    }

    #[test]
    fn a_raising_snippet_returns_class_and_message() {
        let error = eval("raise ArgumentError, 'boom'").expect_err("the snippet raises");
        assert_eq!(error.class_name, "ArgumentError");
        assert_eq!(error.message, "boom");
        // The interpreter goes on.
        assert_eq!(eval("1 + 1").as_deref(), Ok("2"));
    }

    #[test]
    fn the_game_directory_holds_the_generated_files() {
        let picture = game_dir().join("Graphics/Pictures/Dot.png");
        let decoded = vkxp_image::decode(&fs::read(picture).unwrap()).unwrap();
        assert_eq!((decoded.width, decoded.height), (2, 2));
        assert_eq!(decoded.rgba, DOT_PIXELS.as_flattened());
        assert!(
            game_dir()
                .with_file_name("rtp")
                .join("Graphics/Pictures/RtpOnly.png")
                .is_file()
        );
    }
}
