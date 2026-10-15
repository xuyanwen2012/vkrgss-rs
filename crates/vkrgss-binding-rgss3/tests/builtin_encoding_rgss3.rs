//! The source encoding of the built-in scripts of RGSS3: the original evaluates them as
//! bytes, so a string literal of theirs is ASCII-8BIT, where a literal of a game script is
//! UTF-8. A save of the game shows it: the empty name of `RPG::BGS.new`, a default of the
//! built-in scripts, is dumped without an encoding. A binary of its own, because the
//! interpreter of a test process keeps what an earlier built-in scripts file defined.
#![cfg(not(vkrgss_interpreter_absent))]

use vkrgss_binding_rgss3::testing::host::{self, Game, Script};
use vkrgss_binding_rgss3::testing::{self, Harness};
use vkrgss_vfs::Runtime;

/// The harness, started as a game of RGSS3.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss3)
}

#[test]
fn a_literal_of_the_built_in_scripts_is_bytes_and_one_of_a_game_is_text() {
    let game = Game::of(
        harness(),
        Runtime::Rgss3,
        "builtin_encoding",
        "Ace",
        &[("Main", "$game_literal = ''")],
    );
    std::fs::write(
        game.builtin_scripts(),
        "module RPG\nend\n$builtin_literal = ''\n$builtin_dump = Marshal.dump(['', 'a'])\n",
    )
    .unwrap();
    // `load_data` of RGSS3 reads from the current directory, which the glue of a guest sets
    // to the game directory.
    std::env::set_current_dir(game.dir()).expect("the game directory is entered");
    let played = host::run_game(
        harness(),
        &game,
        game.run_config(),
        Script::new(),
        "[$builtin_literal.encoding.to_s, $game_literal.encoding.to_s, \
         $builtin_dump.unpack('H*')[0]]",
    );
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    // `["", "a"]` dumped without an encoding of either string: 04 08 5b 07 22 00 22 06 61.
    assert_eq!(
        played.after.as_deref(),
        Ok(r#"["ASCII-8BIT", "UTF-8", "04085b072200220661"]"#)
    );
}
