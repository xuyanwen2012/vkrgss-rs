//! Bit-exact pixel arithmetic (blend, tone, colour, hue) with its reverse-engineering source.
//!
//! Side: shared.
//!
//! Every function here is a pure function on integer channels. The CPU reference compositor
//! and the guest's bitmap operations call them, and the GLSL of the Vulkan backend mirrors them
//! line for line, so this crate is the single definition of what a blended pixel is.
//!
//! Each function declares where its formula comes from. Blending onto an opaque destination,
//! the normal blend onto a destination with alpha, the colour, the tone and stretch sampling
//! follow measurements of the original runtime; the rest are textbook placeholders, replaced
//! one by one as measurements and reverse-engineering specs of the original runtime's inner
//! loops land. [`PROVENANCE`] lists the current state.
//!
//! Arithmetic that only the guest's bitmap operations use (hue rotation, stretch sampling) runs
//! on the CPU alone and has no GLSL counterpart. It is listed separately, in
//! [`BITMAP_PROVENANCE`].

#![forbid(unsafe_code)]

mod arith;
mod bitmap;

pub use arith::{
    PROVENANCE, Rgba, apply_gray, apply_tone, blend_add, blend_add_translucent, blend_color,
    blend_normal, blend_normal_translucent, blend_sub, blend_sub_translucent, effective_alpha,
};
pub use bitmap::{BITMAP_PROVENANCE, rotate_hue, stretch_index};

/// Where the formula of an arithmetic function comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provenance {
    /// A placeholder: the usual textbook formula, not checked against the original runtime.
    Textbook,
    /// Derived from the original runtime, as documented in the named reverse-engineering spec
    /// or measurement.
    ReverseEngineered { source: &'static str },
}

impl Provenance {
    /// Whether the formula is still a placeholder.
    pub const fn is_placeholder(&self) -> bool {
        matches!(self, Provenance::Textbook)
    }
}

impl std::fmt::Display for Provenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provenance::Textbook => f.write_str("textbook placeholder"),
            Provenance::ReverseEngineered { source } => write!(f, "reverse-engineered: {source}"),
        }
    }
}

/// One public arithmetic function and the origin of its formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArithmeticFn {
    /// Name of the function in this crate.
    pub name: &'static str,
    /// The formula it evaluates, in the notation of its documentation.
    pub formula: &'static str,
    pub provenance: Provenance,
}

/// The provenance of every public arithmetic function, in declaration order.
pub fn provenance() -> &'static [ArithmeticFn] {
    PROVENANCE
}

/// The provenance of the arithmetic of bitmap operations, in declaration order. These functions
/// run only on the CPU; none of them appears in [`provenance`], the list GPU backends mirror.
pub fn bitmap_provenance() -> &'static [ArithmeticFn] {
    BITMAP_PROVENANCE
}
