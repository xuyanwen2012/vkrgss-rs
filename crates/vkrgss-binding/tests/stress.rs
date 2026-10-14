//! The collection stress test: every Ruby snippet of the binding's tests gives the same
//! result when a garbage collection runs at every allocation.
//!
//! `GC.stress = true` makes the interpreter collect whenever it allocates an object, which
//! is every place where a value the binding holds only in Rust memory would be lost, and
//! where a free function could run while a method is at work.

mod cases;

use vkrgss_binding::testing;

/// The snippet with stress on while it runs, and off again whatever happens.
fn stressed(source: &str) -> String {
    format!("GC.stress = true\nbegin\n{source}\nensure\nGC.stress = false\nend")
}

#[test]
fn the_interpreter_supports_stress() {
    assert_eq!(
        testing::eval("GC.stress = true; on = GC.stress; GC.stress = false; [on, GC.stress]")
            .as_deref(),
        Ok("[true, false]")
    );
}

#[test]
fn every_snippet_gives_the_same_result_under_stress() {
    let mut wrong = Vec::new();
    for (group, cases) in cases::GROUPS {
        // Without stress first: a snippet that fails here is not a finding of this test.
        let plain = cases::failures(cases, str::to_owned);
        assert!(plain.is_empty(), "{group}: {}", plain.join("\n"));
        wrong.extend(
            cases::failures(cases, stressed)
                .into_iter()
                .map(|failure| format!("{group}: {failure}")),
        );
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn unreachable_bitmaps_are_freed_under_stress() {
    assert_eq!(
        testing::eval(&stressed(cases::UNREACHABLE_BITMAPS)).as_deref(),
        Ok("nil")
    );
    let remaining = testing::bitmap_count();
    assert!(remaining < 500, "{remaining} bitmaps are left in the arena");
}

/// A class whose `_dump` returns a string that nothing else refers to, and a port whose
/// `write` collects and then allocates, so that a freed buffer is used again at once.
const FRESH_DUMP: &str = r#"
class StressFreshDump
  TEXT = "abcdefghijklmnopqrstuvwxyz"
  def initialize(count); @count = count; end
  def _dump(limit); TEXT * @count; end
  def self._load(bytes); bytes; end
end
class StressCollectingPort
  attr_reader :data
  def initialize; @data = ""; end
  def write(bytes)
    @data << bytes
    GC.start
    (0...64).map { |i| i.to_s * 52 }
    nil
  end
end
"#;

#[test]
fn a_dumped_string_survives_a_collection_while_it_is_written() {
    vkrgss_binding::pending_over_ruby_1_9_2!();
    assert_eq!(testing::eval(FRESH_DUMP).as_deref(), Ok("nil"));

    // To a string: growing the buffer collects only with stress on.
    let to_string = "d = Marshal.dump(StressFreshDump.new(40)); \
         [d.size, Marshal.load(d) == StressFreshDump::TEXT * 40]";
    assert_eq!(
        testing::eval(&stressed(to_string)).as_deref(),
        Ok("[1063, true]")
    );

    // To a port, without stress: the buffer is handed to `write` whenever it holds 8192
    // bytes. The padding moves that point over the bytes around the dumped string, the
    // length in front of it included, which is where the string is no longer referred to.
    let to_port = "wrong = []; \
         (8160..8200).each do |padding| \
           port = StressCollectingPort.new; \
           Marshal.dump(['p' * padding, StressFreshDump.new(4)], port); \
           wrong << padding unless Marshal.load(port.data)[1] == StressFreshDump::TEXT * 4; \
         end; wrong";
    assert_eq!(testing::eval(to_port).as_deref(), Ok("[]"));
}

#[test]
fn a_loaded_symbol_survives_a_collection_while_it_is_interned() {
    // The name is built here and never interned before `Marshal.load` reads it.
    let load = r#"name = "stress_" + "fresh_symbol_" + "name"; Marshal.load("\004\010:" + (name.size + 5).chr + name).to_s"#;
    assert_eq!(
        testing::eval(&stressed(load)).as_deref(),
        Ok("\"stress_fresh_symbol_name\"")
    );
}
