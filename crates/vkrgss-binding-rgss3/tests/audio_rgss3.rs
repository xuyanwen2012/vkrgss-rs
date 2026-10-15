//! `Audio` in a game of RGSS3 against a scripted host: the scenarios of `rgss-modules` that
//! name RGSS3, "Functions of Audio in RGSS3", "Volume, pitch and start of a play command of
//! RGSS3", "The same music again in RGSS3", "Position of the music and of the background
//! sound" and "A music effect holds the position of the music". A file of this package
//! alone, because `Audio` is defined once per process, as the runtime of the run has it, and
//! RGSS3 runs over Ruby 1.9.2 only.
//!
//! The values are those measured on the original runtime of RGSS3 on 2026-10-13
//! (`ace-v1-api-REPORT.md` sections 6.1 to 6.3, kept outside the repository). The sounds
//! are WAV files of silence that this file writes into the game directory of the harness.
#![cfg(not(vkrgss_interpreter_absent))]

use std::sync::Once;

use vkrgss_binding_rgss3::testing::host::{self, Record, Script};
use vkrgss_binding_rgss3::testing::{self, Harness};
use vkrgss_proto::{AudioChannel, AudioCommand, GuestToHost};
use vkrgss_vfs::Runtime;

use AudioChannel::{Bgm, Bgs, Me, Se};

/// The harness, started as a game of RGSS3.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss3)
}

/// The music of the measurements: 20 s at 44100 Hz in stereo, streamed.
const S44: &str = "Audio/BGM/s44";
/// Music effects of 2 s and of 5 s at 44100 Hz in stereo.
const ME2: &str = "Audio/ME/me2";
const ME5: &str = "Audio/ME/me5";
/// A sound effect of 1 s at 44100 Hz in stereo.
const SE44: &str = "Audio/SE/se44";
/// A sound effect of 1 s at 22050 Hz in mono.
const SE22: &str = "Audio/SE/se22";
/// A file that begins with the header of a Standard MIDI file, one of text, an empty file.
const MIDI: &str = "Audio/BGM/midi";
const TEXT: &str = "Audio/BGM/text.txt";
const EMPTY: &str = "Audio/BGM/empty.ogg";

/// A WAV file of silence: 16-bit PCM of `frames` sample frames.
fn wav(rate: u32, channels: u16, frames: u32) -> Vec<u8> {
    let block = channels * 2;
    let data = frames * u32::from(block);
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&(36 + data).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt \x10\0\0\0\x01\0");
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * u32::from(block)).to_le_bytes());
    bytes.extend_from_slice(&block.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data.to_le_bytes());
    bytes.resize(bytes.len() + data as usize, 0);
    bytes
}

/// Starts the harness as RGSS3 and writes the sounds, once.
fn setup() {
    static WRITTEN: Once = Once::new();
    WRITTEN.call_once(|| {
        let game = harness().game_dir();
        let files: [(&str, Vec<u8>); 8] = [
            ("Audio/BGM/s44.wav", wav(44100, 2, 44100 * 20)),
            ("Audio/ME/me2.wav", wav(44100, 2, 44100 * 2)),
            ("Audio/ME/me5.wav", wav(44100, 2, 44100 * 5)),
            ("Audio/SE/se44.wav", wav(44100, 2, 44100)),
            ("Audio/SE/se22.wav", wav(22050, 1, 22050)),
            (
                "Audio/BGM/midi.mid",
                b"MThd\0\0\0\x06\0\0\0\x01\x01\xe0MTrk\0\0\0\x04\0\xff\x2f\0".to_vec(),
            ),
            (TEXT, b"This is no sound.\r\n".to_vec()),
            (EMPTY, Vec::new()),
        ];
        for (name, bytes) in files {
            let path = game.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, bytes).unwrap();
        }
    });
}

/// The value of `source`, or its exception as `Class: message`, and what the host saw.
fn play(source: &str, script: Script) -> (String, Record) {
    setup();
    let (outcome, record) = host::play(harness(), source, script);
    let outcome = match outcome {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    };
    (outcome, record)
}

