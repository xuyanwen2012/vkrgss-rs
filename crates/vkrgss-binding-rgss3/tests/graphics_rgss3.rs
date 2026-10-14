//! The module `Graphics` in a game of RGSS3 over Ruby 1.9.2: Ruby snippets evaluated against
//! a scripted host, and what that host saw. The scenarios of "Method surface of Graphics in
//! RGSS3", "Freeze and snapshot of RGSS3", "Transition of RGSS3", "Movies of RGSS3" and
//! "Graphics values of RGSS3" of `rgss-modules`. A file of this package alone, so that it
//! never runs as RGSS1 or RGSS2 over Ruby 1.8.7.
//!
//! The expected values are those measured on the original runtime of VX Ace (DLL versions
//! 3.0.0.1 and 3.0.1.1, 2026-10-13, `ace-v1-sprite-REPORT.md` section 5 with its exploration
//! log and `ace-v0-REPORT.md` section 6, kept outside the repository), but for
//! `play_movie` of a file that exists, which plays nothing in vkrgss.
#![cfg(not(vkrgss_interpreter_absent))]
// The method list module of `vkrgss-binding` names the binding by that name.
extern crate vkrgss_binding_rgss3 as vkrgss_binding;

#[path = "../../vkrgss-binding/tests/method_list/mod.rs"]
mod method_list;

use std::sync::OnceLock;

use vkrgss_binding::testing;
use vkrgss_binding::testing::host::{self, Record, Script};
use vkrgss_proto::{FreezeRule, GuestToHost, Size, TransitionRule};
use vkrgss_vfs::Runtime;

const METHOD_LIST: &str = include_str!("../../vkrgss-binding/tests/data/rgss3_api.txt");

/// Starts the harness as a game of RGSS3 whose game directory holds `Movies/clip.ogv`,
/// written before the guest first looks into the directory.
fn start() {
    static MOVIE: OnceLock<()> = OnceLock::new();
    testing::start_as(Runtime::Rgss3);
    MOVIE.get_or_init(|| {
        let movies = testing::game_dir().join("Movies");
        std::fs::create_dir_all(&movies).expect("the directory of the movie");
        std::fs::write(movies.join("clip.ogv"), b"not really Ogg Theora").expect("the movie");
    });
}

/// The value of a snippet, or its exception as `Class: message`, and what the host saw. The
/// brightness is 255 and nothing is frozen when the snippet starts, and a snippet leaves
/// nothing frozen, the screen at 544 by 416 and the frame rate at 60 when it ends. The scene
/// is shared by the tests, so a snippet disposes what it made before it ends.
fn eval(source: &str) -> (String, Record) {
    start();
    let source = format!("Graphics.brightness = 255\n{source}");
    let (outcome, record) = host::play(&source, Script::new());
    let outcome = match outcome {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    };
    (outcome, record)
}

/// Ruby that the snippets of [`eval`] use: `error { ... }` is the class and the message of
/// the exception the block raises, `frames { ... }` the number of frames the block shows.
const HELPERS: &str = "def error; yield; rescue Exception; \"#{$!.class}: #{$!.message}\"; end
     def frames; count = Graphics.frame_count; yield; Graphics.frame_count - count; end
     ";

/// [`eval`] with [`HELPERS`].
fn eval_with_helpers(source: &str) -> (String, Record) {
    eval(&format!("{HELPERS}{source}"))
}

/// The transition messages the host received, as (duration, whether it has a mask, rule).
fn transitions(record: &Record) -> Vec<(u32, bool, TransitionRule)> {
    let transitions = record.received.iter().filter_map(|message| match message {
        GuestToHost::Transition {
            duration,
            mask,
            rule,
            ..
        } => Some((*duration, mask.is_some(), *rule)),
        _ => None,
    });
    transitions.collect()
}

// --- Method surface of Graphics in RGSS3 ------------------------------------------------------

/// Scenario "The functions, measured on the original", and the functions of the list.
#[test]
fn the_functions_are_those_of_the_original() {
    let (value, _) = eval("Graphics.singleton_methods(false).sort");
    assert_eq!(
        value,
        "[:__reset__, :brightness, :brightness=, :fadein, :fadeout, :frame_count, \
         :frame_count=, :frame_rate, :frame_rate=, :frame_reset, :freeze, :height, \
         :play_movie, :resize_screen, :snap_to_bitmap, :transition, :update, :visible, \
         :visible=, :wait, :width]"
    );
    let mut listed: Vec<String> = method_list::parse(METHOD_LIST)
        .iter()
        .filter(|entry| entry.class == "Graphics" && entry.class_level)
        .map(|entry| format!(":{}", entry.method))
        .collect();
    listed.sort();
    assert_eq!(value, format!("[{}]", listed.join(", ")));
}

