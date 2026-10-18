//! The pointers that `malloc` returns on Android.
//!
//! Bionic tags them in their top byte, which arm64 ignores when it loads and stores (Top Byte
//! Ignore), so that a heap address read as a signed number is negative. Ruby gives an object
//! the id of its address (`Object#object_id`, and `Object#id` of Ruby 1.8.7), and scripts of
//! old games compare such ids with small numbers: a script that asks an enemy of VX for its
//! `id`, which it has no method of, gets its object id. On Windows, Linux and macOS that is
//! positive, on Android it would be negative and the script would take another branch. A
//! guest process therefore turns the tags off before its interpreter allocates anything.

use std::os::raw::c_int;

/// `M_BIONIC_SET_HEAP_TAGGING_LEVEL` of bionic's `malloc.h`.
const SET_HEAP_TAGGING_LEVEL: c_int = -204;

/// `M_HEAP_TAGGING_LEVEL_NONE` of bionic's `malloc.h`.
const HEAP_TAGGING_LEVEL_NONE: c_int = 0;

unsafe extern "C" {
    fn mallopt(option: c_int, value: c_int) -> c_int;
}

/// Makes `malloc` return untagged pointers from now on; bionic still accepts the tagged
/// pointers it returned before. A failure is reported and the game goes on.
pub fn untag_pointers() {
    // SAFETY: the option only changes how later allocations are tagged, which bionic allows
    // at any time; it takes no pointer.
    let done = unsafe { mallopt(SET_HEAP_TAGGING_LEVEL, HEAP_TAGGING_LEVEL_NONE) };
    if done == 0 {
        tracing::warn!("the heap of the guest process still tags its pointers");
    }
}
