## ADDED Requirements

### Requirement: Backward searches check the least length of a match
The regular expression engine SHALL report no match from a backward search whose pattern's
least length of a match exceeds the length of the subject, before it forms any position
from that length.

#### Scenario: Pattern longer than the subject
- **WHEN** `"ab".rindex(/abc/)` and backward searches with a literal prefix, a character class and an anchor, each with a least length above the subject's length, are evaluated
- **THEN** each returns `nil`

#### Scenario: Patch is recorded
- **WHEN** the vendored source of the engine is checked against its manifest
- **THEN** the change to `backward_search_range` is a patch recorded in `PATCHES.md` that names upstream's commit 33e5b47c
