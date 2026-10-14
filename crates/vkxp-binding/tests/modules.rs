//! The scenarios of `rgss-modules`, and of the data-file and message-output requirements of
//! `rgss-runtime`: Ruby snippets evaluated against a scripted host, and what that host saw.

use vkxp_binding::testing::host::{self, Record, SOUND_BYTES, Script, with_error_stream};
use vkxp_binding::testing::{self, DOT_PIXELS};
use vkxp_proto::{
    AudioChannel, AudioCommand, BitmapDelta, Button, GuestToHost, HostToGuest, InputSnapshot,
    TransitionRule,
};

/// The value of a snippet, or its exception as `Class: message`, and what the host saw.
fn play(source: &str, script: Script) -> (String, Record) {
    let (outcome, record) = host::play(source, script);
    let outcome = match outcome {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    };
    (outcome, record)
}

fn eval(source: &str) -> (String, Record) {
    play(source, Script::new())
}

fn audio(command: AudioCommand) -> GuestToHost {
    GuestToHost::Audio(command)
}

fn sound(channel: AudioChannel, volume: u8, pitch: u16) -> GuestToHost {
    audio(AudioCommand::Play {
        channel,
        data: SOUND_BYTES.to_vec(),
        volume,
        pitch,
    })
}

// --- Graphics ---------------------------------------------------------------------------------

#[test]
fn update_sends_one_frame_per_call() {
    let (value, record) = eval("3.times { Graphics.update }");
    assert_eq!(value, "3");
    assert_eq!(record.frames().len(), 3);
    assert_eq!(record.received.len(), 3);
    assert_eq!(record.ticks, 3);
}

#[test]
fn update_counts_frames() {
    let (value, _) =
        eval("Graphics.frame_count = 0; 5.times { Graphics.update }; Graphics.frame_count");
    assert_eq!(value, "5");
    let (value, _) = eval("Graphics.frame_count = 2 ** 62; Graphics.frame_count");
    assert_eq!(value, "4611686018427387904");
}

#[test]
fn a_quit_request_raises_system_exit_in_update() {
    let (value, record) = play(
        "updates = 0
         begin
           loop { Graphics.update; updates += 1 }
         rescue SystemExit
           [:quit, updates]
         end",
        Script::new().after_frame(2, HostToGuest::Quit),
    );
    assert_eq!(value, "[:quit, 1]");
    assert_eq!(record.frames().len(), 2);
}

#[test]
fn a_reset_request_raises_reset_in_update() {
    let (value, _) = play(
        "begin
           Graphics.update
           :returned
         rescue Reset
           :reset
         end",
        Script::new().after_frame(1, HostToGuest::Reset),
    );
    assert_eq!(value, ":reset");
    // Raised once: the next frame is an ordinary one.
    let (value, _) = play(
        "first = begin
           Graphics.update
         rescue Reset
           :reset
         end
         [first, Graphics.update]",
        Script::new().after_frame(1, HostToGuest::Reset),
    );
    assert_eq!(value, "[:reset, nil]");
}

#[test]
fn frame_rate_is_clamped_and_travels_in_the_frames() {
    let (value, record) = eval(
        "default = Graphics.frame_rate
         Graphics.frame_rate = 20
         Graphics.update
         Graphics.frame_rate = 5
         low = Graphics.frame_rate
         Graphics.frame_rate = 500
         high = Graphics.frame_rate
         Graphics.frame_rate = default
         [default, low, high]",
    );
    assert_eq!(value, "[40, 10, 120]");
    assert_eq!(record.frames()[0].frame_rate.get(), 20);
}

#[test]
fn frame_reset_tells_the_host() {
    let (value, record) = eval("Graphics.frame_reset");
    assert_eq!(value, "nil");
    assert_eq!(record.received, [GuestToHost::FrameReset]);
}

#[test]
fn freeze_and_the_default_transition() {
    let (value, record) = eval("Graphics.freeze; Graphics.transition");
    assert_eq!(value, "nil");
    assert_eq!(record.received.len(), 10);
    assert_eq!(record.received[0], GuestToHost::Freeze);
    assert_eq!(
        record.received[1],
        GuestToHost::Transition {
            duration: 8,
            mask: None,
            vague: 20,
            rule: TransitionRule::Rgss1,
        }
    );
    assert_eq!(record.frames().len(), 8);
    assert_eq!(record.ticks, 8);
}

