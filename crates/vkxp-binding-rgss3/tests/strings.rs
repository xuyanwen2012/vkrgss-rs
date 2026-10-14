//! Text and bytes over Ruby 1.9.2: the strings the binding creates are ASCII-8BIT, as those
//! of the original runtime of RGSS3 are, except script source, which is UTF-8.
#![cfg(not(vkxp_interpreter_absent))]

use std::fs;

use vkxp_binding_rgss3::testing;
use vkxp_binding_rgss3::testing::host::{self, Game, Script};
use vkxp_vfs::Runtime;

/// Plays `game` as `host::run_game` does, with the current directory of the process in the
/// game directory, where the glue of a guest puts it: `load_data` of RGSS3 looks for a loose
/// file from there. One game at a time enters its directory.
fn run_game_in(
    game: &Game,
    run: vkxp_binding_rgss3::RunConfig,
    script: Script,
    after: &str,
) -> host::Played {
    static PLAYING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _playing = PLAYING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::env::set_current_dir(game.dir()).expect("the game directory is entered");
    host::run_game(game, run, script, after)
}

/// `inspect` of the value of `source` in a game of RGSS3, or the class and the message of
/// the exception that left it.
fn eval(source: &str) -> String {
    testing::start_as(Runtime::Rgss3);
    match testing::eval(source) {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

/// One value of each class that has a serialized form.
const VALUES: &str = "t = Table.new(2, 3); t[1, 2] = 7; \
     values = [Color.new(1, 2, 3, 255), Tone.new(-1, 2, 3, 4), Rect.new(1, 2, 3, 4), t]";

#[test]
fn serialized_values_are_bytes_and_their_dump_has_no_instance_variables() {
    assert_eq!(
        eval(&format!(
            "{VALUES}; values.map {{ |value| [value._dump(0).encoding.name, Marshal.dump(value)[2]] }}"
        )),
        r#"[["ASCII-8BIT", "u"], ["ASCII-8BIT", "u"], ["ASCII-8BIT", "u"], ["ASCII-8BIT", "u"]]"#
    );
}

#[test]
fn values_as_strings_are_bytes() {
    assert_eq!(
        eval(
            "[Color.new(1, 2, 3, 255).to_s, Rect.new(1, 2, 3, 4).to_s]
               .map { |text| [text, text.encoding.name] }"
        ),
        r#"[["(1.000000, 2.000000, 3.000000, 255.000000)", "ASCII-8BIT"], ["(1, 2, 3, 4)", "ASCII-8BIT"]]"#
    );
}

#[test]
fn a_font_name_is_bytes() {
    // In a game of RGSS3 the default name is a list of names.
    assert_eq!(
        eval("[Font.new.name, Font.default_name].flatten.map { |name| name.encoding.name }.uniq"),
        r#"["ASCII-8BIT"]"#
    );
    // A name a script gives is kept as given, as measured on the original
    // (`ace-v1-api-REPORT.md` section 3.3): the String of the script's source is UTF-8.
    assert_eq!(
        eval("Font.new('Test Sans').name.encoding.name"),
        r#""UTF-8""#
    );
}

#[test]
fn the_message_of_an_error_of_rgss_is_bytes() {
    assert_eq!(
        eval(
            "bitmap = Bitmap.new(1, 1)
             bitmap.dispose
             begin
               bitmap.clear
             rescue RGSSError => error
               [error.message, error.message.encoding.name]
             end"
        ),
        r#"["disposed bitmap", "ASCII-8BIT"]"#
    );
}

/// `Color`, `Tone` and `Rect` are compared with their `==`. `Table` has none, as on the
/// original runtime, so a table is compared through its serialized form.
#[test]
fn values_round_trip_through_marshal() {
    assert_eq!(
        eval(&format!(
            "{VALUES}; values.map {{ |value|
               loaded = Marshal.load(Marshal.dump(value))
               value.is_a?(Table) ? loaded._dump(0) == value._dump(0) : loaded == value
             }}"
        )),
        "[true, true, true, true]"
    );
    assert_eq!(
        eval(&format!("{VALUES}; Marshal.load(Marshal.dump(t))[1, 2]")),
        "7"
    );
}

#[test]
fn script_source_is_text() {
    testing::start_as(Runtime::Rgss3);
    let stores = |name: &str| format!("{name} = ['\u{e9}'.encoding.name, '\u{e9}'.size]\n");
    let game = Game::of(Runtime::Rgss3, "source_text", "Text", &[("Main", "nil")]);
    let builtin = fs::read_to_string(game.builtin_scripts()).unwrap();
    fs::write(
        game.builtin_scripts(),
        builtin + &stores("$vkxp_text_builtin"),
    )
    .unwrap();
    let preload = game.dir().with_extension("preload.rb");
    fs::write(&preload, stores("$vkxp_text_preload")).unwrap();
    let mut run = game.run_config();
    run.preload = vec![preload];
    let played = run_game_in(
        &game,
        run,
        Script::new(),
        "[$vkxp_text_builtin, $vkxp_text_preload]",
    );
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(
        played.after.as_deref(),
        // The built-in scripts are bytes, as on the original (a save of a game shows it).
        Ok(r#"[["ASCII-8BIT", 2], ["UTF-8", 1]]"#)
    );
}