fn eval(source: &str) -> (String, Record) {
    play(source, Script::new())
}

/// The audio commands the host received, the data of play commands left out.
fn commands(record: &Record) -> Vec<AudioCommand> {
    record
        .received
        .iter()
        .filter_map(|message| match message {
            GuestToHost::Audio(AudioCommand::Play {
                channel,
                volume,
                pitch,
                start,
                music_return_ms,
                ..
            }) => Some(AudioCommand::Play {
                channel: *channel,
                data: Vec::new(),
                volume: *volume,
                pitch: *pitch,
                start: *start,
                music_return_ms: *music_return_ms,
            }),
            GuestToHost::Audio(command) => Some(command.clone()),
            _ => None,
        })
        .collect()
}

/// A play command without its data.
fn sound(channel: AudioChannel, volume: u8, pitch: u16, start: u64) -> AudioCommand {
    AudioCommand::Play {
        channel,
        data: Vec::new(),
        volume,
        pitch,
        start,
        music_return_ms: if channel == Me { 1000 } else { 0 },
    }
}

fn volume(channel: AudioChannel, volume: u8) -> AudioCommand {
    AudioCommand::Volume { channel, volume }
}

/// `source` after `name = name`.
fn with_names(source: &str, name: &str) -> String {
    format!("name = '{name}'\n{source}")
}

// --- Functions of Audio in RGSS3 --------------------------------------------------------------

/// Scenario "Surface, measured on the original".
#[test]
fn the_functions_and_their_arities() {
    let (value, _) = eval(
        "(Audio.singleton_methods - [:__reset__]).sort.map { |m| [m, Audio.method(m).arity] }",
    );
    assert_eq!(
        value,
        "[[:bgm_fade, 1], [:bgm_play, -1], [:bgm_pos, 0], [:bgm_stop, 0], [:bgs_fade, 1], \
         [:bgs_play, -1], [:bgs_pos, 0], [:bgs_stop, 0], [:me_fade, 1], [:me_play, -1], \
         [:me_stop, 0], [:se_play, -1], [:se_stop, 0], [:setup_midi, 0]]"
    );
}

/// Scenario "Four arguments, measured on the original".
#[test]
fn music_and_background_sound_take_four_arguments() {
    let (value, record) = eval(&with_names(
        "[Audio.bgm_play(name, 80, 100, 0), Audio.bgs_play(name, 80, 100, 0)]",
        SE44,
    ));
    assert_eq!(value, "[nil, nil]");
    assert_eq!(
        commands(&record),
        [sound(Bgm, 80, 100, 0), sound(Bgs, 80, 100, 0)]
    );
    let file = std::fs::read(harness().game_dir().join("Audio/SE/se44.wav")).unwrap();
    for message in &record.received {
        assert!(
            matches!(message, GuestToHost::Audio(AudioCommand::Play { data, .. }) if *data == file)
        );
    }
}

/// Scenario "Before anything plays, measured on the original".
#[test]
fn nothing_plays_at_first() {
    let (value, record) =
        eval("[Audio.bgm_pos, Audio.bgs_pos, Audio.setup_midi, Audio.setup_midi]");
    assert_eq!(value, "[0, 0, nil, nil]");
    assert!(commands(&record).is_empty());
}

/// Scenario "Counts, measured on the original".
#[test]
fn counts_of_arguments() {
    let calls = [
        ("Audio.bgm_play", "(1 for 0)"),
        ("Audio.bgs_play", "(1 for 0)"),
        ("Audio.bgm_play(name, 1, 2, 3, 4)", "(4 for 5)"),
        ("Audio.bgs_play(name, 1, 2, 3, 4)", "(4 for 5)"),
        ("Audio.me_play(name, 1, 2, 3)", "(3 for 4)"),
        ("Audio.se_play(name, 1, 2, 3)", "(3 for 4)"),
    ];
    for (call, counts) in calls {
        let (value, record) = eval(&with_names(call, SE44));
        assert_eq!(
            value,
            format!("ArgumentError: wrong number of arguments {counts}"),
            "{call}"
        );
        assert!(commands(&record).is_empty());
    }
}

