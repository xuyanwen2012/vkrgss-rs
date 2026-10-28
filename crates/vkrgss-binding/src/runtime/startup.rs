//! What a game of each runtime starts with: the frame size and rate, the rules of the core,
//! the empty scene, `RGSS_VERSION` and the globals of test mode.

use vkrgss_core::{Rules, Scene};
use vkrgss_proto::Size;
use vkrgss_vfs::{LibraryVersion, Runtime};

/// `RGSS_VERSION` of a game of RGSS3 whose DLL has `version`: `"3.0.0"` for a version
/// whose first three numbers are 3.0.0 ([`LibraryVersion::is_3_0_0`]), and `"3.0.1"`, the
/// version vkrgss follows, for every other version, a DLL without a version record and no
/// DLL. This is the table of the glyph rule of `docs/decisions.md`.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1 and RGSS300 3.0.0.1: each
/// states its own version.
pub(super) fn rgss3_version(version: Option<LibraryVersion>) -> &'static str {
    match version {
        Some(version) if version.is_3_0_0() => "3.0.0",
        _ => "3.0.1",
    }
}

/// The source that defines `RGSS_VERSION` as `version`, in ASCII-8BIT. A process prepares
/// once; the harness of the tests prepares again for every game it plays on its one
/// interpreter, so a constant of an earlier preparation is removed first and the constant
/// is defined once, without the warning of a constant defined again.
pub(super) fn rgss3_version_source(version: &str) -> String {
    format!(
        "Object.send(:remove_const, :RGSS_VERSION) if defined?(RGSS_VERSION)\n\
         RGSS_VERSION = \"{version}\".force_encoding(\"ASCII-8BIT\")\n"
    )
}

/// The source that sets the globals of test mode. RGSS1 has `$DEBUG` and `$BTEST`. The
/// later runtimes have `$TEST` instead of `$DEBUG`, which stays `false`, and a battle test
/// sets `$TEST` as well: measured on the original runtime of VX.
pub(super) fn test_mode_globals(runtime: Runtime, debug: bool, battle_test: bool) -> String {
    match runtime {
        Runtime::Rgss1 => format!("$DEBUG = {debug}\n$BTEST = {battle_test}\n"),
        Runtime::Rgss2 | Runtime::Rgss3 => format!(
            "$DEBUG = false\n$TEST = {}\n$BTEST = {battle_test}\n",
            debug || battle_test
        ),
    }
}

/// The size of the frame of a game of RGSS1.
const RGSS1_FRAME_SIZE: Size = Size::new(640, 480);
/// The frame rate a game of RGSS1 starts with.
const RGSS1_FRAME_RATE: i64 = 40;
/// The size of the frame of a game of RGSS2, measured on the original runtime.
const RGSS2_FRAME_SIZE: Size = Size::new(544, 416);
/// The frame rate a game of RGSS2 starts with, measured on the original runtime.
const RGSS2_FRAME_RATE: i64 = 60;

/// The size of the frame a game of `runtime` starts with, which is the logical size of its
/// frame messages.
pub fn frame_size(runtime: Runtime) -> Size {
    match runtime {
        Runtime::Rgss1 => RGSS1_FRAME_SIZE,
        Runtime::Rgss2 | Runtime::Rgss3 => RGSS2_FRAME_SIZE,
    }
}

/// The rules of the core the scene and the input state of a game of `runtime` follow where the
/// runtimes differ.
pub(crate) fn rules(runtime: Runtime) -> Rules {
    match runtime {
        Runtime::Rgss1 => Rules::Rgss1,
        Runtime::Rgss2 => Rules::Rgss2,
        Runtime::Rgss3 => Rules::Rgss3,
    }
}

/// The empty scene a game of `runtime` starts with: its rules, its frame size and its frame
/// rate.
pub(crate) fn new_scene(runtime: Runtime) -> Scene {
    let mut scene = Scene::with_rules(rules(runtime), frame_size(runtime));
    scene.set_frame_rate(match runtime {
        Runtime::Rgss1 => RGSS1_FRAME_RATE,
        Runtime::Rgss2 | Runtime::Rgss3 => RGSS2_FRAME_RATE,
    });
    scene
}
