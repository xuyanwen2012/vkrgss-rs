# Design

## Context

Decided on 2026-10-13 (change `plan-vx-ace`, `docs/decisions.md`, "VX Ace"): a game of
RGSS3 runs in a guest executable of its own (decision 2); Ruby 1.9.2-p0 is built by
`vkxp-ruby192-sys`, which offers the Rust API of `vkxp-ruby-sys` (decision 4); and
(decision 5) there is one source for the RGSS classes: `vkxp-binding` is compiled a second
time, as a package of its own over `vkxp-ruby192-sys`, what differs by interpreter is behind
the API of the sys crates or a runtime check, and there is no second copy of the classes.
The two interpreters define 868 global symbols in common and are never linked together
(capability `embedded-ruby-1-9-2`, rule 5 of `xtask/src/deps.rs`).

Read for this change, seen from a clone under `wt/`:

- `crates/vkxp-binding` (8.8 k lines; the C calls go through `src/rb.rs` and `src/wrap.rs`),
  `crates/vkxp-guest-main`, the two interpreter crates, `xtask/src/{deps,gate,stress}.rs`.
- The design of `vendor-ruby-1-9-2` with its Follow-ups (archived).
- The feasibility study, `../../ref/tools/ruby192-feasibility/REPORT.md` ("the study"):
  section 4 (the embedding API and the shape of the binding; a small workspace of this
  shape is in `hosts/cargo-two-backends/`), sections 5, 6, 8 and work package C.
- The measurements of the original runtime of RGSS3,
  `../../ref/tools/oracle/measurements/ace-v0-REPORT.md` (section 8, "the V0 report") and
  `ace-v1-api-REPORT.md` ("the API report"), and `../../ref/RGSS301_api.md`.

What the binding and its tests name of `vkxp-ruby-sys` today (counted in `src` and `tests`):

| Kind | Items | In `vkxp-ruby192-sys` |
|---|---|---|
| values and tests on them | `ffi::{VALUE, Qnil, Qtrue, Qfalse, nil_p, rtest, fixnum_p, fix2long, long2fix, num2long, fixable, special_const_p, builtin_type, rb_type, data_ptr, c_long, CStr, TAG_RAISE}` | the same names and signatures |
| type tags | `ffi::T_DATA`, `ffi::T_ARRAY` | the same names, other values |
| C functions | `ffi::{rb_intern, rb_obj_is_kind_of, rb_str_new, rb_path2class, rb_obj_classname, rb_obj_alloc, rb_num2dbl, rb_ivar_set, rb_attr_get, rb_float_new, rb_define_alloc_func, rb_data_object_alloc, rb_ary_new, rb_ary_push}` | the same |
| classes | `ffi::{rb_cObject, rb_eArgError, rb_eTypeError, rb_eRangeError, rb_eIOError, rb_eStandardError, rb_eSystemExit, rb_eLoadError}` | the same |
| safe layer | `Ruby`, `Interpreter`, `Options`, `Raise`, `Raised`, `RubyError`, `HostResult`, `Arity`, `protect`; of `Ruby`: `eval`, `eval_protected`, `funcall`, `inspect`, `describe`, `set_current_error`, `str_new`, `str_bytes`, `define_class`, `define_class_under`, `define_module`, `define_method_with_arity`, `define_singleton_method_with_arity`, `define_global_function_with_arity` | the same |
| the main object | `ffi::ruby_top_self` (2 uses, `runtime.rs`) | absent; `Ruby::top_self()` |
| array layout | `ffi::rarray()` and the fields of `RArray` (1 use, `rb.rs`) | absent; `ffi::rarray_ptr()`, `ffi::rarray_len()` |
| symbol table | `SymbolTable`, `Options::symbols` (`testing.rs`, three test files, `vkxp-guest-main`) | absent |
| start of the game's scripts | `Ruby::mark_game_scripts` (1 use, `runtime.rs`) | absent |
| timer of threads in a copy | `restart_thread_timer` (`vkxp-guest-main` only) | absent |