#[test]
fn a_transition_of_no_frames() {
    let (_, record) = eval("Graphics.transition(0)");
    assert_eq!(
        record.received,
        [GuestToHost::Transition {
            duration: 0,
            mask: None,
            vague: 20,
            rule: TransitionRule::Rgss1,
        }]
    );
    // A picture is not announced when no frame would carry it, and does not stay behind.
    let before = testing::bitmap_count();
    let (value, record) = eval("Graphics.transition(0, 'Graphics/Transitions/004-Blind04', 7)");
    assert_eq!(value, "nil");
    assert_eq!(
        record.received,
        [GuestToHost::Transition {
            duration: 0,
            mask: None,
            vague: 7,
            rule: TransitionRule::Rgss1,
        }]
    );
    assert_eq!(testing::bitmap_count(), before);
}

/// Scenario "Default vagueness".
#[test]
fn a_transition_without_a_vagueness_passes_20() {
    let (_, record) = eval("Graphics.transition(5, 'Graphics/Transitions/004-Blind04')");
    assert!(matches!(
        record.received[0],
        GuestToHost::Transition {
            duration: 5,
            mask: Some(_),
            vague: 20,
            rule: TransitionRule::Rgss1,
        }
    ));
}

#[test]
fn a_transition_picture_is_created_in_the_first_frame() {
    let (value, record) = eval(
        "Graphics.transition(40, 'Graphics/Transitions/004-Blind04')
         Graphics.update",
    );
    assert_eq!(value, "nil");
    let GuestToHost::Transition {
        duration: 40,
        mask: Some(mask),
        vague: 20,
        rule: TransitionRule::Rgss1,
    } = record.received[0]
    else {
        panic!("{:?} is not the transition", record.received[0]);
    };
    let frames = record.frames();
    assert_eq!(frames.len(), 41);
    let created = frames[0].bitmaps.iter().position(
        |delta| matches!(delta, BitmapDelta::Create { id, width: 2, height: 2 } if *id == mask),
    );
    let created = created.expect("the first frame creates the mask");
    assert_eq!(
        frames[0].bitmaps[created + 1],
        BitmapDelta::Update {
            id: mask,
            rect: vkxp_proto::Rect::new(0, 0, 2, 2),
            rgba: DOT_PIXELS.as_flattened().to_vec(),
        }
    );
    // The mask is gone after the last frame of the transition.
    for frame in &frames[1..40] {
        assert!(frame.bitmaps.iter().all(|delta| !refers_to(delta, mask)));
    }
    assert!(
        frames[40]
            .bitmaps
            .contains(&BitmapDelta::Destroy { id: mask })
    );
}

fn refers_to(delta: &BitmapDelta, bitmap: vkxp_proto::BitmapId) -> bool {
    match delta {
        BitmapDelta::Create { id, .. }
        | BitmapDelta::Update { id, .. }
        | BitmapDelta::Destroy { id } => *id == bitmap,
    }
}

#[test]
fn a_missing_transition_picture_raises() {
    let (value, record) = eval("Graphics.transition(10, 'Graphics/Transitions/Nope')");
    assert_eq!(
        value,
        "Errno::ENOENT: No such file or directory - Graphics/Transitions/Nope"
    );
    assert!(record.received.is_empty());
}

#[test]
fn a_quit_request_ends_a_transition() {
    let (value, record) = play(
        "begin
           Graphics.transition(20, 'Graphics/Transitions/004-Blind04')
         rescue SystemExit
           :quit
         end",
        Script::new().after_frame(3, HostToGuest::Quit),
    );
    assert_eq!(value, ":quit");
    assert_eq!(record.frames().len(), 3);
}

#[test]
fn transition_counts_its_arguments() {
    let (value, _) = eval("Graphics.transition(1, nil, 2, 3)");
    assert_eq!(value, "ArgumentError: wrong number of arguments (4 for 3)");
}

/// Scenario "Not in XP", and the same of the other functions RGSS2 adds.
#[test]
fn the_functions_of_rgss2_are_not_defined() {
    let (value, record) = eval("Graphics.brightness");
    assert!(value.starts_with("NoMethodError: "), "{value}");
    assert!(record.received.is_empty());
    let (value, record) = eval(
        "[:brightness, :brightness=, :wait, :fadeout, :fadein, :snap_to_bitmap].select do |name|
           Graphics.respond_to?(name)
         end",
    );
    assert_eq!(value, "[]");
    assert!(record.received.is_empty());
}

/// Requirement "Resizing the screen", scenario "XP".
#[test]
fn resize_screen_is_not_defined() {
    let (value, record) = eval("Graphics.resize_screen(640, 480)");
    assert!(value.starts_with("NoMethodError: "), "{value}");
    assert!(record.received.is_empty());
}

#[test]
fn visible_is_a_stored_flag() {
    let (value, _) = eval(
        "default = Graphics.visible
         Graphics.visible = false
         hidden = Graphics.visible
         Graphics.visible = true
         [default, hidden]",
    );
    assert_eq!(value, "[true, false]");
}

