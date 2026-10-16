//! The encodings the original runtime was measured to have: four listed at start-up, three
//! more usable by name, and every other name unknown, without a file ever being looked for.
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::OnceLock;

use common::eval_inspect;
use vkrgss_ruby192_sys::{Interpreter, Options};

/// The one interpreter of this test process, with the result of the very first evaluation,
/// and the file that a `require` of the encoding EUC-JP would find.
struct Shared {
    interpreter: Interpreter,
    first_evaluation: Result<String, String>,
    decoy: PathBuf,
}

/// Starts the interpreter in a directory that holds `enc/euc_jp.so` and
/// `enc/trans/japanese_euc.so`, the files the release would require for the encoding EUC-JP
/// and for its transcoder. The load path of the interpreter is `["."]`.
fn shared() -> &'static Shared {
    static SHARED: OnceLock<Shared> = OnceLock::new();
    SHARED.get_or_init(|| {
        let directory = common::scratch_dir("encodings");
        fs::create_dir_all(directory.join("enc/trans")).expect("scratch directory");
        let decoy = directory.join("enc/euc_jp.so");
        fs::write(&decoy, b"not a library").expect("the decoy");
        fs::write(
            directory.join("enc/trans/japanese_euc.so"),
            b"not a library",
        )
        .expect("the second decoy");
        std::env::set_current_dir(&directory).expect("the working directory changes");

        let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
        let first_evaluation =
            interpreter.run(|ruby| eval_inspect(ruby, "Encoding.list.map(&:name).sort"));
        Shared {
            interpreter,
            first_evaluation,
            decoy,
        }
    })
}

fn eval(source: &'static str) -> Result<String, String> {
    shared()
        .interpreter
        .run(move |ruby| eval_inspect(ruby, source))
}

fn ok(text: &str) -> Result<String, String> {
    Ok(text.to_owned())
}

#[test]
fn four_encodings_are_listed_at_start_up() {
    assert_eq!(
        shared().first_evaluation,
        ok("[\"ASCII-8BIT\", \"US-ASCII\", \"UTF-16LE\", \"UTF-8\"]")
    );
}

#[test]
fn transcoding_to_the_encodings_of_the_original_works() {
    assert_eq!(
        eval(
            "['abc'.encode('UTF-16LE'), 'あ'.encode('Shift_JIS'), 'あ'.encode('Windows-31J'), \
              'é'.encode('ISO-8859-1')].map {|s| s.unpack('H*')[0] }"
        ),
        ok("[\"610062006300\", \"82a0\", \"82a0\", \"e9\"]")
    );
    // And back, with the names the strings then carry.
    assert_eq!(
        eval(
            "[\"\\x82\\xa0\".force_encoding('Shift_JIS').encode('UTF-8'), \
              \"\\x82\\xa0\".force_encoding('Windows-31J').encode('UTF-8'), \
              \"\\xe9\".force_encoding('ISO-8859-1').encode('UTF-8'), \
              \"a\\x00\".force_encoding('UTF-16LE').encode('UTF-8'), \
              'あ'.encode('Windows-31J').encoding.name, 'あ'.encode('Shift_JIS').encoding.name]"
        ),
        ok("[\"あ\", \"あ\", \"é\", \"a\", \"Windows-31J\", \"Shift_JIS\"]")
    );
}

#[test]
fn a_converter_object_works() {
    assert_eq!(
        eval("Encoding::Converter.new('UTF-8', 'UTF-16LE').convert('ab').unpack('H*')[0]"),
        ok("\"61006200\"")
    );
}

#[test]
fn encodings_that_are_not_linked_are_unknown() {
    for name in ["EUC-JP", "GBK", "Big5"] {
        let outcome = shared()
            .interpreter
            .run(move |ruby| eval_inspect(ruby, &format!("'あ'.encode('{name}')")));
        assert_eq!(
            outcome,
            Err(format!(
                "Encoding::ConverterNotFoundError: code converter not found (UTF-8 to {name})"
            ))
        );
        let outcome = shared()
            .interpreter
            .run(move |ruby| eval_inspect(ruby, &format!("Encoding.find('{name}')")));
        assert_eq!(
            outcome,
            Err(format!("ArgumentError: unknown encoding name - {name}"))
        );
    }
    assert_eq!(eval("1 + 1"), ok("2"));
}

/// Watches a file for being opened or read.
struct Watch {
    descriptor: libc::c_int,
}

impl Watch {
    fn new(path: &std::path::Path) -> Self {
        let path = CString::new(path.as_os_str().as_bytes()).expect("a path without NUL");
        // SAFETY: plain system calls with a NUL-terminated path; the results are checked.
        unsafe {
            let descriptor = libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC);
            assert!(descriptor >= 0, "inotify is available");
            let watch =
                libc::inotify_add_watch(descriptor, path.as_ptr(), libc::IN_OPEN | libc::IN_ACCESS);
            assert!(watch >= 0, "the file can be watched");
            Self { descriptor }
        }
    }

    /// Whether the file was opened or read since the watch began or was last asked.
    fn seen(&self) -> bool {
        let mut buffer = [0_u8; 4096];
        // SAFETY: the buffer is valid for its length; the descriptor does not block.
        let read = unsafe { libc::read(self.descriptor, buffer.as_mut_ptr().cast(), buffer.len()) };
        read > 0
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        // SAFETY: the descriptor is ours and open.
        unsafe { libc::close(self.descriptor) };
    }
}

#[test]
fn no_file_is_looked_for() {
    let shared = shared();
    assert_eq!(
        eval("[$:, File.file?('enc/euc_jp.so')]"),
        ok("[[\".\"], true]")
    );
    let watch = Watch::new(&shared.decoy);
    assert_eq!(
        eval("'あ'.encode('EUC-JP')"),
        Err(
            "Encoding::ConverterNotFoundError: code converter not found (UTF-8 to EUC-JP)"
                .to_owned()
        )
    );
    assert_eq!(
        eval("''.force_encoding('EUC-JP')"),
        Err("ArgumentError: unknown encoding name - EUC-JP".to_owned())
    );
    assert_eq!(eval("$LOADED_FEATURES"), ok("[\"enumerator.so\"]"));
    assert!(!watch.seen(), "the file was opened");

    // The watch does see an open: reading the file through Ruby is reported.
    assert_eq!(eval("File.binread('enc/euc_jp.so').size"), ok("13"));
    assert!(watch.seen(), "the watch reports nothing at all");
}
