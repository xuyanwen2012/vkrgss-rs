//! The names a tilemap of RGSS1 interns, through `run_game` in a process of its own: the
//! probe runs as the only script of a game, and after each of its steps a new name has the
//! number measured on the original runtime of XP.

use std::fs;

use vkrgss_binding::testing::host::{RPG_STAND_IN, Script, ruby_literal, scripted_host};
use vkrgss_binding::testing::{CellBackend, FONT_FAMILIES};
use vkrgss_binding::{Config, RunConfig, run_game};
use vkrgss_ruby_sys::{Interpreter, Options};
use vkrgss_vfs::{GameFiles, GameIni};

/// The steps of the probe that ran on the original which the scenario names. The whole
/// script is parsed before it runs, so every identifier written here has its number before
/// the first step; after each step the probe writes the serial of a marker made with
/// `String#to_sym` (the number `>> 3`) and the names that appeared during the step.
const PROBE: &str = r#"$zz_known = {}
$zz_out = []
def zz_scan
  zz_new = []
  Symbol.all_symbols.each do |zz_s|
    zz_i = zz_s.to_i
    next if $zz_known[zz_i]
    $zz_known[zz_i] = true
    zz_new.push([zz_i, zz_s.to_s])
  end
  zz_new
end
def zz_step(zz_label)
  zz_status = "ok"
  begin
    yield
  rescue Exception => zz_error
    zz_status = zz_error.class.to_s
  end
  zz_new = zz_scan
  zz_marker = ("zz_m" + $zz_out.size.to_s).to_sym.to_i
  $zz_known[zz_marker] = true
  zz_names = zz_new.sort.map { |zz_n| zz_n[0].to_s + " " + zz_n[1] }
  $zz_out.push([$zz_out.size.to_s, zz_label, zz_status, (zz_marker >> 3).to_s, zz_names.join(", ")].join("|"))
  File.open("tilemap-names.txt", "wb") { |zz_file| zz_file.write($zz_out.join("\n") + "\n") }
end

zz_scan
$zz_t = nil
zz_step("nothing") { }
zz_step("Tilemap.new") { $zz_t = Tilemap.new }
zz_step("t.autotiles[0]") { $zz_t.autotiles[0] }
zz_step("t.autotiles[0] = nil") { $zz_t.autotiles[0] = nil }
"#;

/// The lines the original wrote for those steps. There `cargo xtask oracle` ran its own probe
/// as the first script, which interns names this game does not have, so the test compares
/// how far each number is from the marker of the first step ([`advances`]).
///
/// Provenance: oracle measurement 2026-10-27, RGSS103J, change
/// `call-rgss-from-script-threads` (design, "Runs on the originals").
const ORIGINAL: &str = "\
0|nothing|ok|2070|
1|Tilemap.new|ok|2071|
2|t.autotiles[0]|ok|2073|16577 at0
3|t.autotiles[0] = nil|ok|2074|
";

/// The lines of the probe's output with every serial, of a marker or of a new name, taken as
/// the advance from the marker of the first step.
fn advances(text: &str) -> Vec<String> {
    let mut base = None;
    text.lines()
        .map(|line| {
            let fields: Vec<&str> = line.split('|').collect();
            let serial: i64 = fields[3].parse().expect("a serial");
            let base = *base.get_or_insert(serial);
            let names: Vec<String> = fields[4]
                .split(", ")
                .filter(|name| !name.is_empty())
                .map(|name| {
                    let (number, name) = name.split_once(' ').expect("a number and a name");
                    let number: i64 = number.parse().expect("a number");
                    format!("+{} {name}", (number >> 3) - base)
                })
                .collect();
            let advance = serial - base;
            format!(
                "{}|{}|{}|+{advance}|{}",
                fields[0],
                fields[1],
                fields[2],
                names.join(", ")
            )
        })
        .collect()
}

/// Scenario "Tilemap, measured on the original".
#[test]
fn tilemap_names_of_rgss1_as_measured() {
    vkrgss_binding::pending_over_ruby_1_9_2!();
    let executable = std::env::current_exe().unwrap();
    let root = executable.with_extension("vkrgss-run-game-tilemap-names");
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    let game = root.join("game");
    fs::create_dir_all(game.join("Data")).unwrap();
    fs::write(
        game.join("Game.ini"),
        "[Game]\r\nLibrary=RGSS103J.dll\r\nScripts=Data\\Scripts.rxdata\r\nTitle=Names\r\n",
    )
    .unwrap();
    let builtin_scripts = root.join("rpg.rb");
    fs::write(&builtin_scripts, RPG_STAND_IN).unwrap();
    let assemble = format!(
        "File.open({}, 'wb') {{ |file| Marshal.dump([[1, 'Main', Zlib::Deflate.deflate({})]], file) }}",
        ruby_literal(&game.join("Data/Scripts.rxdata").to_string_lossy()),
        ruby_literal(PROBE)
    );
    // The probe names its file as a game does, from the game directory. This binary has one
    // test.
    std::env::set_current_dir(&game).unwrap();

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
    host.finish();
    let output = fs::read_to_string(game.join("tilemap-names.txt")).unwrap();
    assert_eq!(advances(&output), advances(ORIGINAL));
    assert_eq!(
        advances(ORIGINAL),
        [
            "0|nothing|ok|+0|",
            "1|Tilemap.new|ok|+1|",
            "2|t.autotiles[0]|ok|+3|+2 at0",
            "3|t.autotiles[0] = nil|ok|+4|",
        ]
    );
}
