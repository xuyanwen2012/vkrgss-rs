## MODIFIED Requirements

### Requirement: Executable of the guest process
The program SHALL take the executable it starts as its guest process from the runtime of
the game, in the directory of its own executable file: `vkrgss-guest-rgss12` for RGSS1 and
RGSS2, `vkrgss-guest-rgss3` for RGSS3. The program SHALL NOT start itself as a guest. When that file is missing, the run SHALL end before a
guest, a window or a device is started, with an error that names the file and the directory.

#### Scenario: XP and VX
- **WHEN** the executable of the guest process is asked for RGSS1 and for RGSS2
- **THEN** both answers are the path `vkrgss-guest-rgss12` in the directory that holds the running executable

#### Scenario: VX Ace
- **WHEN** it is asked for RGSS3
- **THEN** the answer is the path `vkrgss-guest-rgss3` in the directory that holds the running executable

#### Scenario: Missing guest executable
- **WHEN** a copy of the program that lies alone in a directory is run headless on a generated game of RGSS3
- **THEN** it exits with a non-zero status, its error stream names `vkrgss-guest-rgss3` and that directory, and no script has run

#### Scenario: A guest executable that ends at once
- **WHEN** a file `vkrgss-guest-rgss3` that only exits with status 3 lies beside that copy and the game is run again
- **THEN** the program exits with a non-zero status and its error stream says how the guest process ended, with the status 3

#### Scenario: XP and VX need their guest executable
- **WHEN** that copy, alone in its directory, runs a generated game of RGSS1 with `--guest process`
- **THEN** it exits with a non-zero status, its error stream names `vkrgss-guest-rgss12` and that directory, and no script has run

#### Scenario: XP and VX need no second file
- **WHEN** that copy, alone in its directory, runs a generated game of RGSS1 and one of RGSS2 headless with `--guest thread`
- **THEN** both runs exit with status 0
