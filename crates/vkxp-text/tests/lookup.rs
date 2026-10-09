//! Font sources, family lookup, substitutions and the availability query.

mod common;

use common::{
    CHINESE_PRC, Derivation, ENGLISH_US, LIBERATION, LIBERATION_FILE, NAME_FAMILY,
    NAME_TYPOGRAPHIC_FAMILY, SYMBOLS, SYMBOLS_FILE, copy_font, derive_font, engine, request,
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

#[test]
fn hidden_system_face_is_not_the_font_of_last_resort() {
    // The caller has only a bold face of the family; the system's regular face is hidden and
    // must not come back when no requested name and no fallback family matches.
    let system = tempfile::tempdir().unwrap();
    let caller = tempfile::tempdir().unwrap();
    copy_font(system.path(), LIBERATION_FILE);
    let change = Derivation {
        weight: Some(700),
        wider_by: 150,
        ..Derivation::default()
    };
    let caller_file = derive_font(LIBERATION_FILE, &caller.path().join("bold.ttf"), change);
    let mut engine = TextEngine::new(&FontSources {
        system: SystemFonts::Directories(vec![system.path().to_owned()]),
        directories: vec![caller.path().to_owned()],
        fallback_families: Some(Vec::new()),
        ..FontSources::default()
    });
    assert_eq!(engine.families(), [LIBERATION]);

    for family in [LIBERATION, "NoSuchFont"] {
        for bold in [false, true] {
            let request = TextRequest {
                bold,
                ..request(family, 22)
            };
            let face = engine.resolve(&request).unwrap();
            assert_eq!(face.path.canonicalize().unwrap(), caller_file, "{family}");
        }
    }
}

#[test]
fn hidden_system_face_does_not_supply_missing_characters() {
    // The caller's font of the family maps no character; the system's font of the same
    // family has them all, but is hidden and must not be used for per-character fallback.
    let system = tempfile::tempdir().unwrap();
    let caller = tempfile::tempdir().unwrap();
    copy_font(system.path(), LIBERATION_FILE);
    let change = Derivation {
        wider_by: 150,
        empty_cmap: true,
        ..Derivation::default()
    };
    let caller_file = derive_font(LIBERATION_FILE, &caller.path().join("caller.ttf"), change);
    let mut only_caller = TextEngine::new(&FontSources::directories_only([caller.path()]));
    let with_system = FontSources {
        system: SystemFonts::Directories(vec![system.path().to_owned()]),
        directories: vec![caller.path().to_owned()],
        ..FontSources::default()
    };
    let mut both = TextEngine::new(&with_system);
    let request = request(LIBERATION, 22);

    let face = both.resolve(&request).unwrap();
    assert_eq!(face.path.canonicalize().unwrap(), caller_file);
    for text in ["A", "Text 0123"] {
        let expected = only_caller.rasterize(&request, text);
        assert!(expected.width > 0);
        assert_eq!(both.measure(&request, text), (expected.width, 22));
        assert_eq!(both.rasterize(&request, text), expected, "{text:?}");
    }
    // The system's own drawing of the letter is a different one.
    assert_ne!(
        both.rasterize(&request, "A"),
        engine().rasterize(&request, "A")
    );

    // A system font of another family is not hidden and does supply characters.
    copy_font(system.path(), SYMBOLS_FILE);
    let mut both = TextEngine::new(&with_system);
    let symbol = common::SYMBOL.to_string();
    assert_ne!(
        both.rasterize(&request, &symbol),
        only_caller.rasterize(&request, &symbol)
    );
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

// Family names a font declares: typographic, legacy and localised

const TYPOGRAPHIC: &str = "Typographic Family";
const ALIAS: &str = "本地别名 Alias";

/// Liberation Sans with an English typographic family name (name ID 16) and a legacy family
/// name (name ID 1) in another language added. Its original legacy name stays.
fn aliased_font(directory: &std::path::Path) -> std::path::PathBuf {
    let change = Derivation {
        names: &[
            (NAME_TYPOGRAPHIC_FAMILY, ENGLISH_US, TYPOGRAPHIC),
            (NAME_FAMILY, CHINESE_PRC, ALIAS),
        ],
        ..Derivation::default()
    };
    derive_font(LIBERATION_FILE, &directory.join("aliased.ttf"), change)
}

#[test]
fn every_declared_family_name_is_available() {
    let directory = tempfile::tempdir().unwrap();
    aliased_font(directory.path());
    let engine = TextEngine::new(&FontSources::directories_only([directory.path()]));

    assert_eq!(engine.families(), [TYPOGRAPHIC]);
    for name in [
        TYPOGRAPHIC,
        ALIAS,
        LIBERATION,
        "本地别名 ALIAS",
        "typographic family",
    ] {
        assert!(engine.is_available(name), "{name}");
    }
    assert!(!engine.is_available("本地别名"));
    assert!(!engine.is_available(SYMBOLS));
}

#[test]
fn every_declared_family_name_resolves() {
    let directory = tempfile::tempdir().unwrap();
    let aliased = aliased_font(directory.path());
    let symbols = copy_font(directory.path(), SYMBOLS_FILE);
    // With Noto Sans Symbols 2 as the only fallback, a name that is not matched shows.
    let mut engine = TextEngine::new(&FontSources {
        fallback_families: Some(vec![SYMBOLS.to_owned()]),
        ..FontSources::directories_only([directory.path()])
    });
    let mut path_of = |families: &[&str]| {
        let face = engine.resolve(&TextRequest::new(families.iter().copied(), 22));
        face.unwrap().path.canonicalize().unwrap()
    };

    for name in [TYPOGRAPHIC, ALIAS, LIBERATION, "本地别名 alias"] {
        assert_eq!(path_of(&[name]), aliased, "{name}");
        assert_eq!(path_of(&["NoSuchFont", name]), aliased, "{name}");
    }
    // Arial is substituted by Liberation Sans, here only a legacy name of the font.
    assert_eq!(path_of(&["Arial"]), aliased);
    assert_eq!(path_of(&["NoSuchFont"]), symbols);

    let request = request(ALIAS, 22);
    let mask = engine.rasterize(&request, "Text");
    assert_eq!((mask.width, mask.height), engine.measure(&request, "Text"));
    assert!(mask.coverage.iter().any(|&value| value != 0));
    assert_eq!(
        mask,
        common::engine().rasterize(&common::request(LIBERATION, 22), "Text")
    );
}

#[test]
fn caller_directory_wins_under_every_declared_family_name() {
    let system = tempfile::tempdir().unwrap();
    let caller = tempfile::tempdir().unwrap();
    let system_file = aliased_font(system.path());
    let system_symbols = copy_font(system.path(), SYMBOLS_FILE);
    let caller_file = aliased_font(caller.path());
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

    for name in [TYPOGRAPHIC, ALIAS, LIBERATION] {
        assert_eq!(path_of(&system_only, name), system_file, "{name}");
        assert_eq!(path_of(&both, name), caller_file, "{name}");
        assert!(TextEngine::new(&both).is_available(name), "{name}");
    }
    assert_eq!(path_of(&both, SYMBOLS), system_symbols);
    assert_eq!(TextEngine::new(&both).families(), [SYMBOLS, TYPOGRAPHIC]);

    // The caller's plain Liberation Sans hides the system font under the one name they
    // share; the system font keeps the names only it declares.
    let plain = tempfile::tempdir().unwrap();
    let plain_file = copy_font(plain.path(), LIBERATION_FILE);
    let mixed = FontSources {
        directories: vec![plain.path().to_owned()],
        ..system_only.clone()
    };
    assert_eq!(path_of(&mixed, LIBERATION), plain_file);
    assert_eq!(path_of(&mixed, ALIAS), system_file);
    assert_eq!(path_of(&mixed, TYPOGRAPHIC), system_file);
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
