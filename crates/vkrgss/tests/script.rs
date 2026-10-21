//! The built `vkrgss` executable in script mode: the scenarios of `script-mode`, each over
//! Ruby 1.8.7 (`--runtime rgss1`) and over Ruby 1.9.2 (`rgss3`, the default), whose
//! interpreter is that of the guest executable `vkrgss-guest-rgss3` beside the program.
//!
//! A script opens no window and no device, so these tests need no Vulkan driver. Ruby 1.9.2
//! is built for some targets only; elsewhere the runs over it are left out.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

/// Whether this target has Ruby 1.9.2, and with it the guest executable of RGSS3.
const RUBY_192: bool = cfg!(any(
    all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_os = "linux",
        target_env = "gnu"
    ),
    all(target_arch = "aarch64", target_os = "macos")
));

/// An interpreter a script runs with: the options that choose it and its `RUBY_VERSION`.
#[derive(Debug, Clone, Copy)]
struct Interpreter {
    options: &'static [&'static str],
    version: &'static str,
}

/// The interpreters of this target: Ruby 1.8.7 of `rgss1`, and Ruby 1.9.2 of the default
/// runtime, `rgss3`. Fails the test, before any child is started, when the guest executable
/// of RGSS3 is not beside the program.
fn interpreters() -> Vec<Interpreter> {
    let mut interpreters = vec![Interpreter {
        options: &["--runtime", "rgss1"],
        version: "1.8.7",
    }];
    if RUBY_192 {
        let guest = Path::new(env!("CARGO_BIN_EXE_vkrgss")).with_file_name("vkrgss-guest-rgss3");
        assert!(
            guest.is_file(),
            "there is no guest executable {} beside the program, which runs the scripts of \
             RGSS3 in it: build and test both with `cargo test -p vkrgss -p vkrgss-guest-rgss3`",
            guest.display()
        );
        interpreters.push(Interpreter {
            options: &[],
            version: "1.9.2",
        });
    }
    interpreters
}

/// A directory with a directory `scripts` of the files of a test, and a directory `outside`
/// beside it.
struct Scripts {
    dir: TempDir,
}

impl Scripts {
    fn new(files: &[(&str, &str)]) -> Scripts {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for (name, text) in files {
            let path = dir.path().join("scripts").join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        fs::create_dir_all(dir.path().join("outside")).unwrap();
        Scripts { dir }
    }

    /// Runs `vkrgss` with `interpreter`, `options` and `--script` with the file `name` and
    /// `arguments`, from the directory of the scripts' parent, with `input` as the standard
    /// input and none of the program's variables set.
    fn run(
        &self,
        interpreter: Interpreter,
        options: &[&str],
        name: &str,
        arguments: &[&str],
        input: &str,
    ) -> Ran {
        let file = Path::new("scripts").join(name);
        let mut child = Command::new(env!("CARGO_BIN_EXE_vkrgss"))
            .args(interpreter.options)
            .args(options)
            .arg("--script")
            .arg(&file)
            .args(arguments)
            .current_dir(self.dir.path())
            .env_remove("VKRGSS_LOG")
            .env_remove("VKRGSS_GUEST")
            .env_remove("VKRGSS_SANDBOX")
            .env_remove("VKXP_LOG")
            .env_remove("VKXP_GUEST")
            .env_remove("VKXP_SANDBOX")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the program starts");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let output = child.wait_with_output().expect("the program ends");
        Ran {
            output,
            version: interpreter.version,
        }
    }
}

/// What a run of a script did.
struct Ran {
    output: Output,
    /// The version of the interpreter it ran with.
    version: &'static str,
}

impl Ran {
    fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).into_owned()
    }

    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }

    #[track_caller]
    fn assert_status(&self, status: i32) {
        assert_eq!(
            self.output.status.code(),
            Some(status),
            "Ruby {}: {}\nstdout: {}\nstderr: {}",
            self.version,
            self.output.status,
            self.stdout(),
            self.stderr()
        );
    }
}

