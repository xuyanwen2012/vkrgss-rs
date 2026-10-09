//! The emulated `Win32API` functions: the scenarios of `rgss-runtime` about them, evaluated
//! as Ruby snippets against a scripted host.

use vkxp_binding::testing::host::{self, Script, with_error_stream};

/// The value of a snippet, or its exception as `Class: message`.
fn eval(source: &str) -> String {
    match host::play(source, Script::new()).0 {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

#[test]
fn an_emulated_function_overwrites_a_buffer_in_place() {
    assert_eq!(
        eval(
            "copy = Win32API.new('kernel32', 'RtlMoveMemory', 'ppl', '')
             buffer = 'abcdef'
             same = buffer
             id = buffer.object_id
             result = copy.call(buffer, 'XYZ!', 3)
             [result, buffer, same, buffer.object_id == id]"
        ),
        r#"[0, "XYZdef", "XYZdef", true]"#
    );
    // A buffer is never grown, and an integer where a buffer belongs copies nothing.
    assert_eq!(
        eval(
            "copy = Win32API.new('kernel32', 'RtlMoveMemory', 'ppl', '')
             short = 'ab'
             copy.call(short, 'XYZ!', 4)
             untouched = 'ab'
             copy.call(untouched, 4096, 2)
             copy.call(nil, 'XYZ!', 2)
             [short, untouched]"
        ),
        r#"["XY", "ab"]"#
    );
}

#[test]
fn library_names_match_without_regard_to_case_and_suffix() {
    assert_eq!(
        eval(&with_error_stream(
            "['kernel32', 'Kernel32.dll', 'KERNEL32.DLL', 'kernel32.DLL'].map do |library|
               buffer = '....'
               Win32API.new(library, 'RtlMoveMemory', 'ppl', '').Call(buffer, 'ab', 2)
               buffer
             end"
        )),
        r#"[["ab..", "ab..", "ab..", "ab.."], ""]"#
    );
}

#[test]
fn an_unknown_function_returns_zero_with_one_report() {
    assert_eq!(
        eval(&with_error_stream(
            "api = Win32API.new('user32', 'GetKeyState', 'i', 'i')
             other = Win32API.new('user32.dll', 'RtlMoveMemory', 'ppl', '')
             buffer = 'ab'
             [api.call(13), api.call(13), Win32API.new('user32', 'GetKeyState', 'i', 'i').Call(1),
              other.call(buffer, 'XY', 2), buffer]"
        )),
        "[[0, 0, 0, 0, \"ab\"], \
         \"vkxp: Win32API user32 GetKeyState is not emulated and returns 0\\n\
         vkxp: Win32API user32.dll RtlMoveMemory is not emulated and returns 0\\n\"]"
    );
}
