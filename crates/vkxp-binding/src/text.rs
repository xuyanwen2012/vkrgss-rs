//! The text backend: where glyphs and the knowledge of installed fonts come from.

use vkxp_core::TextRasterizer;

/// What the guest needs from a font engine: glyph coverage, through core's
/// [`TextRasterizer`], and whether a font family exists.
///
/// The guest holds one, given in its [`Config`](crate::Config). `Bitmap#draw_text` and
/// `Bitmap#text_size` pass it to the bitmap arena, and `Font.exist?` asks it.
pub trait TextBackend: TextRasterizer {
    /// Whether a font family of this name can be drawn with.
    fn font_exists(&self, name: &str) -> bool;
}
