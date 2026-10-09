# Spec Delta

## ADDED Requirements

### Requirement: Shared fonts
With one or more `--font FILE` options the harness SHALL make both runtimes draw text with
those files: it SHALL install each file into the Wine prefix of the run, register replacements
from the common game font names to the family name read from the first file, and pass vkxp a
font directory holding the same files. A file that does not exist or is not a font SHALL end
the run with an error naming it. Without the option the harness SHALL print that text is not
comparable.

#### Scenario: Family name
- **WHEN** a font file is given whose family name is `Test Sans`
- **THEN** the registry script prepared for the prefix maps `SimHei` and `黑体` to `Test Sans` and the vkxp command line names a font directory that holds a copy of the file

#### Scenario: Not a font
- **WHEN** `--font` names a text file
- **THEN** the run ends with an error naming the file before either runtime starts

#### Scenario: No font given
- **WHEN** the harness runs without `--font`
- **THEN** its output has a line saying that text is not comparable