/// Scenario "Kinds of arguments, measured on the original".
#[test]
fn kinds_of_arguments() {
    let calls = [
        (
            "Audio.bgm_play(name, nil)",
            "TypeError: no implicit conversion from nil to integer",
        ),
        (
            "Audio.bgm_play(name, 'a')",
            "TypeError: can't convert String into Integer",
        ),
        (
            "Audio.bgm_play(name, 100, true)",
            "TypeError: can't convert true into Integer",
        ),
        ("Audio.bgm_play(name, 100, 100, 'a')", "TypeError"),
        ("Audio.bgm_play(name, 100, 100, 2**31)", "RangeError"),
    ];
    for (call, raised) in calls {
        let (value, record) = eval(&with_names(call, SE44));
        assert!(value.starts_with(raised), "{call}: {value}");
        assert!(commands(&record).is_empty());
    }
}

/// Scenario "Forms that are accepted, measured on the original".
#[test]
fn floats_and_a_pos_of_nil() {
    let (value, record) = eval(&with_names(
        "[Audio.bgm_play(name, 99.9, 100.7, nil), Audio.bgs_play(name, 100, 100, 44100.7)]",
        SE44,
    ));
    assert_eq!(value, "[nil, nil]");
    assert_eq!(
        commands(&record),
        [sound(Bgm, 99, 100, 0), sound(Bgs, 100, 100, 11025)]
    );
}

/// Scenario "Missing file while a music plays, measured on the original".
#[test]
fn a_missing_file_leaves_the_music() {
    let (value, record) = eval(&with_names(
        "Audio.bgm_play(name)
         60.times { Graphics.update }
         before = Audio.bgm_pos
         error = begin
           Audio.bgm_play('Audio/BGM/Nope')
         rescue => e
           [e.class, e.message]
         end
         12.times { Graphics.update }
         [error, Audio.bgm_pos > before]",
        S44,
    ));
    assert_eq!(
        value,
        r#"[[Errno::ENOENT, "No such file or directory - Audio/BGM/Nope"], true]"#
    );
    assert_eq!(commands(&record), [sound(Bgm, 100, 100, 0)]);
}

// --- Volume, pitch and start of a play command of RGSS3 ---------------------------------------

/// Scenario "Volume, measured on the original".
#[test]
fn volumes_of_rgss3() {
    let (_, record) = eval(&with_names(
        "[-1000, -1, 0, 1, 99, 100, 101, 2147483647].each { |v| Audio.se_play(name, v) }",
        SE44,
    ));
    let volumes = [100, 100, 0, 1, 99, 100, 100, 100];
    assert_eq!(
        commands(&record),
        volumes.map(|volume| sound(Se, volume, 100, 0))
    );
}

/// Scenario "Pitch, measured on the original".
#[test]
fn pitches_of_rgss3() {
    let (_, record) = eval(&format!(
        "[-1, 0, 1, 49, 151, 200, 453, 454, 1000].each {{ |p| Audio.se_play('{SE44}', 100, p) }}
         [907, 1000].each {{ |p| Audio.se_play('{SE22}', 100, p) }}"
    ));
    let pitches = [100, 100, 1, 49, 151, 200, 453, 100, 100, 907, 100];
    assert_eq!(
        commands(&record),
        pitches.map(|pitch| sound(Se, 100, pitch, 0))
    );
}

/// Scenario "Start".
#[test]
fn starts_of_rgss3() {
    let (_, record) = eval(&format!(
        "[0, 3, 4, 100001, -44100].each {{ |pos|
           Audio.bgm_stop
           Audio.bgm_play('{S44}', 100, 100, pos)
         }}
         Audio.bgm_play('{SE22}', 100, 100, 22050)"
    ));
    let starts: Vec<AudioCommand> = commands(&record)
        .into_iter()
        .filter(|command| matches!(command, AudioCommand::Play { .. }))
        .collect();
    let expected = [0, 0, 1, 25000, 0, 11025].map(|start| sound(Bgm, 100, 100, start));
    assert_eq!(starts, expected);
}

