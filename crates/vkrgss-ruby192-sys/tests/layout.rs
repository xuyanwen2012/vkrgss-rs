//! The hand-written layouts, constants and declarations of `ffi` must agree with the C
//! headers.
//!
//! A small C program, compiled against the vendored headers with the flags of the real build,
//! prints `sizeof`, `offsetof` and the constants; the same list is computed on the Rust side.
//! A second C file declares every function and variable of `ffi` with the types `ffi` gives
//! it: the compiler refuses a declaration that conflicts with the one of the headers.
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::mem::{offset_of, size_of};

use vkrgss_ruby192_sys::ffi;

/// Emits one `printf` per C expression, labelled with the expression itself.
const C_PROGRAM: &str = r#"
#include <stdio.h>
#include <stddef.h>
#include <limits.h>
#include "ruby.h"
#include "eval_intern.h"

#define SIZE(type) printf("sizeof(" #type ") %lu\n", (unsigned long)sizeof(type))
#define OFFSET(type, member) \
    printf("offsetof(" #type ", " #member ") %lu\n", (unsigned long)offsetof(type, member))
#define CONST(name) printf(#name " %ld\n", (long)(name))

int main(void)
{
    SIZE(VALUE);
    SIZE(ID);
    SIZE(long);

    SIZE(struct RBasic);
    OFFSET(struct RBasic, flags);
    OFFSET(struct RBasic, klass);

    SIZE(struct RObject);
    OFFSET(struct RObject, basic);
    OFFSET(struct RObject, as);
    OFFSET(struct RObject, as.heap.numiv);
    OFFSET(struct RObject, as.heap.ivptr);
    OFFSET(struct RObject, as.heap.iv_index_tbl);
    OFFSET(struct RObject, as.ary);

    SIZE(struct RFloat);
    OFFSET(struct RFloat, basic);
    OFFSET(struct RFloat, float_value);

    SIZE(struct RString);
    OFFSET(struct RString, basic);
    OFFSET(struct RString, as);
    OFFSET(struct RString, as.heap.len);
    OFFSET(struct RString, as.heap.ptr);
    OFFSET(struct RString, as.heap.aux);
    OFFSET(struct RString, as.ary);

    SIZE(struct RArray);
    OFFSET(struct RArray, basic);
    OFFSET(struct RArray, as);
    OFFSET(struct RArray, as.heap.len);
    OFFSET(struct RArray, as.heap.aux);
    OFFSET(struct RArray, as.heap.ptr);
    OFFSET(struct RArray, as.ary);

    SIZE(struct RData);
    OFFSET(struct RData, basic);
    OFFSET(struct RData, dmark);
    OFFSET(struct RData, dfree);
    OFFSET(struct RData, data);

    CONST(Qfalse);
    CONST(Qtrue);
    CONST(Qnil);
    CONST(Qundef);
    CONST(FIXNUM_FLAG);
    CONST(IMMEDIATE_MASK);
    CONST(SYMBOL_FLAG);
    CONST(FIXNUM_MAX);
    CONST(FIXNUM_MIN);

    CONST(T_NONE);
    CONST(T_OBJECT);
    CONST(T_CLASS);
    CONST(T_MODULE);
    CONST(T_FLOAT);
    CONST(T_STRING);
    CONST(T_REGEXP);
    CONST(T_ARRAY);
    CONST(T_HASH);
    CONST(T_STRUCT);
    CONST(T_BIGNUM);
    CONST(T_FILE);
    CONST(T_DATA);
    CONST(T_MATCH);
    CONST(T_COMPLEX);
    CONST(T_RATIONAL);
    CONST(T_NIL);
    CONST(T_TRUE);
    CONST(T_FALSE);
    CONST(T_SYMBOL);
    CONST(T_FIXNUM);
    CONST(T_UNDEF);
    CONST(T_NODE);
    CONST(T_ICLASS);
    CONST(T_ZOMBIE);
    CONST(T_MASK);

    CONST(FL_MARK);
    CONST(FL_FINALIZE);
    CONST(FL_TAINT);
    CONST(FL_UNTRUSTED);
    CONST(FL_EXIVAR);
    CONST(FL_FREEZE);
    CONST(FL_USHIFT);
    CONST(FL_SINGLETON);
    CONST(ELTS_SHARED);
    CONST(RSTRING_NOEMBED);
    CONST(RSTRING_EMBED_LEN_MASK);
    CONST(RSTRING_EMBED_LEN_SHIFT);
    CONST(RSTRING_EMBED_LEN_MAX);
    CONST(RARRAY_EMBED_FLAG);
    CONST(RARRAY_EMBED_LEN_MASK);
    CONST(RARRAY_EMBED_LEN_SHIFT);
    CONST(RARRAY_EMBED_LEN_MAX);
    CONST(ROBJECT_EMBED);
    CONST(ROBJECT_EMBED_LEN_MAX);

    CONST(TAG_RAISE);
    CONST(TAG_FATAL);

    /* The macros that are functions on the Rust side, at a few telling points. */
    CONST(INT2FIX(0));
    CONST(INT2FIX(1));
    CONST(INT2FIX(-1));
    CONST(INT2FIX(FIXNUM_MAX));
    CONST(INT2FIX(FIXNUM_MIN));
    CONST(FIX2LONG(INT2FIX(-12345)));
    CONST(ID2SYM(10001));
    CONST(SYM2ID(ID2SYM(10001)));
    CONST(FIXABLE(FIXNUM_MAX));
    CONST(FIXABLE(FIXNUM_MAX + 1));
    CONST(FIXABLE(FIXNUM_MIN));
    CONST(FIXABLE(FIXNUM_MIN - 1));
    return 0;
}
"#;

/// The same list as the C program prints, from the Rust declarations.
fn rust_side() -> Vec<(String, i64)> {
    let mut out: Vec<(String, i64)> = Vec::new();

    macro_rules! size {
        ($c:literal, $t:ty) => {
            out.push((format!("sizeof({})", $c), size_of::<$t>() as i64))
        };
    }
    macro_rules! offset {
        ($c:literal, $member:literal, $t:ty, $($field:tt)+) => {
            out.push((
                format!("offsetof({}, {})", $c, $member),
                offset_of!($t, $($field)+) as i64,
            ))
        };
    }
    macro_rules! constant {
        ($name:ident) => {
            out.push((stringify!($name).to_owned(), ffi::$name as i64))
        };
    }
    macro_rules! value {
        ($c:literal, $value:expr) => {
            out.push(($c.to_owned(), $value as i64))
        };
    }

    size!("VALUE", ffi::VALUE);
    size!("ID", ffi::ID);
    size!("long", std::ffi::c_long);

    size!("struct RBasic", ffi::RBasic);
    offset!("struct RBasic", "flags", ffi::RBasic, flags);
    offset!("struct RBasic", "klass", ffi::RBasic, klass);

    // A member of a union starts where the union starts, and `offset_of!` does not look
    // into one: the offsets below are that of the union plus that within its member.
    let object = offset_of!(ffi::RObject, r#as);
    size!("struct RObject", ffi::RObject);
    offset!("struct RObject", "basic", ffi::RObject, basic);
    value!("offsetof(struct RObject, as)", object);
    value!(
        "offsetof(struct RObject, as.heap.numiv)",
        object + offset_of!(ffi::RObjectHeap, numiv)
    );
    value!(
        "offsetof(struct RObject, as.heap.ivptr)",
        object + offset_of!(ffi::RObjectHeap, ivptr)
    );
    value!(
        "offsetof(struct RObject, as.heap.iv_index_tbl)",
        object + offset_of!(ffi::RObjectHeap, iv_index_tbl)
    );
    value!("offsetof(struct RObject, as.ary)", object);

    size!("struct RFloat", ffi::RFloat);
    offset!("struct RFloat", "basic", ffi::RFloat, basic);
    offset!("struct RFloat", "float_value", ffi::RFloat, value);

    let string = offset_of!(ffi::layout::RString, r#as);
    size!("struct RString", ffi::layout::RString);
    offset!("struct RString", "basic", ffi::layout::RString, basic);
    value!("offsetof(struct RString, as)", string);
    value!(
        "offsetof(struct RString, as.heap.len)",
        string + offset_of!(ffi::layout::RStringHeap, len)
    );
    value!(
        "offsetof(struct RString, as.heap.ptr)",
        string + offset_of!(ffi::layout::RStringHeap, ptr)
    );
    value!(
        "offsetof(struct RString, as.heap.aux)",
        string + offset_of!(ffi::layout::RStringHeap, aux)
    );
    value!("offsetof(struct RString, as.ary)", string);

    let array = offset_of!(ffi::layout::RArray, r#as);
    size!("struct RArray", ffi::layout::RArray);
    offset!("struct RArray", "basic", ffi::layout::RArray, basic);
    value!("offsetof(struct RArray, as)", array);
    value!(
        "offsetof(struct RArray, as.heap.len)",
        array + offset_of!(ffi::layout::RArrayHeap, len)
    );
    value!(
        "offsetof(struct RArray, as.heap.aux)",
        array + offset_of!(ffi::layout::RArrayHeap, aux)
    );
    value!(
        "offsetof(struct RArray, as.heap.ptr)",
        array + offset_of!(ffi::layout::RArrayHeap, ptr)
    );
    value!("offsetof(struct RArray, as.ary)", array);

    size!("struct RData", ffi::RData);
    offset!("struct RData", "basic", ffi::RData, basic);
    offset!("struct RData", "dmark", ffi::RData, dmark);
    offset!("struct RData", "dfree", ffi::RData, dfree);
    offset!("struct RData", "data", ffi::RData, data);

    constant!(Qfalse);
    constant!(Qtrue);
    constant!(Qnil);
    constant!(Qundef);
    constant!(FIXNUM_FLAG);
    constant!(IMMEDIATE_MASK);
    constant!(SYMBOL_FLAG);
    constant!(FIXNUM_MAX);
    constant!(FIXNUM_MIN);

    constant!(T_NONE);
    constant!(T_OBJECT);
    constant!(T_CLASS);
    constant!(T_MODULE);
    constant!(T_FLOAT);
    constant!(T_STRING);
    constant!(T_REGEXP);
    constant!(T_ARRAY);
    constant!(T_HASH);
    constant!(T_STRUCT);
    constant!(T_BIGNUM);
    constant!(T_FILE);
    constant!(T_DATA);
    constant!(T_MATCH);
    constant!(T_COMPLEX);
    constant!(T_RATIONAL);
    constant!(T_NIL);
    constant!(T_TRUE);
    constant!(T_FALSE);
    constant!(T_SYMBOL);
    constant!(T_FIXNUM);
    constant!(T_UNDEF);
    constant!(T_NODE);
    constant!(T_ICLASS);
    constant!(T_ZOMBIE);
    constant!(T_MASK);

    constant!(FL_MARK);
    constant!(FL_FINALIZE);
    constant!(FL_TAINT);
    constant!(FL_UNTRUSTED);
    constant!(FL_EXIVAR);
    constant!(FL_FREEZE);
    constant!(FL_USHIFT);
    constant!(FL_SINGLETON);
    constant!(ELTS_SHARED);
    constant!(RSTRING_NOEMBED);
    constant!(RSTRING_EMBED_LEN_MASK);
    constant!(RSTRING_EMBED_LEN_SHIFT);
    constant!(RSTRING_EMBED_LEN_MAX);
    constant!(RARRAY_EMBED_FLAG);
    constant!(RARRAY_EMBED_LEN_MASK);
    constant!(RARRAY_EMBED_LEN_SHIFT);
    constant!(RARRAY_EMBED_LEN_MAX);
    constant!(ROBJECT_EMBED);
    constant!(ROBJECT_EMBED_LEN_MAX);

    constant!(TAG_RAISE);
    constant!(TAG_FATAL);

    value!("INT2FIX(0)", ffi::long2fix(0));
    value!("INT2FIX(1)", ffi::long2fix(1));
    value!("INT2FIX(-1)", ffi::long2fix(-1));
    value!("INT2FIX(FIXNUM_MAX)", ffi::long2fix(ffi::FIXNUM_MAX));
    value!("INT2FIX(FIXNUM_MIN)", ffi::long2fix(ffi::FIXNUM_MIN));
    value!(
        "FIX2LONG(INT2FIX(-12345))",
        ffi::fix2long(ffi::long2fix(-12345))
    );
    value!("ID2SYM(10001)", ffi::id2sym(10001));
    value!("SYM2ID(ID2SYM(10001))", ffi::sym2id(ffi::id2sym(10001)));
    value!("FIXABLE(FIXNUM_MAX)", ffi::fixable(ffi::FIXNUM_MAX));
    value!("FIXABLE(FIXNUM_MAX + 1)", ffi::fixable(ffi::FIXNUM_MAX + 1));
    value!("FIXABLE(FIXNUM_MIN)", ffi::fixable(ffi::FIXNUM_MIN));
    value!("FIXABLE(FIXNUM_MIN - 1)", ffi::fixable(ffi::FIXNUM_MIN - 1));
    out
}

/// Parses the C program's output: the label, a space, the value.
fn c_side(output: &str) -> Vec<(String, i64)> {
    output
        .lines()
        .map(|line| {
            let (label, value) = line.rsplit_once(' ').expect("label and value");
            (label.to_owned(), value.parse().expect("an integer"))
        })
        .collect()
}

#[test]
fn layout_and_constants_match_the_c_headers() {
    let scratch = common::scratch_dir("layout");
    let source = scratch.join("layout.c");
    fs::write(&source, C_PROGRAM).expect("layout.c is writable");
    let executable = scratch.join("layout");
    common::run(
        common::compiler(&scratch)
            .arg(&source)
            .arg("-o")
            .arg(&executable),
    );
    let c_values = c_side(&common::run(&mut std::process::Command::new(&executable)));
    let rust_values = rust_side();

    let c_labels: Vec<&String> = c_values.iter().map(|(label, _)| label).collect();
    let rust_labels: Vec<&String> = rust_values.iter().map(|(label, _)| label).collect();
    assert_eq!(c_labels, rust_labels, "the two sides list different items");

    let c_map: BTreeMap<_, _> = c_values.into_iter().collect();
    let mismatches: Vec<String> = rust_values
        .iter()
        .filter(|(label, value)| c_map[label] != *value)
        .map(|(label, value)| format!("{label}: C has {}, Rust has {value}", c_map[label]))
        .collect();
    assert!(
        mismatches.is_empty(),
        "ffi disagrees with the C headers:\n  {}",
        mismatches.join("\n  ")
    );
}

/// The source of `ffi`, from which the C declarations are made.
const FFI_SOURCE: &str = include_str!("../src/ffi.rs");

/// The C type of every Rust type that `ffi` uses in a declaration.
const C_TYPES: &[(&str, &str)] = &[
    ("VALUE", "VALUE"),
    ("ID", "ID"),
    ("c_int", "int"),
    ("c_long", "long"),
    ("c_ulong", "unsigned long"),
    ("c_longlong", "long long"),
    ("c_double", "double"),
    ("usize", "size_t"),
    ("*const c_char", "const char *"),
    ("*mut c_char", "char *"),
    ("*const VALUE", "const VALUE *"),
    ("*mut VALUE", "VALUE *"),
    ("*mut c_int", "int *"),
    ("*mut c_void", "void *"),
    ("*mut rb_encoding", "rb_encoding *"),
    ("AnyArgsFn", "VALUE (*)(ANYARGS)"),
    ("ProtectFn", "VALUE (*)(VALUE)"),
    ("Option<DataFn>", "void (*)(void *)"),
    ("unsafe extern \"C\" fn()", "void (*)(void)"),
    ("unsafe extern \"C\" fn(VALUE) -> VALUE", "VALUE (*)(VALUE)"),
];

/// The functions whose `VALUE *` parameter is `volatile VALUE *` in the header, which a Rust
/// type cannot say and which makes no difference to a caller.
const VOLATILE_VALUE_POINTERS: &[&str] = &[
    "ruby_init_stack",
    "rb_string_value",
    "rb_string_value_ptr",
    "rb_string_value_cstr",
];

fn c_type(rust: &str) -> &'static str {
    C_TYPES
        .iter()
        .find(|(name, _)| *name == rust)
        .map(|(_, c)| *c)
        .unwrap_or_else(|| panic!("ffi uses the type `{rust}`, which this test cannot translate"))
}

/// Splits the parameter list of a Rust declaration at the commas outside parentheses.
fn split_parameters(list: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0, 0);
    for (index, character) in list.char_indices() {
        match character {
            '(' | '<' => depth += 1,
            ')' | '>' if depth > 0 => depth -= 1,
            ',' if depth == 0 => {
                parts.push(list[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(list[start..].trim());
    parts.retain(|part| !part.is_empty());
    parts
}

/// The declarations of the `extern "C"` blocks of `ffi`, as C: the name and the C
/// declaration of each function and variable.
fn c_declarations() -> Vec<(String, String)> {
    let mut declarations = Vec::new();
    let mut inside = false;
    let mut statement = String::new();
    for line in FFI_SOURCE.lines() {
        let line = line.trim();
        if line == "unsafe extern \"C\" {" {
            inside = true;
            continue;
        }
        if !inside || line.starts_with("//") {
            continue;
        }
        if line == "}" {
            inside = false;
            continue;
        }
        statement.push_str(line);
        statement.push(' ');
        if !line.ends_with(';') {
            continue;
        }
        let text = statement.trim().trim_end_matches(';').to_owned();
        statement.clear();

        if let Some(rest) = text
            .strip_prefix("pub static mut ")
            .or_else(|| text.strip_prefix("pub static "))
        {
            let (name, rust) = rest.split_once(": ").expect("a typed variable");
            declarations.push((
                name.to_owned(),
                format!("extern {} {name};", c_type(rust.trim())),
            ));
            continue;
        }
        let rest = text
            .strip_prefix("pub fn ")
            .unwrap_or_else(|| panic!("unexpected item in an extern block of ffi: {text}"));
        let (name, rest) = rest.split_once('(').expect("a parameter list");
        // The parameter list ends at the last `)` that is not part of the return type.
        let (parameters, returns) = match rest.rsplit_once(") -> ") {
            Some((parameters, returns)) if !returns.contains(')') => (parameters, returns.trim()),
            _ => (rest.strip_suffix(')').expect("a closed list"), ""),
        };
        let mut c_parameters: Vec<String> = split_parameters(parameters)
            .into_iter()
            .map(|parameter| match parameter {
                "..." => "...".to_owned(),
                _ => {
                    let (_, rust) = parameter.split_once(": ").expect("a typed parameter");
                    let c = c_type(rust.trim());
                    if c == "VALUE *" && VOLATILE_VALUE_POINTERS.contains(&name) {
                        "volatile VALUE *".to_owned()
                    } else {
                        c.to_owned()
                    }
                }
            })
            .collect();
        if c_parameters.is_empty() {
            c_parameters.push("void".to_owned());
        }
        let c_returns = match returns {
            "" | "!" => "void",
            other => c_type(other),
        };
        declarations.push((
            name.to_owned(),
            format!("{c_returns} {name}({});", c_parameters.join(", ")),
        ));
    }
    declarations
}

#[test]
fn every_declaration_of_ffi_agrees_with_the_c_headers() {
    let declarations = c_declarations();
    assert!(
        declarations.len() > 190,
        "only {} declarations were found in ffi.rs",
        declarations.len()
    );

    // The headers define some of the names as macros as well; the declaration is of the
    // function or variable behind the macro.
    let mut source =
        String::from("#include \"ruby.h\"\n#include \"ruby/encoding.h\"\n#include <stddef.h>\n");
    for (name, declaration) in &declarations {
        writeln!(source, "#undef {name}\n{declaration}").unwrap();
    }
    // And every name is defined by the library: the program refers to each of them.
    source.push_str("void *const vkrgss_every_name[] = {\n");
    for (name, _) in &declarations {
        writeln!(source, "    (void *)&{name},").unwrap();
    }
    source.push_str("};\nint main(void) { return vkrgss_every_name[0] == 0; }\n");

    let scratch = common::scratch_dir("declarations");
    let file = scratch.join("declarations.c");
    fs::write(&file, source).expect("declarations.c is writable");
    common::run(
        common::compiler(&scratch)
            .arg("-Werror")
            .arg(&file)
            .args(common::link_arguments())
            .arg("-o")
            .arg(scratch.join("declarations")),
    );
}
