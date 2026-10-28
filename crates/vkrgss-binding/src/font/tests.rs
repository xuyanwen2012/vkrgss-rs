//! Tests of the shared accessors and the RGSS3 objects of `Font` that need no interpreter.

use super::rgss3::{chosen_element, cut_name};

/// Scenario "The name drawn with": the choice of [`drawn_name`] without the interpreter,
/// with the elements of a list as [`asked_name`] gives them and every other object as
/// its `to_s`.
#[test]
fn the_name_drawn_with_is_chosen_from_the_object() {
    let exists = |name: &str| ["Test Sans", "Liberation Sans"].contains(&name);
    let name = |text: &str| Some(text.to_owned());
    let from_list = |names: &[Option<String>], texts: &[&str]| {
        chosen_element(names, exists).and_then(|index| cut_name(texts[index]))
    };
    assert_eq!(
        from_list(
            &[name("Nope"), name("Test Sans"), name("Liberation Sans")],
            &["Nope", "Test Sans", "Liberation Sans"]
        ),
        name("Test Sans")
    );
    assert_eq!(
        from_list(&[name("Nope"), name("Other")], &["Nope", "Other"]),
        name("Nope")
    );
    // An Integer is no name: it does not exist, and its `to_s` is used only when it is
    // the first and nothing exists.
    assert_eq!(
        from_list(&[None, name("Test Sans")], &["1", "Test Sans"]),
        name("Test Sans")
    );
    assert_eq!(from_list(&[None, name("Nope")], &["1", "Nope"]), name("1"));
    assert_eq!(from_list(&[], &[]), None);
    // A Symbol by its name, a String as it is, `nil` by its empty `to_s`.
    assert_eq!(cut_name("Test Sans"), name("Test Sans"));
    assert_eq!(cut_name("test sans"), name("test sans"));
    assert_eq!(cut_name(""), None);
    let long = format!("Test Sans{}", " ".repeat(30));
    assert_eq!(
        cut_name(&long),
        Some(format!("Test Sans{}", " ".repeat(22)))
    );
    assert_eq!(
        cut_name(&"ゴ".repeat(40)).map(|cut| cut.chars().count()),
        Some(31)
    );
}