/// Scenario "A file whose format is not known".
#[test]
fn a_file_whose_format_is_not_known_keeps_its_pitch_and_starts_at_0() {
    let (_, record) = eval(&with_names("Audio.bgm_play(name, 100, 1000, 100000)", MIDI));
    assert_eq!(commands(&record), [sound(Bgm, 100, 1000, 0)]);
}

/// Scenario "Return time".
#[test]
fn a_music_effect_carries_the_return_time() {
    let (_, record) = eval(&with_names(
        "Audio.me_play(name); Audio.bgm_play(name); Audio.se_play(name)",
        SE44,
    ));
    let returns: Vec<u32> = commands(&record)
        .iter()
        .map(|command| match command {
            AudioCommand::Play {
                music_return_ms, ..
            } => *music_return_ms,
            _ => panic!("{command:?}"),
        })
        .collect();
    assert_eq!(returns, [1000, 0, 0]);
}

// --- The same music again in RGSS3 ------------------------------------------------------------

/// `Audio.bgm_pos` after `then` follows the music of the measurements and 60 frames, and the
/// audio commands the host received.
fn after_60_frames(then: &str) -> (String, Vec<AudioCommand>) {
    let (value, record) = eval(&with_names(
        &format!("Audio.bgm_play(name)\n60.times {{ Graphics.update }}\n{then}\nAudio.bgm_pos"),
        S44,
    ));
    (value, commands(&record))
}

/// Scenario "Same again, measured on the original".
#[test]
fn the_same_music_again_goes_on() {
    let (value, sent) = after_60_frames("Audio.bgm_play(name, 100, 100, 300000)");
    assert_eq!(value, "132304");
    assert_eq!(sent, [sound(Bgm, 100, 100, 0)]);
}

/// Scenario "Another volume, measured on the original".
#[test]
fn another_volume_is_a_volume_command() {
    let (value, sent) = after_60_frames("Audio.bgm_play(name, 80)");
    assert_eq!(value, "132304");
    assert_eq!(sent, [sound(Bgm, 100, 100, 0), volume(Bgm, 80)]);
}

/// Scenario "Another pitch, measured on the original".
#[test]
fn another_pitch_starts_again() {
    let (value, sent) = after_60_frames("Audio.bgm_play(name, 100, 150)");
    assert_eq!(value, "0");
    assert_eq!(sent, [sound(Bgm, 100, 100, 0), sound(Bgm, 100, 150, 0)]);
}

/// Scenario "Another spelling of the name, measured on the original".
#[test]
fn another_spelling_starts_again() {
    let (value, sent) = after_60_frames("Audio.bgm_play(name.upcase)");
    assert_eq!(value, "0");
    assert_eq!(sent, [sound(Bgm, 100, 100, 0), sound(Bgm, 100, 100, 0)]);
}

/// Scenario "After a fade, measured on the original".
#[test]
fn after_a_fade_the_music_starts_again() {
    let (value, sent) = after_60_frames(
        "Audio.bgm_fade(1000)
         120.times { Graphics.update }
         Audio.bgm_play(name)",
    );
    assert_eq!(value, "0");
    let fade = AudioCommand::Fade {
        channel: Bgm,
        duration_ms: 1000,
    };
    assert_eq!(
        sent,
        [sound(Bgm, 100, 100, 0), fade, sound(Bgm, 100, 100, 0)]
    );
}

/// Scenario "Background sound".
#[test]
fn the_same_background_sound_again() {
    let (_, record) = eval(&with_names(
        "Audio.bgs_play(name, 100); Audio.bgs_play(name, 50); Audio.bgs_play(name, 50)",
        S44,
    ));
    assert_eq!(
        commands(&record),
        [sound(Bgs, 100, 100, 0), volume(Bgs, 50)]
    );
}

