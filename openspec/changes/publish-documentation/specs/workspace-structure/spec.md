## ADDED Requirements

### Requirement: Public documents
The repository SHALL have a `README.md` of at most 200 lines that states what vkxp is, its
status, how to build it and run a game, and the legal notice, and SHALL keep the manual in
topic files under `docs/`. It SHALL have `CHANGELOG.md`, `CONTRIBUTING.md` and
`THIRD-PARTY.md`. No tracked document SHALL state rules of one developer's machine.

#### Scenario: Landing page
- **WHEN** `README.md` is read
- **THEN** it has at most 200 lines and links to `docs/running.md`, `docs/building.md`, `docs/architecture.md`, `docs/development.md`, `docs/compatibility.md`, `CHANGELOG.md`, `CONTRIBUTING.md` and `THIRD-PARTY.md`

#### Scenario: Manual kept
- **WHEN** the headings of the README before this change are looked for in the files under `docs/`
- **THEN** each is found, apart from "Third-party source", whose text is in `THIRD-PARTY.md`

#### Scenario: Machine rules
- **WHEN** `AGENTS.md`, `CLAUDE.md`, `README.md`, `CONTRIBUTING.md` and the files under `docs/` are read
- **THEN** none of them names a graphics card of one developer's machine, the language of a chat or a directory outside the repository

### Requirement: Links of the documents resolve
A test SHALL fail when a relative link or image of `README.md`, `CHANGELOG.md`,
`CONTRIBUTING.md`, `THIRD-PARTY.md`, `AGENTS.md` or a file `docs/*.md` names a file or
directory that does not exist.

#### Scenario: All links valid
- **WHEN** the test runs on the repository
- **THEN** it passes

#### Scenario: Broken link
- **WHEN** the check is given a document that links to `docs/missing.md`
- **THEN** it reports that document and that target