/// Runs the script `text` as `s.rb` with every interpreter, with `options` and `arguments`.
fn each(text: &str, options: &[&str], arguments: &[&str], input: &str) -> Vec<Ran> {
    let scripts = Scripts::new(&[("s.rb", text)]);
    interpreters()
        .into_iter()
        .map(|interpreter| scripts.run(interpreter, options, "s.rb", arguments, input))
        .collect()
}

/// Scenarios "Output and arguments" and "The interpreter of RGSS1".
#[test]
fn a_script_prints_its_arguments_and_the_version_of_its_interpreter() {
    for ran in each("p ARGV\nputs RUBY_VERSION\n", &[], &["a", "b"], "") {
        ran.assert_status(0);
        assert_eq!(
            ran.stdout(),
            format!("[\"a\", \"b\"]\n{}\n", ran.version),
            "{}",
            ran.stderr()
        );
    }
}

/// The file is `$0` and the file of the top level, so the usual test of a main file holds.
#[test]
fn the_file_of_a_script_is_its_program_name() {
    for ran in each(
        "puts(__FILE__ == $0)\nputs File.basename($0)\n",
        &[],
        &[],
        "",
    ) {
        ran.assert_status(0);
        assert_eq!(ran.stdout(), "true\ns.rb\n", "{}", ran.stderr());
    }
}

/// Scenario "No RGSS".
#[test]
fn a_script_has_none_of_the_classes_of_rgss() {
    let text = "\
%w(Bitmap Sprite Graphics Input Audio RPG Win32API).each do |name|
  begin
    Object.const_get(name)
    puts \"#{name} defined\"
  rescue NameError
    puts \"#{name} missing\"
  end
end
Bitmap
";
    for ran in each(text, &[], &[], "") {
        ran.assert_status(1);
        let expected = "Bitmap Sprite Graphics Input Audio RPG Win32API"
            .split(' ')
            .map(|name| format!("{name} missing\n"))
            .collect::<String>();
        assert_eq!(ran.stdout(), expected, "{}", ran.stderr());
        assert!(ran.stderr().contains("(NameError)"), "{}", ran.stderr());
    }
}

/// Scenario "Standard input".
#[test]
fn a_script_reads_the_standard_input() {
    let text = "puts STDIN.read.split.inject(0) { |sum, number| sum + number.to_i }\n";
    for ran in each(text, &[], &[], "1 2 3\n") {
        ran.assert_status(0);
        assert_eq!(ran.stdout(), "6\n", "{}", ran.stderr());
    }
    for ran in each(
        "while line = gets\n  print line.upcase\nend\n",
        &[],
        &[],
        "a\nb\n",
    ) {
        ran.assert_status(0);
        assert_eq!(ran.stdout(), "A\nB\n", "{}", ran.stderr());
    }
}

/// Scenario "Exit status", and the end of the file.
#[test]
fn the_status_is_that_of_exit() {
    for ran in each("puts 'before'\nexit 3\nputs 'after'\n", &[], &[], "") {
        ran.assert_status(3);
        assert_eq!(ran.stdout(), "before\n");
    }
    for ran in each("exit\n", &[], &[], "") {
        ran.assert_status(0);
    }
    for ran in each("exit false\n", &[], &[], "") {
        ran.assert_status(1);
    }
    for ran in each(
        "STDOUT.sync = false\nprint 'flushed'\nexit! 4\n",
        &[],
        &[],
        "",
    ) {
        ran.assert_status(4);
    }
    for ran in each("$stderr.puts 'to the error stream'\n", &[], &[], "") {
        ran.assert_status(0);
        assert_eq!(ran.stderr(), "to the error stream\n");
    }
}

/// Scenario "Uncaught exception".
#[test]
fn an_uncaught_exception_is_reported_with_status_1() {
    for ran in each("def f\n  raise 'boom'\nend\nf\n", &[], &[], "") {
        ran.assert_status(1);
        let stderr = ran.stderr();
        let first = stderr.lines().next().unwrap_or_default();
        assert!(first.contains("s.rb:2"), "{stderr}");
        assert!(first.ends_with("boom (RuntimeError)"), "{stderr}");
        assert!(stderr.contains("\tfrom "), "{stderr}");
        assert!(stderr.contains("s.rb:4"), "{stderr}");
    }
}

