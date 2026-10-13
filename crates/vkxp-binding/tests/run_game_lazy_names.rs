//! The names a game of RGSS2 gets after the runtime's first uses, through `run_game` in a
//! process of its own: the probe of first uses runs as the only script of a game, and after
//! each of its steps a new name has the number measured on the original runtime of VX.

use std::fs;

use vkxp_binding::testing::host::{Script, ruby_literal, scripted_host};
use vkxp_binding::testing::{CellBackend, FONT_FAMILIES};
use vkxp_binding::{Config, RunConfig, run_game};
use vkxp_ruby_sys::{Interpreter, Options, SymbolTable};
use vkxp_vfs::{GameFiles, GameIni, Runtime};

/// The probe, as it ran on the original.
const PROBE: &str = include_str!("data/lazy_names_probe.rb");
/// The number of a new name after each step of the probe on the original.
const ORIGINAL: &str = include_str!("data/lazy_names_probe.original.txt");

/// The built-in scripts of this game, which are not those of the original: of the `RPG`
/// classes they have what the step "RPG marshal" dumps and loads, a map with its music and
/// its background sound, where the original interns the three class paths. Every name here
/// is one of the start-up symbol table. The two steps before it use classes this lacks and
/// raise `NameError`, which moves nothing.
const BUILTIN_SCRIPTS: &str = "\
module RPG
  class BGM
  end

  class BGS
  end

  class Map
    def initialize(width, height)
      @bgm = RPG::BGM.new
      @bgs = RPG::BGS.new
    end
  end
end
";

/// The lines of the probe's output or of the measurement as (index, label, number).
fn steps(text: &str, number: usize) -> Vec<(usize, String, i64)> {
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('|').collect();
            (
                fields[0].parse().expect("an index"),
                fields[1].to_owned(),
                fields[number].parse().expect("a number"),
            )
        })
        .collect()
}

/// Scenario "Every measured step". No step is left out. What a step ends with is not compared:
/// `require` and `NKF` differ from the original, which the change that measured them leaves
/// alone, and this game has no pictures, no sounds and few `RPG` classes.
#[test]
fn a_new_name_has_the_measured_number_after_every_step_of_the_probe() {
    let executable = std::env::current_exe().unwrap();
    let root = executable.with_extension("vkxp-run-game-lazy-names");
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
    fs::write(&builtin_scripts, BUILTIN_SCRIPTS).unwrap();
    let assemble = format!(
        "File.open({}, 'wb') {{ |file| Marshal.dump([[1, 'Main', Zlib::Deflate.deflate({})]], file) }}",
        ruby_literal(&game.join("Data/Scripts.rvdata").to_string_lossy()),
        ruby_literal(PROBE)
    );
    // The probe names its files as a game does, from the game directory. This binary has
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
    let options = Options {
        symbols: SymbolTable::Rgss2,
        ..Options::default()
    };
    let interpreter = Interpreter::start(options).unwrap();
    let outcome = interpreter.run(move |ruby| {
        // The archive is made with the interpreter's own `Marshal` and `Zlib`.
        ruby.eval_protected(&assemble, "test", 1).unwrap();
        run_game(ruby, config, link)
    });
    assert!(outcome.is_ok(), "{outcome:?}");
    host.finish();

    let measured = steps(ORIGINAL, 2);
    let output = fs::read_to_string(game.join("lazy.txt")).unwrap();
    let found = steps(&output, 3);
    assert_eq!(measured.len(), 178);
    assert_eq!(found.len(), measured.len());
    let mut differing = Vec::new();
    for (measured, found) in measured.iter().zip(&found) {
        assert_eq!((measured.0, &measured.1), (found.0, &found.1));
        if measured.2 != found.2 {
            differing.push(format!(
                "{} {}: {} for {}",
                found.0, found.1, found.2, measured.2
            ));
        }
    }
    assert!(differing.is_empty(), "{}", differing.join("\n"));
}
