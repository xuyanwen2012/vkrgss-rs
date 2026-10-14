//! A script of RGSS3 flavour through the classes over Ruby 1.9.2: a bitmap, a sprite that a
//! Fiber moves, and the frame the guest's scene gives. No window, no guest process and no
//! file of an original runtime is involved: the frame is drawn by the reference compositor.
#![cfg(not(vkxp_interpreter_absent))]
// The support module of `vkxp-binding` names the binding by that name.
extern crate vkxp_binding_rgss3 as vkxp_binding;

#[path = "../../vkxp-binding/tests/scene_support/mod.rs"]
mod scene_support;

use vkxp_binding::testing;
use vkxp_vfs::Runtime;

/// `inspect` of the value of `source` in a game of RGSS3, or the class and the message of
/// the exception that left it.
fn eval(source: &str) -> String {
    testing::start_as(Runtime::Rgss3);
    scene_support::install();
    match testing::eval(source) {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

/// Fills a bitmap of 4 by 4 with opaque red and shows it with a sprite; a Fiber sets `x` to
/// 10, yields, and sets `y` to 20. The value is what the frame shows after the first resume
/// at (10, 0) and (10, 20), and after the second at (10, 20), (9, 20) and (10, 19), each as
/// whether the pixel is red, and whether the Fiber has ended. The scene is shared, so the script disposes what it made.
const SCRIPT: &str = "
bitmap = Bitmap.new(4, 4)
sprite = Sprite.new
begin
  bitmap.fill_rect(bitmap.rect, Color.new(255, 0, 0, 255))
  sprite.bitmap = bitmap
  fiber = Fiber.new do
    sprite.x = 10
    Fiber.yield
    sprite.y = 20
  end
  red = ->(x, y) { pixel(x, y) == [255, 0, 0, 255] }
  fiber.resume
  between = [red.(10, 0), red.(10, 20)]
  fiber.resume
  after = [red.(10, 20), red.(9, 20), red.(10, 19)]
  # The core of Ruby 1.9.2 has no `Fiber#alive?`: a Fiber that has ended refuses a resume.
  ended = begin
    fiber.resume
    false
  rescue FiberError
    true
  end
  [between, after, ended]
ensure
  sprite.dispose
  bitmap.dispose
end
";

const FRAMES: &str = "[[true, false], [true, false, false], true]";

#[test]
fn the_sprite_is_where_the_fiber_left_it() {
    assert_eq!(eval(SCRIPT), FRAMES);
}

#[test]
fn the_frames_are_the_same_under_collector_stress() {
    let stressed = format!("GC.stress = true\nbegin\n{SCRIPT}\nensure\n  GC.stress = false\nend");
    assert_eq!(eval(&stressed), FRAMES);
}

#[test]
fn the_script_sees_the_language_of_ruby_1_9_2() {
    assert_eq!(
        eval(
            "[RUBY_VERSION, \"\u{e9}\".encoding.name, ->(n) { n + 1 }.(1), { a: 1 }.keys, \
             Fiber.new { Fiber.yield 1; 2 }.resume]"
        ),
        r#"["1.9.2", "UTF-8", 2, [:a], 1]"#
    );
}
