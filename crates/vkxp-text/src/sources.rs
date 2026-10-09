//! Where an engine takes its fonts from.

use std::path::PathBuf;

use crate::line_box::LineBox;

/// Which fonts count as the system's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SystemFonts {
    /// No system fonts: the engine only knows the caller's directories.
    None,
    /// The fonts installed on the machine, found through its font configuration.
    #[default]
    Installed,
    /// These directories stand in for the system's font directories. Meant for tests and for
    /// installations that keep their fonts in an unusual place.
    Directories(Vec<PathBuf>),
}

/// Everything a [`crate::TextEngine`] is built from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FontSources {
    /// The system's fonts.
    pub system: SystemFonts,
    /// Directories of the caller, searched recursively. Their fonts shadow system fonts of the
    /// same family, and an earlier directory shadows a later one.
    pub directories: Vec<PathBuf>,
    /// Substitutions tried before the built-in ones: a family name and the families that may
    /// replace it, in order of preference.
    pub substitutions: Vec<(String, Vec<String>)>,
    /// Line boxes that take precedence over the built-in ones: a family name and the line box
    /// a font is set in when it replaces that family. See [`crate::default_line_box`].
    pub line_boxes: Vec<(String, LineBox)>,
    /// Whether every font is set in its own line box, a replacement included: the line boxes
    /// of replaced families, the built-in ones too, are not used.
    pub own_metrics: bool,
    /// Families used when no requested name matches and for characters the selected font
    /// lacks, in order. `None` keeps [`crate::DEFAULT_FALLBACK_FAMILIES`].
    pub fallback_families: Option<Vec<String>>,
}

impl FontSources {
    /// The system's installed fonts and nothing else.
    pub fn system() -> Self {
        Self::default()
    }

    /// Only the fonts in `directories`, without system fonts.
    pub fn directories_only(directories: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        Self {
            system: SystemFonts::None,
            directories: directories.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }
}