/// Scenario "Require beside the script", and `load` and the letter case of a name as in a
/// game.
#[test]
fn a_script_requires_a_file_beside_it() {
    let scripts = Scripts::new(&[
        (
            "s.rb",
            "puts require('helper')\nputs require('helper')\nputs HELPER\nload 'Lib/More.rb'\n",
        ),
        ("helper.rb", "HELPER = 'helped'\n"),
        ("lib/more.rb", "puts 'loaded'\n"),
    ]);
    for interpreter in interpreters() {
        let ran = scripts.run(interpreter, &[], "s.rb", &[], "");
        ran.assert_status(0);
        assert_eq!(
            ran.stdout(),
            "true\nfalse\nhelped\nloaded\n",
            "{}",
            ran.stderr()
        );
    }
}

/// `require_relative` and `autoload` of RGSS3, over Ruby 1.9.2.
#[test]
fn a_script_of_rgss3_requires_relative_to_a_file() {
    if !RUBY_192 {
        return;
    }
    let scripts = Scripts::new(&[
        (
            "s.rb",
            "require_relative 'lib/a'\nautoload :Later, 'lib/later'\nputs Later\n",
        ),
        ("lib/a.rb", "require_relative 'b'\n"),
        ("lib/b.rb", "puts 'b'\n"),
        ("lib/later.rb", "Later = 'later'\n"),
    ]);
    let interpreter = *interpreters().last().unwrap();
    let ran = scripts.run(interpreter, &[], "s.rb", &[], "");
    ran.assert_status(0);
    assert_eq!(ran.stdout(), "b\nlater\n", "{}", ran.stderr());
}

/// Scenario "No standard library", and a file outside the directory of the script.
#[test]
fn a_script_has_no_standard_library() {
    let text = "\
['set', '../outside/x'].each do |name|
  begin
    require name
    puts \"#{name} loaded\"
  rescue LoadError
    puts \"#{name}: LoadError\"
  end
end
";
    for ran in each(text, &[], &[], "") {
        ran.assert_status(0);
        assert_eq!(
            ran.stdout(),
            "set: LoadError\n../outside/x: LoadError\n",
            "{}",
            ran.stderr()
        );
    }
}

/// The options of games together with `--script` are an error that names them, and
/// `--script` needs a file.
#[test]
fn options_of_games_are_refused_with_a_script() {
    let scripts = Scripts::new(&[("s.rb", "puts 'ran'\n")]);
    let interpreter = interpreters()[0];
    let ran = scripts.run(
        interpreter,
        &["--headless", "--frames", "3"],
        "s.rb",
        &[],
        "",
    );
    ran.assert_status(2);
    assert!(ran.stderr().contains("--headless"), "{}", ran.stderr());
    assert!(ran.stderr().contains("--frames"), "{}", ran.stderr());
    assert_eq!(ran.stdout(), "");

    let output = Command::new(env!("CARGO_BIN_EXE_vkrgss"))
        .arg("--script")
        .output()
        .expect("the program starts");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--script needs a value"), "{stderr}");
}

/// The options after the file are arguments of the script.
#[test]
fn every_argument_after_the_file_is_the_scripts() {
    for ran in each("p ARGV\n", &[], &["--headless", "--script", "-x"], "") {
        ran.assert_status(0);
        assert_eq!(ran.stdout(), "[\"--headless\", \"--script\", \"-x\"]\n");
    }
}

/// A file that is not there ends the run with a message that names it.
#[test]
fn a_missing_script_is_an_error_naming_it() {
    let scripts = Scripts::new(&[]);
    for interpreter in interpreters() {
        let ran = scripts.run(interpreter, &[], "absent.rb", &[], "");
        ran.assert_status(1);
        assert!(ran.stderr().contains("absent.rb"), "{}", ran.stderr());
    }
}
