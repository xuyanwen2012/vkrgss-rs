//! The build script asks for the hardening flags itself, whatever the compiler's defaults
//! are, and the instrumented builds of the tests leave `_FORTIFY_SOURCE` out.

mod common;

use std::path::Path;

/// The arguments the build script gave the compiler of every C file.
fn build_arguments() -> Vec<&'static str> {
    env!("VKRGSS_RUBY_CFLAGS").split(' ').collect()
}

fn position(arguments: &[&str], flag: &str) -> Option<usize> {
    arguments.iter().position(|argument| *argument == flag)
}

#[test]
fn the_build_has_the_hardening_flags_and_cet_only_on_x86_64() {
    let arguments = build_arguments();
    for flag in [
        "-D_FORTIFY_SOURCE=3",
        "-fstack-protector-strong",
        "-fstack-clash-protection",
    ] {
        assert!(arguments.contains(&flag), "{flag} missing in {arguments:?}");
    }
    let undefine = position(&arguments, "-U_FORTIFY_SOURCE");
    let define = position(&arguments, "-D_FORTIFY_SOURCE=3");
    assert!(
        matches!((undefine, define), (Some(undefine), Some(define)) if undefine < define),
        "{arguments:?}"
    );
    assert_eq!(
        arguments.contains(&"-fcf-protection=full"),
        cfg!(target_arch = "x86_64"),
        "{arguments:?}"
    );
}

#[test]
fn the_instrumented_builds_leave_out_fortify_and_keep_the_rest() {
    let command = common::instrumented_compiler(Path::new("."), &[], &[]);
    let arguments: Vec<&str> = command
        .get_args()
        .map(|argument| argument.to_str().unwrap())
        .collect();
    assert!(
        !arguments
            .iter()
            .any(|argument| argument.contains("_FORTIFY_SOURCE")),
        "{arguments:?}"
    );
    assert!(arguments.contains(&"-fstack-protector-strong"));
}

/// Neither the interpreter nor CORE-MATH is compiled with contraction of floating-point
/// operations, which a compiler does on one architecture and not on another.
#[test]
fn the_interpreter_and_core_math_have_no_contraction() {
    let core_math: Vec<&str> = env!("VKRGSS_RUBY_CORE_MATH_CFLAGS").split(' ').collect();
    for arguments in [build_arguments(), core_math] {
        assert!(arguments.contains(&"-ffp-contract=off"), "{arguments:?}");
    }
}
