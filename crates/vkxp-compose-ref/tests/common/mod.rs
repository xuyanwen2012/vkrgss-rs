//! Helpers shared by the integration tests that compare against committed images.

use std::path::PathBuf;

/// Path of the committed expected image of a fixture.
pub(crate) fn expected_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/expected")
        .join(format!("{name}.png"))
}

/// The committed expected image of a fixture, as PNG bytes.
pub(crate) fn expected_png(name: &str) -> Vec<u8> {
    let path = expected_path(name);
    std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "cannot read {}: {error}\n\
             If the fixture is new, create its image with \
             `VKXP_BLESS=1 cargo test -p vkxp-compose-ref --test fixtures` and review it.",
            path.display()
        )
    })
}
