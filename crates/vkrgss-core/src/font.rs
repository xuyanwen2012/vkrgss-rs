//! `Font`: the attributes text is drawn with, and the defaults new fonts copy.

use crate::Color;

/// The attributes of a font: which families to try, in order, and how to draw.
///
/// A font is a plain value. It is created from [`FontDefaults`] and keeps its own copy of
/// every attribute, so later changes to the defaults do not reach it.
#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    /// Family names, tried in order.
    pub names: Vec<String>,
    pub size: i32,
    pub bold: bool,
    pub italic: bool,
    pub color: Color,
    /// Whether text is drawn with a shadow. RGSS2 and RGSS3 have it; a font of RGSS1 has
    /// none.
    pub shadow: bool,
    /// Whether text is drawn with an outline. Only RGSS3 has it.
    pub outline: bool,
    /// The colour of the outline; read only when the font has one.
    pub out_color: Color,
}

impl Font {
    /// A font with every attribute taken from `defaults`.
    pub fn new(defaults: &FontDefaults) -> Font {
        Font::with(defaults, None, None)
    }

    /// A font with the given family names and size; whatever is `None`, and every other
    /// attribute, is taken from `defaults`.
    pub fn with(defaults: &FontDefaults, names: Option<Vec<String>>, size: Option<i32>) -> Font {
        Font {
            names: names.unwrap_or_else(|| defaults.names.clone()),
            size: size.unwrap_or(defaults.size),
            bold: defaults.bold,
            italic: defaults.italic,
            color: defaults.color,
            shadow: defaults.shadow,
            outline: defaults.outline,
            out_color: defaults.out_color,
        }
    }
}

/// The attributes a new font starts with.
///
/// This is an ordinary value owned by whoever owns the runtime state, not a global.
#[derive(Debug, Clone, PartialEq)]
pub struct FontDefaults {
    /// Family names, tried in order.
    pub names: Vec<String>,
    pub size: i32,
    pub bold: bool,
    pub italic: bool,
    pub color: Color,
    /// Whether a new font draws a shadow. Off in RGSS1, which has no shadow.
    pub shadow: bool,
    /// Whether a new font draws an outline. Off in RGSS1 and RGSS2, which have none.
    pub outline: bool,
    /// The out colour of a new font: black at alpha 128 until it is changed.
    pub out_color: Color,
}

impl FontDefaults {
    /// Size of a font until the default is changed.
    pub const INITIAL_SIZE: i32 = 22;

    /// Size of a font of RGSS2 until the default is changed.
    pub const INITIAL_SIZE_RGSS2: i32 = 20;

    /// The family names `Font.default_name` of RGSS2 starts with, in its English version.
    pub const INITIAL_NAMES_RGSS2: [&str; 3] = ["Verdana", "Arial", "Courier New"];

    /// Size of a font of RGSS3 until the default is changed.
    pub const INITIAL_SIZE_RGSS3: i32 = 24;

    /// The family name `Font.default_name` of RGSS3 starts with.
    pub const INITIAL_NAME_RGSS3: &str = "VL Gothic";

    /// The initial defaults of RGSS1: size 22, not bold, not italic, opaque white, no
    /// shadow, no outline. The default family names depend on the game's language variant, so
    /// the caller gives them.
    pub fn new(names: Vec<String>) -> FontDefaults {
        FontDefaults {
            names,
            size: FontDefaults::INITIAL_SIZE,
            bold: false,
            italic: false,
            color: Color::rgb(255.0, 255.0, 255.0),
            shadow: false,
            outline: false,
            out_color: Color::new(0.0, 0.0, 0.0, 128.0),
        }
    }

    /// The initial defaults of RGSS2, as measured on the original runtime of VX: the names
    /// "Verdana", "Arial" and "Courier New", size 20, not bold, not italic, opaque white,
    /// with a shadow and no outline.
    pub fn rgss2() -> FontDefaults {
        FontDefaults {
            names: FontDefaults::INITIAL_NAMES_RGSS2
                .iter()
                .map(|&name| name.to_owned())
                .collect(),
            size: FontDefaults::INITIAL_SIZE_RGSS2,
            shadow: true,
            ..FontDefaults::new(Vec::new())
        }
    }