/// Scenario "No _reset for scripts".
#[test]
fn reset_is_no_function_of_the_scripts() {
    let (value, _) = eval("[Graphics.respond_to?(:_reset), Graphics.__reset__]");
    assert_eq!(value, "[false, nil]");
}

// --- Freeze and snapshot of RGSS3 -------------------------------------------------------------

/// Scenario "Freeze rule".
#[test]
fn freeze_keeps_the_picture_by_the_buffer_rule() {
    let (value, record) = eval("Graphics.freeze; Graphics.transition(2)");
    assert_eq!(value, "nil");
    assert_eq!(
        record.received[0],
        GuestToHost::Freeze {
            rule: FreezeRule::Buffer
        }
    );
    assert_eq!(
        record.received[1],
        GuestToHost::Transition {
            duration: 2,
            mask: None,
            vague: 40,
            rule: TransitionRule::Rgss2,
        }
    );
    assert_eq!(record.frames().len(), 2);
    assert_eq!(record.received.len(), 4);
}

/// The pixel (10, 10) of a snapshot as `[red, green, blue, alpha]`, read by `read`.
const READ: &str = "read = lambda do
       shot = Graphics.snap_to_bitmap
       c = shot.get_pixel(10, 10)
       r = [[c.red, c.green, c.blue, c.alpha], shot.width, shot.height]
       shot.dispose
       r
     end
     ";

/// Scenarios "Brightness 0, measured on the original" and "Other brightness, measured on
/// the original".
#[test]
fn a_snapshot_at_brightness_0_is_all_zeros() {
    let (value, record) = eval(&format!(
        "{READ}white = Bitmap.new(544, 416)
         white.fill_rect(0, 0, 544, 416, Color.new(255, 255, 255))
         sprite = Sprite.new
         sprite.bitmap = white
         Graphics.brightness = 0
         dark = read.call
         Graphics.brightness = 100
         dim = read.call[0]
         Graphics.brightness = 255
         sprite.dispose
         white.dispose
         [dark, dim, read.call[0]]"
    ));
    assert_eq!(
        value,
        "[[[0.0, 0.0, 0.0, 0.0], 544, 416], [99.0, 99.0, 99.0, 255.0], [0.0, 0.0, 0.0, 255.0]]"
    );
    assert!(record.received.is_empty());
}

/// Scenario "Argument, measured on the original".
#[test]
fn a_snapshot_takes_no_argument() {
    let (value, _) = eval("Graphics.snap_to_bitmap(1)");
    assert!(value.starts_with("ArgumentError: "), "{value}");
}

// --- Transition of RGSS3 ----------------------------------------------------------------------

/// Scenario "Nothing frozen, measured on the original".
#[test]
fn a_transition_with_nothing_frozen_reads_no_argument() {
    let (value, record) = eval(
        "[Graphics.transition, Graphics.transition(5, 'missing'),
          Graphics.transition(1, 2, 3, 4), Graphics.transition(nil)]",
    );
    assert_eq!(value, "[nil, nil, nil, nil]");
    assert!(record.received.is_empty());
}

/// Scenario "Durations, measured on the original".
#[test]
fn a_transition_shows_its_duration_truncated() {
    let (value, record) = eval_with_helpers(
        "shown = [[], [3], [2.9], [-3], [0]].map do |args|
           Graphics.brightness = 100
           Graphics.freeze
           [frames { Graphics.transition(*args) }, Graphics.brightness]
         end
         shown + [frames { Graphics.transition(3) }]",
    );
    assert_eq!(
        value,
        "[[10, 255], [3, 255], [2, 255], [0, 255], [0, 255], 0]"
    );
    assert_eq!(record.frames().len(), 15);
}

