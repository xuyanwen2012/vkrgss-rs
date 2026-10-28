//! The fuzz target `jpeg`: arbitrary bytes that begin as a JPEG does, decoded by
//! `vkrgss_image::decode`, which takes them to the decoder that follows the original and, where
//! that gives up, to `jpeg-decoder`.

#![no_main]

#[path = "../check.rs"]
mod check;
#[path = "../../../../crates/vkrgss-image/tests/counting/mod.rs"]
mod counting;

use vkrgss_image::ImageFormat;

#[global_allocator]
static ALLOCATOR: counting::Counting = counting::Counting;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    check::image(data, ImageFormat::Jpeg, false);
});
