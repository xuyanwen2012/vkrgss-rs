//! The relative links and images of the public documents name files that exist.

use std::fs;
use std::path::{Path, PathBuf};

/// The documents that are checked besides every `docs/*.md`.
const DOCUMENTS: [&str; 5] = [
    "README.md",
    "CHANGELOG.md",
    "CONTRIBUTING.md",
    "THIRD-PARTY.md",
    "AGENTS.md",
];

/// The targets of the Markdown links and images in `text` that are neither a URL nor a pure
/// `#fragment`, without the fragment after the path.
fn relative_targets(text: &str) -> Vec<&str> {
    text.split("](")
        .skip(1)
        .filter_map(|rest| {
            let (target, _) = rest.split_once(')')?;
            let target = target.split_whitespace().next()?;
            let (path, _) = target.split_once('#').unwrap_or((target, ""));
            let is_url = target.contains("://") || target.starts_with("mailto:");
            (!is_url && !path.is_empty()).then_some(path)
        })
        .collect()
}

/// The links of `document`, a path below `root`, that name no file or directory, each as the
/// document and the target.
fn broken_links(root: &Path, document: &str) -> Vec<(String, String)> {
    let file = root.join(document);
    let text = fs::read_to_string(&file).unwrap_or_else(|error| panic!("{document}: {error}"));
    let dir = file.parent().unwrap();
    relative_targets(&text)
        .into_iter()
        .filter(|target| !dir.join(target).exists())
        .map(|target| (document.to_owned(), target.to_owned()))
        .collect()
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}

#[test]
fn the_links_of_the_documents_resolve() {
    let root = repository();
    let mut documents: Vec<String> = DOCUMENTS.iter().map(|&name| name.to_owned()).collect();
    for entry in fs::read_dir(root.join("docs")).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if name.ends_with(".md") {
            documents.push(format!("docs/{name}"));
        }
    }
    assert!(documents.len() > DOCUMENTS.len(), "no document under docs");

    let broken: Vec<(String, String)> = documents
        .iter()
        .flat_map(|document| broken_links(&root, document))
        .collect();
    assert!(broken.is_empty(), "links to nothing: {broken:?}");
}

#[test]
fn a_broken_link_is_reported_with_its_document() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("docs")).unwrap();
    fs::write(root.path().join("docs/present.md"), "").unwrap();
    fs::write(
        root.path().join("README.md"),
        "[here](docs/present.md#part), [gone](docs/missing.md), ![picture](docs), \
         [top](#top) and [site](https://example.org/docs/other.md).\n",
    )
    .unwrap();

    assert_eq!(
        broken_links(root.path(), "README.md"),
        [("README.md".to_owned(), "docs/missing.md".to_owned())]
    );
}

#[test]
fn a_link_is_relative_to_its_document() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("docs")).unwrap();
    fs::write(root.path().join("LICENSE"), "").unwrap();
    fs::write(
        root.path().join("docs/guide.md"),
        "[licence](../LICENSE) and [licence](LICENSE)\n",
    )
    .unwrap();

    assert_eq!(
        broken_links(root.path(), "docs/guide.md"),
        [("docs/guide.md".to_owned(), "LICENSE".to_owned())]
    );
}
