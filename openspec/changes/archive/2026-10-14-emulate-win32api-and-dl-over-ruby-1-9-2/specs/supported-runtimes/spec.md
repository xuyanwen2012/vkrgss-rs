## ADDED Requirements

### Requirement: Win32API and DL of VX Ace in the documents
The section "RPG Maker VX Ace" of `docs/compatibility.md` SHALL say what `Win32API` and `DL`
of a game of VX Ace do as measured on the original, which libraries vkxp finds, and SHALL
list as known differences, each with its reason, the rules vkxp does not follow. It SHALL
NOT change what the documents say about the state of VX Ace.

#### Scenario: What the section says
- **WHEN** the section is compared with the decisions 1 to 10 and the Follow-ups of this change's `design.md`
- **THEN** it names the class and its declarations, the libraries found with the list of system libraries, the conversions and the low 32 bits with the reason of the object ids, the values of the exports, the bare profile name, the refused address trick, `DL` refused, and that functions of a library a game ships are not run

#### Scenario: The old statement is gone
- **WHEN** the list of known differences of the start of VX Ace is read
- **THEN** it no longer says that `Win32API` and `DL` are not those of RGSS3

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged
