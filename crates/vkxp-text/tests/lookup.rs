//! Font sources, family lookup, substitutions and the availability query.

mod common;

use common::{
    LIBERATION, LIBERATION_FILE, SYMBOLS, SYMBOLS_FILE, copy_font, engine, request,
    resolved_family, test_sources,
};
use vkxp_text::{FontSources, SystemFonts, TextEngine, TextRequest, default_substitutions};

/// An engine on the test fonts whose only fallback family is Noto Sans Symbols 2, so that a
/// lookup that wrongly ends in the fallback does not return Liberation Sans by accident.
fn engine_with_symbols_fallback() -> TextEngine {
    TextEngine::new(&FontSources {
        fallback_families: Some(vec![SYMBOLS.to_owned()]),
        ..test_sources()
    })
}

// Requirement: Font sources

#[test]
fn caller_directory_only() {
    let directory = tempfile::tempdir().unwrap();
    copy_font(directory.path(), LIBERATION_FILE);
    let engine = TextEngine::new(&FontSources::directories_only([directory.path()]));

    assert_eq!(engine.families(), [LIBERATION]);
    assert!(engine.is_available(LIBERATION));
    assert!(!engine.is_available(SYMBOLS));
}

#[test]
fn caller_directory_wins() {
    let system = tempfile::tempdir().unwrap();
    let caller = tempfile::tempdir().unwrap();
    let system_file = copy_font(system.path(), LIBERATION_FILE);
    let system_symbols = copy_font(system.path(), SYMBOLS_FILE);
    let caller_file = copy_font(caller.path(), LIBERATION_FILE);
    let system_only = FontSources {
        system: SystemFonts::Directories(vec![system.path().to_owned()]),
        ..FontSources::default()
    };
    let both = FontSources {
        directories: vec![caller.path().to_owned()],
        ..system_only.clone()
    };
    let path_of = |sources: &FontSources, family: &str| {
        let face = TextEngine::new(sources).resolve(&request(family, 22));
        face.unwrap().path.canonicalize().unwrap()
    };

    assert_eq!(path_of(&system_only, LIBERATION), system_file);
    assert_eq!(path_of(&both, LIBERATION), caller_file);
    // A family the caller does not have still comes from the system.
    assert_eq!(path_of(&both, SYMBOLS), system_symbols);
    assert_eq!(TextEngine::new(&both).families(), [LIBERATION, SYMBOLS]);

    let mut engine = TextEngine::new(&both);
    let mask = engine.rasterize(&request(LIBERATION, 22), "Text");
    assert!(mask.coverage.iter().any(|&value| value != 0));
}

#[test]
fn engine_without_fonts_does_not_fail() {
    let directory = tempfile::tempdir().unwrap();
    let mut engine = TextEngine::new(&FontSources::directories_only([directory.path()]));

    assert!(engine.families().is_empty());
    assert_eq!(engine.resolve(&request(LIBERATION, 22)), None);
    assert_eq!(engine.measure(&request(LIBERATION, 22), "Text"), (0, 22));
    let mask = engine.rasterize(&request(LIBERATION, 22), "Text");
    assert_eq!((mask.width, mask.height, mask.coverage.len()), (0, 22, 0));
}

// Requirement: Family lookup

#[test]
fn first_available_name() {
    let mut engine = engine_with_symbols_fallback();
    assert_eq!(
        resolved_family(&mut engine, &["NoSuchFont", LIBERATION]),
        LIBERATION
    );
    assert_eq!(
        resolved_family(&mut engine, &[LIBERATION, SYMBOLS]),
        LIBERATION
    );
    let mut engine = common::engine();
    assert_eq!(
        resolved_family(&mut engine, &["NoSuchFont", SYMBOLS]),
        SYMBOLS
    );
}

#[test]
fn substitution() {
    let mut engine = engine_with_symbols_fallback();
    assert!(!engine.families().iter().any(|family| family == "Arial"));
    assert_eq!(resolved_family(&mut engine, &["Arial"]), LIBERATION);
    // A substitute of an earlier name is preferred to a later name that is installed.
    assert_eq!(
        resolved_family(&mut engine, &["Arial", SYMBOLS]),
        LIBERATION
    );
}

#[test]
fn case() {
    let mut engine = engine_with_symbols_fallback();
    assert_eq!(
        resolved_family(&mut engine, &["liberation sans"]),
        LIBERATION
    );
    assert_eq!(
        resolved_family(&mut engine, &["LIBERATION SANS"]),
        LIBERATION
    );
    assert_eq!(resolved_family(&mut engine, &["arial"]), LIBERATION);
    let mut engine = common::engine();
    assert_eq!(
        resolved_family(&mut engine, &["noto sans SYMBOLS 2"]),
        SYMBOLS
    );
}

#[test]
fn nothing_matches() {
    let draws = |engine: &mut TextEngine| {
        let request = TextRequest::new(["NoSuchFont", "NoSuchFontEither"], 22);
        let (width, height) = engine.measure(&request, "Text");
        let mask = engine.rasterize(&request, "Text");
        assert!(width > 0);
        assert_eq!((mask.width, mask.height), (width, height));
        assert!(mask.coverage.iter().any(|&value| value != 0));
    };

    // The default fallback families include Liberation Sans.
    let mut engine = engine();
    assert_eq!(resolved_family(&mut engine, &["NoSuchFont"]), LIBERATION);
    draws(&mut engine);

    // The caller's fallback families replace the default ones.
    let mut engine = engine_with_symbols_fallback();
    assert_eq!(resolved_family(&mut engine, &["NoSuchFont"]), SYMBOLS);

    // Even when no fallback family is installed, some font is used.
    let mut engine = TextEngine::new(&FontSources {
        fallback_families: Some(vec!["NoSuchFallback".to_owned()]),
        ..test_sources()
    });
    draws(&mut engine);
    let mut engine = TextEngine::new(&FontSources {
        fallback_families: Some(Vec::new()),
        ..test_sources()
    });
    draws(&mut engine);
}

// Requirement: Default substitutions

#[test]
fn every_required_family_has_a_built_in_entry() {
    let required = [
        "Arial",
        "Tahoma",
        "Verdana",
        "Times New Roman",
        "Courier New",
        "MS Gothic",
        "MS PGothic",
        "MS Mincho",
        "SimSun",
        "宋体",
        "SimHei",
        "黑体",
        "KaiTi",
        "楷体",
        "楷体_GB2312",
        "Microsoft YaHei",
        "微软雅黑",
    ];
    for family in required {
        assert!(
            !default_substitutions(family).is_empty(),
            "no built-in substitution for {family}"
        );
        assert!(
            !default_substitutions(&family.to_uppercase()).is_empty(),
            "the entry for {family} depends on letter case"
        );
    }
    assert!(default_substitutions("NoSuchFont").is_empty());
}

#[test]
fn chinese_family_name() {
    assert!(default_substitutions("黑体").contains(&"Noto Sans CJK SC"));
    assert!(default_substitutions("楷体_GB2312").contains(&"Noto Sans CJK SC"));
    assert!(default_substitutions("宋体").contains(&"Noto Serif CJK SC"));
    assert_eq!(default_substitutions("Arial")[0], LIBERATION);
}

#[test]
fn caller_entry_takes_precedence() {
    let mut engine = TextEngine::new(&FontSources {
        substitutions: vec![("ARIAL".to_owned(), vec![SYMBOLS.to_owned()])],
        ..test_sources()
    });
    assert_eq!(resolved_family(&mut engine, &["Arial"]), SYMBOLS);

    // A caller entry whose families are missing leaves the built-in replacements in place.
    let mut engine = TextEngine::new(&FontSources {
        substitutions: vec![("Arial".to_owned(), vec!["NoSuchFont".to_owned()])],
        fallback_families: Some(vec![SYMBOLS.to_owned()]),
        ..test_sources()
    });
    assert_eq!(resolved_family(&mut engine, &["Arial"]), LIBERATION);

    // The caller can add names the built-in table does not know.
    let engine = TextEngine::new(&FontSources {
        substitutions: vec![("My Game Font".to_owned(), vec![LIBERATION.to_owned()])],
        ..test_sources()
    });
    assert!(engine.is_available("my game font"));
}

// Requirement: Availability query

#[test]
fn substituted_name_counts_as_available() {
    let engine = engine();
    assert!(!engine.families().iter().any(|family| family == "Arial"));
    assert!(engine.is_available("Arial"));
    assert!(engine.is_available("arial"));
    assert!(engine.is_available(LIBERATION));
}

#[test]
fn unknown_name() {
    let engine = engine();
    assert!(!engine.is_available("NoSuchFont"));
    // Substitutable, but none of its replacements is among the test fonts.
    assert!(!engine.is_available("黑体"));
    assert!(!engine.is_available(""));
}
