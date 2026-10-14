//! `tests/prelude.rs` of `vkrgss-binding`, over Ruby 1.9.2.
#![cfg(not(vkrgss_interpreter_absent))]
extern crate vkrgss_binding_rgss3 as vkrgss_binding;
#[path = "../../vkrgss-binding/tests/prelude.rs"]
mod t;
