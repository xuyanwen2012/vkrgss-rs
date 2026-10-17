//! Symbol numbers of RGSS2: an interpreter started with the table of the original runtime of
//! VX gives the names of `data/symbols.startup.rgss2.txt` their numbers there, and after the
//! mark new names continue that runtime's sequence.
//!
//! A process has one interpreter and so one table, which is why these tests are a binary of
//! their own beside `symbol_ids.rs`. They share one interpreter and one mark;
//! `the_sequence_continues_the_original_after_the_mark` is the one test that runs after it.

mod common;

use std::ffi::CString;
use std::sync::{Mutex, OnceLock};

use common::eval_inspect;
use vkrgss_ruby_sys::{Interpreter, Options, SymbolTable, ffi};

/// The interpreter; a test holds the lock for as long as its steps must not interleave with
/// those of another.
fn interpreter() -> &'static Mutex<Interpreter> {
    static INTERPRETER: OnceLock<Mutex<Interpreter>> = OnceLock::new();
    INTERPRETER.get_or_init(|| {
        let options = Options {
            symbols: SymbolTable::Rgss2,
            ..Options::default()
        };
        Mutex::new(Interpreter::start(options).expect("the interpreter starts"))
    })
}

/// A measured table as (name, number) pairs.
fn table(file: &str) -> Vec<(String, u64)> {
    let path = common::manifest_dir().join("data").join(file);
    std::fs::read_to_string(path)
        .expect("the table is readable")
        .lines()
        .filter(|line| !line.starts_with("# "))
        .map(|line| {
            let (id, name) = line.split_once(' ').expect("a number and a name");
            (name.to_owned(), id.parse().expect("a number"))
        })
        .collect()
}

fn rgss2() -> Vec<(String, u64)> {
    table("symbols.startup.rgss2.txt")
}

fn measured(table: &[(String, u64)], name: &str) -> Option<u64> {
    table
        .iter()
        .find(|(entry, _)| entry == name)
        .map(|&(_, id)| id)
}

/// Interns `name` through the C API.
fn intern(interpreter: &Interpreter, name: &str) -> u64 {
    let name = CString::new(name).expect("no NUL in a name");
    // SAFETY: on the interpreter thread; `rb_intern` of a name of the table or before the
    // overflow of the counters does not raise.
    interpreter.run(move |_| unsafe { ffi::rb_intern(name.as_ptr()) as u64 })
}

#[test]
fn table_names_have_their_measured_numbers() {
    let table = rgss2();
    let interpreter = interpreter().lock().unwrap();
    assert_eq!(
        interpreter.run(|_| eval_inspect("[:s.to_i, :$TEST.to_i, :brightness.to_i]")),
        Ok("[16081, 15179, 15281]".to_owned())
    );
    for (name, id) in [("s", 16081), ("$TEST", 15179), ("brightness", 15281)] {
        assert_eq!(measured(&table, name), Some(id), "the number of {name}");
    }
}

/// Every entry goes through `rb_intern` here, operators and setters included, and so through
/// the check of the scope bits against the table.
#[test]
fn every_table_name_has_its_measured_number() {
    let table = rgss2();
    assert_eq!(table.len(), 1996);
    let interpreter = interpreter().lock().unwrap();
    for (name, id) in &table {
        assert_eq!(intern(&interpreter, name), *id, "the number of {name}");
    }
}

/// The instance variables in the order `instance_variables` gives them, computed from their
/// symbol numbers as `symbol_ids.rs` does: a table of 11 bins keyed by the number, a bin
/// lists the entry added last first, and the bins are walked in order.
fn expected_order(numbers: &[(&str, u64)]) -> Vec<String> {
    let mut order = Vec::new();
    for bin in 0..11 {
        for (name, number) in numbers.iter().rev() {
            if number % 11 == bin {
                order.push((*name).to_owned());
            }
        }
    }
    order
}

#[test]
fn the_sequence_continues_the_original_after_the_mark() {
    let rgss1 = table("symbols.startup.txt");
    let rgss2 = rgss2();
    let interpreter = interpreter().lock().unwrap();
    let eval = |source: &'static str| interpreter.run(move |_| eval_inspect(source));

    interpreter.run(|ruby| ruby.mark_game_scripts());

    // Measured on the original: the first two locals of the first game script.
    assert_eq!(
        eval("zz_first = 1; zz_second = 2; [:zz_first.to_i, :zz_second.to_i]"),
        Ok("[16089, 16097]".to_owned())
    );
    // A name that only the table of RGSS1 lists is a new name here.
    assert_eq!(measured(&rgss1, "tileset_id"), Some(10777));
    assert_eq!(measured(&rgss2, "tileset_id"), None);
    assert_eq!(
        eval("tileset_id = 3; zz_third = 4; [:tileset_id.to_i, :zz_third.to_i]"),
        Ok(format!("[{}, {}]", 16097 + 8, 16097 + 2 * 8))
    );

    // The order of instance variables: one name of the table and three new ones.
    const NAMES: [&str; 4] = [
        "@map_interpreter",
        "@torch_keep",
        "@volume",
        "@message_frame",
    ];
    let listed = measured(&rgss2, NAMES[2]).expect("the one name of the four in the table");
    let script = format!(
        "o = Object.new; names = %w({}); \
         names.each {{ |n| o.instance_variable_set(n, 1) }}; \
         names.map {{ |n| n.to_sym.to_i }}.join(' ') + '|' + o.instance_variables.join(' ')",
        NAMES.join(" ")
    );
    let result = interpreter
        .run(move |_| eval_inspect(&script))
        .expect("the script runs");
    let (numbers, order) = result
        .trim_matches('"')
        .split_once('|')
        .expect("numbers and order");
    let numbers: Vec<(&str, u64)> = numbers
        .split(' ')
        .map(|number| number.parse().expect("a number"))
        .zip(NAMES)
        .map(|(number, name)| (name, number))
        .collect();
    assert_eq!(numbers[2].1, listed);
    // The three new names continue the sequence, after the local variables of the script.
    assert_eq!(numbers[0].1, (16097 + 6 * 8) & !7 | 2);
    let order: Vec<&str> = order.split(' ').collect();
    assert_eq!(order, expected_order(&numbers));
}
