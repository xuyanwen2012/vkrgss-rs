## ADDED Requirements

### Requirement: Word checks of the matcher stay within the subject
Matching any pattern against any subject SHALL NOT read a byte before the start or at or
after the end of the subject, including in the checks of a word boundary, of the beginning
and end of a word, and in every read of a character that these make.

#### Scenario: The saved input of nightly run 40296906749
- **WHEN** the input `crash-a04733116b3f41b160d8af29f7ded68eb83147d1` of that run is replayed through the fuzz harness of `regex` under the sanitizers
- **THEN** no report is made

#### Scenario: The same input through Ruby 1.9.2
- **WHEN** the same input is replayed through the fuzz harness of `regex192` under the sanitizers
- **THEN** no report is made
