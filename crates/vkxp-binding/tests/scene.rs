//! The scenarios of `rgss-scene-classes`, written in Ruby and evaluated in the embedded
//! interpreter. The snippets are in `scene_cases`; what they see of the scene comes from
//! the helpers of `scene_support`.

mod scene_support;

use vkxp_binding::{scene_testing, testing};
use vkxp_core::FontDefaults;

#[test]
fn ruby_reads_the_pixel_of_a_sprite_made_in_rust() {
    scene_support::install();
    // In a corner of the screen that no snippet draws to.
    let (sprite, dot) = testing::run(|_ruby| {
        scene_testing::with_scene(|scene, bitmaps| {
            let defaults = FontDefaults::new(Vec::new());
            let dot = bitmaps
                .create_from_rgba(1, 1, vec![10, 20, 30, 255], &defaults)
                .unwrap();
            let sprite = scene.create_sprite(None).unwrap();
            let element = scene.sprite_mut(sprite).unwrap();
            element.set_bitmap(Some(dot), bitmaps);
            element.x = 639;
            element.y = 479;
            (sprite, dot)
        })
    });
    assert_eq!(
        testing::eval("[pixel(639, 479), pixel(640, 479), pixel(0, 480)]").as_deref(),
        Ok("[[10, 20, 30, 255], nil, nil]")
    );
    testing::run(move |_ruby| {
        scene_testing::with_scene(|scene, bitmaps| {
            scene.dispose_sprite(sprite);
            bitmaps.dispose(dot);
        });
    });
    assert_eq!(
        testing::eval("pixel(639, 479)").as_deref(),
        Ok("[0, 0, 0, 255]")
    );
}

#[test]
fn elements_are_counted_by_kind() {
    scene_support::install();
    assert_eq!(
        testing::eval("[scene_count(:sprite).class, scene_count(:tilemap).class, scene_count(:x)]")
            .as_deref(),
        Ok("[Fixnum, Fixnum, nil]")
    );
}
