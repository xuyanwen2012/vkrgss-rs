## MODIFIED Requirements

### Requirement: The Rust API of vkxp-ruby-sys
The crate SHALL offer the public modules and items of `vkxp-ruby-sys` under the same names
with the same meaning. An item without a counterpart in Ruby 1.9.2 SHALL be absent and
listed in the crate documentation with what takes its place; `mark_game_scripts` SHALL
exist and do nothing. What `ffi` mirrors SHALL be that of the vendored headers, and a test
SHALL compare the two. `str_new` SHALL make an ASCII-8BIT string and the constructors for
text a UTF-8 string.

#### Scenario: Public items
- **WHEN** a test crate names `ffi`, `Interpreter`, `Options`, `StartError`, `Ruby`, `protect`, `Raised`, `RubyError`, `HostResult`, `Raise`, `Arity`, `Root`, `MIN_STACK_SIZE`, `STACK_HEADROOM`, `GC_STRESS_VARIABLE` and `SIGNALS_TAKEN_BY_RUBY` of `vkxp_ruby192_sys`, and the methods `eval`, `eval_protected`, `funcall`, `describe`, `set_current_error`, `str_new`, `str_new_text`, `str_new_utf8`, `str_bytes`, `inspect`, `root`, `check_arity`, `top_self`, `mark_game_scripts` and the `define_*` methods of `Ruby`, `RUBY_API` and `Options::for_rgss`
- **THEN** it compiles, and the crate documentation lists `ruby_errinfo`, `ruby_top_self`, `ruby_set_stack_size`, the four `ruby_vkxp_` hooks of 1.8.7, `SymbolTable`, `rstring` and `rarray` as absent, each with what takes its place

#### Scenario: Value
- **WHEN** `6 * 7` is evaluated through `eval_protected`
- **THEN** the host receives the integer 42

#### Scenario: Exception
- **WHEN** a two-line source whose second line is `raise ArgumentError, 'boom'` is evaluated as file `Section001` from line 1
- **THEN** the host receives class `ArgumentError`, message `boom` and a backtrace whose first entry starts with `Section001:2`

#### Scenario: Syntax error
- **WHEN** `def (` is evaluated
- **THEN** the host receives a `SyntaxError` and the next evaluation works

#### Scenario: Host method, raise and panic
- **WHEN** host functions are registered as methods of a new class, one that doubles its argument, one that raises `ArgumentError` and one that panics, and Ruby calls the first with 21, the second inside `begin ... rescue ArgumentError` and the third inside `begin ... rescue Exception`
- **THEN** Ruby receives 42, the first `rescue` branch runs, and the second `rescue` branch sees an exception carrying the panic message

#### Scenario: Fixed arity
- **WHEN** a host function is registered with a fixed arity of 2 and Ruby calls it with one argument and asks `method(:name).arity`
- **THEN** the call raises `ArgumentError` before the body runs and the arity is 2

#### Scenario: Rooted string
- **WHEN** a string is created, rooted, referenced only from host heap memory, and many collections are forced
- **THEN** the string still has its contents

#### Scenario: Bytes and text
- **WHEN** the host makes a string of the bytes `e3 81 82` with `str_new` and one with the text constructor and Ruby evaluates `encoding.name` and `size` of each
- **THEN** the first gives `"ASCII-8BIT"` and 3 and the second `"UTF-8"` and 1, and `str_bytes` returns the three bytes for both

#### Scenario: Embedded and heap values
- **WHEN** the host reads a string of 3 bytes and one of 100 bytes, and an array of 2 elements and one of 100, through the string and array accessors of `ffi`
- **THEN** it reads the lengths and contents that Ruby sees for each

#### Scenario: Headers and declarations agree
- **WHEN** the test compiles a C program against the vendored headers with the flags of the build
- **THEN** the sizes, offsets, type tags, flag bits and special constants it prints equal those of `ffi`, and a C file that declares every function of `ffi` with the types `ffi` gives it compiles against the headers without a conflicting declaration
