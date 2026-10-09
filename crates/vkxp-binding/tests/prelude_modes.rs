//! The prelude with the settings a caller can change. A binary of its own, because the
//! interpreter of a process is prepared once.

use vkxp_binding::RunConfig;
use vkxp_binding::testing::host::{self, Script};

#[test]
fn debug_mode_battle_test_and_process_functions_follow_the_caller() {
    host::configure(RunConfig {
        debug: true,
        battle_test: true,
        process_functions: true,
        ..RunConfig::default()
    });
    let (value, _) = host::play(
        "[$DEBUG, $BTEST, system('true'), system('false'), `echo out`]",
        Script::new(),
    );
    assert_eq!(
        value.as_deref(),
        Ok(r#"[true, true, true, false, "out\n"]"#)
    );
}
