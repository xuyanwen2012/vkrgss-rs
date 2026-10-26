//! What a game of RGSS3 loads and registers beside its scripts: the scenarios of "Loading
//! code in RGSS3", those of "Loading code" that the `require` and `load` of RGSS3 meet, and
//! those of "at_exit and exit! in RGSS3" of `rgss-runtime` that need no process of their own
//! (`exit!` ends the process, and is tested through the program), as measured on the
//! original runtime of RGSS3 (`ace-v0-REPORT.md` section 8 and `ace-v1-api-REPORT.md`
//! section 1.6 of the measurements kept outside the repository).
#![cfg(not(vkrgss_interpreter_absent))]

use vkrgss_binding_rgss3::testing::host::{self, Script};
use vkrgss_binding_rgss3::testing::{self, Harness};
use vkrgss_vfs::Runtime;

/// The harness, started as a game of RGSS3.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss3)
}

/// `inspect` of the value of `source`, evaluated on the guest of a game of RGSS3 with its
/// prelude and [`RAISED`], or the class and message of what it raised.
fn eval(source: &str) -> String {
    match host::play(harness(), &format!("{RAISED}{source}"), Script::new()).0 {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

// --- at_exit ----------------------------------------------------------------------------------

/// Requirement "at_exit and exit! in RGSS3", scenario "Value of at_exit".
#[test]
fn at_exit_returns_its_block() {
    assert_eq!(
        eval("[at_exit { 1 }.class, Kernel.at_exit { 2 }.class]"),
        "[Proc, Proc]"
    );
    assert_eq!(
        eval("b = proc { 3 }; [at_exit(&b).equal?(b), method(:at_exit).arity]"),
        "[true, 0]"
    );
    assert_eq!(eval("at_exit"), "ArgumentError: called without a block");
}

// --- Loading code -----------------------------------------------------------------------------

/// A Ruby function `vkrgss_raised` that returns what its block raised, a `LoadError` too,
/// which a `rescue` modifier does not catch, or the value of the block.
const RAISED: &str = "def vkrgss_raised
  yield
rescue Exception => error
  error
end
";

/// Writes `contents` to `relative` below the harness's game directory.
fn put(relative: &str, contents: &str) {
    let path = harness().game_dir().join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// Requirement "Loading code in RGSS3", scenario "require" (`ace-v0-REPORT.md` section 8),
/// from the loaded features of a fresh interpreter.
#[test]
fn require_loads_a_ruby_file_of_the_game_once() {
    assert_eq!(
        eval(
            "$LOADED_FEATURES.replace(['enumerator.so'])
             [require('./Extra.rb'), require('Extra'), $LOADED_FEATURES.size,
              vkrgss_raised { require('nosuch') }.message, vkrgss_raised { require('zlib') }.class,
              $LOADED_FEATURES[1].end_with?('Extra.rb')]"
        ),
        r#"[true, false, 2, "no such file to load -- nosuch", LoadError, true]"#
    );
}

/// Requirement "Loading code in RGSS3", scenario "require_relative".
#[test]
fn require_relative_takes_the_name_from_the_file_that_calls() {
    put(
        "lib/A.rb",
        "$vkrgss_a = ($vkrgss_a || 0) + 1\nrequire_relative 'B'\n",
    );
    put("lib/B.rb", "$vkrgss_b = ($vkrgss_b || 0) + 1\n");
    put("../outside.rb", "$vkrgss_outside = true\n");
    assert_eq!(
        eval(
            "[require_relative('lib/A'), require_relative('lib/A'), $vkrgss_a, $vkrgss_b,
              vkrgss_raised { require_relative('../outside') }.class, $vkrgss_outside,
              method(:require_relative).arity]"
        ),
        "[true, false, 1, 1, LoadError, nil, 1]"
    );
}

/// Requirement "Loading code in RGSS3", scenario "autoload".
#[test]
fn autoload_loads_the_file_at_the_first_use() {
    put(
        "Lazy.rb",
        "$vkrgss_lazy = ($vkrgss_lazy || 0) + 1\nclass Lazy\nend\n",
    );
    assert_eq!(
        eval(
            "autoload(:Lazy, 'Lazy')
             before = $vkrgss_lazy.to_i
             Lazy
             [before, $vkrgss_lazy, require('Lazy')]"
        ),
        "[0, 1, false]"
    );
    put("LazyModule.rb", "module VkrgssLazyModule\nend\n");
    assert_eq!(
        eval(
            "Object.autoload(:VkrgssLazyModule, 'LazyModule')
             [Object.autoload?(:VkrgssLazyModule).end_with?('LazyModule.rb'),
              VkrgssLazyModule.class]"
        ),
        "[true, Module]"
    );
}

/// Requirement "Loading code in RGSS3", scenario "autoload outside the game directory".
#[test]
fn autoload_of_a_file_that_require_cannot_load_raises_at_the_call() {
    assert_eq!(
        eval(
            "gone = vkrgss_raised { Object.autoload(:Gone, 'nosuch') }
             [vkrgss_raised { autoload(:Far, '/etc/passwd') }.class, gone.class, gone.message,
              Object.autoload?(:Far), Object.autoload?(:Gone)]"
        ),
        r#"[LoadError, LoadError, "no such file to load -- nosuch", nil, nil]"#
    );
}

/// Requirement "Loading code in RGSS3", scenario "Compiled as UTF-8": as the scripts are.
#[test]
fn a_required_or_loaded_file_is_compiled_as_utf_8() {
    put(
        "Utf8.rb",
        "$vkrgss_enc = [\"測\".encoding.name, \"測\".size]\n",
    );
    put(
        "Loaded.rb",
        "$vkrgss_loaded = [__ENCODING__.name, \"測\".size]\n",
    );
    assert_eq!(
        eval("require 'Utf8'; load 'Loaded.rb'; [$vkrgss_enc, $vkrgss_loaded]"),
        r#"[["UTF-8", 1], ["UTF-8", 1]]"#
    );
}

/// Requirement "Loading code", scenario "A file that raised is required again", with the
/// `require` of RGSS3.
#[test]
fn a_file_that_raised_is_required_again() {
    put(
        "Flaky.rb",
        "$vkrgss_flaky = ($vkrgss_flaky || 0) + 1\n\
         raise 'the first time' if $vkrgss_flaky == 1\n\
         VKRGSS_FLAKY = $vkrgss_flaky\n",
    );
    assert_eq!(
        eval(
            "first = (require 'Flaky' rescue nil)
             [first, require('Flaky'), VKRGSS_FLAKY, require('Flaky')]"
        ),
        "[nil, true, 2, false]"
    );
}

/// Requirement "Loading code", scenario "A file that requires itself", with the `require` of
/// RGSS3.
#[test]
fn a_file_that_requires_itself_is_evaluated_once() {
    put(
        "Self.rb",
        "$vkrgss_self = ($vkrgss_self || 0) + 1\n$vkrgss_inner = require 'Self'\n",
    );
    assert_eq!(
        eval("[require('Self'), $vkrgss_self, $vkrgss_inner]"),
        "[true, 1, false]"
    );
}

/// Requirement "Loading code", scenario "Wrapped load", with the `load` of RGSS3, which
/// compiles the file as UTF-8 in the anonymous module too.
#[test]
fn a_wrapped_load_keeps_its_constants() {
    put("Part.rb", "PART = \"測\"\n$vkrgss_part = Module.nesting\n");
    assert_eq!(
        eval(
            "[load('Part.rb', true), defined?(PART), $vkrgss_part.size,
              $vkrgss_part[0].const_get(:PART).encoding.name]"
        ),
        r#"[true, nil, 1, "UTF-8"]"#
    );
}
