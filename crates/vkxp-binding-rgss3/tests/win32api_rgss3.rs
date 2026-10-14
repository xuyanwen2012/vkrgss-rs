//! `Win32API` and `DL` in a game of RGSS3 over Ruby 1.9.2: Ruby snippets evaluated against a
//! scripted host. The scenarios of "Win32API of RGSS3", "Libraries of Win32API in RGSS3",
//! "Declarations and count of Win32API in RGSS3", "Parameters of Win32API in RGSS3",
//! "Values of Win32API in RGSS3", "Measured calls of Win32API in RGSS3" and "DL of RGSS3" of
//! `rgss-runtime`. A file of this package alone, so that it never runs as RGSS1 or RGSS2
//! over Ruby 1.8.7.
//!
//! The expected values marked as measured are those of the original runtime of VX Ace (DLL
//! version 3.0.1.1, 2026-10-13, `ace-v1-api-REPORT.md` sections 5.8 and 5.9, kept outside
//! the repository).
//!
//! The game directory of the harness gets a `Game.ini` that names `System\RGSS301.dll`, that
//! file and a `tktk_bitmap.dll`, all empty stand-ins written by the test, and the guest gets
//! the settings read from that `Game.ini`.
#![cfg(not(vkxp_interpreter_absent))]
extern crate vkxp_binding_rgss3 as vkxp_binding;

use std::fs;
use std::sync::Once;

use vkxp_binding::RunConfig;
use vkxp_binding::testing;
use vkxp_binding::testing::host::{self, Script, with_error_stream};
use vkxp_vfs::{GameIni, Runtime};

/// The title of the game, 16 bytes as in the measurement.
const TITLE: &str = "Win32API of Ace!";

/// Starts the harness as a game of RGSS3 with the files of this test, once.
fn start() {
    testing::start_as(Runtime::Rgss3);
    static STARTED: Once = Once::new();
    STARTED.call_once(|| {
        let game = testing::game_dir();
        let ini = format!(
            "[Game]\r\nLibrary=System\\RGSS301.dll\r\nScripts=Data\\Scripts.rvdata2\r\n\
             Title={TITLE}\r\n"
        );
        fs::write(game.join("Game.ini"), ini).unwrap();
        fs::create_dir_all(game.join("System")).unwrap();
        fs::write(game.join("System/RGSS301.dll"), b"").unwrap();
        fs::write(game.join("tktk_bitmap.dll"), b"").unwrap();
        host::configure(RunConfig {
            ini: GameIni::read(game, None).unwrap(),
            runtime: Runtime::Rgss3,
            ..RunConfig::default()
        });
    });
}

