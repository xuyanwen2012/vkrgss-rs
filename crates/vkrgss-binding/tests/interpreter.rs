//! What one test file can assert of both interpreters: which Ruby the harness runs, and what
//! the preparation defines over either.

use vkrgss_binding::testing::host::{self, Game, Script};
use vkrgss_binding::testing::{self, CellBackend, Harness};
use vkrgss_binding::{Config, RunError};
use vkrgss_ruby_sys::{RUBY_API, RubyApi};
use vkrgss_vfs::{GameFiles, Runtime};

/// The harness, started as a game of RGSS1.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss1)
}

/// The value of a snippet evaluated after the preparation, or its exception as
/// `Class: message`.
fn prepared(source: &str) -> String {
    match host::play(harness(), source, Script::new()).0 {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

#[test]
fn the_interpreter_is_the_one_its_crate_names() {
    let expected = match RUBY_API {
        RubyApi::Ruby18 => r#""1.8.7""#,
        RubyApi::Ruby19 => r#""1.9.2""#,
    };
    assert_eq!(harness().eval("RUBY_VERSION").as_deref(), Ok(expected));
}

#[test]
fn both_interpreters_get_reset_save_data_and_load_data() {
    assert_eq!(
        prepared("[Reset.superclass, defined?(save_data).nil?, defined?(load_data).nil?]"),
        "[Exception, false, false]"
    );
}

/// Requirement "Preparation of a game of RGSS3": over Ruby 1.8.7 `run_game` refuses a game
/// of RGSS3 before anything of it is evaluated; over Ruby 1.9.2 the binding runs the games
/// of the other runtimes too, as these tests do.
#[test]
fn a_game_of_rgss3_runs_over_ruby_1_9_2_only() {
    let xp = Game::new(
        harness(),
        "interpreter_xp",
        "XP",
        &[("Main", "$vkrgss_xp_ran = true")],
    );
    let played = host::run_game(
        harness(),
        &xp,
        xp.run_config(),
        Script::new(),
        "$vkrgss_xp_ran",
    );
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok("true"));
    if RUBY_API != RubyApi::Ruby18 {
        return;
    }

    let ace = Game::of(
        harness(),
        Runtime::Rgss3,
        "interpreter_ace",
        "Ace",
        &[("Main", "$vkrgss_ace_ran = true")],
    );
    let config = Config {
        files: GameFiles::new(ace.dir(), []),
        font_names: vec!["Test Sans".to_owned()],
        text: Box::new(CellBackend::new(["Test Sans"])),
        run: ace.run_config(),
    };
    let (link, scripted) = host::scripted_host(Script::new());
    let error = harness()
        .run(move |ruby| vkrgss_binding::run_game(ruby, config, link))
        .expect_err("a game of RGSS3 is refused over Ruby 1.8.7");
    assert!(matches!(error, RunError::Interpreter { .. }), "{error:?}");
    let message = error.to_string();
    assert!(message.contains("RPG Maker VX Ace"), "{message}");
    assert!(message.contains("Ruby 1.8.7"), "{message}");
    assert!(
        scripted.finish().received.is_empty(),
        "the host was told nothing"
    );
    assert_eq!(harness().eval("$vkrgss_ace_ran").as_deref(), Ok("nil"));
}
