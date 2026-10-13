//! The method surface: every method the original runtime defines for the classes implemented
//! so far exists with the arity the original has.
//!
//! `data/rgss1_api.txt` is the method list read from the original runtime, one entry per
//! line: `Class#method arity` for an instance method, `Class.method arity` for a class-level
//! one, where -1 stands for any number of arguments.

mod method_list;

use method_list::{Entry, mismatches, parse};

const METHOD_LIST: &str = include_str!("data/rgss1_api.txt");

/// The classes whose entries are checked. Later binding changes extend this until it is
/// every class of the list.
const IMPLEMENTED: &[&str] = &[
    "Color", "Tone", "Rect", "Table", "Font", "Bitmap", "Graphics", "Input", "Audio",
];

#[test]
fn the_method_list_parses() {
    let entries = parse(METHOD_LIST);
    assert_eq!(entries.len(), 286);
    assert_eq!(entries.iter().filter(|entry| entry.arity == -1).count(), 24);
    assert!(entries.contains(&Entry {
        class: "Color",
        class_level: true,
        method: "_load",
        arity: 1,
    }));
    for class in IMPLEMENTED {
        assert!(
            entries.iter().any(|entry| entry.class == *class),
            "the list has no class {class}"
        );
    }
}

#[test]
fn every_listed_method_of_the_implemented_classes_has_the_listed_arity() {
    let wrong = mismatches(&parse(METHOD_LIST), IMPLEMENTED);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_wrong_entry_is_reported() {
    // `String` stands in for an implemented class: the check itself is what is tested.
    let entries =
        parse("String#length 0\nString#length 3\nString#no_such_method 0\nString.new -1\n");
    assert_eq!(
        mismatches(&entries, &["String"]),
        [
            "String#length has arity 0, the list says 3",
            "String#no_such_method is missing: undefined method `no_such_method' for class `String'",
        ]
    );
}

/// The scene classes, which the change `add-rgss-scene-classes` added.
const SCENE_CLASSES: &[&str] = &[
    "Viewport",
    "Sprite",
    "Plane",
    "Window",
    "Tilemap",
    "TilemapAutotiles",
];

#[test]
fn every_listed_method_of_the_scene_classes_has_the_listed_arity() {
    let entries = parse(METHOD_LIST);
    for class in SCENE_CLASSES {
        assert!(
            entries.iter().any(|entry| entry.class == *class),
            "the list has no class {class}"
        );
    }
    let wrong = mismatches(&entries, SCENE_CLASSES);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
