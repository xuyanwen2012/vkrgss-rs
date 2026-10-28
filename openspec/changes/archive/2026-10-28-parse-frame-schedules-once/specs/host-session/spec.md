## ADDED Requirements

### Requirement: Frame entries are read by one rule
The input script and the states schedule SHALL be read by one rule for their entries:
separated by commas or line breaks, blanks and empty entries ignored, the frame before the
first `:` as a number from 0. Each SHALL keep its own reading of what follows the colon and
its own error messages, which name the entry as they did before.

#### Scenario: Same results
- **WHEN** an input script and a states schedule with blanks, empty entries and a trailing comma are read
- **THEN** each gives the entries it gave before this change

#### Scenario: Same messages
- **WHEN** an entry without a colon or without a frame number is read by each parser
- **THEN** the error message is word for word the one it was before this change, naming the entry

#### Scenario: One reader
- **WHEN** the source of `crates/vkrgss-host/src/script.rs` and `states.rs` is read
- **THEN** neither splits the text into entries or parses the frame number itself; both use the shared reader
