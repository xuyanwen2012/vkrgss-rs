//! The functions of `Graphics` in a game of RGSS2: Ruby snippets evaluated against a scripted
//! host, and what that host saw. A binary of its own, because `Graphics` is defined once, as
//! the runtime the harness starts with has it.

mod scene_support;

use vkxp_binding::testing;
use vkxp_binding::testing::host::{self, Record, Script};
use vkxp_proto::{BitmapDelta, GuestToHost, Rect, Size, TransitionRule};
use vkxp_vfs::Runtime;

/// The value of a snippet, or its exception as `Class: message`, and what the host saw. The
/// brightness is 255 and nothing is frozen when the snippet starts. The scene is shared by the
/// tests, so a snippet disposes what it made before it ends.
fn eval(source: &str) -> (String, Record) {
    testing::start_as(Runtime::Rgss2);
    let source = format!("Graphics.brightness = 255\n{source}");
    let (outcome, record) = host::play(&source, Script::new());
    let outcome = match outcome {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    };
    (outcome, record)
}

/// The brightness of every frame the host received.
fn levels(record: &Record) -> Vec<u8> {
    let frames = record.frames();
    frames.iter().map(|frame| frame.brightness).collect()
}

// --- Brightness of RGSS2 ----------------------------------------------------------------------

/// Scenario "Limits, measured on the original".
#[test]
fn the_brightness_starts_at_255_and_is_limited() {
    let (value, record) = eval(
        "start = Graphics.brightness
         Graphics.brightness = -5
         low = Graphics.brightness
         Graphics.brightness = 300
         [start, low, Graphics.brightness]",
    );
    assert_eq!(value, "[255, 0, 255]");
    assert!(record.received.is_empty());
}

/// Scenario "In the packet".
#[test]
fn the_frames_carry_the_brightness() {
    let (_, record) = eval("Graphics.brightness = 100; Graphics.update; Graphics.update");
    assert_eq!(levels(&record), [100, 100]);
}

// --- Waits and fades of RGSS2 -----------------------------------------------------------------

/// Scenario "Wait, measured on the original".
#[test]
fn wait_shows_its_frames_and_none_for_a_count_below_1() {
    let (value, record) = eval(
        "Graphics.frame_count = 0
         waited = [Graphics.wait(7), Graphics.wait(0), Graphics.wait(-3)]
         [waited, Graphics.frame_count]",
    );
    assert_eq!(value, "[[nil, nil, nil], 7]");
    assert_eq!(record.frames().len(), 7);
    assert_eq!(record.received.len(), 7);
    assert_eq!(record.ticks, 7);
}

/// Scenario "Fade out, measured on the original".
#[test]
fn fadeout_steps_down_to_0() {
    let (value, record) = eval(
        "Graphics.brightness = 200
         [Graphics.fadeout(3), Graphics.brightness]",
    );
    assert_eq!(value, "[nil, 0]");
    assert_eq!(levels(&record), [134, 67, 0]);
}

/// Scenario "Fade in, measured on the original".
#[test]
fn fadein_steps_up_to_255() {
    let (value, record) = eval(
        "Graphics.brightness = 100
         [Graphics.fadein(4), Graphics.brightness]",
    );
    assert_eq!(value, "[nil, 255]");
    assert_eq!(levels(&record), [138, 177, 216, 255]);
}

/// Scenario "No frame". A count below 0 is taken as 0, which was not measured.
#[test]
fn a_fade_without_a_frame_leaves_its_end_value() {
    let (value, record) = eval(
        "Graphics.fadeout(0)
         out = Graphics.brightness
         Graphics.fadein(0)
         back = Graphics.brightness
         Graphics.fadeout(-2)
         [out, back, Graphics.brightness]",
    );
    assert_eq!(value, "[0, 255, 0]");
    assert!(record.received.is_empty());
}

/// Scenario "A redefined update is not called".
#[test]
fn waits_and_fades_do_not_call_a_redefined_update() {
    let (value, record) = eval(
        "$vkxp_updates = 0
         class << Graphics
           alias vkxp_update update
           def update
             $vkxp_updates += 1
             vkxp_update
           end
         end
         begin
           Graphics.wait(5)
           Graphics.fadeout(5)
         ensure
           class << Graphics
             alias update vkxp_update
           end
         end
         $vkxp_updates",
    );
    assert_eq!(value, "0");
    assert_eq!(record.frames().len(), 10);
}

// --- Transition of RGSS2 ----------------------------------------------------------------------

/// Scenario "Default, measured on the original".
#[test]
fn freeze_and_the_default_transition() {
    let (value, record) = eval("Graphics.freeze; Graphics.transition");
    assert_eq!(value, "nil");
    assert_eq!(record.received.len(), 12);
    assert_eq!(record.received[0], GuestToHost::Freeze);
    assert_eq!(
        record.received[1],
        GuestToHost::Transition {
            duration: 10,
            mask: None,
            vague: 20,
            rule: TransitionRule::Rgss2,
        }
    );
    assert_eq!(record.frames().len(), 10);
    assert_eq!(record.ticks, 10);
}

