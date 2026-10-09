# Spec Delta

## ADDED Requirements

### Requirement: Seed, preload and replay options
The program SHALL accept `--seed N`, `--preload FILE` (repeatable), `--record FILE` and
`--replay FILE`. Without `--seed` a headless run SHALL use seed 0 and a windowed run a seed
taken from the clock; a replay SHALL use the seed stored in the recording unless `--seed` is
given. `--record` together with `--replay` SHALL be refused with the usage message.

#### Scenario: Headless default
- **WHEN** a game that draws a sprite at a `rand` position is run headless twice without `--seed`
- **THEN** the dumped frames are byte-identical

#### Scenario: Record and replay together
- **WHEN** both `--record` and `--replay` are given
- **THEN** the program prints the usage message and exits with a non-zero status

### Requirement: Encrypted archive of the game
The program SHALL read a game whose data is in `Game.rgssad`, and SHALL end with an error
naming the archive when it is damaged.

#### Scenario: Game in an archive
- **WHEN** a generated game whose `Data` folder exists only inside `Game.rgssad` is run headless
- **THEN** it runs like the same game with loose files
