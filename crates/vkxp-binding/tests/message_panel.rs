//! The message panel: the scenarios of the requirement of that name in `rgss-runtime`. The
//! frames a scripted host recorded while a script waited are drawn with the reference
//! compositor.

mod scene_support;

use vkxp_binding::RunConfig;
use vkxp_binding::testing::host::{self, Game, Script};
use vkxp_compose_ref::Frame;
use vkxp_proto::{GuestToHost, HostToGuest};

/// The largest area a panel covers on the screen of 640 by 480.
fn in_panel_area(x: u32, y: u32) -> bool {
    (40..600).contains(&x) && (40..440).contains(&y)
}

/// The positions at which two frames differ.
fn differences(a: &Frame, b: &Frame) -> Vec<(u32, u32)> {
    assert_eq!(a.size, b.size);
    let mut differing = Vec::new();
    for y in 0..a.size.height {
        for x in 0..a.size.width {
            if a.pixel(x, y) != b.pixel(x, y) {
                differing.push((x, y));
            }
        }
    }
    differing
}

#[test]
fn print_shows_a_panel_over_the_scene_until_the_dismissal() {
    let (value, record) = host::play(
        "sprite = Sprite.new
         sprite.bitmap = Bitmap.new(640, 480)
         sprite.bitmap.fill_rect(0, 0, 640, 480, Color.new(255, 0, 0))
         Graphics.update
         before = Graphics.frame_count
         print('hello')
         during = Graphics.frame_count - before
         Graphics.update
         sprite.bitmap.dispose
         sprite.dispose
         during",
        Script::new().dismiss_after(3),
    );
    // The frames of the wait are not counted.
    assert_eq!(value.as_deref(), Ok("0"));
    // One frame, the message, the three frames the host waits for, one frame.
    let kinds: Vec<bool> = record
        .received
        .iter()
        .map(|message| matches!(message, GuestToHost::Frame(_)))
        .collect();
    assert_eq!(kinds, [true, false, true, true, true, true]);
    assert_eq!(
        record.received[1],
        GuestToHost::Modal {
            text: "hello".to_owned()
        }
    );

    let frames = scene_support::rendered(&record.frames());
    let before = &frames[0];
    assert_eq!(before.pixel(320, 240), [255, 0, 0, 255]);
    for shown in &frames[1..4] {
        let differing = differences(before, shown);
        assert!(!differing.is_empty(), "the panel is drawn");
        assert!(differing.iter().all(|&(x, y)| in_panel_area(x, y)));
        // The panel is centred, and the scene shows through its box.
        let left = differing.iter().map(|&(x, _)| x).min().unwrap();
        let right = differing.iter().map(|&(x, _)| x).max().unwrap();
        let top = differing.iter().map(|&(_, y)| y).min().unwrap();
        let bottom = differing.iter().map(|&(_, y)| y).max().unwrap();
        assert!((left + right).abs_diff(639) <= 1, "{left}..={right}");
        assert!((top + bottom).abs_diff(479) <= 1, "{top}..={bottom}");
        assert_eq!(shown.pixel(left, top), [200, 200, 200, 255]);
        assert_eq!(shown.pixel(left + 2, top + 2), [50, 0, 0, 255]);
        assert_eq!(shown, &frames[1], "the panel stays as it is");
    }
    assert_eq!(&frames[4], before);
}

#[test]
fn the_panel_is_in_front_of_a_window_with_the_highest_z() {
    let (value, record) = host::play(
        "window = Window.new
         window.x = 0
         window.y = 0
         window.width = 640
         window.height = 480
         window.z = 2147483647
         window.contents = Bitmap.new(608, 448)
         window.contents.fill_rect(0, 0, 608, 448, Color.new(0, 255, 0))
         Graphics.update
         print('hello')
         Graphics.update
         window.contents.dispose
         window.dispose
         nil",
        Script::new().dismiss_after(3),
    );
    assert_eq!(value.as_deref(), Ok("nil"));
    let frames = scene_support::rendered(&record.frames());
    assert_eq!(frames.len(), 5);
    let before = &frames[0];
    assert_eq!(before.pixel(320, 240), [0, 255, 0, 255]);
    for shown in &frames[1..4] {
        let differing = differences(before, shown);
        assert!(!differing.is_empty(), "the panel is drawn over the window");
        assert!(differing.iter().all(|&(x, y)| in_panel_area(x, y)));
        let left = differing.iter().map(|&(x, _)| x).min().unwrap();
        let top = differing.iter().map(|&(_, y)| y).min().unwrap();
        assert_eq!(shown.pixel(left, top), [200, 200, 200, 255]);
        assert_eq!(shown.pixel(left + 2, top + 2), [0, 50, 0, 255]);
    }
    assert_eq!(&frames[4], before);
}

#[test]
fn a_reset_request_ends_the_wait_and_is_raised() {
    let (value, record) = host::play(
        "begin
           print('waiting')
           :dismissed
         rescue Reset
           :reset
         end",
        Script::new()
            .dismiss_after(100)
            .after_frame(2, HostToGuest::Reset),
    );
    assert_eq!(value.as_deref(), Ok(":reset"));
    assert_eq!(record.frames().len(), 2);
}

#[test]
fn an_uncaught_error_is_shown_in_a_panel_before_the_shutdown() {
    let game = Game::new("panel_error", "Panel", &[("Main", "raise 'broken'")]);
    let played = host::run_game(
        &game,
        game.run_config(),
        Script::new().dismiss_after(2),
        "nil",
    );
    assert!(played.outcome.is_err());
    let received = &played.record.received;
    let position = |wanted: fn(&GuestToHost) -> bool| {
        received
            .iter()
            .position(wanted)
            .expect("the message was sent")
    };
    let modal = position(|message| matches!(message, GuestToHost::Modal { .. }));
    let shutdown = position(|message| matches!(message, GuestToHost::Shutdown));
    assert!(matches!(received[modal + 1], GuestToHost::Frame(_)));
    assert!(matches!(received[modal + 2], GuestToHost::Frame(_)));
    assert!(modal + 2 < shutdown);
    let frames = scene_support::rendered(&played.record.frames());
    assert_eq!(frames.len(), 2);
    for frame in &frames {
        // The border of the panel; the scene itself is empty.
        assert!(
            frame
                .rgba
                .as_chunks::<4>()
                .0
                .contains(&[200, 200, 200, 255])
        );
    }
}

#[test]
fn log_only_p_writes_to_the_error_stream_and_sends_nothing() {
    let game = Game::new(
        "panel_log_p",
        "Panel",
        &[(
            "Main",
            "capture = Object.new
             def capture.write(text)
               ($vkxp_log_p ||= '') << text.to_s
             end
             stderr = $stderr
             $stderr = capture
             begin
               p 1
             ensure
               $stderr = stderr
             end",
        )],
    );
    let config = RunConfig {
        log_p: true,
        ..game.run_config()
    };
    let played = host::run_game(&game, config, Script::new(), "$vkxp_log_p");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok(r#""1\n""#));
    assert!(played.record.frames().is_empty());
    assert!(
        !played
            .record
            .received
            .iter()
            .any(|message| matches!(message, GuestToHost::Modal { .. }))
    );
}
