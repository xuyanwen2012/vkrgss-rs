//! The method surface: every method the original runtime defines for the classes implemented
//! so far exists with the arity the original has.
//!
//! `data/rgss1_api.txt` is the method list read from the original runtime, one entry per
//! line: `Class#method arity` for an instance method, `Class.method arity` for a class-level
//! one, where -1 stands for any number of arguments.

use vkxp_binding::testing;

const METHOD_LIST: &str = include_str!("data/rgss1_api.txt");

/// The classes whose entries are checked. Later binding changes extend this until it is
/// every class of the list.
const IMPLEMENTED: &[&str] = &["Color", "Tone", "Rect", "Table", "Font"];

/// One entry of the method list.
#[derive(Debug, PartialEq, Eq)]
struct Entry<'a> {
    class: &'a str,
    /// Whether the method is called on the class itself.
    class_level: bool,
    method: &'a str,
    arity: i32,
}

fn parse(list: &str) -> Vec<Entry<'_>> {
    list.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (name, arity) = line
                .rsplit_once(' ')
                .unwrap_or_else(|| panic!("no arity in `{line}`"));
            let separator = name
                .find(['#', '.'])
                .unwrap_or_else(|| panic!("no method in `{line}`"));
            Entry {
                class: &name[..separator],
                class_level: name.as_bytes()[separator] == b'.',
                method: &name[separator + 1..],
                arity: arity
                    .parse()
                    .unwrap_or_else(|_| panic!("the arity of `{line}` is not a number")),
            }
        })
        .collect()
}

/// What is wrong with the entries of `classes` in the running interpreter, one line each.
fn mismatches(entries: &[Entry<'_>], classes: &[&str]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| classes.contains(&entry.class))
        .filter_map(|entry| {
            let lookup = if entry.class_level {
                "method"
            } else {
                "instance_method"
            };
            let source = format!("{}.{lookup}(:{:?}).arity", entry.class, entry.method);
            let name = format!(
                "{}{}{}",
                entry.class,
                if entry.class_level { '.' } else { '#' },
                entry.method
            );
            match testing::eval(&source) {
                Ok(arity) if arity == entry.arity.to_string() => None,
                Ok(arity) => Some(format!(
                    "{name} has arity {arity}, the list says {}",
                    entry.arity
                )),
                Err(error) => Some(format!("{name} is missing: {}", error.message)),
            }
        })
        .collect()
}

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