/// Scenario "Errors that keep the freeze, measured on the original".
#[test]
fn errors_of_the_arguments_keep_the_freeze() {
    let (value, record) = eval_with_helpers(
        "['Graphics.transition(nil)', 'Graphics.transition(3, nil)',
          'Graphics.transition(3, 5)', 'Graphics.transition(3, \"\", 10, 4)'].map do |call|
           Graphics.freeze
           [frames { error { eval(call) } }, frames { Graphics.transition(3) }]
         end",
    );
    assert_eq!(
        value, "[[0, 3], [0, 3], [0, 3], [0, 3]]",
        "the frames of each erroneous transition and of the one after it"
    );
    let (value, _) = eval_with_helpers(
        "['Graphics.transition(nil)', 'Graphics.transition(3, nil)',
          'Graphics.transition(3, 5)', 'Graphics.transition(3, \"\", 10, 4)'].map do |call|
           Graphics.freeze
           e = error { eval(call) }
           Graphics.transition(0)
           e
         end",
    );
    assert_eq!(
        value,
        "[\"TypeError: no implicit conversion from nil to integer\", \
         \"TypeError: can't convert nil into String\", \
         \"TypeError: can't convert Fixnum into String\", \
         \"ArgumentError: wrong number of arguments (3 for 4)\"]"
    );
    assert_eq!(record.frames().len(), 12);
}

/// Scenario "A missing picture ends the freeze, measured on the original".
#[test]
fn a_missing_picture_ends_the_freeze() {
    let (value, record) = eval_with_helpers(
        "Graphics.freeze
         [error { Graphics.transition(3, 'missing') }, frames { Graphics.transition(3) }]",
    );
    assert_eq!(
        value,
        "[\"Errno::ENOENT: No such file or directory - missing\", 0]"
    );
    assert!(record.frames().is_empty());
    // The host is told that the freeze has ended.
    assert_eq!(transitions(&record), [(0, false, TransitionRule::Rgss2)]);
}

/// Scenario "Empty name, measured on the original".
#[test]
fn an_empty_name_is_no_mask() {
    let (value, record) = eval("Graphics.freeze; Graphics.transition(3, '')");
    assert_eq!(value, "nil");
    assert_eq!(transitions(&record), [(3, false, TransitionRule::Rgss2)]);
    assert_eq!(record.frames().len(), 3);
}

/// A picture that exists is the mask, as in RGSS2.
#[test]
fn a_picture_that_exists_is_the_mask() {
    let (value, record) =
        eval("Graphics.freeze; Graphics.transition(2, 'Graphics/Transitions/004-Blind04', 10)");
    assert_eq!(value, "nil");
    assert_eq!(transitions(&record), [(2, true, TransitionRule::Rgss2)]);
    assert_eq!(record.frames().len(), 2);
}

/// Scenario "Two freezes, measured on the original".
#[test]
fn two_freezes_make_one_transition() {
    let (value, record) =
        eval("Graphics.freeze; Graphics.freeze; Graphics.transition(2); Graphics.transition(2)");
    assert_eq!(value, "nil");
    assert_eq!(record.frames().len(), 2);
}

// --- Movies of RGSS3 --------------------------------------------------------------------------

/// Scenario "Errors, measured on the original".
#[test]
fn play_movie_raises_as_measured() {
    let (value, record) = eval_with_helpers(
        "[(begin; Graphics.play_movie; rescue ArgumentError; ArgumentError; end),
          error { Graphics.play_movie(nil) }, error { Graphics.play_movie(5) },
          error { Graphics.play_movie('Movies/missing') }, error { Graphics.play_movie('') }]",
    );
    assert_eq!(
        value,
        "[ArgumentError, \"TypeError: can't convert nil into String\", \
         \"TypeError: can't convert Fixnum into String\", \
         \"Errno::ENOENT: No such file or directory - Movies/missing\", \
         \"Errno::ENOENT: No such file or directory - \"]"
    );
    assert!(record.received.is_empty());
}

/// Scenario "A file that exists".
#[test]
fn play_movie_of_a_file_that_exists_plays_nothing() {
    let (value, record) =
        eval("[Graphics.play_movie('Movies/clip'), Graphics.play_movie('Movies/clip.ogv')]");
    assert_eq!(value, "[nil, nil]");
    assert!(record.received.is_empty());
    assert_eq!(record.ticks, 0);
}

// --- Graphics values of RGSS3 -----------------------------------------------------------------

/// Scenario "Waits and fades, measured on the original".
#[test]
fn waits_and_fades_count_their_frames_as_measured() {
    let (value, record) = eval_with_helpers(
        "shown = []
         levels = []
         [[:wait, 3], [:wait, 2.9], [:wait, -2]].each do |name, count|
           shown << frames { Graphics.send(name, count) }
         end
         [[:fadeout, 2.9], [:fadein, 2.9], [:fadeout, -3], [:fadein, -3]].each do |name, count|
           shown << frames { Graphics.send(name, count) }
           levels << Graphics.brightness
         end
         Graphics.brightness = 100
         shown << frames { Graphics.fadeout(4) }
         levels << Graphics.brightness
         [shown, levels]",
    );
    assert_eq!(value, "[[3, 2, 0, 2, 2, 0, 0, 4], [0, 255, 0, 255, 0]]");
    assert_eq!(record.frames().len(), 13);
}