Everything else of the 8.8 k lines does not know its interpreter. Three facts the study did
not have:

1. **The library name of the second package cannot be `vkxp_binding`.** Two packages of a
   workspace with one library name collide in `target/` and in the documentation. A
   development dependency of a package on itself under another name does not give tests
   the name either (tried: the test does not find the crate). So a test file that says
   `use vkxp_binding::testing` needs the name given to it where it is compiled a second
   time.
2. **`vkxp-ruby192-sys` is empty on every target but `x86_64-unknown-linux-gnu`.** A
   package that compiles the binding over it must be empty there too, tests included, or
   `cargo test --workspace` of the job `gate-aarch64` stops building.
3. **Most strings the classes of RGSS3 create are not UTF-8.** Measured on the original
   (API report, 3.3): `Color#to_s`, `Rect#to_s`, `Font#name`, the messages of the errors
   of RGSS, `RGSS_VERSION` and the file name of a script are ASCII-8BIT; `_dump` returns
   ASCII-8BIT (2.4). Text in the sense of UTF-8 is what a script writes, not what the
   runtime makes. The classification below is therefore mostly "bytes".

## Goals / Non-Goals

**Goals:**

- The source of `vkxp-binding` compiles and passes its tests over both interpreters, with
  no `cfg` and no cargo feature that tells them apart.
- Every place where the binding makes a Ruby string says whether it is text or bytes.
- What cannot pass over Ruby 1.9.2 yet is a list with reasons that later changes shorten,
  and a listed test that starts to pass is reported.
- The binding's C and Rust code has been run under the collector stress of Ruby 1.9.2.
- XP and VX are unchanged.

**Non-Goals:**

- `vkxp-guest-rgss3`, the second package of `vkxp-guest-main`, how the program finds and
  starts the executable, `--guest thread` for RGSS3, save states over Ruby 1.9.2.
- Any behaviour that RGSS3 has and RGSS2 has not: `Window`, `Tilemap`, `Font`, `Input`,
  `Graphics`, `Audio`, `rgss_main`, `rgss_stop`, `RGSSReset`, `msgbox`, the loader of
  `Scripts.rvdata2`, `load_data` that relabels, the wording of messages, Symbols for
  `Input`. Over Ruby 1.9.2 the classes behave as those of RGSS2 in this change. Each is
  under Follow-ups with its measured source.
- The start-up symbol table of RGSS3 and the names the original interns lazily (decision
  5).
- Strings a script passes in another encoding than UTF-8: they are read as their bytes.
- Encodings beyond what `vkxp-ruby192-sys` has; `DL`; `Win32API` as RGSS3 has it; the
  policy on the functions of Ruby 1.9 that start processes.
- A change of any behaviour of Ruby 1.8.7 or of `third_party/`.

## Decisions

