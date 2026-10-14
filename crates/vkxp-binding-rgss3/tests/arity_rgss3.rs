//! The method surface in a game of RGSS3. A file of this package alone, because the classes
//! are defined once per process, as the runtime of the run has them, and RGSS3 runs over
//! Ruby 1.9.2 only.
//!
//! `data/rgss3_api.txt` of `vkxp-binding` is the method list read from the original runtime
//! of VX Ace by reflection (2026-10-13, the same on its versions 3.0.0.1 and 3.0.1.1), in the
//! form of `data/rgss1_api.txt`: every class and module of RGSS3 that has methods, a property
//! as its reader and its writer. The test checks the classes of [`BOUND`], those whose
//! surface of RGSS3 is bound; a change that binds another class adds it there.
#![cfg(not(vkxp_interpreter_absent))]
// The method list module of `vkxp-binding` names the binding by that name.
extern crate vkxp_binding_rgss3 as vkxp_binding;

#[path = "../../vkxp-binding/tests/method_list/mod.rs"]
mod method_list;

use method_list::{mismatches, parse};
use vkxp_binding::testing;
use vkxp_vfs::Runtime;

const METHOD_LIST: &str = include_str!("../../vkxp-binding/tests/data/rgss3_api.txt");

/// The classes and modules of the list.
const CLASSES: &[&str] = &[
    "Graphics",
    "Audio",
    "Input",
    "Sprite",
    "Plane",
    "Viewport",
    "Tilemap",
    "Tilemap::BitmapArray",
    "Window",
    "Bitmap",
    "Font",
    "Color",
    "Tone",
    "Rect",
    "Table",
    "Win32API",
];

/// The classes whose surface of RGSS3 is bound, which the test checks.
const BOUND: &[&str] = &[
    "Color",
    "Tone",
    "Rect",
    "Table",
    "Window",
    "Sprite",
    "Plane",
    "Viewport",
    "Tilemap",
    "Tilemap::BitmapArray",
];

fn eval(source: &str) -> String {
    testing::start_as(Runtime::Rgss3);
    match testing::eval(source) {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

/// Scenario "The whole list".
#[test]
fn the_method_list_parses() {
    for line in METHOD_LIST.lines() {
        let (name, arity) = line.rsplit_once(' ').expect("an entry has an arity");
        assert!(name.contains(['#', '.']) && !name.contains(' '), "`{line}`");
        assert!(arity.parse::<i32>().is_ok(), "`{line}`");
    }
    let entries = parse(METHOD_LIST);
    assert_eq!(entries.len(), 344);
    assert_eq!(entries.iter().filter(|entry| entry.class_level).count(), 63);
    assert_eq!(entries.iter().filter(|entry| entry.arity == -1).count(), 30);
    for class in CLASSES {
        assert!(
            entries.iter().any(|entry| entry.class == *class),
            "the list has no class {class}"
        );
    }
    for entry in &entries {
        assert!(CLASSES.contains(&entry.class), "{}", entry.class);
    }
    // A property stands with its reader of arity 0 and its writer of arity 1.
    let is_writer = |method: &str| {
        method.strip_suffix('=').is_some_and(|reader| {
            !reader.is_empty()
                && reader
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
    };
    for writer in entries.iter().filter(|entry| is_writer(entry.method)) {
        let reader = &writer.method[..writer.method.len() - 1];
        assert_eq!(writer.arity, 1, "{}#{}", writer.class, writer.method);
        assert!(
            entries.iter().any(|entry| entry.class == writer.class
                && entry.class_level == writer.class_level
                && entry.method == reader
                && entry.arity == 0),
            "{}#{} has no reader",
            writer.class,
            writer.method
        );
    }
}

/// Scenario "Every listed method exists": the entries of the bound classes exist with their
/// arities.
#[test]
fn every_listed_method_of_a_bound_class_has_the_listed_arity() {
    testing::start_as(Runtime::Rgss3);
    let entries = parse(METHOD_LIST);
    let counts: Vec<usize> = BOUND
        .iter()
        .map(|class| entries.iter().filter(|entry| entry.class == *class).count())
        .collect();
    assert_eq!(counts, [17, 17, 18, 10, 53, 55, 29, 20, 21, 2]);
    let wrong = mismatches(&entries, BOUND);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert_eq!(
        eval("[Rect.instance_method(:initialize).arity, Rect.instance_method(:set).arity]"),
        "[-1, -1]"
    );
}

/// Scenario "Nothing beside the list": no bound class has a public method the list lacks.
#[test]
fn no_bound_class_has_a_method_the_list_lacks() {
    let entries = parse(METHOD_LIST);
    for class in BOUND {
        let known: Vec<String> = entries
            .iter()
            .filter(|entry| entry.class == *class && !entry.class_level)
            .map(|entry| format!(":{:?}", entry.method))
            .collect();
        assert_eq!(
            eval(&format!(
                "({class}.public_instance_methods(false) - [{}]).sort",
                known.join(", ")
            )),
            "[]",
            "{class} has methods the list of RGSS3 does not have"
        );
    }
}
