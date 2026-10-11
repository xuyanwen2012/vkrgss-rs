## ADDED Requirements

### Requirement: Dumped strings survive a collection
`Marshal.dump` SHALL write the bytes that a `_dump` method returned, and the names of
classes and modules, also when a garbage collection runs while they are written, whichever
compiler built the interpreter.

#### Scenario: Table under stress
- **WHEN** `Marshal.dump(Table.new(20, 15, 3))` is evaluated with `GC.stress = true` by an interpreter built with gcc 13
- **THEN** the 20 bytes after the header unpack to 3, 20, 15, 3 and 900

#### Scenario: Patch is recorded
- **WHEN** `PATCHES.md` and the manifest test are checked
- **THEN** the change to `marshal.c` has an entry with its diff and patched hash, and the manifest test passes
