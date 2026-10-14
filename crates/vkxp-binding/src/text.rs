//! The text backend: where glyphs and the knowledge of installed fonts come from.

use vkxp_core::TextRasterizer;

/// What the guest needs from a font engine: glyph coverage, through core's
/// [`TextRasterizer`], and whether a font family exists.
///
/// The guest holds one, given in its [`Config`](crate::Config). `Bitmap#draw_text` and
/// `Bitmap#text_size` pass it to the bitmap arena, and `Font.exist?` asks it; in RGSS3 so
/// does the choice of the name a font is drawn with.
pub trait TextBackend: TextRasterizer {
    /// Whether a font family of this name can be drawn with: `Font.exist?` of RGSS1 and
    /// RGSS2.
    fn font_exists(&self, name: &str) -> bool;

    /// Whether a font family of exactly this name exists, letter case included: `Font.exist?`
    /// of RGSS3.
    fn font_exists_exactly(&self, name: &str) -> bool;
}