/// The value of a snippet played against a scripted host, or its exception as
/// `Class: message`.
fn eval_with(source: &str, script: Script) -> String {
    start();
    match host::play(source, script).0 {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

fn eval(source: &str) -> String {
    eval_with(source, Script::new())
}

/// Ruby source that clears the keys and toggles that `SetKeyboardState` and the tests before
/// left in the guest.
const CLEAR_KEYS: &str =
    "Win32API.new('user32', 'SetKeyboardState', 'p', 'i').call(\"\\0\" * 256)\n";

/// Ruby that evaluates each of a list of procs and gives its value, or its exception as
/// `[class, message]`.
const OUTCOMES: &str = "
    def outcomes(*calls)
      calls.map do |call|
        begin
          call.call
        rescue Exception => error
          [error.class, error.message]
        end
      end
    end
";

// --- The class --------------------------------------------------------------------------------

/// Scenario "Arguments, measured on the original".
#[test]
fn the_constructor_takes_four_arguments_and_strings() {
    assert_eq!(
        eval(&format!(
            "{OUTCOMES}
             outcomes(proc {{ Win32API.new('user32', 'GetKeyState', 'i') }},
                      proc {{ Win32API.new(:user32, 'GetKeyState', 'i', 'i') }},
                      proc {{ Win32API.new('user32', 'GetKeyState', nil, nil).class }})"
        )),
        "[[ArgumentError, \"wrong number of arguments(3 for 4)\"], \
         [TypeError, \"can't convert Symbol into String\"], Win32API]"
    );
}

/// Scenario "The object, measured on the original".
#[test]
fn the_object_keeps_its_state_out_of_sight_and_cannot_be_dumped() {
    assert_eq!(
        eval(&format!(
            "{CLEAR_KEYS}{OUTCOMES}
             api = Win32API.new('user32', 'GetKeyState', 'i', 'i')
             [api.instance_variables, api.dup.call(0x41),
              Class.new(Win32API).new('user32', 'GetKeyState', 'i', 'i').call(0x41),
              api.clone.Call(0x41), outcomes(proc {{ Marshal.dump(api) }})]"
        )),
        "[[], 0, 0, 0, [[TypeError, \"no marshal_dump is defined for class Data\"]]]"
    );
}

#[test]
fn an_object_without_a_constructor_call_raises() {
    assert_eq!(
        eval("Win32API.allocate.call"),
        "RuntimeError: uninitialized Win32API"
    );
}

// --- Libraries --------------------------------------------------------------------------------

/// Scenario "A missing library, measured on the original".
#[test]
fn a_missing_library_raises_load_library() {
    assert_eq!(
        eval(&format!(
            "{OUTCOMES}
             outcomes(proc {{ Win32API.new('nosuch', 'Function', '', '') }},
                      proc {{ Win32API.new('System/Absent.dll', 'Function', '', '') }})"
        )),
        "[[RuntimeError, \"LoadLibrary: nosuch\\n\"], \
         [RuntimeError, \"LoadLibrary: System/Absent.dll\\n\"]]"
    );
}

/// Scenario "The game's own libraries, measured on the original".
#[test]
fn the_libraries_of_the_game_are_found_and_their_functions_reported() {
    assert_eq!(
        eval(&with_error_stream(
            "['System/RGSS301.dll', 'RGSS301', 'tktk_bitmap'].map do |library|
               Win32API.new(library, 'Function', 'i', 'i').call(1)
             end"
        )),
        "[[0, 0, 0], \
         \"vkxp: Win32API System/RGSS301.dll Function is not emulated and returns 0\\n\
         vkxp: Win32API RGSS301 Function is not emulated and returns 0\\n\
         vkxp: Win32API tktk_bitmap Function is not emulated and returns 0\\n\"]"
    );
}

/// Scenario "System libraries".
#[test]
fn the_system_libraries_of_the_list_are_found() {
    assert_eq!(
        eval(
            "['wininet', 'Kernel32', 'USER32.DLL', 'msvcrt', 'System\\\\RGSS301', 'TKTK_BITMAP.DLL'].map do |library|
               Win32API.new(library, 'Function', '', '').class
             end.uniq"
        ),
        "[Win32API]"
    );
}

#[test]
fn a_function_outside_the_table_is_reported_once() {
    assert_eq!(
        eval(&with_error_stream(
            "api = Win32API.new('user32', 'GetKeyNameTextA', 'i', 'i')
             [api.call(13), api.call(13), Win32API.new('user32', 'GetKeyNameTextA', '', '').call]"
        )),
        "[[0, 0, 0], \"vkxp: Win32API user32 GetKeyNameTextA is not emulated and returns 0\\n\"]"
    );
}

// --- Declarations and count -------------------------------------------------------------------

/// Scenarios "Ignored characters, measured on the original" and "Count, measured on the
/// original".
#[test]
fn a_call_with_another_count_than_the_declared_one_raises() {
    assert_eq!(
        eval(&format!(
            "{OUTCOMES}
             outcomes(proc {{ Win32API.new('user32', 'GetKeyState', 'pointer', 'i').call(1, 2) }},
                      proc {{ Win32API.new('user32', 'GetKeyState', 'i', 'i').call }},
                      proc {{ Win32API.new('user32', 'GetKeyState', 'I', 'i').call(1, 2) }})"
        )),
        "[[RuntimeError, \"wrong number of parameters: expected 3, got 2\"], \
         [RuntimeError, \"wrong number of parameters: expected 1, got 0\"], \
         [RuntimeError, \"wrong number of parameters: expected 1, got 2\"]]"
    );
}

/// Scenario "Array, spaces and v".
#[test]
fn arrays_spaces_and_v_declare_what_they_say() {
    assert_eq!(
        eval(&with_error_stream(
            "[Win32API.new('wininet', 'InternetOpenUrl', ['l', 'p', 'p', 'l', 'l', 'p'], 'l')
                .call(0, 'http://localhost/', nil, 0, 0, 0),
              Win32API.new('tktk_bitmap', 'Blur', 'n i i i i', 'i').call(1, 2, 3, 4, 5),
              Win32API.new('Kernel32', 'GetLastError', [], 'l').call,
              Win32API.new('user32', 'GetActiveWindow', 'v', 'l').call != 0,
              Win32API.new('user32', 'GetKeyState', ['ii', 'x', ''], 'i').call(0x41)]"
        )),
        "[[0, 0, 0, true, 0], \
         \"vkxp: Win32API wininet InternetOpenUrl is not emulated and returns 0\\n\
         vkxp: Win32API tktk_bitmap Blur is not emulated and returns 0\\n\
         vkxp: Win32API Kernel32 GetLastError is not emulated and returns 0\\n\"]"
    );
}