    /// The initial defaults of RGSS3, as measured on the original runtime of VX Ace: the one
    /// name "VL Gothic", size 24, not bold, not italic, opaque white, no shadow, with an
    /// outline in the out colour (0, 0, 0, 128).
    ///
    /// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1 and RGSS300 3.0.0.1, api
    /// probe.
    pub fn rgss3() -> FontDefaults {
        FontDefaults {
            names: vec![FontDefaults::INITIAL_NAME_RGSS3.to_owned()],
            size: FontDefaults::INITIAL_SIZE_RGSS3,
            outline: true,
            ..FontDefaults::new(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> FontDefaults {
        FontDefaults::new(vec!["Arial".to_owned(), "Fallback".to_owned()])
    }

    #[test]
    fn initial_defaults() {
        let font = Font::new(&defaults());
        assert_eq!(font.size, 22);
        assert!(!font.bold);
        assert!(!font.italic);
        assert_eq!(font.color, Color::new(255.0, 255.0, 255.0, 255.0));
        assert_eq!(font.names, ["Arial", "Fallback"]);
    }

    #[test]
    fn initial_defaults_of_rgss2() {
        let font = Font::new(&FontDefaults::rgss2());
        assert_eq!(font.names, ["Verdana", "Arial", "Courier New"]);
        assert_eq!(font.size, 20);
        assert!(!font.bold);
        assert!(!font.italic);
        assert!(font.shadow);
        assert_eq!(font.color, Color::new(255.0, 255.0, 255.0, 255.0));
        // A font of RGSS1 has no shadow.
        assert!(!Font::new(&defaults()).shadow);
        let mut without = FontDefaults::rgss2();
        without.shadow = false;
        assert!(!Font::with(&without, None, Some(12)).shadow);
    }

    #[test]
    fn initial_defaults_of_rgss3() {
        let font = Font::new(&FontDefaults::rgss3());
        assert_eq!(font.names, ["VL Gothic"]);
        assert_eq!(font.size, 24);
        assert!(!font.bold);
        assert!(!font.italic);
        assert!(!font.shadow);
        assert!(font.outline);
        assert_eq!(font.color, Color::new(255.0, 255.0, 255.0, 255.0));
        assert_eq!(font.out_color, Color::new(0.0, 0.0, 0.0, 128.0));
    }

    #[test]
    fn rgss1_and_rgss2_have_no_outline() {
        assert!(!Font::new(&defaults()).outline);
        assert!(!Font::new(&FontDefaults::rgss2()).outline);
        assert!(!Font::with(&FontDefaults::rgss2(), None, Some(12)).outline);
    }

    #[test]
    fn the_outline_is_copied_from_the_defaults() {
        let mut defaults = FontDefaults::rgss3();
        defaults.outline = false;
        defaults.out_color = Color::new(1.0, 2.0, 3.0, 4.0);
        let font = Font::new(&defaults);
        assert!(!font.outline);
        assert_eq!(font.out_color, Color::new(1.0, 2.0, 3.0, 4.0));
        // A font keeps its out colour when the default changes afterwards.
        defaults.out_color = Color::new(9.0, 9.0, 9.0, 9.0);
        defaults.outline = true;
        assert_eq!(font.out_color, Color::new(1.0, 2.0, 3.0, 4.0));
        assert!(!font.outline);
    }

    #[test]
    fn new_font_copies_the_defaults() {
        let mut defaults = defaults();
        defaults.size = 30;
        defaults.bold = true;
        defaults.italic = true;
        defaults.color = Color::rgb(1.0, 2.0, 3.0);
        defaults.names = vec!["Other".to_owned()];
        let font = Font::new(&defaults);
        assert_eq!(font.size, 30);
        assert!(font.bold);
        assert!(font.italic);
        assert_eq!(font.color, Color::rgb(1.0, 2.0, 3.0));
        assert_eq!(font.names, ["Other"]);
    }

    #[test]
    fn explicit_attributes_override_the_defaults() {
        let mut defaults = defaults();
        defaults.size = 30;
        let named = Font::with(&defaults, Some(vec!["Mine".to_owned()]), None);
        assert_eq!(named.names, ["Mine"]);
        assert_eq!(named.size, 30, "size not given, so it is the default");
        let sized = Font::with(&defaults, None, Some(12));
        assert_eq!(sized.names, ["Arial", "Fallback"]);
        assert_eq!(sized.size, 12);
    }

    #[test]
    fn existing_fonts_keep_their_values() {
        let mut defaults = defaults();
        let font = Font::new(&defaults);
        defaults.color = Color::rgb(0.0, 0.0, 0.0);
        defaults.size = 40;
        defaults.bold = true;
        defaults.names.clear();
        assert_eq!(font.color, Color::rgb(255.0, 255.0, 255.0));
        assert_eq!(font.size, 22);
        assert!(!font.bold);
        assert_eq!(font.names, ["Arial", "Fallback"]);
    }
}
