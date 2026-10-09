# Spec Delta

## MODIFIED Requirements

### Requirement: Title and modal messages
A title message SHALL set the window title. A modal message SHALL be written to the error
stream. A headless run SHALL answer it with a dismissal at once; a windowed run SHALL answer
when a key mapped to C or B is newly pressed or a mouse button is clicked, and SHALL NOT show
that press to the game as input. Frames SHALL keep being shown and answered while a modal
message is pending.

#### Scenario: Title
- **WHEN** the guest sends a title
- **THEN** the window title becomes that text

#### Scenario: Modal
- **WHEN** the guest sends a modal message in a headless run
- **THEN** the text appears on the error stream and the guest receives a dismissal

#### Scenario: Waiting for the player
- **WHEN** the guest sends a modal message in a windowed session, three frames follow, and then the key for C is pressed
- **THEN** no dismissal is sent before the press, one is sent after it, the three frames are answered with ticks, and no snapshot shows C held by that press
