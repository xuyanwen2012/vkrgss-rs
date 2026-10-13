//! Build of the embedded Ruby 1.9.2 and raw FFI.
//!
//! Side: guest.
//!
//! The build script compiles the vendored Ruby 1.9.2-p0 of `third_party/ruby-1.9.2/`, the
//! release that the original runtime of RPG Maker VX Ace embeds, with the encodings that
//! runtime has and its `Zlib` extension, with a fixed file list and fixed flags.
//!
//! The crate is for the target `x86_64-unknown-linux-gnu`. For any other target the build
//! script compiles nothing and the crate has no item.

/// Declares the items for the one target the interpreter is built for.
macro_rules! on_target {
    ($($item:item)*) => {
        $(
            #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
            $item
        )*
    };
}

on_target! {
    // The `Zlib` extension calls into the zlib that `libz-sys` builds; naming the crate is
    // what makes rustc link it.
    extern crate libz_sys;

    pub mod ffi;
}
