//! RGSS Ruby API on top of core.
//!
//! Side: guest.

// The foundation has no class on it yet; the allowance goes when the classes arrive.
#![allow(dead_code)]

mod color;
mod error;
mod font;
mod guest;
mod rb;
mod rect;
mod table;
mod text;
mod wrap;

#[cfg(test)]
mod foundation_tests;

#[cfg(feature = "testing")]
pub mod testing;

pub use error::Error;
pub use guest::{Config, Guest};
pub use text::TextBackend;
