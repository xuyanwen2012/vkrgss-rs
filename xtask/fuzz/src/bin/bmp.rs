//! The fuzz target `bmp`: arbitrary bytes that begin as a BMP does, decoded by
//! `vkrgss_image::decode`.

#![no_main]

#[path = "../check.rs"]
mod check;
#[path = "../../../../crates/vkrgss-image/tests/counting/mod.rs"]
mod counting;

use vkrgss_image::ImageFormat;

#[global_allocator]
static ALLOCATOR: counting::Counting = counting::Counting;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    check::image(data, ImageFormat::Bmp, true);
});
