//! The hand-written struct layouts and constants of `ffi` must agree with the C headers.
//!
//! A small C program, compiled against the vendored headers with the flags of the real build,
//! prints `sizeof`, `offsetof` and the constants; the same list is computed on the Rust side.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::mem::{offset_of, size_of};

use vkrgss_ruby_sys::ffi;

/// Emits one `printf` per C expression, labelled with the expression itself.
const C_PROGRAM: &str = r#"
#include <stdio.h>
#include <stddef.h>
#include <limits.h>
#include "ruby.h"

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
    OFFSET(struct RObject, iv_tbl);

    SIZE(struct RFloat);
    OFFSET(struct RFloat, basic);
    OFFSET(struct RFloat, value);

    SIZE(struct RString);
    OFFSET(struct RString, basic);
    OFFSET(struct RString, len);
    OFFSET(struct RString, ptr);
    OFFSET(struct RString, aux);
    OFFSET(struct RString, aux.capa);
    OFFSET(struct RString, aux.shared);

    SIZE(struct RArray);
    OFFSET(struct RArray, basic);
    OFFSET(struct RArray, len);
    OFFSET(struct RArray, aux);
    OFFSET(struct RArray, aux.capa);
    OFFSET(struct RArray, aux.shared);
    OFFSET(struct RArray, ptr);

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
    CONST(T_NIL);
    CONST(T_OBJECT);
    CONST(T_CLASS);
    CONST(T_ICLASS);
    CONST(T_MODULE);
    CONST(T_FLOAT);
    CONST(T_STRING);
    CONST(T_REGEXP);
    CONST(T_ARRAY);
    CONST(T_FIXNUM);
    CONST(T_HASH);
    CONST(T_STRUCT);
    CONST(T_BIGNUM);
    CONST(T_FILE);
    CONST(T_TRUE);
    CONST(T_FALSE);
    CONST(T_DATA);
    CONST(T_MATCH);
    CONST(T_SYMBOL);
    CONST(T_UNDEF);
    CONST(T_MASK);

    CONST(FL_MARK);
    CONST(FL_FINALIZE);
    CONST(FL_TAINT);
    CONST(FL_EXIVAR);
    CONST(FL_FREEZE);
    CONST(FL_USHIFT);
    CONST(FL_SINGLETON);
    CONST(ELTS_SHARED);

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
        ($c:literal, $t:ty, $($member:tt)+) => {
            out.push((
                format!("offsetof({}, {})", $c, stringify!($($member)+).replace(' ', "")),
                offset_of!($t, $($member)+) as i64,
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
    offset!("struct RBasic", ffi::RBasic, flags);
    offset!("struct RBasic", ffi::RBasic, klass);

    size!("struct RObject", ffi::RObject);
    offset!("struct RObject", ffi::RObject, basic);
    offset!("struct RObject", ffi::RObject, iv_tbl);

    size!("struct RFloat", ffi::RFloat);
    offset!("struct RFloat", ffi::RFloat, basic);
    offset!("struct RFloat", ffi::RFloat, value);

    size!("struct RString", ffi::RString);
    offset!("struct RString", ffi::RString, basic);
    offset!("struct RString", ffi::RString, len);
    offset!("struct RString", ffi::RString, ptr);
    offset!("struct RString", ffi::RString, aux);
    // A union's members all start where the union starts.
    value!(
        "offsetof(struct RString, aux.capa)",
        offset_of!(ffi::RString, aux)
    );
    value!(
        "offsetof(struct RString, aux.shared)",
        offset_of!(ffi::RString, aux)
    );

    size!("struct RArray", ffi::RArray);
    offset!("struct RArray", ffi::RArray, basic);
    offset!("struct RArray", ffi::RArray, len);
    offset!("struct RArray", ffi::RArray, aux);
    value!(
        "offsetof(struct RArray, aux.capa)",
        offset_of!(ffi::RArray, aux)
    );
    value!(
        "offsetof(struct RArray, aux.shared)",
        offset_of!(ffi::RArray, aux)
    );
    offset!("struct RArray", ffi::RArray, ptr);

    size!("struct RData", ffi::RData);
    offset!("struct RData", ffi::RData, basic);
    offset!("struct RData", ffi::RData, dmark);
    offset!("struct RData", ffi::RData, dfree);
    offset!("struct RData", ffi::RData, data);

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
    constant!(T_NIL);
    constant!(T_OBJECT);
    constant!(T_CLASS);
    constant!(T_ICLASS);
    constant!(T_MODULE);
    constant!(T_FLOAT);
    constant!(T_STRING);
    constant!(T_REGEXP);
    constant!(T_ARRAY);
    constant!(T_FIXNUM);
    constant!(T_HASH);
    constant!(T_STRUCT);
    constant!(T_BIGNUM);
    constant!(T_FILE);
    constant!(T_TRUE);
    constant!(T_FALSE);
    constant!(T_DATA);
    constant!(T_MATCH);
    constant!(T_SYMBOL);
    constant!(T_UNDEF);
    constant!(T_MASK);

    constant!(FL_MARK);
    constant!(FL_FINALIZE);
    constant!(FL_TAINT);
    constant!(FL_EXIVAR);
    constant!(FL_FREEZE);
    constant!(FL_USHIFT);
    constant!(FL_SINGLETON);
    constant!(ELTS_SHARED);

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
#[cfg_attr(
    target_os = "android",
    ignore = "compiles a C program, and a phone has no C compiler"
)]
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