/// Scenario "Wrong arguments of waits and fades, measured on the original".
#[test]
fn waits_and_fades_refuse_what_is_no_integer() {
    let (value, record) = eval_with_helpers(
        "[error { Graphics.wait(nil) }, error { Graphics.wait('2') },
          error { Graphics.fadeout(nil) }]",
    );
    assert_eq!(
        value,
        "[\"TypeError: no implicit conversion from nil to integer\", \
         \"TypeError: can't convert String into Integer\", \
         \"TypeError: no implicit conversion from nil to integer\"]"
    );
    assert!(record.received.is_empty());
}

/// Scenario "Brightness and frame rate, measured on the original".
#[test]
fn brightness_and_frame_rate_are_limited_and_truncated() {
    let (value, _) = eval_with_helpers(
        "begin
           read = [300, -5, 99.9].map { |b| Graphics.brightness = b; Graphics.brightness } +
                  [9, 121, 59.9, -1].map { |r| Graphics.frame_rate = r; Graphics.frame_rate }
           [read, error { Graphics.brightness = nil }, error { Graphics.frame_rate = nil }]
         ensure
           Graphics.frame_rate = 60
         end",
    );
    assert_eq!(
        value,
        "[[255, 0, 99, 10, 120, 59, 10], \
         \"TypeError: no implicit conversion from nil to integer\", \
         \"TypeError: no implicit conversion from nil to integer\"]"
    );
}

/// Scenario "Start values, measured on the original".
#[test]
fn the_start_values_are_those_of_the_original() {
    let (value, _) = eval(
        "[Graphics.width, Graphics.height, Graphics.frame_rate, Graphics.brightness,
          Graphics.visible]",
    );
    assert_eq!(value, "[544, 416, 60, 255, true]");
}

/// Scenario "Frame count, measured on the original".
#[test]
fn the_frame_count_is_truncated() {
    let (value, _) = eval(
        "count = Graphics.frame_count
         read = [-5, 7.9].map { |c| Graphics.frame_count = c; Graphics.frame_count }
         Graphics.frame_count = count
         read",
    );
    assert_eq!(value, "[-5, 7]");
}

/// Evaluates `source` and sets the screen back to 544 by 416, also when it raises.
fn resizing(source: &str) -> (String, Record) {
    eval_with_helpers(&format!(
        "begin\n{source}\nensure\nGraphics.resize_screen(544, 416)\nend"
    ))
}

/// Scenario "Resize limits, measured on the original".
#[test]
fn resize_screen_limits_each_side() {
    let (value, record) = resizing(
        "[[800, 600], [641, 481], [300, 500], [0, 0], [-5, 10], [200.9, 100.9]].map do |w, h|
           [Graphics.resize_screen(w, h), Graphics.width, Graphics.height]
         end",
    );
    assert_eq!(
        value,
        "[[nil, 640, 480], [nil, 640, 480], [nil, 300, 480], [nil, 1, 1], [nil, 1, 10], \
         [nil, 200, 100]]"
    );
    assert!(record.received.is_empty());
}

/// Scenario "Resize arguments, measured on the original".
#[test]
fn resize_screen_takes_two_integers() {
    let (value, _) = resizing("Graphics.resize_screen(100)");
    assert!(value.starts_with("ArgumentError: "), "{value}");
    let (value, _) = resizing("Graphics.resize_screen(nil, 5)");
    assert_eq!(
        value,
        "TypeError: no implicit conversion from nil to integer"
    );
}

/// Scenario "What a resize keeps, measured on the original".
#[test]
fn a_resize_keeps_the_values_and_the_freeze() {
    let (value, record) = resizing(
        "count = Graphics.frame_count
         begin
           Graphics.brightness = 100
           Graphics.frame_count = 7
           Graphics.frame_rate = 30
           Graphics.freeze
           Graphics.resize_screen(640, 480)
           kept = [Graphics.brightness, Graphics.frame_count, Graphics.frame_rate]
           [kept, frames { Graphics.transition(6) }]
         ensure
           Graphics.frame_rate = 60
           Graphics.frame_count = count
         end",
    );
    assert_eq!(value, "[[100, 7, 30], 6]");
    let frames = record.frames();
    assert_eq!(frames.len(), 6);
    assert!(
        frames
            .iter()
            .all(|frame| frame.logical_size == Size::new(640, 480))
    );
}
