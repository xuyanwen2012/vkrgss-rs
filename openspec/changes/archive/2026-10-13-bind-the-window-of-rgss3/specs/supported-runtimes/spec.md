## ADDED Requirements

### Requirement: The window class of RGSS3 in the documents
`docs/compatibility.md` SHALL have, under the section "RPG Maker VX Ace", a subsection on
the window that says what the class `Window` of a game of VX Ace has, that it is drawn by
the rules measured on the original, where vkxp goes on while the original ends with a page
fault, and what was not measured. It SHALL NOT change what the documents say about the
state of VX Ace.

#### Scenario: What the subsection says
- **WHEN** the subsection is compared with the decisions 2, 4, 6 and 7 and the Follow-ups of this change's `design.md`
- **THEN** it names the constructor without a viewport, the methods RGSS3 adds, the tone object, the calls and sizes at which the original faults with what vkxp does there, a disposed skin or contents bitmap drawn as none, and as not measured or not followed: a window whose viewport is disposed, `viewport=` with a disposed viewport, and the wording of the errors it lists under Follow-ups

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged, and the subsection says nothing that the code does not do

#### Scenario: The section of VX is left alone
- **WHEN** the difference of `docs` against the start of the change is read
- **THEN** it names `compatibility.md` only, and in it nothing outside the section "RPG Maker VX Ace" changed
