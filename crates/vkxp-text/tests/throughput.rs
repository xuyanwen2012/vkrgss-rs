//! Throughput check: redrawing a window full of text every frame must not be a concern.

mod common;

use std::time::{Duration, Instant};

use common::{LIBERATION, engine, request};

#[test]
fn a_thousand_short_strings_at_size_22() {
    let texts: Vec<String> = (0..1000)
        .map(|number| format!("Item {number}: {} HP", 9999 - number * 7))
        .collect();
    let mut engine = engine();
    let request = request(LIBERATION, 22);

    // Warm-up: every glyph is rendered once.
    for text in &texts {
        engine.rasterize(&request, text);
    }

    let start = Instant::now();
    let mut pixels = 0usize;
    for text in &texts {
        let (width, _) = engine.measure(&request, text);
        let mask = engine.rasterize(&request, text);
        assert_eq!(mask.width, width);
        pixels += mask.coverage.len();
    }
    let elapsed = start.elapsed();
    println!("1000 strings at size 22: {elapsed:?} for {pixels} mask pixels");

    // The bound is for optimised code; an unoptimised run only reports its time.
    if !cfg!(debug_assertions) {
        assert!(elapsed < Duration::from_secs(1), "took {elapsed:?}");
    }
}