#[test]
fn reset_disposes_every_bitmap() {
    let (value, _) = eval(
        "class VkxpKeptBitmap < Bitmap
           def dispose
             raise 'a subclass is not asked'
           end
         end
         bitmaps = [Bitmap.new(1, 1), VkxpKeptBitmap.new(2, 2)]
         Graphics._reset
         bitmaps.map { |bitmap| bitmap.disposed? }",
    );
    assert_eq!(value, "[true, true]");
    assert_eq!(testing::bitmap_count(), 0);
}

// --- Input ------------------------------------------------------------------------------------

#[test]
fn button_constants() {
    vkxp_binding::pending_over_ruby_1_9_2!();
    let (value, _) =
        eval("[Input::DOWN, Input::LEFT, Input::RIGHT, Input::UP, Input::A, Input::C, Input::F9]");
    assert_eq!(value, "[2, 4, 6, 8, 11, 13, 29]");
    let (value, _) = eval(
        "names = %w[DOWN LEFT RIGHT UP A B C X Y Z L R SHIFT CTRL ALT F5 F6 F7 F8 F9]
         [Input.constants.sort == names.sort, names.map { |name| Input.const_get(name) }]",
    );
    assert_eq!(
        value,
        "[true, [2, 4, 6, 8, 11, 12, 13, 14, 15, 16, 17, 18, 21, 22, 23, 25, 26, 27, 28, 29]]"
    );
}

#[test]
fn a_held_button_triggers_once() {
    let (value, _) = play(
        "Input.update
         first = [Input.trigger?(Input::C), Input.press?(Input::C)]
         Input.update
         [first, [Input.trigger?(Input::C), Input.press?(Input::C)]]",
        Script::new().at_start(HostToGuest::Input(InputSnapshot::EMPTY.with(Button::C))),
    );
    assert_eq!(value, "[[true, true], [false, true]]");
}

#[test]
fn a_snapshot_received_in_update_waits_for_input_update() {
    let (value, _) = play(
        "Graphics.update
         before = Input.press?(Input::DOWN)
         Input.update
         [before, Input.press?(Input::DOWN), Input.repeat?(Input::DOWN), Input.dir4, Input.dir8]",
        Script::new().after_frame(
            1,
            HostToGuest::Input(InputSnapshot::EMPTY.with(Button::Down)),
        ),
    );
    assert_eq!(value, "[false, true, true, 2, 2]");
}

#[test]
fn an_unknown_button_is_not_pressed() {
    let (value, _) = play(
        "Input.update
         [Input.press?(99), Input.trigger?(0), Input.repeat?(-1), Input.press?(2 ** 40)]",
        Script::new().at_start(HostToGuest::Input(InputSnapshot {
            buttons: u32::MAX,
            ..InputSnapshot::EMPTY
        })),
    );
    assert_eq!(value, "[false, false, false, false]");
}

// --- Audio ------------------------------------------------------------------------------------

#[test]
fn play_sends_the_file() {
    let (value, record) = eval("Audio.se_play('Audio/SE/001-System01', 80)");
    assert_eq!(value, "nil");
    assert_eq!(record.received, [sound(AudioChannel::Se, 80, 100)]);
}

#[test]
fn volume_and_pitch_are_clamped() {
    let (_, record) = eval(
        "Audio.me_play('audio/se/001-system01.ogg', 500, 10)
         Audio.se_play('Audio/SE/001-System01', -1, 900)
         Audio.bgs_play('Audio/SE/001-System01')",
    );
    assert_eq!(
        record.received,
        [
            sound(AudioChannel::Me, 100, 50),
            sound(AudioChannel::Se, 0, 150),
            sound(AudioChannel::Bgs, 100, 100),
        ]
    );
}

#[test]
fn a_missing_sound_raises() {
    let (value, record) = eval("Audio.bgm_play('Audio/BGM/Nope')");
    assert_eq!(
        value,
        "Errno::ENOENT: No such file or directory - Audio/BGM/Nope"
    );
    assert!(record.received.is_empty());
}

