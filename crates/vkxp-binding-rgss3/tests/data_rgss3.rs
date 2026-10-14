//! `load_data` and `save_data` of a game of RGSS3: the scenarios of "Data files of RGSS3"
//! of `rgss-runtime`, measured on the original runtime of RGSS3 (`ace-v1-api-REPORT.md`
//! sections 2.1, 2.3 and 2.4 of the measurements kept outside the repository).
#![cfg(not(vkxp_interpreter_absent))]

use std::path::Path;

use vkxp_binding_rgss3::testing;
use vkxp_binding_rgss3::testing::host::{self, Game, Script, ruby_literal};
use vkxp_vfs::Runtime;

/// Plays a game of RGSS3 whose one script is `source`, with the data files `files` (path
/// below the game directory and bytes) in place, and returns what `$vkxp_data` holds then.
/// `GAME` in the script stands for the game directory as a Ruby string.
fn run(name: &str, files: &[(&str, &[u8])], source: &str) -> String {
    testing::start_as(Runtime::Rgss3);
    let game = Game::of(Runtime::Rgss3, name, "Ace", &[("Main", "nil")]);
    for (path, bytes) in files {
        let path = game.dir().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    rewrite(
        &game,
        &source.replace("GAME", &ruby_literal(&game.dir().to_string_lossy())),
    );
    let played = host::run_game(&game, game.run_config(), Script::new(), "$vkxp_data");
    assert!(played.outcome.is_ok(), "{name}: {:?}", played.outcome);
    match played.after {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

/// Writes the script archive of `game` again with the one script `source`.
fn rewrite(game: &Game, source: &str) {
    let archive = game.dir().join(Path::new("Data/Scripts.rvdata2"));
    let write = format!(
        "File.open({}, 'wb') {{ |file| Marshal.dump([[0, 'Main', Zlib::Deflate.deflate({})]], file) }}",
        ruby_literal(&archive.to_string_lossy()),
        ruby_literal(source)
    );
    testing::run(move |ruby| ruby.eval_protected(&write, "testing", 1).map(|_| ()))
        .expect("the archive is written");
}

/// `[array of "abc" and "測"]` without encodings.
const PLAIN: &[u8] = b"\x04\x08\x5b\x07\x22\x08abc\x22\x08\xe6\xb5\x8b";

#[test]
fn plain_strings_are_labelled_utf_8_and_marshal_load_leaves_them_bytes() {
    assert_eq!(
        run(
            "plain",
            &[("Data/Plain.rvdata2", PLAIN)],
            "loaded = load_data('Data/Plain.rvdata2').map { |s| [s.encoding.name, s.valid_encoding?] }
             bytes = File.open(GAME + '/Data/Plain.rvdata2', 'rb') { |file| file.read }
             $vkxp_data = [loaded, Marshal.load(bytes).map { |s| s.encoding.name }]"
        ),
        r#"[[["UTF-8", true], ["UTF-8", true]], ["ASCII-8BIT", "ASCII-8BIT"]]"#
    );
}

#[test]
fn strings_inside_objects_are_labelled_too() {
    // `{"k" => ["v"]}` without encodings.
    let dump: &[u8] = b"\x04\x08\x7b\x06\x22\x06k\x5b\x06\x22\x06v";
    assert_eq!(
        run(
            "inside",
            &[("Data/Inside.rvdata2", dump)],
            "h = load_data('Data/Inside.rvdata2')
             $vkxp_data = [h.keys[0].encoding.name, h.values[0][0].encoding.name]"
        ),
        r#"["UTF-8", "UTF-8"]"#
    );
}

#[test]
fn strings_that_carry_an_encoding_keep_it() {
    assert_eq!(
        run(
            "carried",
            &[],
            "save_data(['a'.force_encoding('UTF-8'), 'b'.force_encoding('US-ASCII'),
                        \"c\\0\".force_encoding('UTF-16LE')], GAME + '/Data/Carried.rvdata2')
             $vkxp_data = load_data('Data/Carried.rvdata2').map { |s| s.encoding.name }"
        ),
        r#"["UTF-8", "US-ASCII", "UTF-16LE"]"#
    );
}

#[test]
fn bytes_that_are_not_utf_8_are_labelled_utf_8_all_the_same() {
    assert_eq!(
        run(
            "invalid",
            &[("Data/Invalid.rvdata2", b"\x04\x08\x22\x06\xff")],
            "s = load_data('Data/Invalid.rvdata2')
             $vkxp_data = [s.encoding.name, s.valid_encoding?]"
        ),
        r#"["UTF-8", false]"#
    );
}

#[test]
fn a_redefined_marshal_load_is_what_load_data_calls() {
    assert_eq!(
        run(
            "redefined",
            &[("Data/Plain.rvdata2", PLAIN)],
            "class << Marshal
               alias vkxp_marshal_load load
               def load(*args)
                 :mine
               end
             end
             begin
               $vkxp_data = load_data('Data/Plain.rvdata2')
             ensure
               class << Marshal
                 alias load vkxp_marshal_load
               end
             end"
        ),
        ":mine"
    );
}

#[test]
fn save_data_writes_the_dump_and_returns_nil() {
    assert_eq!(
        run(
            "saved",
            &[],
            "object = ['abc'.force_encoding('ASCII-8BIT'), Color.new(1, 2, 3, 4), {1 => 2}]
             value = save_data(object, GAME + '/Out.rvdata2')
             bytes = File.open(GAME + '/Out.rvdata2', 'rb') { |file| file.read }
             $vkxp_data = [value, bytes == Marshal.dump(object),
                           load_data('Out.rvdata2')[0].encoding.name]"
        ),
        r#"[nil, true, "UTF-8"]"#
    );
}

#[test]
fn a_missing_file_raises_no_entry_with_the_name_as_given() {
    assert_eq!(
        run(
            "missing",
            &[],
            "$vkxp_data = begin
               load_data('Data/Nope.rvdata2')
             rescue SystemCallError => e
               [e.class, e.message]
             end"
        ),
        r#"[Errno::ENOENT, "No such file or directory - Data/Nope.rvdata2"]"#
    );
}

#[test]
fn the_value_classes_come_back_with_their_dumps() {
    assert_eq!(
        run(
            "values",
            &[],
            "table = Table.new(2, 2)
             table[1, 1] = 5
             values = [table, Color.new(1, 2, 3, 4), Tone.new(-1, 2, 3, 4), Rect.new(1, 2, 3, 4)]
             save_data(values, GAME + '/Data/Values.rvdata2')
             loaded = load_data('Data/Values.rvdata2')
             $vkxp_data = loaded.zip(values).map { |l, v| [l.class, l._dump(0) == v._dump(0)] }"
        ),
        "[[Table, true], [Color, true], [Tone, true], [Rect, true]]"
    );
}