// --- Position of the music and of the background sound ----------------------------------------

/// Scenario "At once, measured on the original".
#[test]
fn the_position_at_once() {
    let (value, _) = eval(&with_names(
        "[0, 1, 44100, 100000, 100001].map { |p|
           Audio.bgm_stop
           Audio.bgm_play(name, 0, 100, p)
           Audio.bgm_pos
         }",
        S44,
    ));
    assert_eq!(value, "[0, 0, 11024, 66924, 66925]");
}

/// Scenario "Every sixth frame, measured on the original".
#[test]
fn the_position_every_sixth_frame() {
    let (value, _) = eval(&with_names(
        "Audio.bgm_play(name)
         readings = [Audio.bgm_pos]
         14.times { 6.times { Graphics.update }; readings << Audio.bgm_pos }
         readings",
        S44,
    ));
    assert_eq!(
        value,
        "[0, 0, 0, 0, 33076, 33076, 66152, 66152, 99228, 99228, 132304, 132304, 165380, \
         165380, 198456]"
    );
}

/// Scenario "Background sound and both together, measured on the original".
#[test]
fn the_music_and_the_background_sound_together() {
    let (value, _) = eval(&with_names(
        "Audio.bgm_play(name); Audio.bgs_play(name)
         60.times { Graphics.update }
         [Audio.bgm_pos, Audio.bgs_pos]",
        S44,
    ));
    assert_eq!(value, "[132304, 132304]");
}

/// Scenario "Frames of a wait".
#[test]
fn the_frames_of_a_wait_count() {
    let (value, _) = eval(&with_names(
        "Audio.bgm_play(name); Graphics.wait(60); Audio.bgm_pos",
        S44,
    ));
    assert_eq!(value, "132304");
}

/// Scenario "No frame, no step".
#[test]
fn without_a_frame_the_position_stands() {
    let (value, _) = eval(&with_names(
        "Audio.bgm_play(name)
         30.times { Graphics.update }
         readings = (1..30).map { |i|
           Graphics.frame_count = 1000 if i == 15
           Audio.bgm_pos
         }
         [readings.uniq.size, readings.first]",
        S44,
    ));
    assert_eq!(value, "[1, 33076]");
}

/// Scenario "Frames of a modal message".
#[test]
fn the_frames_of_a_message_do_not_count() {
    let (value, record) = play(
        &with_names(
            "Audio.bgm_play(name)
             60.times { Graphics.update }
             msgbox('stop')
             Audio.bgm_pos",
            S44,
        ),
        Script::new().dismiss_after(30),
    );
    assert_eq!(value, "132304");
    assert_eq!(record.frames().len(), 90);
}

/// Scenario "After a stop, measured on the original".
#[test]
fn a_stop_keeps_the_position() {
    let (value, _) = eval(&with_names(
        "Audio.bgm_play(name)
         24.times { Graphics.update }
         Audio.bgm_stop
         at_once = Audio.bgm_pos
         18.times { Graphics.update }
         [at_once, Audio.bgm_pos]",
        S44,
    ));
    assert_eq!(value, "[33076, 33076]");
}

/// Scenario "During a fade, measured on the original".
#[test]
fn the_position_goes_on_during_a_fade() {
    let (value, _) = eval(&with_names(
        "Audio.bgm_play(name)
         Audio.bgm_fade(1000)
         60.times { Graphics.update }
         Audio.bgm_pos",
        S44,
    ));
    assert_eq!(value, "132304");
}

/// Scenario "Formats without a position, measured on the original".
#[test]
fn formats_without_a_position() {
    let (value, record) = eval(&format!(
        "['{MIDI}', '{TEXT}', '{EMPTY}'].map {{ |name|
           played = Audio.bgm_play(name, 100, 100, 100000)
           60.times {{ Graphics.update }}
           [played, Audio.bgm_pos]
         }}"
    ));
    assert_eq!(value, "[[nil, 0], [nil, 0], [nil, 0]]");
    assert_eq!(commands(&record).len(), 3);
}

