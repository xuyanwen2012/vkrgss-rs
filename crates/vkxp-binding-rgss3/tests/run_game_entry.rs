//! `tests/run_game_entry.rs` of `vkxp-binding`, over Ruby 1.9.2.
#![cfg(not(vkxp_interpreter_absent))]
extern crate vkxp_binding_rgss3 as vkxp_binding;
#[path = "../../vkxp-binding/tests/run_game_entry.rs"]
mod t;
