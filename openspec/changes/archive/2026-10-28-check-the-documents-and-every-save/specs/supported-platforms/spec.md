## ADDED Requirements

### Requirement: Every save of a comparison game is compared
`cargo xtask xarch` SHALL compare every file a run wrote below the copy of a game, whatever its
name, SHALL report per game how many saves it compared, and SHALL fail for a game whose entry
declares `saves = N` when fewer than N saves were compared.

#### Scenario: Numbered save
- **WHEN** two outputs of a game hold `Save/1.rvdata` with different bytes
- **THEN** the comparison reports that save as different and fails

#### Scenario: Data files are not saves
- **WHEN** the copy holds `Data/Map001.rxdata` unchanged from the game's directory
- **THEN** it is not counted as a save

#### Scenario: Expected saves missing
- **WHEN** a game declares `saves = 2` and its outputs hold no file written by the run
- **THEN** the comparison fails and names the game and the expected count
