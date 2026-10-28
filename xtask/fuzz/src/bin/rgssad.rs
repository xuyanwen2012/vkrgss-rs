//! The fuzz target `rgssad`: arbitrary bytes read as an encrypted archive of version 1 and of
//! version 3, with the data of every entry.

#![no_main]

#[path = "../../../../crates/vkrgss-image/tests/counting/mod.rs"]
mod counting;

#[global_allocator]
static ALLOCATOR: counting::Counting = counting::Counting;

/// The bytes reading an archive may hold for each byte of its file, and the allowance beyond
/// them, as `vkrgss-vfs` states them.
const MULTIPLE: usize = 64;
const ALLOWANCE: usize = 1 << 20;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let (_, usage) = counting::measure(|| vkrgss_vfs::read_both_versions(data));
    let bound = MULTIPLE * data.len() + ALLOWANCE;
    assert!(
        usage.peak <= bound,
        "reading the archive held {} bytes at once ({} in all), above the bound of {bound}",
        usage.peak,
        usage.total
    );
});
