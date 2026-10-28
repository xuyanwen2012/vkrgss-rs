//! Tests of the emulated functions and of `Win32API` in a game of RGSS1.

use super::profile::{inside_game, profile_lookup, profile_set};
use super::text::{detect, encode};
use super::*;
use crate::testing::{self, Harness};

fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss1)
}

#[test]
fn a_profile_is_read_as_the_configuration_file_is() {
    let file = b"\xEF\xBB\xBF; note\r\n[Other]\r\nTitle=no\r\n[ game ]\r\n title = A=B \r\n\
                 Title=later\r\nQuoted=\" x \"\r\nEmpty=\r\n";
    assert_eq!(profile_lookup(file, b"Game", b"TITLE"), Some(&b"A=B"[..]));
    assert_eq!(profile_lookup(file, b"Game", b"quoted"), Some(&b" x "[..]));
    assert_eq!(profile_lookup(file, b"Game", b"Empty"), Some(&b""[..]));
    assert_eq!(profile_lookup(file, b"Game", b"note"), None);
    assert_eq!(profile_lookup(file, b"Absent", b"Title"), None);
}

#[test]
fn a_profile_is_changed_where_the_key_belongs() {
    let set = |contents: &str, key: &str| {
        String::from_utf8(profile_set(contents.as_bytes(), b"b", key.as_bytes(), b"9")).unwrap()
    };
    assert_eq!(set("", "k"), "[b]\r\nk=9\r\n");
    assert_eq!(set("[a]\nx=1", "k"), "[a]\nx=1\n[b]\nk=9\n");
    assert_eq!(
        set("[B]\r\nx=1\r\nK = 2\r\n[c]\r\nk=3\r\n", "k"),
        "[B]\r\nx=1\r\nK=9\r\n[c]\r\nk=3\r\n"
    );
    assert_eq!(
        set("[b]\nx=1\n\n[c]\ny=2\n", "k"),
        "[b]\nx=1\nk=9\n\n[c]\ny=2\n"
    );
    assert_eq!(set("[b]\nx=1", "k"), "[b]\nx=1\nk=9\n");
    assert_eq!(set("\u{feff}[b]\n", "k"), "\u{feff}[b]\nk=9\n");
}

#[test]
fn the_code_page_of_a_game_is_the_encoding_of_its_configuration_file() {
    assert_eq!(detect(b""), encoding_rs::UTF_8);
    assert_eq!(detect("Title=测试".as_bytes()), encoding_rs::UTF_8);
    assert_eq!(detect(b"Title=\xB2\xE2\xCA\xD4"), encoding_rs::GBK);
    // Shift-JIS text that is not valid GBK: a lone half-width katakana.
    assert_eq!(detect(b"Title=\x83\x65\xB1"), encoding_rs::SHIFT_JIS);
}

#[test]
fn what_an_encoding_does_not_have_becomes_a_question_mark() {
    assert_eq!(encode(encoding_rs::GBK, "a测b"), b"a\xB2\xE2b");
    assert_eq!(
        encode(encoding_rs::SHIFT_JIS, "a\u{1F600}測"),
        b"a?\x91\xAA"
    );
}

#[test]
fn ticks_follow_the_frames_at_their_rate() {
    let mut state = State::default();
    for _ in 0..40 {
        state.frame_shown(40);
    }
    assert_eq!(state.milliseconds, 1000);
    for _ in 0..3 {
        state.frame_shown(60);
    }
    assert_eq!(state.milliseconds, 1050);
}

/// The scenario "XP and VX" of "DL of RGSS3".
#[test]
fn a_game_of_rgss1_has_no_dl() {
    let (value, _) = testing::host::play(harness(), "defined?(DL)", testing::host::Script::new());
    assert_eq!(value.map_err(|error| error.message).as_deref(), Ok("nil"));
}

#[test]
fn a_profile_name_stays_inside_the_game_directory() {
    let inside = |name: &str| inside_game(name.as_bytes()).map(|parts| parts.join("/"));
    assert_eq!(inside(".\\Game.ini").as_deref(), Some("Game.ini"));
    assert_eq!(inside("Data/../save.ini").as_deref(), Some("save.ini"));
    assert_eq!(
        inside("Data\\Config.ini").as_deref(),
        Some("Data/Config.ini")
    );
    for outside in [
        "C:\\Windows\\win.ini",
        "c:win.ini",
        "../other.ini",
        "/etc/x.ini",
        "\\\\host\\x.ini",
        "a/../../x.ini",
        ".",
        "",
    ] {
        assert_eq!(inside(outside), None, "{outside}");
    }
}
