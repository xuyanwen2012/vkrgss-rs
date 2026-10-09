//! Font lookup, substitution table, glyph rasterisation.
//!
//! Side: guest.

#![forbid(unsafe_code)]

mod engine;
mod face;
mod glyph_cache;
mod index;
mod sources;
mod substitution;

pub use engine::{
    DEFAULT_FALLBACK_FAMILIES, DEFAULT_GLYPH_CACHE_BYTES, ResolvedFace, TextEngine, TextMask,
    TextRequest,
};
pub use sources::{FontSources, SystemFonts};
pub use substitution::default_substitutions;
