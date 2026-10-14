## ADDED Requirements

### Requirement: Known differences of the value classes of RGSS3
`docs/compatibility.md` SHALL have, under a section "RPG Maker VX Ace", a subsection on the
value classes that says what `Color`, `Tone`, `Rect` and `Table` of a game of VX Ace take
that those of VX do not, and names as known differences every place where vkxp does not do
what the original runtime of RGSS3 was measured to do. It SHALL NOT change what the
documents say about the state of VX Ace.

#### Scenario: The differences are there
- **WHEN** the subsection is compared with the decisions 4, 6, 7 and 8 and the Follow-ups of this change's `design.md`
- **THEN** it names: `==` of version 3.0.0.1, which raises where vkxp returns false; `set` with an object of another native class; `Color._load` of 32 bytes, which the original does not limit; `Rect._load` and `Table._load` of a short String; the integers and the message of a `RangeError` that depend on the word size; and `Table#[]=` with one index too few

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged, and the subsection says nothing that the code does not do

#### Scenario: The section is there already
- **WHEN** the change starts from a tree whose `docs/compatibility.md` has a section "RPG Maker VX Ace"
- **THEN** the subsection is added to it, no second section of that name exists, and a sentence of that section that calls the value classes those of RGSS2 is brought up to date
