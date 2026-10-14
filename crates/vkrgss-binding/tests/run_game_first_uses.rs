//! The names the runtime of RGSS2 interns at the first use of some functions, through
//! `run_game` in a process of its own, so that each call of the script is the first use: how
//! far the sequence of symbol numbers moves over each call was measured on the original
//! runtime of VX (RGSS202E 2.0.2.1, 2026-10-12).

use std::fs;

use vkrgss_binding::testing::host::{RPG_STAND_IN, Script, ruby_literal, scripted_host};
use vkrgss_binding::testing::{CellBackend, FONT_FAMILIES};
use vkrgss_binding::{Config, RunConfig, run_game};
use vkrgss_ruby_sys::{Interpreter, Options};
use vkrgss_vfs::{GameFiles, GameIni, Runtime};

/// The game's script. It is parsed before it runs, so its identifiers have their numbers by
/// then; the names noted are made with `String#to_sym`, and a step that interns nothing
/// moves the sequence by the one name noted after it.
const MAIN: &str = r#"
zz_steps = [
  proc { Color.new(0, 0, 0) },
  proc { Bitmap.new(1, 1) },
  proc { Sprite.new },
  proc { Win32API.new("kernel32", "GetTickCount", "", "L") },
  proc { $zz_tilemap = Tilemap.new },
  proc { $zz_tilemap.bitmaps[3] },
  proc { $zz_tilemap.bitmaps[3] = nil },
]
zz_index = 0
zz_before = "zz_name0".to_sym.to_i
zz_moved = zz_steps.map do |zz_step|
  zz_index += 1
  zz_step.call
  zz_after = ("zz_name" + zz_index.to_s).to_sym.to_i
  zz_beyond = ((zz_after - zz_before) >> 3) - 1
  zz_before = zz_after
  zz_beyond
end
File.open("moved.txt", "wb") { |zz_file| zz_file.write(zz_moved.inspect) }
"#;

/// Scenarios "First uses, measured on the original" and "Tilemap, measured on the original".
#[test]
fn the_first_uses_move_the_sequence_as_on_the_original() {
    vkrgss_binding::pending_over_ruby_1_9_2!();
    let executable = std::env::current_exe().unwrap();
    let root = executable.with_extension("vkrgss-run-game-first-uses");
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    let game = root.join("game");
    fs::create_dir_all(game.join("Data")).unwrap();
    fs::write(
        game.join("Game.ini"),
        "[Game]\r\nLibrary=RGSS202E.dll\r\nScripts=Data\\Scripts.rvdata\r\nTitle=Names\r\n",
    )
    .unwrap();
    let builtin_scripts = root.join("rpg.rb");
    fs::write(&builtin_scripts, RPG_STAND_IN).unwrap();
    let assemble = format!(
        "File.open({}, 'wb') {{ |file| Marshal.dump([[1, 'Main', Zlib::Deflate.deflate({})]], file) }}",
        ruby_literal(&game.join("Data/Scripts.rvdata").to_string_lossy()),
        ruby_literal(MAIN)
    );
    // The script names its file as a game does, from the game directory. This binary has
    // one test.
    std::env::set_current_dir(&game).unwrap();

    let config = Config {
        files: GameFiles::new(game.clone(), []),
        font_names: vec![FONT_FAMILIES[0].to_owned()],
        text: Box::new(CellBackend::new(FONT_FAMILIES.iter().copied())),
        run: RunConfig {
            ini: GameIni::read(&game, None).unwrap(),
            runtime: Runtime::Rgss2,
            builtin_scripts,
            ..RunConfig::default()
        },
    };
    let (link, host) = scripted_host(Script::new());
    let options = Options::for_rgss(2);
    let interpreter = Interpreter::start(options).unwrap();
    let outcome = interpreter.run(move |ruby| {
        // The archive is made with the interpreter's own `Marshal` and `Zlib`.
        ruby.eval_protected(&assemble, "test", 1).unwrap();
        run_game(ruby, config, link)
    });
    assert!(outcome.is_ok(), "{outcome:?}");
    host.finish();
    // `Color.new`, `Bitmap.new`, `Sprite.new`, `Win32API.new`; then `Tilemap.new`, the read
    // of a slot and the write of the same slot.
    assert_eq!(
        fs::read_to_string(game.join("moved.txt")).unwrap(),
        "[0, 2, 0, 6, 0, 1, 0]"
    );
}
