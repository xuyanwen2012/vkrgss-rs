//! A game of RGSS2 through `run_game`, in a process of its own, because the symbol table of
//! an interpreter is chosen when it starts and `Graphics` is defined once: the twin of
//! `run_game_symbols.rs`, and the frame a game of RGSS2 starts with.

use std::fs;

use vkxp_binding::testing::host::{RPG_STAND_IN, Script, ruby_literal, scripted_host};
use vkxp_binding::testing::{CellBackend, FONT_FAMILIES};
use vkxp_binding::{Config, RunConfig, run_game};
use vkxp_proto::Size;
use vkxp_ruby_sys::{Interpreter, Options};
use vkxp_vfs::{GameFiles, GameIni, Runtime};

/// Scenarios "First new names of a game of VX, measured on the original", "Known names of a
/// game of VX, measured on the original", "Starting rate of VX, measured on the original" and
/// "Size of VX, measured on the original".
#[test]
fn the_first_script_of_a_game_of_vx_sees_the_measured_numbers_and_frame() {
    let executable = std::env::current_exe().unwrap();
    let root = executable.with_extension("vkxp-run-game-rgss2");
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    let game = root.join("game");
    fs::create_dir_all(game.join("Data")).unwrap();
    fs::write(
        game.join("Game.ini"),
        "[Game]\r\nLibrary=RGSS202E.dll\r\nScripts=Data\\Scripts.rvdata\r\nTitle=Symbols\r\n",
    )
    .unwrap();
    let builtin_scripts = root.join("rpg.rb");
    fs::write(&builtin_scripts, RPG_STAND_IN).unwrap();
    let result = root.join("result.txt");
    let main = format!(
        "zz_first = 1; zz_second = 2
         File.open({}, 'wb') {{ |file| file.write([
           [:zz_first.to_i, :zz_second.to_i], [:initialize.to_i, :to_s.to_i, :update.to_i],
           Graphics.frame_rate, [Graphics.width, Graphics.height]
         ].inspect) }}
         Graphics.update
         Graphics.frame_rate = 5
         Graphics.update
         Graphics.frame_rate = 500
         Graphics.update",
        ruby_literal(&result.to_string_lossy())
    );
    let assemble = format!(
        "File.open({}, 'wb') {{ |file| Marshal.dump([[1, 'Main', Zlib::Deflate.deflate({})]], file) }}",
        ruby_literal(&game.join("Data/Scripts.rvdata").to_string_lossy()),
        ruby_literal(&main)
    );

    let ini = GameIni::read(&game, None).unwrap();
    let runtime = Runtime::detect(&ini).unwrap();
    assert_eq!(runtime, Runtime::Rgss2);
    let config = Config {
        files: GameFiles::new(game.clone(), []),
        font_names: vec![FONT_FAMILIES[0].to_owned()],
        text: Box::new(CellBackend::new(FONT_FAMILIES.iter().copied())),
        run: RunConfig {
            ini,
            runtime,
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
    let record = host.finish();
    assert_eq!(
        fs::read_to_string(result).unwrap(),
        "[[16089, 16097], [2961, 3161, 6481], 60, [544, 416]]"
    );
    let frames = record.frames();
    let rates: Vec<u8> = frames.iter().map(|frame| frame.frame_rate.get()).collect();
    assert_eq!(rates, [60, 10, 120]);
    for frame in frames {
        assert_eq!(frame.logical_size, Size::new(544, 416));
    }
}