/// Scenario "Nothing frozen, measured on the original". The picture is not looked for
/// either, and the transition that ended leaves nothing frozen.
#[test]
fn a_transition_with_nothing_frozen_does_nothing() {
    let (value, record) = eval(
        "Graphics.brightness = 100
         count = Graphics.frame_count
         shown = [Graphics.transition(20), Graphics.transition(20, 'Graphics/Transitions/Nope')]
         [shown, Graphics.frame_count - count, Graphics.brightness]",
    );
    assert_eq!(value, "[[nil, nil], 0, 100]");
    assert!(record.received.is_empty());

    let (value, record) = eval(
        "Graphics.freeze
         Graphics.transition(2)
         Graphics.brightness = 100
         Graphics.transition(3)
         Graphics.brightness",
    );
    assert_eq!(value, "100");
    assert_eq!(record.frames().len(), 2);
    assert_eq!(record.others().len(), 2);
}

/// Scenario "Brightness after a freeze, measured on the original".
#[test]
fn a_transition_after_a_freeze_sets_the_brightness_to_255() {
    let (value, record) = eval(
        "Graphics.fadeout(1)
         Graphics.freeze
         Graphics.transition(0)
         Graphics.brightness",
    );
    assert_eq!(value, "255");
    assert_eq!(levels(&record), [0]);
    assert_eq!(
        record.received[1..],
        [
            GuestToHost::Freeze,
            GuestToHost::Transition {
                duration: 0,
                mask: None,
                vague: 20,
                rule: TransitionRule::Rgss2,
            }
        ]
    );
}

/// The frames of a transition carry the brightness it set.
#[test]
fn the_frames_of_a_transition_are_at_full_brightness() {
    let (_, record) = eval(
        "Graphics.brightness = 40
         Graphics.freeze
         Graphics.transition(3, 'Graphics/Transitions/004-Blind04')",
    );
    assert_eq!(levels(&record), [255, 255, 255]);
    assert!(matches!(
        record.received[1],
        GuestToHost::Transition {
            duration: 3,
            mask: Some(_),
            vague: 20,
            rule: TransitionRule::Rgss2,
        }
    ));
}

// --- Snapshot of the scene --------------------------------------------------------------------

/// Scenario "Brightness, measured on the original".
#[test]
fn a_snapshot_has_the_size_of_the_frame_and_is_darkened_by_the_brightness() {
    let (value, record) = eval(
        "bitmap = Bitmap.new(544, 416)
         bitmap.fill_rect(0, 0, 544, 416, Color.new(200, 200, 200))
         sprite = Sprite.new
         sprite.bitmap = bitmap
         Graphics.brightness = 100
         count = Graphics.frame_count
         shot = Graphics.snap_to_bitmap
         colors = [shot.get_pixel(0, 0), shot.get_pixel(543, 415)].map do |color|
           [color.red, color.green, color.blue, color.alpha]
         end
         result = [shot.class, shot.width, shot.height, colors, shot.font.class,
                   Graphics.frame_count - count]
         [shot, sprite, bitmap].each { |object| object.dispose }
         result",
    );
    let pixel = "[78.0, 78.0, 78.0, 255.0]";
    assert_eq!(
        value,
        format!("[Bitmap, 544, 416, [{pixel}, {pixel}], Font, 0]")
    );
    assert!(record.received.is_empty());
}

/// The alpha is 255 also where nothing is drawn and where the scene is translucent.
#[test]
fn a_snapshot_is_opaque_everywhere() {
    let (value, _) = eval(
        "bitmap = Bitmap.new(8, 8)
         bitmap.fill_rect(0, 0, 8, 8, Color.new(255, 255, 255, 128))
         sprite = Sprite.new
         sprite.bitmap = bitmap
         shot = Graphics.snap_to_bitmap
         result = [shot.get_pixel(4, 4).alpha, shot.get_pixel(100, 100).alpha]
         [shot, sprite, bitmap].each { |object| object.dispose }
         result",
    );
    assert_eq!(value, "[255.0, 255.0]");
}

