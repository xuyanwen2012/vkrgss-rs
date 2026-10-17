//! What differs between the platforms the interpreter is built for: the end of a file and of
//! a pipe, which `io.c` sees in the buffer of a `FILE` that bionic keeps opaque,
//! `String#crypt`, which bionic lacks, and the platform string.

mod common;

use std::fs;
use std::sync::OnceLock;

use common::eval_inspect;
use vkrgss_ruby_sys::{Interpreter, Options};

fn eval(source: String) -> Result<String, String> {
    static INTERPRETER: OnceLock<Interpreter> = OnceLock::new();
    INTERPRETER
        .get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
        .run(move |_| eval_inspect(&source))
}

/// `eof?` of an empty file and of a pipe whose writer is closed is true, and `readpartial`
/// raises `EOFError` instead of waiting; with a byte left in the buffer `eof?` is false and
/// `readpartial` returns it.
#[test]
fn the_end_of_a_file_and_of_a_closed_pipe_is_seen() {
    let dir = common::scratch_dir("platform-io");
    let (empty, two) = (dir.join("empty"), dir.join("two"));
    fs::write(&empty, "").expect("the scratch directory is writable");
    fs::write(&two, "ab").expect("the scratch directory is writable");
    let source = format!(
        "def ends(io); seen = [io.eof?]; \
           begin; io.readpartial(1); seen << :read; rescue EOFError; seen << :eof; end; \
           io.close; seen; end; \
         def rest(io); io.getc; seen = [io.eof?, io.readpartial(10), io.eof?]; \
           io.close; seen; end; \
         r, w = IO.pipe; w.close; s, t = IO.pipe; t.write('ab'); t.close; \
         [ends(File.open('{}')), ends(r), rest(File.open('{}')), rest(s)]",
        empty.display(),
        two.display()
    );
    assert_eq!(
        eval(source),
        Ok("[[true, :eof], [true, :eof], [false, \"b\", true], [false, \"b\", true]]".to_owned())
    );
}

/// bionic has no `crypt(3)`.
#[cfg(target_os = "android")]
#[test]
fn crypt_raises_not_implemented_error() {
    assert_eq!(
        eval("begin; 'x'.crypt('ab'); rescue NotImplementedError => e; e.message; end".into()),
        Ok("\"crypt() is not available on Android\"".to_owned())
    );
}

/// glibc's `crypt(3)`, with a salt of two characters, is DES.
#[cfg(not(target_os = "android"))]
#[test]
fn crypt_is_that_of_the_c_library() {
    assert_eq!(
        eval("'x'.crypt('ab')".into()),
        Ok("\"abiQ6Ep3EYTHc\"".to_owned())
    );
}

#[test]
fn the_platform_names_the_target() {
    let expected = if cfg!(target_os = "android") {
        "aarch64-linux-android"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64-linux"
    } else {
        "x86_64-linux"
    };
    assert_eq!(eval("RUBY_PLATFORM".into()), Ok(format!("{expected:?}")));
}
