//! Symbol numbers are those of the original runtime: the names it has before its first game
//! script have the numbers of the measured table `data/symbols.startup.txt`, and after the
//! mark new names continue its sequence.
//!
//! The tests of this binary share one interpreter and so one mark; `the_sequence` is the one
//! test that runs after it, and does everything that depends on the order of events.

mod common;

use std::ffi::CString;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use common::eval_inspect;
use vkrgss_ruby_sys::{Interpreter, Options, ffi};

/// The interpreter; a test holds the lock for as long as its steps must not interleave with
/// those of another.
fn interpreter() -> &'static Mutex<Interpreter> {
    static INTERPRETER: OnceLock<Mutex<Interpreter>> = OnceLock::new();
    INTERPRETER.get_or_init(|| {
        Mutex::new(Interpreter::start(Options::default()).expect("the interpreter starts"))
    })
}

/// The measured table as (name, number) pairs.
fn table() -> Vec<(String, u64)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/symbols.startup.txt");
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

fn measured(table: &[(String, u64)], name: &str) -> u64 {
    table
        .iter()
        .find(|(entry, _)| entry == name)
        .unwrap_or_else(|| panic!("{name} is not in the table"))
        .1
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
    let table = table();
    let interpreter = interpreter().lock().unwrap();
    assert_eq!(
        interpreter.run(|_| eval_inspect("[:name.to_i, :id.to_i, :s.to_i]")),
        Ok(format!(
            "[{}, {}, {}]",
            measured(&table, "name"),
            measured(&table, "id"),
            measured(&table, "s")
        ))
    );
    assert_eq!(measured(&table, "s"), 16185);
}

/// Interning derives the scope bits, and for an attribute setter the whole number, from the
/// name; the interpreter aborts when the table disagrees with the former. Every entry goes
/// through that here, operators and setters included.
#[test]
fn every_table_name_has_its_measured_number() {
    let table = table();
    assert_eq!(table.len(), 2002);
    let interpreter = interpreter().lock().unwrap();
    for (name, id) in &table {
        assert_eq!(intern(&interpreter, name), *id, "the number of {name}");
    }
}

#[test]
fn the_sequence_continues_the_original_after_the_mark() {
    let interpreter = interpreter().lock().unwrap();
    let eval = |source: &'static str| interpreter.run(move |_| eval_inspect(source));

    // Names that only the host has, interned before the mark: outside the sequence.
    let early = intern(&interpreter, "zz_host_early");
    let setter = intern(&interpreter, "zz_host_attr=");
    assert!(early >> 3 > 1 << 20, "{early} is not far above the table");
    assert_eq!(setter & 7, 4);

    interpreter.run(|ruby| {
        ruby.mark_game_scripts();
        ruby.mark_game_scripts();
    });

    // Measured on the original: the first two locals of the first game script.
    assert_eq!(
        eval("zz_first = 1; zz_second = 2; [:zz_first.to_i, :zz_second.to_i]"),
        Ok("[16193, 16201]".to_owned())
    );
    // A table name that was not interned before keeps its number and consumes none; the
    // first use of an early name consumes one and keeps the name's number, a second none.
    assert_eq!(
        eval("zz_host_early = scripts_fname = zz_host_early; zz_third = 3; :zz_third.to_i"),
        Ok((16201 + 2 * 8).to_string())
    );
    assert_eq!(intern(&interpreter, "zz_host_early"), early);
    // A setter shares the counter of its base name: both together consume one number.
    assert_eq!(
        eval(
            "zz_o = Object.new; def zz_o.zz_host_attr=(name); end; zz_o.zz_host_attr = 1; \
             zz_host_attr = 2; [:zz_o.to_i, :zz_fourth.to_i]"
        ),
        Ok(format!("[{}, {}]", 16201 + 3 * 8, 16201 + 5 * 8))
    );
    assert_eq!(intern(&interpreter, "zz_host_attr="), setter);
    // An instance variable takes the next number with its own scope bits.
    assert_eq!(
        eval(":@zz_ivar.to_i"),
        Ok(((16201 + 6 * 8) & !7 | 2).to_string())
    );
}

/// The four instance variables in the order `instance_variables` gives them, computed from
/// their symbol numbers: the variables are in a table of 11 bins keyed by the number, a bin
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
fn instance_variables_have_the_same_order_in_every_process() {
    const NAMES: [&str; 4] = [
        "@map_interpreter",
        "@torch_keep",
        "@magic_number",
        "@message_frame",
    ];
    let scratch = common::scratch_dir("symbol_ids");
    let ruby = common::build_ruby_executable(&scratch);
    let script = format!(
        "o = Object.new; names = %w({}); \
         names.each {{ |n| o.instance_variable_set(n, 1) }}; \
         puts names.map {{ |n| n.to_sym.to_i }}.join(' '); \
         puts o.instance_variables.join(' ')",
        NAMES.join(" ")
    );
    let run = || common::run(Command::new(&ruby).args(["-e", &script]));
    let first = run();
    assert_eq!(run(), first);

    let mut lines = first.lines();
    let numbers: Vec<(&str, u64)> = lines
        .next()
        .expect("the numbers")
        .split(' ')
        .map(|number| number.parse().expect("a number"))
        .zip(NAMES)
        .map(|(number, name)| (name, number))
        .collect();
    assert_eq!(
        numbers[2].1,
        measured(&table(), "@magic_number"),
        "the one name of the four that the table has"
    );
    let order: Vec<&str> = lines.next().expect("the order").split(' ').collect();
    assert_eq!(order, expected_order(&numbers));
}
