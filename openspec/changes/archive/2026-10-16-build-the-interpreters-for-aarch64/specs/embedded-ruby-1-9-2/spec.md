## MODIFIED Requirements

### Requirement: Hermetic build
On the targets `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`, building `vkrgss-ruby192-sys` SHALL compile the
vendored Ruby 1.9.2, its linked encodings and its `Zlib` extension from a fixed list of
files with `-std=gnu89`, `-fwrapv` and `-fno-strict-aliasing`, using only a C compiler and
a checked-in configuration header: no `configure`, no `make`, no installed Ruby, no
generated source, no zlib of the system. On any other target the crate SHALL build empty
and its build script SHALL warn, naming the target.

#### Scenario: No Ruby on the machine
- **WHEN** the workspace is built on an x86_64 Linux machine with a C compiler and no Ruby installation
- **THEN** the build succeeds and produces the interpreter library

#### Scenario: Another target
- **WHEN** the crate is built for a target other than those two, such as `riscv64gc-unknown-linux-gnu`
- **THEN** the build succeeds without compiling a C file, the crate has no public item, and the build script prints a warning naming that target

#### Scenario: Header of aarch64
- **WHEN** the configuration header of `aarch64-unknown-linux-gnu` is compared with that of `x86_64-unknown-linux-gnu`
- **THEN** they differ only in `RUBY_PLATFORM` and the leading comment
