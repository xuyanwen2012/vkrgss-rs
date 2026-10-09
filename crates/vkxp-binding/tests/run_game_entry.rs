//! `run_game` itself, in a process whose interpreter has no guest yet: the one call that
//! installs the binding and plays a game from start to shutdown.

use std::fs;

use vkxp_binding::testing::host::{RPG_STAND_IN, Script, ruby_literal, scripted_host};
use vkxp_binding::testing::{CellBackend, FONT_FAMILIES};
use vkxp_binding::{Config, RunConfig, run_game};
use vkxp_proto::GuestToHost;
use vkxp_ruby_sys::{Interpreter, Options};
use vkxp_vfs::{GameFiles, GameIni};

#[test]
fn one_call_plays_a_game_from_start_to_shutdown() {
    let executable = std::env::current_exe().unwrap();
    let root = executable.with_extension("vkxp-run-game");
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    let game = root.join("game");
    fs::create_dir_all(game.join("Data")).unwrap();
    fs::write(
        game.join("Game.ini"),
        "[Game]\r\nScripts=Data\\Scripts.rxdata\r\nTitle=Entry\r\n",
    )
    .unwrap();
    let builtin_scripts = root.join("rpg.rb");
    fs::write(&builtin_scripts, RPG_STAND_IN).unwrap();
    let result = root.join("result.txt");
    let main = format!(
        "bitmap = Bitmap.new(2, 2)
         Graphics.update
         File.open({}, 'wb') {{ |file| file.write([defined?(RPG::Cache), bitmap.width].inspect) }}",
        ruby_literal(&result.to_string_lossy())
    );
    let assemble = format!(
        "File.open({}, 'wb') {{ |file| Marshal.dump([[1, 'Main', Zlib::Deflate.deflate({})]], file) }}",
        ruby_literal(&game.join("Data/Scripts.rxdata").to_string_lossy()),
        ruby_literal(&main)
    );

    let config = Config {
        files: GameFiles::new(game.clone(), []),
        font_names: vec![FONT_FAMILIES[0].to_owned()],
        text: Box::new(CellBackend::new(FONT_FAMILIES.iter().copied())),
        run: RunConfig {
            ini: GameIni::read(&game, None).unwrap(),
            builtin_scripts,
            ..RunConfig::default()
        },
    };
    let (link, host) = scripted_host(Script::new());
    let interpreter = Interpreter::start(Options::default()).unwrap();
    let outcome = interpreter.run(move |ruby| {
        // The archive is made with the interpreter's own `Marshal` and `Zlib`.
        ruby.eval_protected(&assemble, "test", 1).unwrap();
        run_game(ruby, config, link)
    });
    assert!(outcome.is_ok(), "{outcome:?}");

    let record = host.finish();
    assert_eq!(
        record.received.first(),
        Some(&GuestToHost::SetTitle {
            title: "Entry".to_owned()
        })
    );
    assert_eq!(record.frames().len(), 1);
    assert_eq!(record.received.last(), Some(&GuestToHost::Shutdown));
    assert_eq!(fs::read_to_string(result).unwrap(), r#"["constant", 2]"#);
}
