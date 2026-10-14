## ADDED Requirements

### Requirement: Endings and loading of VX Ace in the documents
The section "RPG Maker VX Ace" of `docs/compatibility.md` SHALL say which rules of the
endings, of `load_data`, of the program name, of the title, of magic comments and of
loading code are those measured on the original, and SHALL list as known differences, each
with its reason, the rules vkxp does not follow. It SHALL NOT change what the documents
say about the state of VX Ace.

#### Scenario: What the section says
- **WHEN** the section is compared with the decisions 1 to 12 and the Follow-ups of this change's `design.md`
- **THEN** it names the long report shown whole and status 1 after an error with their reasons, `Failed to load script.` with status 1, `at_exit`, `exit!` and the status it gives, `$0`, `Untitled`, the magic comments left to the encodings made at first use, the source, names, lookup and errors of `load_data`, a path outside the game directory, `require_relative`, `autoload` raising at the call, and a required file compiled as UTF-8

#### Scenario: Old statements are gone
- **WHEN** the list of known differences of the start of VX Ace is read
- **THEN** it no longer says that `at_exit` handlers are not run as on the original while `exit!` is not followed, that a failed archive shows no `Failed to load script.`, or that `load_data` freezes no strings and has the errors and lookup of RGSS2

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged
