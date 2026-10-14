//! Smoke test on the fonts installed on the machine.
//!
//! This is the only test that looks at system fonts. What is installed is unknown, so it
//! checks that nothing fails and that results are consistent, not what the text looks like.

use vkrgss_text::{FontSources, TextEngine, TextRequest};

#[test]
fn system_fonts_measure_and_rasterize() {
    let mut engine = TextEngine::new(&FontSources::system());
    let has_fonts = !engine.families().is_empty();

    // The requests of the test game.
    let requests = [
        vec!["楷体_GB2312", "黑体"],
        vec!["黑体"],
        vec!["Arial"],
        vec!["NoSuchFont"],
    ];
    for families in &requests {
        for family in families {
            engine.is_available(family);
        }
        for size in [12, 22, 52] {
            for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
                let request = TextRequest {
                    bold,
                    italic,
                    ..TextRequest::new(families.iter().copied(), size)
                };
                assert_eq!(engine.resolve(&request).is_some(), has_fonts);
                for text in [
                    "Hello, World! 0123",
                    "你好，世界。开始游戏",
                    "A中\u{2BC5}\u{10FFFF}",
                ] {
                    let (width, height) = engine.measure(&request, text);
                    let layout = engine.layout(&request, text);
                    let mask = engine.rasterize(&request, text);
                    if has_fonts {
                        assert_eq!((width, height), (layout.width(), layout.height));
                    } else {
                        assert_eq!((width, height), (0, 0));
                    }
                    let drawn = width + layout.right_overhang;
                    assert_eq!((mask.width, mask.height), (drawn, height));
                    assert_eq!(mask.coverage.len(), (drawn * height) as usize);
                    assert_eq!(engine.rasterize(&request, text), mask);
                }
            }
        }
    }
}
