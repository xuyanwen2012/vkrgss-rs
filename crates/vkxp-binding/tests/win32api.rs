//! The emulated `Win32API` functions: the scenarios of `rgss-runtime` about them, evaluated
//! as Ruby snippets against a scripted host.

use std::fs;
use std::sync::Once;

use vkxp_binding::testing;
use vkxp_binding::testing::host::{self, Script, with_error_stream};

/// The bytes of `测试` in GBK.
const TITLE_GBK: [u8; 4] = [0xB2, 0xE2, 0xCA, 0xD4];

/// Puts a `Game.ini` whose title is in GBK into the game directory, once.
fn write_game_ini() {
    static WRITTEN: Once = Once::new();
    WRITTEN.call_once(|| {
        let mut ini =
            b"[Game]\r\nLibrary=RGSS102E.dll\r\nScripts=Data\\Scripts.rxdata\r\nTitle=".to_vec();
        ini.extend_from_slice(&TITLE_GBK);
        ini.extend_from_slice(b"\r\nCount=42\r\n");
        fs::write(testing::game_dir().join("Game.ini"), ini).unwrap();
    });
}

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

/// Ruby that reads a profile string into a buffer of 256 bytes and returns the length and
/// the bytes up to and including the terminator.
const READ_STRING: &str = "
    def read_string(section, key, default, file, size = 256)
      get = Win32API.new('kernel32', 'GetPrivateProfileStringA', 'pppplp', 'l')
      buffer = \"\\xff\" * 256
      length = get.call(section, key, default, buffer, size, file)
      [length, buffer[0, length + 1].unpack('C*')]
    end
";

#[test]
fn the_title_is_read_from_the_game_configuration_file() {
    write_game_ini();
    assert_eq!(
        eval(&format!(
            "{READ_STRING} read_string('Game', 'Title', 'none', '.\\\\Game.ini')"
        )),
        "[4, [178, 226, 202, 212, 0]]"
    );
    // Section, key and file name without regard to letter case; a value cut to the size.
    assert_eq!(
        eval(&format!(
            "{READ_STRING} [read_string('game', 'LIBRARY', '', 'game.INI'),
                            read_string('Game', 'Library', '', './Game.ini', 5)]"
        )),
        "[[12, [82, 71, 83, 83, 49, 48, 50, 69, 46, 100, 108, 108, 0]], [4, [82, 71, 83, 83, 0]]]"
    );
}

#[test]
fn a_missing_key_gives_the_default() {
    write_game_ini();
    assert_eq!(
        eval(&format!(
            "{READ_STRING} [read_string('Game', 'Absent', 'dflt', '.\\\\Game.ini'),
                            read_string('Absent', 'Title', 'dflt', '.\\\\Game.ini'),
                            read_string('Game', 'Title', 'dflt', '.\\\\Absent.ini'),
                            read_string('Game', 'Title', nil, '.\\\\Absent.ini')]"
        )),
        "[[4, [100, 102, 108, 116, 0]], [4, [100, 102, 108, 116, 0]], \
         [4, [100, 102, 108, 116, 0]], [0, [0]]]"
    );
}

#[test]
fn an_integer_value_is_read() {
    write_game_ini();
    assert_eq!(
        eval(
            "get = Win32API.new('kernel32', 'GetPrivateProfileIntA', 'ppip', 'i')
             [get.call('Game', 'Count', 7, '.\\Game.ini'), get.call('Game', 'Absent', 7, '.\\Game.ini'),
              get.call('Game', 'Title', 7, '.\\Game.ini'),
              Win32API.new('Kernel32.DLL', 'GetPrivateProfileInt', 'ppip', 'i').call('Game', 'Count', 7, 'Game.ini')]"
        ),
        "[42, 7, 0, 42]"
    );
}

#[test]
fn a_written_value_is_read_back() {
    assert_eq!(
        eval(&format!(
            "{READ_STRING}
             set = Win32API.new('kernel32', 'WritePrivateProfileStringA', 'pppp', 'l')
             written = [set.call('Options', 'Volume', '80', '.\\\\Settings.ini'),
                        set.call('Options', 'Name', 'abc', '.\\\\Settings.ini'),
                        set.call('options', 'volume', '65', '.\\\\settings.INI')]
             [written, read_string('Options', 'Volume', '', '.\\\\Settings.ini'),
              read_string('Options', 'Name', '', 'Settings.ini')]"
        )),
        "[[1, 1, 1], [2, [54, 53, 0]], [3, [97, 98, 99, 0]]]"
    );
    assert_eq!(
        fs::read(testing::game_dir().join("Settings.ini")).unwrap(),
        b"[Options]\r\nVolume=65\r\nName=abc\r\n"
    );
}

