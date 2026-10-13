//! The method surface of the scene classes in a game of RGSS2. A binary of its own, because
//! the classes are defined once per process, as the runtime of the run has them.
//!
//! `data/rgss2_api.txt` is the method list of `Sprite`, `Viewport`, `Window`, `Plane`,
//! `Tilemap` and `Tilemap::BitmapArray` as measured on the original runtime of VX, in the
//! form of `data/rgss1_api.txt`. The other classes of RGSS2 are not in it yet.

mod method_list;

use method_list::{Entry, mismatches, parse};
use vkxp_binding::testing;
use vkxp_vfs::Runtime;

const METHOD_LIST: &str = include_str!("data/rgss2_api.txt");

/// The classes of the list.
const CLASSES: &[&str] = &[
    "Plane",
    "Sprite",
    "Tilemap",
    "Tilemap::BitmapArray",
    "Viewport",
    "Window",
];

/// The entries of the list that are not implemented yet, by class and method; a property
/// stands here with its reader and its writer. Each comes with the change that implements
/// its drawing, which takes it off this list: the test fails when one of them exists.
const PENDING: &[(&str, &str)] = &[
    ("Sprite", "bush_opacity"),
    ("Sprite", "bush_opacity="),
    ("Sprite", "wave_amp"),
    ("Sprite", "wave_amp="),
    ("Sprite", "wave_length"),
    ("Sprite", "wave_length="),
    ("Sprite", "wave_speed"),
    ("Sprite", "wave_speed="),
    ("Sprite", "wave_phase"),
    ("Sprite", "wave_phase="),
    ("Tilemap", "bitmaps"),
    ("Tilemap", "passages"),
    ("Tilemap", "passages="),
    ("Tilemap::BitmapArray", "[]"),
    ("Tilemap::BitmapArray", "[]="),
];

/// The methods RGSS2 lacks and vkxp still defines in a game of RGSS2, until the tilemap of
/// RGSS2 takes their place. The test fails when one of them is gone, and when a class has a
/// public method that is neither in the list nor here.
const NOT_IN_RGSS2: &[(&str, &str)] = &[
    ("Tilemap", "tileset"),
    ("Tilemap", "tileset="),
    ("Tilemap", "autotiles"),
    ("Tilemap", "priorities"),
    ("Tilemap", "priorities="),
];

/// The class RGSS2 lacks and vkxp still defines there, with `NOT_IN_RGSS2`.
const CLASS_NOT_IN_RGSS2: &str = "TilemapAutotiles";

fn eval(source: &str) -> String {
    testing::start_as(Runtime::Rgss2);
    match testing::eval(source) {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

fn is_pending(entry: &Entry<'_>) -> bool {
    PENDING.contains(&(entry.class, entry.method))
}

#[test]
fn the_method_list_parses() {
    let entries = parse(METHOD_LIST);
    assert_eq!(entries.len(), 168);
    assert_eq!(entries.iter().filter(|entry| entry.arity == -1).count(), 5);
    assert!(entries.iter().all(|entry| !entry.class_level));
    for class in CLASSES {
        assert!(
            entries.iter().any(|entry| entry.class == *class),
            "the list has no class {class}"
        );
    }
    for entry in &entries {
        assert!(CLASSES.contains(&entry.class), "{}", entry.class);
    }
    for (class, method) in PENDING {
        assert!(
            entries
                .iter()
                .any(|entry| entry.class == *class && entry.method == *method),
            "{class}#{method} is pending and not an entry of the list"
        );
    }
}

/// Scenario "List of RGSS2": every entry that is not pending exists with its arity.
#[test]
fn every_listed_method_that_is_not_pending_has_the_listed_arity() {
    testing::start_as(Runtime::Rgss2);
    let mut entries = parse(METHOD_LIST);
    entries.retain(|entry| !is_pending(entry));
    let wrong = mismatches(&entries, CLASSES);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Scenario "List of RGSS2": none of the pending entries exists.
#[test]
fn no_pending_method_exists() {
    testing::start_as(Runtime::Rgss2);
    let mut entries = parse(METHOD_LIST);
    entries.retain(is_pending);
    let missing = mismatches(&entries, CLASSES);
    let existing: Vec<String> = entries
        .iter()
        .map(|entry| format!("{}#{}", entry.class, entry.method))
        .filter(|name| {
            !missing
                .iter()
                .any(|line| line.starts_with(&format!("{name} is missing")))
        })
        .collect();
    assert!(
        existing.is_empty(),
        "implemented, to be taken off the pending list: {}",
        existing.join(", ")
    );
}

#[test]
fn the_names_rgss2_lacks_are_exactly_the_recorded_ones() {
    for (class, method) in NOT_IN_RGSS2 {
        assert_eq!(
            eval(&format!("{class}.method_defined?(:{method:?})")),
            "true",
            "{class}#{method} is gone: take it off the list"
        );
    }
    assert_eq!(
        eval(&format!("Object.const_defined?(:{CLASS_NOT_IN_RGSS2})")),
        "true"
    );
    let entries = parse(METHOD_LIST);
    for class in CLASSES.iter().filter(|class| !class.contains("::")) {
        let mut known: Vec<&str> = entries
            .iter()
            .filter(|entry| entry.class == *class)
            .map(|entry| entry.method)
            .collect();
        known.extend(
            NOT_IN_RGSS2
                .iter()
                .filter(|(of, _)| of == class)
                .map(|(_, method)| *method),
        );
        let known: Vec<String> = known.iter().map(|method| format!("{method:?}")).collect();
        assert_eq!(
            eval(&format!(
                "({class}.public_instance_methods(false) - [{}]).sort",
                known.join(", ")
            )),
            "[]",
            "{class} has methods the list of RGSS2 does not have"
        );
    }
}

/// `Viewport#disposed?`, `Window#stretch`, `Plane#viewport=` and the reader of a plane's
/// viewport are what the original runtime of VX does not have.
#[test]
fn what_the_original_does_not_have_is_not_defined() {
    assert_eq!(
        eval(
            "[Viewport.method_defined?(:disposed?), Window.method_defined?(:stretch), \
              Window.method_defined?(:stretch=), Plane.method_defined?(:viewport=)]"
        ),
        "[false, false, false, false]"
    );
}
