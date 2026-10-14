//! Bold and italic face selection, on families assembled from variants of the committed
//! Liberation Sans that differ only in their style flags and advance widths.

mod common;

use std::path::{Path, PathBuf};

use common::{Derivation, LIBERATION, LIBERATION_FILE, copy_font, derive_font, request};
use vkrgss_text::{FontSources, TextEngine, TextRequest};

/// Font units by which each styled variant is wider per glyph than the regular face.
const WIDER_BY: u16 = 150;

/// A style, of a face or of a request, as `(bold, italic)`.
type Style = (bool, bool);

const REGULAR: Style = (false, false);
const BOLD: Style = (true, false);
const ITALIC: Style = (false, true);
const BOLD_ITALIC: Style = (true, true);

/// Writes the faces `styles` of Liberation Sans into `directory` and returns their paths.
fn family(directory: &Path, styles: &[Style]) -> Vec<PathBuf> {
    styles
        .iter()
        .map(|&(bold, italic)| {
            if (bold, italic) == REGULAR {
                return copy_font(directory, LIBERATION_FILE);
            }
            let change = Derivation {
                weight: Some(if bold { 700 } else { 400 }),
                italic,
                wider_by: WIDER_BY,
                ..Derivation::default()
            };
            let name = format!("variant-{}-{}.ttf", u8::from(bold), u8::from(italic));
            derive_font(LIBERATION_FILE, &directory.join(name), change)
        })
        .collect()
}

fn styled((bold, italic): Style) -> TextRequest {
    TextRequest {
        bold,
        italic,
        ..request(LIBERATION, 22)
    }
}

/// Builds an engine on a family with the faces `present` and checks, for each pair of a
/// requested style and the style of the face it must select, the file, the reported style
/// and the measured width.
fn check(present: &[Style], expected: &[(Style, Style)]) {
    let directory = tempfile::tempdir().unwrap();
    let paths = family(directory.path(), present);
    let mut engine = TextEngine::new(&FontSources::directories_only([directory.path()]));
    assert_eq!(engine.families(), [LIBERATION]);

    let text = "Styled text 0123";
    let regular_width = {
        let alone = tempfile::tempdir().unwrap();
        family(alone.path(), &[REGULAR]);
        let mut engine = TextEngine::new(&FontSources::directories_only([alone.path()]));
        engine.measure(&styled(REGULAR), text).0
    };

    for &(requested, selected) in expected {
        let position = present.iter().position(|&style| style == selected).unwrap();
        let face = engine.resolve(&styled(requested)).unwrap();
        let context = format!("{requested:?} requested of {present:?}");
        assert_eq!(
            face.path.canonicalize().unwrap(),
            paths[position],
            "{context}"
        );
        assert_eq!((face.weight >= 600, face.italic), selected, "{context}");

        let (width, height) = engine.measure(&styled(requested), text);
        assert_eq!(height, 22);
        let layout = engine.layout(&styled(requested), text);
        let advances: u32 = layout.advances.iter().sum();
        if selected == REGULAR {
            // A bold request without a bold face is synthesised: one pixel per character.
            let extra = u32::from(requested.0) * text.chars().count() as u32;
            assert_eq!(advances, regular_width + extra, "{context}");
        } else {
            assert_eq!(layout.emboldened, u32::from(requested.0 && !selected.0));
            assert!(width > regular_width, "{context}");
        }
        let mask = engine.rasterize(&styled(requested), text);
        let drawn = width + layout.right_overhang;
        assert_eq!((mask.width, mask.height), (drawn, height), "{context}");
        assert!(mask.coverage.iter().any(|&value| value != 0), "{context}");
    }
}

// Requirement: Size, bold and italic

#[test]
fn bold_face_present() {
    check(&[REGULAR, BOLD], &[(REGULAR, REGULAR), (BOLD, BOLD)]);
}

#[test]
fn every_styled_face_present() {
    check(
        &[BOLD_ITALIC, ITALIC, BOLD, REGULAR],
        &[
            (REGULAR, REGULAR),
            (BOLD, BOLD),
            (ITALIC, ITALIC),
            (BOLD_ITALIC, BOLD_ITALIC),
        ],
    );
}

#[test]
fn regular_and_bold_italic_only() {
    // Neither a bold nor an italic request may take the bold italic face.
    check(
        &[BOLD_ITALIC, REGULAR],
        &[
            (REGULAR, REGULAR),
            (BOLD, REGULAR),
            (ITALIC, REGULAR),
            (BOLD_ITALIC, BOLD_ITALIC),
        ],
    );
}

#[test]
fn regular_and_italic_only() {
    check(
        &[ITALIC, REGULAR],
        &[
            (REGULAR, REGULAR),
            (BOLD, REGULAR),
            (ITALIC, ITALIC),
            (BOLD_ITALIC, REGULAR),
        ],
    );
}

#[test]
fn regular_and_bold_only() {
    check(
        &[BOLD, REGULAR],
        &[
            (REGULAR, REGULAR),
            (BOLD, BOLD),
            (ITALIC, REGULAR),
            (BOLD_ITALIC, REGULAR),
        ],
    );
}

#[test]
fn family_without_a_regular_face_still_draws() {
    check(
        &[BOLD_ITALIC],
        &[
            (REGULAR, BOLD_ITALIC),
            (BOLD, BOLD_ITALIC),
            (ITALIC, BOLD_ITALIC),
        ],
    );
}
