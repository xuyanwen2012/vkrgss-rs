//! The committed method lists and how an entry is checked: what `arity.rs` and
//! `arity_rgss2.rs` share.
//!
//! A list has one entry per line: `Class#method arity` for an instance method,
//! `Class.method arity` for a class-level one, where -1 stands for any number of arguments.

#![allow(dead_code)]

use vkrgss_binding::testing::Harness;

/// One entry of the method list.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Entry<'a> {
    pub class: &'a str,
    /// Whether the method is called on the class itself.
    pub class_level: bool,
    pub method: &'a str,
    pub arity: i32,
}

pub(crate) fn parse(list: &str) -> Vec<Entry<'_>> {
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

/// What is wrong with the entries of `classes` in the interpreter of `harness`, one line
/// each.
pub(crate) fn mismatches(
    harness: &Harness,
    entries: &[Entry<'_>],
    classes: &[&str],
) -> Vec<String> {
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
            match harness.eval(&source) {
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
