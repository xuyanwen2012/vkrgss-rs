//! Bit-exact pixel arithmetic (blend, tone, colour, hue) with its reverse-engineering source.
//!
//! Side: shared.
//!
//! Every function here is a pure function on integer channels. The CPU reference compositor
//! and the guest's bitmap operations call them, and the GLSL of the Vulkan backend mirrors them
//! line for line, so this crate is the single definition of what a blended pixel is.
//!
//! Each function declares where its formula comes from. Today all of them are textbook
//! placeholders; they are replaced one by one as reverse-engineering specs of the original
//! runtime's inner loops land. [`PROVENANCE`] lists the current state.

#![forbid(unsafe_code)]

mod arith;

pub use arith::{
    PROVENANCE, Rgba, apply_tone, blend_add, blend_color, blend_normal, blend_sub, effective_alpha,
};

/// Where the formula of an arithmetic function comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provenance {
    /// A placeholder: the usual textbook formula, not checked against the original runtime.
    Textbook,
    /// Derived from the original runtime, as documented in the named reverse-engineering spec.
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