1. **The second package.** `crates/vkxp-binding-rgss3`, package `vkxp-binding-rgss3`,
   library `vkxp_binding_rgss3`:

   ```toml
   [package]
   name = "vkxp-binding-rgss3"
   description = "RGSS Ruby API for vkxp over Ruby 1.9.2: the source of vkxp-binding"
   build = "build.rs"

   [lib]
   path = "../vkxp-binding/src/lib.rs"
   doc = false
   doctest = false

   [features]
   testing = ["dep:png"]

   [dependencies]
   # As vkxp-binding, with this one line different:
   vkxp-ruby-sys = { package = "vkxp-ruby192-sys", path = "../vkxp-ruby192-sys" }

   [dev-dependencies]
   vkxp-binding-rgss3 = { path = ".", features = ["testing"] }
   ```

   The dependency key stays `vkxp-ruby-sys`, so the source goes on saying
   `vkxp_ruby_sys::`. Because the two are different packages, feature unification cannot
   mix them. The directory has no `src`.
   - *The manifests stay alike.* A test of `xtask` reads `cargo metadata` and compares the
     dependencies of the two packages (name, kind, features, optional), after replacing
     the interpreter crate and the package's own name; a difference fails and is named.
   - *Documentation.* `doc = false`: the documentation is rendered once, from
     `vkxp-binding`; the same text twice under two names helps nobody. `doctest = false`:
     the examples say `use vkxp_binding::`, and they are compiled in `vkxp-binding`.
   - *Lint.* `cargo clippy --workspace --all-targets` covers the second package like any
     member. This is wanted: it is what shows a signature that differs between the two
     interpreter crates (an `unsafe` block that one of them does not need is a denied
     warning).
   - *Other targets.* `build.rs` reads the target Cargo gives it and, unless it is
     `x86_64-unknown-linux-gnu`, prints `cargo::rustc-cfg=vkxp_interpreter_absent`. The
     shared `src/lib.rs` gets one line, `#![cfg(not(vkxp_interpreter_absent))]`, after its
     documentation. `vkxp-binding` has no build script and never sets it. The name is
     declared once for the workspace (`[workspace.lints.rust] unexpected_cfgs = { level =
     "warn", check-cfg = ['cfg(vkxp_interpreter_absent)'] }`). This `cfg` says "there is
     no interpreter on this target"; it does not tell the interpreters apart, and no
     other `cfg` or feature is added to the source.
   - *The dependency rules.* `vkxp-binding-rgss3` joins `GUEST_CRATES` and
     `INTERPRETER_CRATES` of `xtask/src/deps.rs`. That no package reaches both
     interpreters holds by construction: the second package's dependencies are those of
     `vkxp-binding` with the interpreter crate replaced, none of the others
     (`vkxp-compose-ref`, `vkxp-core`, `vkxp-image`, `vkxp-proto`, `vkxp-vfs`,
     `encoding_rs`, `png`, `thiserror`) reaches an interpreter, and its only development
     dependency is itself. Rule 5 checks it over the real graph with development
     dependencies counted, and a unit test of the rule gives it a graph in which a package
     reaches both binding packages.
   - *`vkxp-guest-main`* gets its second package with the guest executable, not here:
     nothing in this change runs a guest, and its one item outside the common surface,
     `restart_thread_timer`, belongs to the question of copies of a process with Ruby
     1.9.2 (states are refused while a script thread lives). What this change leaves
     ready: `vkxp-guest-main` starts its interpreter through the common constructor of
     decision 2, so that item is the only one left; the pattern of this decision (manifest,
     build script, the one `cfg` line) applies to it unchanged; and its dependency
     `vkxp-binding` is renamed in the manifest as `vkxp-ruby-sys` is here.

