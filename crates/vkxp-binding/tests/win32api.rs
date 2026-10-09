//! The emulated `Win32API` functions: the scenarios of `rgss-runtime` about them, evaluated
//! as Ruby snippets against a scripted host.

use std::fs;
use std::sync::Once;

use vkxp_binding::testing;
use vkxp_binding::testing::host::{self, Script, with_error_stream};
use vkxp_proto::{HostToGuest, InputSnapshot};

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
    eval_with(source, Script::new())
}

/// The value of a snippet played against a scripted host.
fn eval_with(source: &str, script: Script) -> String {
    match host::play(source, script).0 {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

/// A host that has sent `snapshot` before the snippet starts.
fn holding(snapshot: InputSnapshot) -> Script {
    Script::new().at_start(HostToGuest::Input(snapshot))
}

/// Scenario "Held key".
#[test]
fn a_held_key_has_bit_15_and_a_key_that_is_not_held_is_zero() {
    let held = holding(InputSnapshot::EMPTY.with_key(0x0D));
    assert_eq!(
        eval_with(
            "async = Win32API.new('user32', 'GetAsyncKeyState', 'i', 'i')
             state = Win32API.new('user32.dll', 'GetKeyState', 'i', 'i')
             before = [async.call(0x0D), state.call(0x0D)]
             Input.update
             [before, async.call(0x0D) & 0x8000 == 0x8000, async.call(0x41) & 0x8000,
              state.call(0x0D) & 0x8000 == 0x8000, state.call(0x41), async.call(0x10D),
              async.call(-1), state.call(256)]",
            held
        ),
        "[[0, 0], true, 0, true, 0, 0, 0, 0]"
    );
}

#[test]
fn bit_0_of_the_asynchronous_state_is_set_at_the_update_a_key_goes_down() {
    let script = holding(InputSnapshot::EMPTY.with_key(0x41)).after_frame(
        1,
        HostToGuest::Input(InputSnapshot::EMPTY.with_key(0x41).with_key(0x01)),
    );
    assert_eq!(
        eval_with(
            "async = Win32API.new('user32', 'GetAsyncKeyState', 'i', 'i')
             state = Win32API.new('user32', 'GetKeyState', 'i', 'i')
             bits = proc { |key| [async.call(key) & 0x8000 != 0, async.call(key) & 1] }
             Input.update
             first = [bits.call(0x41), bits.call(0x41), state.call(0x41) & 1]
             Input.update
             second = bits.call(0x41)
             Graphics.update
             Input.update
             [first, second, bits.call(0x41), bits.call(0x01), bits.call(0x02)]",
            script
        ),
        "[[[true, 1], [true, 1], 0], [true, 0], [true, 0], [true, 1], [false, 0]]"
    );
}

/// Scenario "keybd_event".
#[test]
fn key_injection_is_accepted_and_leaves_the_key_state() {
    assert_eq!(
        eval_with(
            &with_error_stream(
                "async = Win32API.new('user32', 'GetAsyncKeyState', 'i', 'i')
                 get = Win32API.new('user32', 'GetKeyboardState', 'p', 'i')
                 state = proc do
                   buffer = \"\\0\" * 256
                   get.call(buffer)
                   [async.call(0x0D), async.call(0x41), buffer]
                 end
                 Input.update
                 before = state.call
                 results = [
                   Win32API.new(\"user32.dll\", \"keybd_event\", \"iiii\", \"v\").call(0x0D, 0, 0, 0),
                   Win32API.new('user32', 'keybd_event', 'iiii', 'v').call(0x41, 0, 2, 0),
                   Win32API.new('user32', 'mouse_event', 'iiiii', 'v').call(2, 0, 0, 0, 0),
                   Win32API.new('user32', 'SetKeyboardState', 'p', 'i').call(\"\\x80\" * 256)]
                 unchanged = state.call == before
                 Input.update
                 [results, unchanged, state.call == before, before[0, 2]]"
            ),
            holding(InputSnapshot::EMPTY.with_key(0x41))
        ),
        r#"[[[0, 0, 0, 1], true, false, [0, -32767]], ""]"#
    );
}

#[test]
fn the_keyboard_state_has_0x80_for_each_held_key() {
    assert_eq!(
        eval_with(
            "state = Win32API.new('user32', 'GetKeyboardState', 'p', 'i')
             buffer = \"\\xff\" * 256
             Input.update
             result = state.call(buffer)
             held = (0...256).select { |key| buffer[key] != 0 }
             short = \"\\xff\" * 0x42
             state.call(short)
             [result, buffer.size, buffer[0x41], held, short.size, short[0x41], state.call(0)]",
            holding(InputSnapshot::EMPTY.with_key(0x41))
        ),
        "[1, 256, 128, [65], 66, 128, 0]"
    );
}

/// Scenario "Pointer".
#[test]
fn the_cursor_position_is_the_pointer_in_the_frame() {
    assert_eq!(
        eval_with(
            "cursor = Win32API.new('user32', 'GetCursorPos', 'p', 'i')
             client = Win32API.new('user32', 'ScreenToClient', 'lp', 'i')
             window = Win32API.new('user32', 'GetActiveWindow', '', 'l').call
             before = \"\\0\" * 8
             cursor.call(before)
             Input.update
             point = \"\\0\" * 8
             results = [cursor.call(point), point.unpack('ll'), client.call(window, point)]
             results + [point.unpack('ll'), before.unpack('ll'), cursor.call(0)]",
            holding(InputSnapshot::EMPTY.with_pointer(Some((100, 50))))
        ),
        "[1, [100, 50], 1, [100, 50], [-1, -1], 0]"
    );
}

#[test]
fn without_a_pointer_the_cursor_position_is_minus_one() {
    assert_eq!(
        eval(
            "point = \"\\0\" * 8
             Input.update
             Win32API.new('user32', 'GetCursorPos', 'p', 'i').call(point)
             point.unpack('ll')"
        ),
        "[-1, -1]"
    );
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
            "api = Win32API.new('user32', 'GetKeyNameTextA', 'i', 'i')
             other = Win32API.new('user32.dll', 'RtlMoveMemory', 'ppl', '')
             buffer = 'ab'
             [api.call(13), api.call(13),
              Win32API.new('user32', 'GetKeyNameTextA', 'i', 'i').Call(1),
              other.call(buffer, 'XY', 2), buffer]"
        )),
        "[[0, 0, 0, 0, \"ab\"], \
         \"vkxp: Win32API user32 GetKeyNameTextA is not emulated and returns 0\\n\
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

#[cfg(unix)]
#[test]
fn a_symbolic_link_does_not_lead_out_of_the_game_directory() {
    use std::os::unix::fs::symlink;

    let game = testing::game_dir();
    let outside_file = game.with_file_name("linked.ini");
    let outside_dir = game.with_file_name("linked");
    let original = b"[Game]\r\nTitle=outside\r\n";
    fs::write(&outside_file, original).unwrap();
    fs::create_dir_all(&outside_dir).unwrap();
    fs::write(outside_dir.join("config.ini"), original).unwrap();
    symlink(&outside_file, game.join("escape.ini")).unwrap();
    symlink(&outside_dir, game.join("escape")).unwrap();
    symlink(
        game.with_file_name("dangling.ini"),
        game.join("dangling.ini"),
    )
    .unwrap();
    assert_eq!(
        eval(&format!(
            "{READ_STRING}
             set = Win32API.new('kernel32', 'WritePrivateProfileStringA', 'pppp', 'l')
             files = ['escape.ini', 'ESCAPE.INI', 'escape/config.ini', 'escape/new.ini',
                      'dangling.ini', 'escape']
             [files.map {{ |file| read_string('Game', 'Title', 'dflt', file) }}.uniq,
              files.map {{ |file| set.call('Game', 'Title', 'changed', file) }}]"
        )),
        "[[[4, [100, 102, 108, 116, 0]]], [0, 0, 0, 0, 0, 0]]"
    );
    assert_eq!(fs::read(&outside_file).unwrap(), original);
    assert_eq!(fs::read(outside_dir.join("config.ini")).unwrap(), original);
    assert!(!outside_dir.join("new.ini").exists());
    assert!(!game.with_file_name("dangling.ini").exists());
}

#[test]
fn a_file_is_created_in_a_directory_of_the_game() {
    fs::create_dir_all(testing::game_dir().join("Saves")).unwrap();
    assert_eq!(
        eval(&format!(
            "{READ_STRING}
             set = Win32API.new('kernel32', 'WritePrivateProfileStringA', 'pppp', 'l')
             [set.call('A', 'k', 'v', 'Saves\\\\Slot.ini'), set.call('A', 'k', 'w', 'saves/SLOT.ini'),
              set.call('A', 'k', 'v', 'Absent\\\\Slot.ini'),
              read_string('a', 'K', '', './Saves/../saves/slot.ini')]"
        )),
        "[1, 1, 0, [1, [119, 0]]]"
    );
    assert_eq!(
        fs::read(testing::game_dir().join("Saves/Slot.ini")).unwrap(),
        b"[A]\r\nk=w\r\n"
    );
}

/// The scenario "Plain name" of `rgss-runtime`.
#[test]
fn a_plain_name_finds_the_ansi_function() {
    write_game_ini();
    assert_eq!(
        eval(&with_error_stream(
            "get = Win32API.new('kernel32', 'GetPrivateProfileString', 'pppplp', 'l')
             buffer = \"\\xff\" * 256
             length = get.call('Game', 'Title', '', buffer, 256, '.\\\\Game.ini')
             [length, buffer[0, length + 1].unpack('C*')]"
        )),
        r#"[[4, [178, 226, 202, 212, 0]], ""]"#
    );
}

/// The scenarios "Copy inside the game directory" and "Copy outside" of `rgss-runtime`.
#[test]
fn a_file_is_copied_inside_the_game_directory_only() {
    let game = testing::game_dir();
    let outside = game.with_file_name("copied.rxdata");
    let _ = fs::remove_file(&outside);
    fs::create_dir_all(game.join("Save")).unwrap();
    fs::write(game.join("Save/a.rxdata"), b"\x04\x08save\x00\xff").unwrap();
    fs::write(game.join("Save/kept.rxdata"), b"kept").unwrap();
    assert_eq!(
        eval(&with_error_stream(&format!(
            "copy = Win32API.new('kernel32', 'CopyFileA', 'ppi', 'i')
             plain = Win32API.new('kernel32', 'CopyFile', 'ppi', 'i')
             [copy.call('Save\\\\a.rxdata', 'Save\\\\b.rxdata', 0) != 0,
              copy.call('Save\\\\a.rxdata', 'Save\\\\kept.rxdata', 1),
              plain.call('Save\\\\a.rxdata', 'Save\\\\c.rxdata', 1),
              copy.call('Save\\\\absent.rxdata', 'Save\\\\d.rxdata', 0),
              copy.call('Save\\\\a.rxdata', '..\\\\copied.rxdata', 0),
              copy.call('Save\\\\a.rxdata', {absolute}, 0),
              copy.call('Save\\\\a.rxdata', 'C:\\\\copied.rxdata', 0)]",
            absolute = host::ruby_literal(&outside.to_string_lossy())
        ))),
        r#"[[true, 0, 1, 0, 0, 0, 0], ""]"#
    );
    let source = fs::read(game.join("Save/a.rxdata")).unwrap();
    assert_eq!(fs::read(game.join("Save/b.rxdata")).unwrap(), source);
    assert_eq!(fs::read(game.join("Save/c.rxdata")).unwrap(), source);
    assert_eq!(fs::read(game.join("Save/kept.rxdata")).unwrap(), b"kept");
    assert!(!game.join("Save/d.rxdata").exists());
    assert!(!outside.exists());
}

#[test]
fn the_priority_is_accepted_and_the_process_times_are_zero() {
    assert_eq!(
        eval(&with_error_stream(
            "times = (1..4).map { \"\\xff\" * 8 }
             [Win32API.new('kernel32', 'SetPriorityClass', 'pi', 'i').call(-1, 0x80),
              Win32API.new('kernel32', 'GetProcessTimes', 'ipppp', 'i').call(-1, *times),
              times.map { |time| time.unpack('C*') }.uniq]"
        )),
        r#"[[1, 1, [[0, 0, 0, 0, 0, 0, 0, 0]]], ""]"#
    );
}
