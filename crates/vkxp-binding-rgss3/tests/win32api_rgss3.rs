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
use vkxp_proto::{HostToGuest, InputSnapshot};
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

// --- Parameters -------------------------------------------------------------------------------

/// Scenario "Integer parameters, measured on the original".
#[test]
fn an_integer_parameter_is_converted_as_an_unsigned_long() {
    assert_eq!(
        eval(&format!(
            "{CLEAR_KEYS}{OUTCOMES}
             state = Win32API.new('user32', 'GetKeyState', 'i', 'i')
             outcomes(proc {{ state.call(nil) }}, proc {{ state.call('a') }},
                      proc {{ state.call(true).class }}, proc {{ state.call(65.9) == state.call(65) }})"
        )),
        "[[TypeError, \"no implicit conversion from nil to integer\"], \
         [TypeError, \"can't convert String into Integer\"], \
         [TypeError, \"can't convert true into Integer\"], true]"
    );
}

/// Scenario "Pointer parameters, measured on the original".
#[test]
fn a_pointer_parameter_takes_a_string_that_is_not_frozen() {
    assert_eq!(
        eval(&format!(
            "{OUTCOMES}
             get = Win32API.new('user32', 'GetKeyboardState', 'p', 'i')
             outcomes(proc {{ get.call((\"\\0\" * 256).freeze) }}, proc {{ get.call(1.5) }},
                      proc {{ get.call(:sym) }}, proc {{ get.call([]) }}, proc {{ get.call(nil) }},
                      proc {{ get.call(0) }}, proc {{ get.call(4096) }})"
        )),
        "[[RuntimeError, \"can't modify frozen string\"], \
         [TypeError, \"can't convert Float into String\"], \
         [TypeError, \"can't convert Symbol into String\"], \
         [TypeError, \"can't convert Array into String\"], 0, 0, 0]"
    );
}

/// Scenario "Object ids".
#[test]
fn an_object_id_is_taken_by_its_low_32_bits() {
    assert_eq!(
        eval(&with_error_stream(
            "bitmap = Bitmap.new(1, 1)
             value = Win32API.new('tktk_bitmap', 'InvertColor', 'n', 'i').call(bitmap.object_id)
             bitmap.dispose
             value"
        )),
        "[0, \"vkxp: Win32API tktk_bitmap InvertColor is not emulated and returns 0\\n\"]"
    );
}

/// Scenario "Unsigned, measured on the original", and the bounds of the 64-bit guest.
#[test]
fn an_integer_parameter_wraps_to_32_bits_up_to_the_bounds_of_the_guest() {
    assert_eq!(
        eval(&format!(
            "{CLEAR_KEYS}{OUTCOMES}
             state = Win32API.new('user32', 'GetKeyState', 'i', 'i')
             outcomes(proc {{ state.call(2**32 - 1) }}, proc {{ state.call(2**64) }},
                      proc {{ state.call(-(2**63) - 1) }}, proc {{ state.call(2**32 + 0x41) }})"
        )),
        "[0, [RangeError, \"bignum too big to convert into `unsigned long'\"], \
         [RangeError, \"bignum out of range of unsigned long\"], 0]"
    );
}

// --- Values -----------------------------------------------------------------------------------

/// Scenario "Exports, measured on the original".
#[test]
fn the_export_decides_the_value() {
    assert_eq!(
        eval(&format!(
            "{OUTCOMES}
             outcomes(*['v', 'p', '', nil, 'x', 'V'].map {{ |export|
               proc {{ Win32API.new('user32', 'GetSystemMetrics', 'i', export).call(5) }}
             }})"
        )),
        "[0, [ArgumentError, \"NULL pointer given\"], 0, 0, 0, 0]"
    );
}

/// Scenario "Signed, measured on the original".
#[test]
fn a_held_key_gives_a_negative_value() {
    let held = Script::new().at_start(HostToGuest::Input(InputSnapshot::EMPTY.with_key(0x0D)));
    assert_eq!(
        eval_with(
            "Graphics.update
             Input.update
             state = Win32API.new('user32', 'GetKeyState', 'i', 'i').call(0x0D)
             async = Win32API.new('user32', 'GetAsyncKeyState', 'i', 'L').call(0x0D)
             [state < 0, state & 0x80, async < 0, async & 0x8000]",
            held
        ),
        "[true, 128, true, 32768]"
    );
}

/// Scenario "Not emulated with p".
#[test]
fn a_function_outside_the_table_gives_an_empty_string_for_p() {
    assert_eq!(
        eval(&with_error_stream(
            "value = Win32API.new('user32', 'GetWindowTextA', 'lpi', 'p').call(0, nil, 0)
             [value, value.encoding.name]"
        )),
        "[[\"\", \"ASCII-8BIT\"], \
         \"vkxp: Win32API user32 GetWindowTextA is not emulated and returns 0\\n\"]"
    );
}

/// Scenario "A buffer keeps its label, measured on the original".
#[test]
fn a_filled_buffer_keeps_its_length_and_label() {
    assert_eq!(
        eval(&format!(
            "handle = Win32API.new('user32', 'FindWindowA', 'pp', 'l').call('RGSS Player', '{TITLE}')
             rect = (\"\\0\" * 16).force_encoding('UTF-8')
             same = rect
             result = Win32API.new('user32', 'GetClientRect', 'lp', 'i').call(handle, rect)
             [result, rect.equal?(same), rect.encoding.name, rect.bytesize, rect.unpack('l4')]"
        )),
        "[1, true, \"UTF-8\", 16, [0, 0, 544, 416]]"
    );
}

// --- Measured calls ---------------------------------------------------------------------------

/// Scenario "Title, measured on the original".
#[test]
fn the_title_is_read_with_a_path_and_not_with_a_bare_name() {
    assert_eq!(
        eval(
            "get = Win32API.new('kernel32', 'GetPrivateProfileStringA', 'pppplp', 'l')
             read = proc do |file|
               buffer = \"\\0\" * 256
               length = get.call('Game', 'Title', '', buffer, 255, file)
               [length, buffer[0, length]]
             end
             [read.call('./Game.ini'), read.call('Game.ini'), read.call('.\\\\Game.ini')]"
        ),
        format!("[[16, \"{TITLE}\"], [0, \"\"], [16, \"{TITLE}\"]]")
    );
}

#[test]
fn a_bare_profile_name_reads_the_default_and_writes_nothing() {
    let game = testing::game_dir();
    fs::write(game.join("Bare.ini"), "[Options]\r\nVolume=80\r\n").unwrap();
    assert_eq!(
        eval(
            "get = Win32API.new('kernel32', 'GetPrivateProfileIntA', 'ppip', 'i')
             set = Win32API.new('kernel32', 'WritePrivateProfileStringA', 'pppp', 'l')
             [get.call('Options', 'Volume', 7, 'Bare.ini'), get.call('Options', 'Volume', 7, './Bare.ini'),
              set.call('Options', 'Volume', '65', 'Bare.ini'), set.call('Options', 'Name', 'a', 'New.ini'),
              set.call('Options', 'Volume', '65', '.\\\\Bare.ini'), get.call('Options', 'Volume', 7, './Bare.ini')]"
        ),
        "[7, 80, 0, 0, 1, 65]"
    );
    assert!(!game.join("New.ini").exists());
}

/// Scenario "Window, measured on the original".
#[test]
fn the_window_class_finds_the_game_window_of_544_by_416() {
    assert_eq!(
        eval(&format!(
            "handle = Win32API.new('user32', 'FindWindowA', 'pp', 'l').call('RGSS Player', '{TITLE}')
             rect = \"\\xff\" * 16
             Win32API.new('user32', 'GetClientRect', 'lp', 'i').call(handle, rect)
             [handle != 0, rect.unpack('l4')]"
        )),
        "[true, [0, 0, 544, 416]]"
    );
}

/// Scenario "Cursor, measured on the original": no other test of this binary calls
/// `ShowCursor`, so this is the first call of the run.
#[test]
fn the_first_show_cursor_0_gives_minus_1() {
    assert_eq!(
        eval(
            "show = Win32API.new('user32', 'ShowCursor', 'i', 'i')
             [show.call(0), show.call(1)]"
        ),
        "[-1, 0]"
    );
}

/// Scenario "Text, measured on the original".
#[test]
fn text_converts_through_utf_16_to_utf_8_and_shift_jis() {
    assert_eq!(
        eval(
            "to_wide = Win32API.new('kernel32', 'MultiByteToWideChar', 'ilpipi', 'i')
             to_bytes = Win32API.new('kernel32', 'WideCharToMultiByte', 'ilpipipp', 'i')
             text = [0xE6, 0x97, 0xA5, 0xE6, 0x9C, 0xAC].pack('C*').force_encoding('UTF-8')
             wide = \"\\0\" * 16
             units = to_wide.call(65001, 0, text, -1, wide, 8)
             [65001, 932].map do |code_page|
               bytes = \"\\0\" * 16
               size = to_bytes.call(code_page, 0, wide, -1, bytes, 16, nil, nil)
               bytes.unpack('C*')[0, size]
             end.unshift(units)"
        ),
        "[3, [230, 151, 165, 230, 156, 172, 0], [147, 250, 150, 123, 0]]"
    );
}

/// Scenario "The address trick".
#[test]
fn an_address_as_the_source_copies_nothing() {
    assert_eq!(
        eval(
            "bitmap = Bitmap.new(2, 2)
             buffer = \"\\1\\2\\3\\4\"
             value = Win32API.new('kernel32', 'RtlMoveMemory', 'pll', 'l')
               .call(buffer, bitmap.object_id * 2 + 16, 4)
             bitmap.dispose
             [value, buffer.unpack('C*')]"
        ),
        "[0, [1, 2, 3, 4]]"
    );
}

// --- DL ---------------------------------------------------------------------------------------

/// Scenario "Constants, measured on the original".
#[test]
fn dl_has_the_constants_of_the_original() {
    assert_eq!(
        eval(
            "[DL::TYPE_VOID, DL::TYPE_INT, DL::TYPE_DOUBLE, DL::SIZEOF_VOIDP, DL::SIZEOF_LONG_LONG,
              DL::ALIGN_SHORT, DL::RTLD_NOW, DL::DLError.superclass, DL::DLTypeError.superclass]"
        ),
        "[0, 4, 8, 4, 8, 2, -1, StandardError, DL::DLError]"
    );
}

/// Scenario "Refused".
#[test]
fn every_way_into_a_library_or_memory_raises_dl_error() {
    let calls = [
        ("DL.dlopen('user32')", "DL.dlopen"),
        ("DL::Handle.new('user32')", "DL::Handle.new"),
        ("DL::CFunc.new(0)", "DL::CFunc.new"),
        ("DL::CPtr.malloc(8)", "DL::CPtr.malloc"),
        ("DL.malloc(8)", "DL.malloc"),
        ("DL::Handle['user32']", "DL::Handle[]"),
        ("DL::Handle.sym('GetKeyState')", "DL::Handle.sym"),
        ("DL::CPtr['abc']", "DL::CPtr[]"),
        ("DL::CPtr.new(0)", "DL::CPtr.new"),
        ("DL::CPtr.to_ptr('abc')", "DL::CPtr.to_ptr"),
        ("DL.dlwrap(1)", "DL.dlwrap"),
        ("DL.dlunwrap(2)", "DL.dlunwrap"),
        ("DL.realloc(0, 8)", "DL.realloc"),
        ("DL.free(0)", "DL.free"),
    ];
    for (call, name) in calls {
        assert_eq!(
            eval(call),
            format!(
                "DL::DLError: {name}: vkxp loads no library and reads no memory of the process"
            ),
            "{call}"
        );
    }
}