2. **The common surface of the interpreter crates.** One name, one signature, `unsafe` and
   `const` included, in both crates:

   | Item | `vkxp-ruby-sys` | `vkxp-ruby192-sys` |
   |---|---|---|
   | `pub enum RubyApi { Ruby18, Ruby19 }`, `pub const RUBY_API: RubyApi` | new: `Ruby18` | new: `Ruby19` |
   | `Options::for_rgss(generation: u8) -> Options` | new: the defaults with `symbols` `SymbolTable::Rgss1` for 1 and `SymbolTable::Rgss2` for any other number, which is what `testing.rs` and `vkxp-guest-main` choose today | new: the defaults for every number |
   | `Ruby::top_self(&self) -> VALUE` | new: reads `ffi::ruby_top_self` | exists |
   | `ffi::rarray_ptr`, `ffi::rarray_len`, `ffi::rstring_ptr`, `ffi::rstring_len` | new, over `rarray()` and `rstring()`, which stay | exist |
   | `Ruby::str_new(&self, bytes: &[u8])` | exists | exists: ASCII-8BIT |
   | `Ruby::str_new_text(&self, bytes: &[u8]) -> Result<VALUE, Raised>` | new: the same string as `str_new` | new: the bytes labelled UTF-8, valid or not; `str_new_utf8(&str)` stays and calls it |
   | `Ruby::mark_game_scripts(&self)` | exists | new: does nothing; Ruby 1.9.2 numbers symbols as the release does |

   `SymbolTable`, `Options::symbols`, `ffi::ruby_top_self`, `ffi::rarray()` stay in
   `vkxp-ruby-sys` and are no longer named by the binding, its tests or
   `vkxp-guest-main`. `restart_thread_timer` is not part of the surface (decision 1).
   - The text constructor takes bytes, not `&str`: a script source or a file name may hold
     bytes that are not UTF-8, and over Ruby 1.8.7 they must reach the interpreter as they
     are.
   - The binding learns its interpreter in one place, `rb.rs`
     (`pub(crate) const RUBY_19: bool = matches!(vkxp_ruby_sys::RUBY_API,
     vkxp_ruby_sys::RubyApi::Ruby19)`), and in this change asks it in two: the preparation
     (decision 4) and the allow-list of the tests (decision 6).
   - What differs behind the same names and needs no code: the values of `T_DATA` and
     `T_ARRAY`; the message of a wrong count for a fixed arity (`wrong number of
     arguments(1 for 2)`, without the space, in Ruby 1.9.2); a backtrace that ends with
     the frame of `eval`; a `Raised` of a `throw` or `break`, whose value the interpreter
     keeps as its current error until the jump is continued or described.
   - The documentation of `vkxp-ruby192-sys` (the table "Differences from
     `vkxp-ruby-sys`") is brought up to date: `mark_game_scripts` is no longer absent.

3. **Text and bytes.** `rb::string_new` is replaced by `rb::bytes_new` (over
   `Ruby::str_new`) and `rb::text_new` (over `Ruby::str_new_text`), so that no call site
   can make a string without choosing. `rb::strings_new` is bytes.

   The rule: a string is **bytes** unless the interpreter is going to read it as script
   source, or it is the path of the script archive. For RGSS3 this is what the original
   does (fact 3). Over Ruby 1.8.7 the two functions give the same string, so the rule
   changes nothing there.

   The 14 places that create a string:

   | Place | What | Kind | Source |
   |---|---|---|---|
   | `kernel.rs`, `load_data` | the bytes of the data file for `Marshal.load` | bytes | |
   | `color.rs`, `_dump` (Color, Tone) | serialized form | bytes | API report 2.4: `_dump` returns ASCII-8BIT, so the dump has no `I` wrapper |
   | `rect.rs`, `_dump` | serialized form | bytes | 2.4 |
   | `table.rs`, `_dump` | serialized form | bytes | 2.4 |
   | `color.rs`, `to_s` | `(1.000000, ...)` | bytes | 3.3 |
   | `rect.rs`, `to_s` | `(1, 2, 3, 4)` | bytes | 3.3 |
   | `font.rs`, one name | `Font#name`, `Font.default_name` | bytes | 3.3 |
   | `font.rs`, list of names (`rb::strings_new`) | the same as an Array | bytes | 3.3 |
   | `win32api.rs`, buffer | what a function wrote into a buffer | bytes | |
   | `kernel.rs`, `vkxp_locate` | a path of the file system for the prelude's `require` | bytes | |
   | `runtime.rs`, `evaluate` | the file name an evaluation is reported under | bytes | 1.1, 3.3: `__FILE__` and backtrace lines are ASCII-8BIT |
   | `runtime.rs`, built-in scripts | source | text | 1.1: source is compiled as UTF-8 |
   | `runtime.rs`, preload scripts | source | text | |
   | `runtime.rs`, path of the script archive | the argument of `vkxp_load_scripts` | text | |

   11 bytes, 3 text. Beside them, made by the interpreter crates and not by the binding:
   the 20 messages of `Raise::new` (7 of them in `error::to_raise`) are made by
   `rb_exc_new`, ASCII-8BIT over Ruby 1.9.2, as measured for the errors of RGSS (3.3);
   the sources of `Ruby::eval` are UTF-8.

   The 21 places that read a string are already of two kinds and keep their functions:

   | Kind | Function | Places |
   |---|---|---|
   | text (16) | `rb::text` (15) and the name of a script in `runtime.rs` | file names (`Bitmap.new`, `Audio`, `load_data`, `vkxp_locate`, `Graphics.transition`, `Font.exist?`, font names, the two names of `Win32API`), drawn and measured text (`draw_text`, `text_size`), the text of a modal message |
   | bytes (4) | `rb::bytes` (3), `rb::string_bytes` (1) | `_load` of Color and Tone, Rect, Table; a buffer of `Win32API` |
   | a type test (1) | `rb::string_bytes(..).is_some()` | "is the argument a String" in `bitmap.rs` |

   Reading does not change in this change, over either interpreter: a string is taken as
   its bytes whatever its label, and text is those bytes with invalid UTF-8 replaced. A
   string in UTF-16LE or another encoding is therefore not converted (Follow-ups).

4. **The preparation.** Of the three prelude files only two lines are for Ruby 1.8:
   `$KCODE = "U"` (Ruby 1.9 warns on the error stream and ignores it; on the original of
   RGSS3 `$KCODE` is `nil`, API report 0) and `$:.clear` (the original of RGSS3 has
   `["."]`, V0 report 8 and API report 1.1, which `vkxp-ruby192-sys` sets). They move to
   `src/prelude_ruby18.rb`, evaluated first and only when `rb::RUBY_19` is false, so the
   order of statements over Ruby 1.8.7 is the one it was. The rest of `prelude.rb`,
   `prelude_rgss1.rb` and `prelude_process_off.rb` is evaluated over both.
   - What the prelude may define over Ruby 1.9.2: what it defines over Ruby 1.8.7 and no
     more. The original of RGSS3 has no prelude of Ruby (`Mutex#synchronize` and
     `Thread.exclusive` do not exist, V0 report 8); the binding's prelude defines nothing
     of it. Its own helpers keep the prefix `vkxp_` and stay private methods of `Object`,
     visible to `private_methods` as over Ruby 1.8.7.
   - `Reset`, `Hangup`, `print` and `p` as modal messages, `require` and `load` in Ruby,
     `vkxp_load_scripts` are those of RGSS1 and RGSS2 and are wrong for RGSS3
     (Follow-ups). They are left as they are: this change makes the existing classes run,
     and the tests that hold them run over both interpreters.

5. **What of the machinery of Ruby 1.8.7 the binding uses.**

   | 1.8.7 | Over Ruby 1.9.2 in this change |
   |---|---|
   | `SymbolTable`, `Options::symbols` | `Options::for_rgss`; no table. **Symbols are not numbered here, and need not be for saves:** in a dump of Ruby 1.9 the instance variables of an object come in the order in which its class first saw each name, not in symbol order (API report 4.2). What decides a save is the text of the built-in scripts and the order in which scripts run. The numbers show only in `Symbol#object_id` and the order of `Symbol.all_symbols`. The start-up table of 2170 names (`ace-v0-symbols.startup.txt`) and the lazily interned names (4.1) are a follow-up, taken up when a game shows a need. |
   | `mark_game_scripts` | called as before; does nothing |
   | the hooks `ruby_vkxp_gc_stress`, `ruby_vkxp_gc_count` | the binding does not name them. Its tests use `GC.stress` and `GC.start`, which Ruby 1.9.2 has itself; `VKXP_GC_STRESS` is read by the interpreter crate |
   | the timer (`SIGVTALRM`), `restart_thread_timer` in the copier | not in the binding; with the guest executable. The timer thread of Ruby 1.9.2 does not start before a script makes a thread |
   | `protect`, `Raised`, `Ruby::describe` | the same calls. `report` of `runtime.rs` reads positions as `file:line`, which the entries of Ruby 1.9.2 (``file:line:in `<main>'``) still start with; the last entry, the frame of `eval`, has no line and is passed over |
   | the prelude | decision 4 |

6. **The tests over both interpreters.**
   - *Unit tests* are in the shared source and run in both packages.
   - *Integration tests.* For each `crates/vkxp-binding/tests/<name>.rs` the second
     package has `tests/<name>.rs` with exactly:

     ```rust
     //! `tests/<name>.rs` of `vkxp-binding`, over Ruby 1.9.2.
     #![cfg(not(vkxp_interpreter_absent))]
     extern crate vkxp_binding_rgss3 as vkxp_binding;
     #[path = "../../vkxp-binding/tests/<name>.rs"]
     mod t;
     ```

     Cargo finds them as it finds any test file; no `[[test]]` entry is needed. The alias
     at the root is what makes `vkxp_binding::` resolve in the shared file (fact 1); the
     attribute empties the target where there is no interpreter (fact 2). A file loaded
     through `#[path]` finds its own modules (`cases`, `scene_cases`, `scene_support`,
     `method_list`) and its `include_str!` data beside itself. The tests are named
     `t::<name>` there. (Tried in a scratch workspace: one source, two packages, the
     alias, nested modules, the empty build, `clippy --all-targets`, `doc`.)
   - A test of `xtask` lists the `.rs` files of both `tests` directories and fails for a
     file of `vkxp-binding` without its counterpart, naming it. It runs on every target.
   - *Edits to the shared test files*, and no other:
     1. `tests/scene_cases/mod.rs` says `use super::cases::{Case, Expect}` instead of
        `crate::cases`, which is the same module in `vkxp-binding` and resolves under `t`.
     2. The three files that name `SymbolTable` (`run_game_rgss2.rs`,
        `run_game_first_uses.rs`, `run_game_lazy_names.rs`) use `Options::for_rgss(2)`.
     3. The marker of the allow-list, as the first statement of a listed test.
     4. `tests/cases/mod.rs`, `failures`: the allow-list for snippet cases.
     No expected value, no snippet and no test name changes.
   - *The allow-list*, `crates/vkxp-binding-rgss3/tests/ruby192.allow`, in the format of
     `xtask/src/stress.skip`: entries separated by blank lines, each a reason in comment
     lines followed by one or more lines

     ```
     test <binary> <name of the test as vkxp-binding prints it>
     case <name of the snippet case>
     ```

     where `<binary>` is the name of the test file, or `lib` for a unit test.
   - *A listed test.* The harness gets `vkxp_binding::pending_over_ruby_1_9_2!()`, a macro
     without arguments that a listed test has as its first statement. It finds the name of
     the function it stands in (`std::any::type_name_of_val` of a local function) and
     calls `testing::pending`, which answers:

     | Situation | Answer |
     |---|---|
     | `rb::RUBY_19` is false | go on: the list is not read |
     | the test is not on the list | go on |
     | on the list, and this process is the child (below) | go on: the body runs |
     | on the list, otherwise | start this executable again with the one test (`--exact`, one thread) and a variable that marks the child. The child reports `1 failed`: print the reason, return, and the macro returns from the test. The child reports `1 passed`: panic, naming the test and the list. Anything else (no test ran, the child ended by a signal): panic, saying so |

     So a listed test is run alone, in a process with its own interpreter, where it cannot
     leave anything behind for other tests; it counts as passed while it fails; and
     removing its entry makes the marker inert, so the test must then pass. The list is
     read from `tests/ruby192.allow` under the directory Cargo gives a test run
     (`CARGO_MANIFEST_DIR`). An entry without a reason and a line of neither form are
     errors. The part that decides (list, name, outcome of the child) is a function
     without side effects and has unit tests.
   - *A listed case.* `cases::failures` asks `testing::pending_case(name)`. A listed case
     is evaluated like the others; when its outcome differs from the expectation nothing
     is reported, and when it is as expected the failure line says that the case passes
     over Ruby 1.9.2 and is to be taken off the list.
   - *What may be on the list*, and nothing else. The reason of an entry states one of:
     (a) the test expects behaviour of RGSS1 or RGSS2 that a section of the V0 report or
     the API report says RGSS3 does differently: the section is named;
     (b) the Ruby of the test itself, or the text it expects from Ruby, is that of 1.8 and
     another in 1.9 (`Symbol#to_i`, `$KCODE`, the wording of a message, the order of a
     Hash, a backtrace): the construct is named;
     (c) the test holds the symbol numbering of Ruby 1.8.1.
     Not reasons: a crash, a value lost under stress, a hang, a wrong pixel, an error in
     the Rust or C of the binding or of an interpreter crate. Those are faults and are
     fixed; a fix in `third_party/ruby-1.9.2` is a recorded patch as `vendor-ruby-1-9-2`
     describes.

7. **Collector stress.** `cargo xtask stress` gets a fifth step, `binding-1.9.2`: `cargo
   test -p vkxp-binding-rgss3 --locked --no-fail-fast` with `VKXP_GC_STRESS=1`, which over
   Ruby 1.9.2 is `GC.stress = true` from the end of start-up. The children of decision 6
   inherit the variable. `xtask/src/stress.skip` takes lines `binding-1.9.2 <test>` as it
   takes those of `binding`. The gate does not change: its test step runs the second
   package's tests without the variable, among them `stress` and `scene_stress`, which
   switch `GC.stress` on around every snippet themselves. The study names values lost
   with a current compiler as the first risk; the step is where the binding's own code
   meets it.

8. **A script of RGSS3 flavour.** `crates/vkxp-binding-rgss3/tests/rgss3_script.rs`, a
   file of the second package only (it has the `cfg` attribute of decision 6 and uses
   `vkxp_binding_rgss3` directly). It starts the harness as a game of `Runtime::Rgss3`,
   evaluates a script that fills a `Bitmap` with one colour, shows it with a `Sprite`,
   moves the sprite in a `Fiber` that yields between the change of `x` and the change of
   `y`, and resumes the Fiber twice; then it extracts a frame from the guest's scene and
   draws it with the reference compositor, as `tests/scene_support` of the binding does
   (loaded through `#[path]`), and reads three pixels. A second test holds what the script
   itself sees of the language. Two further files of the package hold the scenarios of
   text and bytes and of the preparation. None needs a window, a guest process or a file
   of an original runtime.

## Implementation notes

- A flow reads `../../ref/` and copies nothing from it. No Wine and no original runtime is
  run. Scratch output goes under `target/` or `local/` of the clone.
- Before anything changes, keep `cargo test -p vkxp-binding --locked -- --list` and the
  same for `vkxp-ruby-sys`, `vkxp-guest-main` and `vkxp` under `local/`: the names must be
  the same at the end.
- `Cargo.lock` gains the package; the gate builds with `--locked`.
- A fixed-arity message is Ruby's own and differs between the interpreters; the messages
  the binding words itself (`rb::check_count`, `rb::count_error`) stay as they are. Tests
  that compare the former are case (b) of the list.
- A `Raised` that the binding drops without describing or continuing it (`script_name`,
  `rb::class_at(..).is_ok_and(..)`) comes from an evaluation of a fixed source that
  cannot `throw`; leave them, and do not add such a place for a call that runs script
  code.
- If a lint fires in a shared test file only where it is nested under `t`, allow it on
  the `mod t;` line of the counterpart, not in the shared file.
- The shared test file `tests/interpreter.rs` is new in `vkxp-binding`: it holds what one
  file can assert for both interpreters (`RUBY_VERSION` against `RUBY_API`, what both
  preparations define).
- How the build script of `vkxp-ruby192-sys` reads its target is the model for `build.rs`.
- `VKXP_GC_STRESS` over Ruby 1.9.2 is on or off; there is no "every n-th".
- If `stress` or `scene_stress` of the second package takes more than ten minutes in the
  plain test run, record the time under Follow-ups and go on; do not shorten a shared
  test.
- `type_name_of_val` of a function nested in a test gives `<crate>::<modules>::<test>::<
  local>`; the first segment is the binary (the library's name stands for `lib`), the
  last is dropped, a leading `t` is the wrapper. If the form is ever another, the child
  runs no test and the macro panics, which is the wanted outcome.
- Documents: `AGENTS.md` (the crates that may hold `unsafe`), `README.md` (the table of
  crates and the sentence that no executable links the second interpreter: still true),
  `docs/architecture.md` and `docs/decisions.md` (the table of crates, the lists of guest
  crates, both interpreter crates, which `vendor-ruby-1-9-2` left for this change),
  `docs/development.md` (the step, the allow-list and how an entry is removed).

## Follow-ups

For the changes that follow. Sections are those of the API report unless another report is
named.

- **The guest executable** `vkxp-guest-rgss3`, the second package of `vkxp-guest-main`,
  what `restart_thread_timer` is for Ruby 1.9.2, states refused while a script thread
  lives, SIGVTALRM after the first script thread (Follow-ups of `vendor-ruby-1-9-2`): the
  study, sections 5 and 7, work package D. Before a game runs: the functions of Ruby 1.9
  that start processes and that `prelude_process_off.rb` does not know (`spawn`,
  `Process.spawn`, `open("|...")`, the list forms of `system` and `IO.popen`): 5.7.
- **Script loading**: `eval(source, binding of main, "{NNNN}")`, the source compiled as
  UTF-8 although its String is ASCII-8BIT, `s[3, 0] =`, the count read once, `Failed to
  load script.`: 1.1. The error box: 1.2. `rgss_main`, `rgss_stop`, `RGSSReset`: 1.3. F12:
  1.4. `Graphics.__reset__`: 1.5. What ends the game: 1.6. `msgbox`, `msgbox_p`, `p`,
  `print`: 1.7. `require` and `load` work on the original: 0, table row "kernel".
- **`load_data`** as `Marshal.load(io, proc)` with the relabelling: 2.1; with the archive:
  2.2; file names that are not ASCII and the two DLL versions: 0.2. `save_data`: 2.3.
- **Values**: `set` with one object, `new` without arguments, `==` with another class,
  `_load` of a short string: 0 and 0.2; the wording of messages (`can't convert`, the
  count message with its two numbers exchanged): 0.
- **Labels the classes do not have right yet**: a font name a script gave keeps its label;
  a buffer of `Win32API` keeps its label (the binding replaces the contents with a
  string of bytes); `RGSS_VERSION`; `Table#inspect`, `Bitmap#inspect`: 3.3.
- **Strings in other encodings** as arguments (text to draw, file names): 3.1, 3.3; the
  study, work package H.
- **Symbols**: the names interned at first use (`observer` alone, `bmp<n>`, the six of
  `Win32API`): 4.1; the start-up table: V0 report 8 and `ace-v0-symbols.startup.txt`;
  saves: 4.2 to 4.4.
- **Encodings of the interpreter crate**: the dummy encodings made by a first conversion,
  the constants that do not exist: 3.1, 3.2.
- **Limits**: stack depth 5.1, 5.2; `Win32API` 5.8; `DL` 5.9; `Audio` 6; the Symbols of
  `Input`: `../../ref/RGSS301_api.md`.
- An entry of the allow-list whose test has no marker is not noticed (the test simply
  runs). A check would have to search the sources.
- The durations of the gate's steps before and after this change, to be written here by
  the flow.
- An "every n-th allocation" form of the stress switch for Ruby 1.9.2, if the step
  `binding-1.9.2` is too slow.
