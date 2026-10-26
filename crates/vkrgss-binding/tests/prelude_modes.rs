//! The prelude with the settings a caller can change. A binary of its own, because the
//! interpreter of a process is prepared once.

use vkrgss_binding::RunConfig;
use vkrgss_binding::testing::host::{self, Script};
use vkrgss_binding::testing::{self, Harness};
use vkrgss_vfs::Runtime;

/// The harness, started as a game of RGSS1.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss1)
}

#[test]
fn debug_mode_battle_test_and_process_functions_follow_the_caller() {
    host::configure(
        harness(),
        RunConfig {
            debug: true,
            battle_test: true,
            process_functions: true,
            ..RunConfig::default()
        },
    );
    let (value, _) = host::play(
        harness(),
        "[$DEBUG, $BTEST, system('true'), system('false'), `echo out`, open('|echo x').read,
          IO.read('|echo y')]",
        Script::new(),
    );
    assert_eq!(
        value.as_deref(),
        Ok(r#"[true, true, true, false, "out\n", "x\n", "y\n"]"#)
    );
}