/// Scenario "A file that is no sound after a music, measured on the original".
#[test]
fn a_file_that_is_no_sound_after_a_music() {
    let (value, _) = eval(&format!(
        "Audio.bgm_play('{S44}')
         60.times {{ Graphics.update }}
         Audio.bgm_play('{TEXT}')
         Audio.bgm_pos"
    ));
    assert_eq!(value, "0");
}

// --- A music effect holds the position of the music -------------------------------------------

/// `Audio.bgm_pos` after the music of the measurements, 60 frames and `then`, read at each
/// count of further frames.
fn music_then(then: &str, readings: &[u32]) -> String {
    let (value, _) = eval(&format!(
        "Audio.bgm_play('{S44}')
         60.times {{ Graphics.update }}
         {then}
         shown = 0
         {readings:?}.map {{ |frames|
           (frames - shown).times {{ Graphics.update }}
           shown = frames
           Audio.bgm_pos
         }}"
    ));
    value
}

/// Scenario "Two seconds, measured on the original".
#[test]
fn an_effect_of_two_seconds_holds_the_music() {
    let value = music_then(&format!("Audio.me_play('{ME2}')"), &[60, 120, 180]);
    assert_eq!(value, "[132304, 132304, 297684]");
}

/// Scenario "The background sound goes on, measured on the original".
#[test]
fn an_effect_does_not_hold_the_background_sound() {
    let (value, _) = eval(&format!(
        "Audio.bgs_play('{S44}')
         60.times {{ Graphics.update }}
         Audio.me_play('{ME2}')
         60.times {{ Graphics.update }}
         Audio.bgs_pos"
    ));
    assert_eq!(value, "297684");
}

/// Scenario "Stopped effect, measured on the original".
#[test]
fn a_stop_of_the_effect_ends_the_hold() {
    let value = music_then(
        &format!(
            "Audio.me_play('{ME5}')
             60.times {{ Graphics.update }}
             Audio.me_stop"
        ),
        &[60],
    );
    assert_eq!(value, "[297684]");
}

/// Scenario "Faded effect, measured on the original".
#[test]
fn a_fade_of_the_effect_does_not_end_the_hold() {
    let value = music_then(
        &format!(
            "Audio.me_play('{ME5}')
             60.times {{ Graphics.update }}
             Audio.me_fade(500)"
        ),
        &[180, 300],
    );
    assert_eq!(value, "[132304, 297684]");
}

/// Scenario "Half the pitch".
#[test]
fn an_effect_at_half_the_pitch_holds_twice_as_long() {
    let value = music_then(&format!("Audio.me_play('{ME2}', 100, 50)"), &[240, 300]);
    assert_eq!(value, "[132304, 297684]");
}

/// Scenario "An effect whose format is not known".
#[test]
fn an_effect_whose_format_is_not_known_holds_nothing() {
    let value = music_then(&format!("Audio.me_play('{MIDI}')"), &[60]);
    assert_eq!(value, "[297684]");
}

/// `Audio.__reset__` stops the four channels: both positions keep their values, and a play
/// after it starts at its `pos` (`ace-v1-api-REPORT.md` section 6.2, "reset" samples).
#[test]
fn a_reset_freezes_both_positions() {
    let (value, _) = eval(&with_names(
        "Audio.bgm_play(name); Audio.bgs_play(name)
         24.times { Graphics.update }
         Audio.__reset__
         at_once = [Audio.bgm_pos, Audio.bgs_pos]
         18.times { Graphics.update }
         later = [Audio.bgm_pos, Audio.bgs_pos]
         Audio.bgm_play(name, 100, 100, 100000)
         [at_once, later, Audio.bgm_pos]",
        S44,
    ));
    assert_eq!(value, "[[33076, 33076], [33076, 33076], 66924]");
}