#[test]
fn nothing_is_written_outside_the_game_directory() {
    let outside = testing::game_dir().with_file_name("other.ini");
    fs::write(&outside, "[Game]\r\nTitle=outside\r\n").unwrap();
    assert_eq!(
        eval(&format!(
            "{READ_STRING}
             set = Win32API.new('kernel32', 'WritePrivateProfileStringA', 'pppp', 'l')
             files = ['C:\\\\Windows\\\\win.ini', '../other.ini', {absolute}]
             [files.map {{ |file| set.call('Game', 'Title', 'changed', file) }},
              files.map {{ |file| read_string('Game', 'Title', 'dflt', file)[0] }}]",
            absolute = host::ruby_literal(&outside.to_string_lossy())
        )),
        "[[0, 0, 0], [4, 4, 4]]"
    );
    assert_eq!(fs::read(&outside).unwrap(), b"[Game]\r\nTitle=outside\r\n");
    // A drive is not taken for a directory of the game.
    assert!(!testing::game_dir().join("C:").exists());
}

#[test]
fn text_is_converted_between_code_pages() {
    assert_eq!(
        eval(
            "to_wide = Win32API.new('kernel32', 'MultiByteToWideChar', 'ilpipi', 'i')
             to_bytes = Win32API.new('kernel32', 'WideCharToMultiByte', 'ilpipipp', 'i')
             text = '测试'
             units = to_wide.call(65001, 0, text, -1, nil, 0)
             wide = \"\\xff\" * (units * 2)
             stored = to_wide.call(65001, 0, text, -1, wide, units)
             size = to_bytes.call(936, 0, wide, -1, nil, 0, nil, nil)
             bytes = \"\\xff\" * size
             written = to_bytes.call(936, 0, wide, -1, bytes, size, nil, nil)
             [units, stored, wide.unpack('v*'), size, written, bytes.unpack('C*')]"
        ),
        "[3, 3, [27979, 35797, 0], 5, 5, [178, 226, 202, 212, 0]]"
    );
    // A counted source has no terminator; a buffer that is too small gives 0; a code page
    // outside the table gives 0.
    assert_eq!(
        eval(
            "to_wide = Win32API.new('kernel32', 'MultiByteToWideChar', 'ilpipi', 'i')
             to_bytes = Win32API.new('kernel32', 'WideCharToMultiByte', 'ilpipipp', 'i')
             gbk = [178, 226, 202, 212].pack('C*')
             wide = \"\\xff\" * 6
             short = \"\\xff\" * 2
             [to_wide.call(936, 0, gbk, 4, nil, 0), to_wide.call(936, 0, gbk, 4, wide, 3),
              wide.unpack('v*'), to_wide.call(936, 0, gbk, 4, short, 1), short.unpack('v*'),
              to_bytes.call(65001, 0, wide, 2, nil, 0, nil, nil), to_wide.call(1252, 0, gbk, 4, nil, 0)]"
        ),
        "[2, 2, [27979, 35797, 65535], 0, [27979], 6, 0]"
    );
}

#[test]
fn the_screen_size_is_the_logical_size_and_ticks_follow_the_frames() {
    assert_eq!(
        eval(
            "metrics = Win32API.new('user32', 'GetSystemMetrics', 'i', 'i')
             ticks = Win32API.new('kernel32', 'GetTickCount', '', 'l')
             time = Win32API.new('winmm.dll', 'timeGetTime', 'v', 'l')
             first = ticks.call
             same = ticks.call
             4.times { Graphics.update }
             [metrics.call(0), metrics.call(1), same - first, ticks.call - first, time.call - first]"
        ),
        "[640, 480, 0, 100, 100]"
    );
}

#[test]
fn the_game_window_has_a_handle_and_a_rectangle() {
    assert_eq!(
        eval(
            "find = Win32API.new('user32', 'FindWindowA', 'pp', 'l')
             handle = find.call('RGSS Player', nil)
             client = \"\\xff\" * 16
             window = \"\\xff\" * 16
             other = \"\\xff\" * 4
             results = [Win32API.new('user32', 'GetClientRect', 'lp', 'i').call(handle, client),
                        Win32API.new('user32', 'GetWindowRect', 'lp', 'i').call(handle, window),
                        Win32API.new('user32', 'GetClientRect', 'lp', 'i').call(0, other)]
             [handle != 0, handle == Win32API.new('user32', 'GetActiveWindow', '', 'l').call,
              handle == Win32API.new('user32', 'GetForegroundWindow', '', 'l').call,
              handle == Win32API.new('user32', 'FindWindow', 'pp', 'l').call(nil, 'Title'),
              results, client.unpack('l4'), window.unpack('l4'), other.unpack('C*')]"
        ),
        "[true, true, true, true, [1, 1, 0], [0, 0, 640, 480], [0, 0, 640, 480], \
         [255, 255, 255, 255]]"
    );
}

#[test]
fn the_cursor_keeps_its_display_count() {
    assert_eq!(
        eval(
            "show = Win32API.new('user32', 'ShowCursor', 'i', 'i')
             start = show.call(0)
             [show.call(0) - start, show.call(1) - start, show.call(1) - start]"
        ),
        "[-1, 0, 1]"
    );
}