/// Scenario "Changes not yet shown and a freeze, measured on the original".
#[test]
fn a_snapshot_holds_the_scene_as_its_objects_are_also_after_a_freeze() {
    let (value, record) = eval(
        "bitmap = Bitmap.new(32, 32)
         bitmap.fill_rect(0, 0, 32, 32, Color.new(255, 0, 0))
         sprite = Sprite.new
         sprite.bitmap = bitmap
         Graphics.update
         Graphics.freeze
         bitmap.fill_rect(0, 0, 32, 32, Color.new(0, 0, 255))
         shot = Graphics.snap_to_bitmap
         colors = [shot.get_pixel(5, 5), shot.get_pixel(40, 5)].map do |color|
           [color.red, color.green, color.blue, color.alpha]
         end
         [shot, sprite, bitmap].each { |object| object.dispose }
         colors",
    );
    assert_eq!(value, "[[0.0, 0.0, 255.0, 255.0], [0.0, 0.0, 0.0, 255.0]]");
    assert_eq!(record.frames().len(), 1);
    assert_eq!(record.received.len(), 2);
}

/// Scenario "Nothing owed to the host is lost".
#[test]
fn a_snapshot_takes_nothing_from_the_next_frame() {
    let script = |snapshot: &str| {
        format!(
            "bitmap = Bitmap.new(32, 32)
             bitmap.fill_rect(0, 0, 32, 32, Color.new(255, 0, 0))
             sprite = Sprite.new
             sprite.bitmap = bitmap
             Graphics.update
             bitmap.fill_rect(0, 0, 16, 16, Color.new(0, 0, 255))
             count = Graphics.frame_count
             {snapshot}
             Graphics.update
             [sprite, bitmap].each {{ |object| object.dispose }}
             Graphics.frame_count - count"
        )
    };
    let (value, with) = eval(&script("Graphics.snap_to_bitmap.dispose"));
    assert_eq!(value, "1");
    let (value, without) = eval(&script(""));
    assert_eq!(value, "1");

    let frames = with.frames();
    assert_eq!(with.received.len(), 2);
    assert_eq!(frames[1].frame_index, frames[0].frame_index + 1);
    let changes: Vec<_> = frames[1]
        .bitmaps
        .iter()
        .filter_map(|delta| match delta {
            BitmapDelta::Update { rect, rgba, .. } => Some((*rect, rgba.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        changes,
        [(Rect::new(0, 0, 16, 16), [0, 0, 255, 255].repeat(256))]
    );
    // The bitmap of the snapshot itself never reaches the host.
    for frame in &frames {
        let creations = |delta: &&BitmapDelta| matches!(delta, BitmapDelta::Create { .. });
        assert!(frame.bitmaps.iter().filter(creations).count() <= 1);
    }

    let shown = scene_support::rendered(&frames);
    let expected = scene_support::rendered(&without.frames());
    assert_eq!(shown[1].pixel(5, 5), [0, 0, 255, 255]);
    assert_eq!(shown[1].pixel(20, 20), [255, 0, 0, 255]);
    for (shown, expected) in shown.iter().zip(&expected) {
        assert_eq!(shown.rgba, expected.rgba);
    }
}

// --- Resizing the screen ----------------------------------------------------------------------

/// A snippet that gives the frame the size of the start back when it ends, because the scene
/// is shared by the tests.
fn resizing(source: &str) -> (String, Record) {
    eval(&format!(
        "begin\n{source}\nensure\nGraphics.resize_screen(544, 416)\nend"
    ))
}

/// Scenario "Limits, measured on the original".
#[test]
fn resize_screen_limits_each_side() {
    let (value, record) = resizing(
        "[[641, 481], [1000, 1000], [0, 0], [-1, -1], [100, 50], [640, 1]].map do |w, h|
           [Graphics.resize_screen(w, h), Graphics.width, Graphics.height]
         end",
    );
    assert_eq!(
        value,
        "[[nil, 640, 480], [nil, 640, 480], [nil, 1, 1], [nil, 1, 1], [nil, 100, 50], \
         [nil, 640, 1]]"
    );
    assert!(record.received.is_empty());
}

/// Scenario "Frames and snapshot, measured on the original".
#[test]
fn frames_and_snapshots_follow_the_size_set() {
    let (value, record) = resizing(
        "count = Graphics.frame_count
         Graphics.resize_screen(320, 240)
         shot = Graphics.snap_to_bitmap
         size = [shot.width, shot.height]
         shot.dispose
         Graphics.update
         [size, Graphics.frame_count - count]",
    );
    assert_eq!(value, "[[320, 240], 1]");
    let frames = record.frames();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].logical_size, Size::new(320, 240));
    assert_eq!(record.received.len(), 1);
}

/// Scenario "Arguments, measured on the original".
#[test]
fn resize_screen_takes_two_integers() {
    let (value, _) = resizing("Graphics.resize_screen(640)");
    assert!(value.starts_with("ArgumentError: "), "{value}");
    let (value, _) = resizing("Graphics.resize_screen(nil, 480)");
    assert!(value.starts_with("TypeError: "), "{value}");
    let (value, _) = resizing(
        "Graphics.resize_screen(100.9, 50.9)
         [Graphics.width, Graphics.height]",
    );
    assert_eq!(value, "[100, 50]");
}
