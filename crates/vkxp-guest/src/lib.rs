//! The guest role: what makes a process the guest of a program.
//!
//! Side: guest.
//!
//! A game's scripts can run in a process of their own, which a program starts and which
//! plays the game over a socket of that program. This crate is what such a process does
//! apart from running the game, so that an executable other than the program can be a
//! guest too:
//!
//! - [`role`]: the hidden option that names the program's socket and carries the seed.
//! - [`signals`]: the signal settings of a guest.
//! - [`sandbox`]: the restriction a guest process puts on itself.
//!
//! The crate links no interpreter and no binding: the executable that links it gives it the
//! function that runs the game.
//!
//! `unsafe` is denied here and allowed in the modules that call the platform directly.

#![deny(unsafe_code)]

pub mod role;
pub mod sandbox;
#[allow(unsafe_code)]
pub mod signals;
