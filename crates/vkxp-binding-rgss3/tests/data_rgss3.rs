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

// --- Source and errors of load_data -----------------------------------------------------------

/// A Ruby function `vkxp_try` that returns the value of its block, or the class and message
/// of what the block raised.
const TRY: &str = "def vkxp_try
  yield
rescue Exception => error
  [error.class, error.message]
end
";

/// Requirement "Source and errors of load_data in RGSS3", scenario "A frozen string"
/// (`ace-v1-api-REPORT.md` section 2.1, two of 71 samples).
#[test]
fn a_frozen_string_is_labelled_and_stays_frozen() {
    let source = "class VkxpFrozen
       def self._load(data)
         'ret'.force_encoding('ASCII-8BIT').freeze
       end
       def _dump(level)
         ''
       end
     end
     save_data(VkxpFrozen.new, GAME + '/Data/Frozen.rvdata2')
     f = 'Data/Frozen.rvdata2'
     $vkxp_data = [load_data(f).encoding.name, load_data(f).frozen?]";
    assert_eq!(run("frozen", &[], source), r#"["UTF-8", true]"#);
}

/// Requirement "Source and errors of load_data in RGSS3", scenarios "Cut loose files", "A
/// string cut short", "Not a dump" and "Only the first object" (`ace-v1-api-REPORT.md`
/// section 2.1): a loose file is read as an open `File`.
#[test]
fn a_loose_file_is_read_as_a_file() {
    let files: &[(&str, &[u8])] = &[
        ("Data/Empty.rvdata2", b""),
        ("Data/Byte.rvdata2", b"\x04"),
        // `[1, 2, 3]` without its last byte.
        ("Data/Array.rvdata2", b"\x04\x08[\x08i\x06i\x07i"),
        // `"abcdef"` without its last two bytes.
        ("Data/String.rvdata2", b"\x04\x08\"\x0babcd"),
        ("Data/Game.rvdata2", b"[Game]\r\nTitle=x\r\n"),
        // `1` and then `2`.
        ("Data/Two.rvdata2", b"\x04\x08i\x06\x04\x08i\x07"),
    ];
    let source = format!(
        "{TRY}
         $vkxp_data = %w[Empty Byte Array String Game Two].map do |name|
           vkxp_try {{ load_data(\"Data/#{{name}}.rvdata2\") }}
         end
         $vkxp_data[4][1] = $vkxp_data[4][1][0, 48]"
    );
    assert_eq!(
        run("cut", files, &source),
        "[[EOFError, \"end of file reached\"], [EOFError, \"end of file reached\"], \
         [EOFError, \"end of file reached\"], [ArgumentError, \"marshal data too short\"], \
         [TypeError, \"incompatible marshal file format (can't be read)\"], 1]"
    );
}

/// Requirement "Source and errors of load_data in RGSS3": the file is closed again, also
/// when `Marshal.load` raises, and an entry of the archive is a `String`, which the
/// harness's games do not have: through the program in `crates/vkxp/tests/rgss3.rs`.
#[test]
fn the_file_is_closed_after_the_load() {
    let source = "class << Marshal
       alias vkxp_marshal_load_source load
       def load(source, *rest)
         $vkxp_sources << source
         vkxp_marshal_load_source(source, *rest)
       end
     end
     $vkxp_sources = []
     begin
       load_data('Data/Thing.rvdata2')
       load_data('Data/Bad.rvdata2') rescue nil
     ensure
       class << Marshal
         alias load vkxp_marshal_load_source
       end
     end
     $vkxp_data = $vkxp_sources.map { |s| [s.class, s.closed?] }";
    let files: &[(&str, &[u8])] = &[
        ("Data/Thing.rvdata2", b"\x04\x08i\x06"),
        ("Data/Bad.rvdata2", b"\x04"),
    ];
    assert_eq!(run("closed", files, source), "[[File, true], [File, true]]");
}

/// Requirement "Source and errors of load_data in RGSS3", scenario "A directory"
/// (`ace-v1-api-REPORT.md` section 2.1).
#[test]
fn a_directory_raises_permission_denied() {
    let source = format!("{TRY}\n$vkxp_data = vkxp_try {{ load_data('d') }}");
    assert_eq!(
        run("directory", &[("d/p.bin", b"\x04\x08i\x06")], &source),
        r#"[Errno::EACCES, "Permission denied - d"]"#
    );
}

/// Requirement "Source and errors of load_data in RGSS3", scenarios "Arguments" and "An
/// object with to_str" (`ace-v1-api-REPORT.md` section 2.1).
#[test]
fn load_data_takes_one_string() {
    let source = format!(
        "{TRY}
         named = Object.new
         def named.to_str
           'Data/One.rvdata2'
         end
         $vkxp_data = [vkxp_try {{ load_data }}, vkxp_try {{ load_data('a', 'b') }},
                       vkxp_try {{ load_data(nil) }}, vkxp_try {{ load_data(:sym) }},
                       load_data(named)]"
    );
    assert_eq!(
        run(
            "arguments",
            &[("Data/One.rvdata2", b"\x04\x08i\x06")],
            &source
        ),
        "[[ArgumentError, \"wrong number of arguments(0 for 1)\"], \
         [ArgumentError, \"wrong number of arguments(2 for 1)\"], \
         [TypeError, \"can't convert nil into String\"], \
         [TypeError, \"can't convert Symbol into String\"], 1]"
    );
}

// --- Names of data files ----------------------------------------------------------------------

/// Requirement "Names of data files in RGSS3", scenarios "NUL", "Bytes that are not UTF-8"
/// and "UTF-16LE" (`ace-v1-api-REPORT.md` section 0.2, 3.0.1).
#[test]
fn names_with_a_nul_or_bytes_that_are_not_utf_8_raise() {
    let source = format!(
        "{TRY}
         Dir.chdir(GAME) do
           sj = \"d/\\x83\\x65.bin\".force_encoding('ASCII-8BIT')
           $vkxp_data = [vkxp_try {{ load_data(\"Game.ini\\0x\") }},
                         vkxp_try {{ save_data('sj', sj) }}, vkxp_try {{ load_data(sj) }},
                         vkxp_try {{ load_data('d/p.bin'.encode('UTF-16LE')) }},
                         Dir.entries('d').sort]
         end"
    );
    assert_eq!(
        run("bad_names", &[("d/p.bin", b"\x04\x08i\x06")], &source),
        "[[ArgumentError, \"string contains null byte\"], \
         [Encoding::UndefinedConversionError, \"\\\"\\\\x83\\\" from ASCII-8BIT to UTF-8\"], \
         [Encoding::UndefinedConversionError, \"\\\"\\\\x83\\\" from ASCII-8BIT to UTF-8\"], \
         [ArgumentError, \"string contains null byte\"], [\".\", \"..\", \"p.bin\"]]"
    );
}

/// Requirement "Names of data files in RGSS3", scenarios "Invalid bytes labelled UTF-8" and
/// "A name of other characters" (`ace-v1-api-REPORT.md` section 0.2, 3.0.1).
#[test]
fn names_labelled_utf_8_and_names_of_other_characters_are_files() {
    testing::start_as(Runtime::Rgss3);
    let game = Game::of(Runtime::Rgss3, "other_names", "Ace", &[("Main", "nil")]);
    std::fs::create_dir_all(game.dir().join("d")).unwrap();
    rewrite(
        &game,
        &"Dir.chdir(GAME) do
           bad = save_data('bad', \"d/\\xFF\\xFE.bin\".force_encoding('UTF-8'))
           save_data([1], '測試.rvdata2')
           $vkxp_data = [bad, Dir.entries('d').size - 2, load_data('測試.rvdata2'),
                         load_data('測試.rvdata2'.force_encoding('ASCII-8BIT'))]
         end"
        .replace("GAME", &ruby_literal(&game.dir().to_string_lossy())),
    );
    let played = host::run_game(&game, game.run_config(), Script::new(), "$vkxp_data");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    assert_eq!(played.after.as_deref(), Ok("[nil, 1, [1], [1]]"));
    assert!(game.dir().join("測試.rvdata2").is_file());
}
