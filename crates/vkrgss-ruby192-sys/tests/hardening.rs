//! The build script asks for the hardening flags itself, whatever the compiler's defaults
//! are.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

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