#[test]
fn the_same_music_again_keeps_playing() {
    let (_, record) = eval("2.times { Audio.bgm_play('Audio/SE/001-System01', 90, 110) }");
    assert_eq!(record.received, [sound(AudioChannel::Bgm, 90, 110)]);

    // Another description plays, and so does the same one after a stop or a fade; sound
    // effects and music effects always play.
    let (_, record) = eval(
        "name = 'Audio/SE/001-System01'
         Audio.bgs_play(name)
         Audio.bgs_play(name, 50)
         Audio.bgs_play(name, 50)
         Audio.bgs_stop
         Audio.bgs_play(name, 50)
         Audio.bgs_fade(100)
         Audio.bgs_play(name, 50)
         2.times { Audio.se_play(name) }
         2.times { Audio.me_play(name) }",
    );
    let stop = audio(AudioCommand::Stop {
        channel: AudioChannel::Bgs,
    });
    let fade = audio(AudioCommand::Fade {
        channel: AudioChannel::Bgs,
        duration_ms: 100,
    });
    assert_eq!(
        record.received,
        [
            sound(AudioChannel::Bgs, 100, 100),
            sound(AudioChannel::Bgs, 50, 100),
            stop,
            sound(AudioChannel::Bgs, 50, 100),
            fade,
            sound(AudioChannel::Bgs, 50, 100),
            sound(AudioChannel::Se, 100, 100),
            sound(AudioChannel::Se, 100, 100),
            sound(AudioChannel::Me, 100, 100),
            sound(AudioChannel::Me, 100, 100),
        ]
    );
}

#[test]
fn stop_and_fade() {
    let (value, record) = eval("Audio.bgm_stop; Audio.bgm_fade(800)");
    assert_eq!(value, "nil");
    assert_eq!(
        record.received,
        [
            audio(AudioCommand::Stop {
                channel: AudioChannel::Bgm
            }),
            audio(AudioCommand::Fade {
                channel: AudioChannel::Bgm,
                duration_ms: 800
            }),
        ]
    );
    let (_, record) = eval("Audio.bgs_stop; Audio.me_stop; Audio.se_stop; Audio.me_fade(5)");
    assert_eq!(
        record.received,
        [
            audio(AudioCommand::Stop {
                channel: AudioChannel::Bgs
            }),
            audio(AudioCommand::Stop {
                channel: AudioChannel::Me
            }),
            audio(AudioCommand::Stop {
                channel: AudioChannel::Se
            }),
            audio(AudioCommand::Fade {
                channel: AudioChannel::Me,
                duration_ms: 5
            }),
        ]
    );
}

// --- Data files and message output ------------------------------------------------------------

#[test]
fn load_data_reads_a_data_file() {
    let (value, _) = eval("load_data('Data/Thing.rxdata')");
    assert_eq!(value, r#"[1, "two"]"#);
}

#[test]
fn load_data_of_a_missing_file_raises() {
    let (value, _) = eval("load_data('Data/Nope.rxdata')");
    assert_eq!(
        value,
        "Errno::ENOENT: No such file or directory - Data/Nope.rxdata"
    );
}

#[test]
fn save_data_then_load_data() {
    let source = format!(
        "Dir.chdir({}) {{ save_data({{1 => 2}}, 'Out.rxdata') }}
         load_data('Out.rxdata')",
        host::ruby_literal(&testing::game_dir().to_string_lossy())
    );
    let (value, _) = eval(&source);
    assert_eq!(value, "{1=>2}");
}

#[test]
fn print_shows_a_modal_message() {
    let (value, record) = eval(&with_error_stream("print('a', 1)"));
    assert_eq!(value, r#"[nil, "a1\n"]"#);
    // The message, and one frame with the panel before the dismissal is seen.
    assert_eq!(
        record.others(),
        [&GuestToHost::Modal {
            text: "a1".to_owned()
        }]
    );
    assert_eq!(record.frames().len(), 1);
    assert!(matches!(record.received[0], GuestToHost::Modal { .. }));
}

#[test]
fn p_shows_inspect_of_each_argument() {
    let (value, record) = eval(&with_error_stream("p 'a', nil"));
    assert_eq!(value, r#"[nil, "\"a\"\nnil\n"]"#);
    assert_eq!(
        record.others(),
        [&GuestToHost::Modal {
            text: "\"a\"\nnil".to_owned()
        }]
    );
    assert_eq!(record.frames().len(), 1);
}

#[test]
fn the_snippets_give_the_same_results_with_a_collection_at_every_allocation() {
    let (value, record) = eval(
        "GC.stress = true
         begin
           data = load_data('Data/Thing.rxdata')
           Audio.se_play('Audio/SE/001-System01')
           Graphics.transition(2, 'Graphics/Transitions/004-Blind04')
           Input.update
           missing = begin
             load_data('Data/Nope.rxdata')
           rescue Errno::ENOENT => error
             error.message
           end
           [data, missing, Graphics.frame_rate, Input.dir4, Graphics._reset]
         ensure
           GC.stress = false
         end",
    );
    assert_eq!(
        value,
        r#"[[1, "two"], "No such file or directory - Data/Nope.rxdata", 40, 0, nil]"#
    );
    assert_eq!(record.frames().len(), 2);
    assert_eq!(record.received.len(), 4);
}
