//! The reset of a game of RGSS3 and the two functions `Graphics.__reset__` and
//! `Audio.__reset__` against a scripted host: the scenarios of "Reset of RGSS3" of
//! `rgss-runtime` and "Reset functions of RGSS3" of `rgss-modules`, measured on the original
//! runtime of RGSS3 (`ace-v1-api-REPORT.md` sections 1.3 to 1.5 of the measurements kept
//! outside the repository).
#![cfg(not(vkxp_interpreter_absent))]

use vkxp_binding_rgss3::testing;
use vkxp_binding_rgss3::testing::host::{self, Game, Played, Script};
use vkxp_proto::{AudioChannel, AudioCommand, GuestToHost, HostToGuest, TransitionRule};
use vkxp_vfs::Runtime;

/// A game of RGSS3 named `name` with these scripts.
fn game(name: &str, scripts: &[(&str, &str)]) -> Game {
    testing::start_as(Runtime::Rgss3);
    Game::of(Runtime::Rgss3, name, "Ace", scripts)
}

/// Plays `game` against a host that follows `script` and reads `after`, after a run that
/// ended without a failure.
fn play_with(game: &Game, script: Script, after: &str) -> Played {
    let played = host::run_game(game, game.run_config(), script, after);
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    played
}

/// What `after` reads after a game of the one script `source`.
fn run(name: &str, source: &str, after: &str) -> String {
    let played = play_with(&game(name, &[("Main", source)]), Script::new(), after);
    match played.after {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

// --- Reset of RGSS3 ---------------------------------------------------------------------------

#[test]
fn a_reset_by_the_host_raises_rgss_reset_with_an_empty_message() {
    let source = "begin
  loop { Graphics.update }
rescue RGSSReset => e
  $vkxp_host_reset = [e.class, e.message]
end";
    let game = game("host_reset", &[("Main", source)]);
    let played = play_with(
        &game,
        Script::new().after_frame(2, HostToGuest::Reset),
        "$vkxp_host_reset",
    );
    assert_eq!(played.after.as_deref(), Ok(r#"[RGSSReset, ""]"#));
}

#[test]
fn a_bare_rescue_does_not_catch_it_and_the_scripts_start_over() {
    let source = "$vkxp_bare_runs = ($vkxp_bare_runs || 0) + 1
if $vkxp_bare_runs == 1
  begin
    loop { Graphics.update }
  rescue => e
    $vkxp_bare_caught = true
  end
end";
    let game = game("bare_rescue", &[("Main", source)]);
    let played = play_with(
        &game,
        Script::new().after_frame(2, HostToGuest::Reset),
        "[$vkxp_bare_runs, $vkxp_bare_caught]",
    );
    assert_eq!(played.after.as_deref(), Ok("[2, nil]"));
}

#[test]
fn a_reset_at_the_top_level_runs_every_script_again() {
    let played = play_with(
        &game(
            "top_level",
            &[
                (
                    "First",
                    "$vkxp_top_runs = ($vkxp_top_runs || 0) + 1
                     ($vkxp_top_sprites ||= []) << Sprite.new",
                ),
                (
                    "Second",
                    "$vkxp_top_raised = ($vkxp_top_raised || 0) + 1
                     raise RGSSReset if $vkxp_top_raised <= 2",
                ),
            ],
        ),
        Script::new(),
        "[$vkxp_top_runs, $vkxp_top_sprites.map { |sprite| sprite.disposed? }]",
    );
    assert_eq!(played.after.as_deref(), Ok("[3, [true, true, false]]"));
}

#[test]
fn a_reset_inside_the_main_function_runs_its_block_again() {
    let block = "rgss_main do
  $vkxp_main_runs = ($vkxp_main_runs || 0) + 1
  if $vkxp_main_runs == 1
    $vkxp_main_sprite = Sprite.new
    Graphics.frame_count = 1000
    Graphics.brightness = 99
    Graphics.frame_rate = 30
    raise RGSSReset
  end
  $vkxp_main_seen = [$vkxp_main_sprite.disposed?, Graphics.brightness, Graphics.frame_rate,
                     Graphics.frame_count >= 1000]
end";
    let played = play_with(
        &game(
            "main_function",
            &[
                ("First", "$vkxp_main_first = ($vkxp_main_first || 0) + 1"),
                ("Main", block),
            ],
        ),
        Script::new(),
        "[$vkxp_main_first, $vkxp_main_runs, $vkxp_main_seen]",
    );
    assert_eq!(played.after.as_deref(), Ok("[1, 2, [true, 255, 60, true]]"));
}

#[test]
fn random_numbers_across_a_reset_are_those_of_one_seed() {
    let source = "$vkxp_numbers = []
rgss_main do
  $vkxp_numbers << rand(1000)
  raise RGSSReset if $vkxp_numbers.size == 1
end";
    let numbers: Vec<String> = (0..2)
        .map(|round| {
            let game = game(&format!("seeded_{round}"), &[("Main", source)]);
            let mut run = game.run_config();
            run.seed = Some(7);
            let played = host::run_game(&game, run, Script::new(), "$vkxp_numbers");
            assert!(played.outcome.is_ok(), "{:?}", played.outcome);
            played.after.expect("the numbers")
        })
        .collect();
    assert_eq!(numbers[0], numbers[1]);
    assert_eq!(numbers[0].matches(',').count(), 1, "{}", numbers[0]);
}

// --- Reset functions of RGSS3 -----------------------------------------------------------------

#[test]
fn both_take_no_argument_and_return_nil() {
    assert_eq!(
        run(
            "arity",
            "$vkxp_arity = [Graphics.method(:__reset__).arity, Graphics.__reset__,
                            Audio.method(:__reset__).arity, Audio.__reset__]",
            "$vkxp_arity"
        ),
        "[0, nil, 0, nil]"
    );
}

#[test]
fn every_bitmap_and_element_is_disposed() {
    let source = "class VkxpSprite < Sprite
end
free = Bitmap.new(4, 4)
sprite = VkxpSprite.new
sprite.bitmap = Bitmap.new(4, 4)
viewport = Viewport.new(0, 0, 32, 32)
window = Window.new
window.windowskin = Bitmap.new(128, 128)
window.contents = Bitmap.new(8, 8)
plane = Plane.new
tilemap = Tilemap.new
Graphics.__reset__
failure = lambda do |&block|
  begin
    block.call
  rescue RGSSError => e
    e.message
  end
end
# A viewport of RGSS2 has no `disposed?`, which RGSS3 adds with the class surface of
# RGSS3; its methods tell that it is disposed.
$vkxp_disposed = [[free, sprite, window, plane, tilemap].map { |o| o.disposed? },
                  failure.call { viewport.rect }, sprite.bitmap.class,
                  failure.call { sprite.x }]";
    assert_eq!(
        run("disposed", source, "$vkxp_disposed"),
        r#"[[true, true, true, true, true], "disposed viewport", Bitmap, "disposed sprite"]"#
    );
}

#[test]
fn the_brightness_and_the_frame_rate_are_set_back_and_a_freeze_ends() {
    let source = "Graphics.brightness = 100
Graphics.frame_rate = 120
Graphics.freeze
Graphics.__reset__
Graphics.update
$vkxp_set_back = [Graphics.brightness, Graphics.frame_rate]";
    let played = play_with(
        &game("set_back", &[("Main", source)]),
        Script::new(),
        "$vkxp_set_back",
    );
    assert_eq!(played.after.as_deref(), Ok("[255, 60]"));
    // The host is told that the freeze has ended before the frame of the next update.
    let messages: Vec<&GuestToHost> = played
        .record
        .received
        .iter()
        .skip_while(|message| **message != GuestToHost::Freeze)
        .collect();
    assert!(
        matches!(
            messages.get(1),
            Some(GuestToHost::Transition {
                duration: 0,
                mask: None,
                rule: TransitionRule::Rgss2,
                ..
            })
        ),
        "{messages:?}"
    );
    assert!(matches!(messages.get(2), Some(GuestToHost::Frame(_))));
}

#[test]
fn the_frame_count_the_screen_the_fonts_and_values_stay() {
    let source = "Graphics.frame_count = 1000
Graphics.resize_screen(320, 240)
size = Font.default_size
Font.default_size = 30
sprite = Sprite.new
color = sprite.color
color.set(1, 2, 3, 4)
Graphics.__reset__
$vkxp_stays = [Graphics.frame_count, [Graphics.width, Graphics.height], Font.default_size,
               [color.red, color.green, color.blue, color.alpha]]
Graphics.resize_screen(544, 416)
Font.default_size = size";
    assert_eq!(
        run("stays", source, "$vkxp_stays"),
        "[1000, [320, 240], 30, [1.0, 2.0, 3.0, 4.0]]"
    );
}

/// The audio commands of a record between two frames, by their numbers counted from 1; 0
/// is the start.
fn stops_between(played: &Played, after: usize, before: usize) -> Vec<AudioChannel> {
    let mut frame = 0;
    let mut stops = Vec::new();
    for message in &played.record.received {
        match message {
            GuestToHost::Frame(_) => frame += 1,
            GuestToHost::Audio(AudioCommand::Stop { channel })
                if frame >= after && frame < before =>
            {
                stops.push(*channel);
            }
            _ => {}
        }
    }
    stops
}

#[test]
fn audio_reset_stops_the_four_channels_and_graphics_reset_none() {
    let game = game(
        "audio",
        &[(
            "Main",
            "Audio.bgm_play('Audio/BGM/m')
             Audio.bgs_play('Audio/BGS/b')
             Audio.me_play('Audio/ME/e')
             Audio.se_play('Audio/SE/s')
             Graphics.__reset__
             Graphics.update
             Audio.__reset__
             Graphics.update",
        )],
    );
    for (folder, stem) in [("BGM", "m"), ("BGS", "b"), ("ME", "e"), ("SE", "s")] {
        let path = game
            .dir()
            .join("Audio")
            .join(folder)
            .join(format!("{stem}.ogg"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"not really Ogg Vorbis").unwrap();
    }
    let played = play_with(&game, Script::new(), "nil");
    assert_eq!(stops_between(&played, 0, 1), []);
    assert_eq!(
        stops_between(&played, 1, 2),
        [
            AudioChannel::Bgm,
            AudioChannel::Bgs,
            AudioChannel::Me,
            AudioChannel::Se
        ]
    );
}

#[test]
fn the_main_function_calls_both_as_a_script_redefined_them() {
    let source = "$vkxp_order = []
class << Audio
  alias vkxp_audio_reset __reset__
  def __reset__
    $vkxp_order << 'Audio'
    vkxp_audio_reset
  end
end
class << Graphics
  alias vkxp_graphics_reset __reset__
  def __reset__
    $vkxp_order << 'Graphics'
    vkxp_graphics_reset
  end
end
$vkxp_redefined_runs = 0
rgss_main do
  $vkxp_redefined_runs += 1
  raise RGSSReset if $vkxp_redefined_runs == 1
end
$vkxp_redefined = $vkxp_order.dup";
    assert_eq!(
        run("redefined", source, "$vkxp_redefined"),
        r#"["Audio", "Graphics"]"#
    );
}
