//! The build script asks for the hardening flags itself, whatever the compiler's defaults
//! are.
#![cfg(not(vkrgss_interpreter_absent))]

#[test]
fn the_build_has_the_hardening_flags_and_cet_only_on_x86_64() {
    let arguments: Vec<&str> = env!("VKRGSS_RUBY192_CFLAGS").split(' ').collect();
    for flag in [
        "-D_FORTIFY_SOURCE=3",
        "-fstack-protector-strong",
        "-fstack-clash-protection",
    ] {
        assert!(arguments.contains(&flag), "{flag} missing in {arguments:?}");
    }
    let position = |flag| arguments.iter().position(|argument| *argument == flag);
    assert!(
        matches!(
            (position("-U_FORTIFY_SOURCE"), position("-D_FORTIFY_SOURCE=3")),
            (Some(undefine), Some(define)) if undefine < define
        ),
        "{arguments:?}"
    );
    assert_eq!(
        arguments.contains(&"-fcf-protection=full"),
        cfg!(target_arch = "x86_64"),
        "{arguments:?}"
    );
}

/// Neither the interpreter nor CORE-MATH is compiled with contraction of floating-point
/// operations, which a compiler does on one architecture and not on another.
#[test]
fn the_interpreter_and_core_math_have_no_contraction() {
    for flags in [
        env!("VKRGSS_RUBY192_CFLAGS"),
        env!("VKRGSS_RUBY192_CORE_MATH_CFLAGS"),
    ] {
        let arguments: Vec<&str> = flags.split(' ').collect();
        assert!(arguments.contains(&"-ffp-contract=off"), "{arguments:?}");
    }
}
