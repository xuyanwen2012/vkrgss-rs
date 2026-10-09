//! The text backend of a running game: the binding's interface over the font engine.
//!
//! It lives in the binary so that the binding does not depend on the font engine.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use vkxp_binding::TextBackend;
use vkxp_core::{Font, Mask, TextRasterizer};
use vkxp_text::{FontSources, TextEngine, TextRequest};

/// A [`TextBackend`] that draws with a [`TextEngine`].
///
/// The binding's interface takes `&self` and the engine fills caches through `&mut self`;
/// everything runs on the interpreter thread, so a `RefCell` joins the two.
pub struct FontBackend {
    engine: RefCell<TextEngine>,
}

impl FontBackend {
    /// A backend over the fonts of `sources`.
    pub fn new(sources: &FontSources) -> FontBackend {
        FontBackend {
            engine: RefCell::new(TextEngine::new(sources)),
        }
    }

    /// The backend of a game: the fonts of `font_dirs` first, then those of the game's
    /// `Fonts` directory if it has one, then the fonts installed on the system. With
    /// `own_metrics` a font that replaces another is sized by its own line box.
    pub fn for_game(game_dir: &Path, font_dirs: &[PathBuf], own_metrics: bool) -> FontBackend {
        let mut directories = font_dirs.to_vec();
        let game_fonts = game_dir.join("Fonts");
        if game_fonts.is_dir() {
            directories.push(game_fonts);
        }
        FontBackend::new(&FontSources {
            directories,
            own_metrics,
            ..FontSources::system()
        })
    }
}

/// What the engine is asked for a core font: its names in order, its size (a size below 1 is
/// drawn as 1), and its weight and slant.
fn request(font: &Font) -> TextRequest {
    TextRequest {
        families: font.names.clone(),
        size: font.size.max(1).unsigned_abs(),
        bold: font.bold,
        italic: font.italic,
    }
}

impl TextRasterizer for FontBackend {
    fn measure(&self, font: &Font, text: &str) -> (u32, u32) {
        self.engine.borrow_mut().measure(&request(font), text)
    }

    fn rasterize(&self, font: &Font, text: &str) -> Mask {
        let mask = self.engine.borrow_mut().rasterize(&request(font), text);
        Mask::new(mask.width, mask.height, mask.coverage)
    }
}

impl TextBackend for FontBackend {
    fn font_exists(&self, name: &str) -> bool {
        self.engine.borrow().is_available(name)
    }
}

#[cfg(test)]
mod tests {
    use vkxp_core::FontDefaults;

    use super::*;

    /// A backend whose only source is the test fonts of `vkxp-text`.
    fn backend() -> FontBackend {
        let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../vkxp-text/tests/fonts");
        FontBackend::new(&FontSources::directories_only([fonts]))
    }

    fn font(name: &str, size: i32) -> Font {
        let defaults = FontDefaults::new(vec![name.to_owned()]);
        Font::with(&defaults, None, Some(size))
    }

    #[test]
    fn a_measured_string_has_the_size_of_its_mask() {
        let backend = backend();
        let font = font("Liberation Sans", 22);
        let (width, height) = backend.measure(&font, "Hello");
        let mask = backend.rasterize(&font, "Hello");
        assert!(width > 0);
        assert_eq!(height, 22);
        assert_eq!((mask.width(), mask.height()), (width, height));
    }

    #[test]
    fn a_rasterised_latin_string_has_ink() {
        let backend = backend();
        let mask = backend.rasterize(&font("Liberation Sans", 24), "Hello");
        let ink = (0..mask.height())
            .flat_map(|y| (0..mask.width()).map(move |x| (x, y)))
            .any(|(x, y)| mask.get(x, y) > 0);
        assert!(ink);
    }

    #[test]
    fn a_size_below_one_is_drawn_as_one() {
        let backend = backend();
        for size in [0, -5] {
            let font = font("Liberation Sans", size);
            let (_, height) = backend.measure(&font, "Hello");
            assert_eq!(height, 1);
            assert_eq!(backend.rasterize(&font, "Hello").height(), 1);
        }
    }

    #[test]
    fn font_exists_answers_from_the_engine() {
        let backend = backend();
        assert!(backend.font_exists("Liberation Sans"));
        assert!(!backend.font_exists("NoSuchFamilyAnywhere"));
    }
}
