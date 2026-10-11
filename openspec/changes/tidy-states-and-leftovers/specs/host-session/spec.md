## ADDED Requirements

### Requirement: State notices end and failures are told apart
A notice about states SHALL leave the window title when its time is over, also when the guest
sends no frame meanwhile. A failed answer SHALL be acted on by what it says failed: only a
failed load SHALL end the wait for a load of its slot.

#### Scenario: No frames
- **WHEN** a notice is shown, no frame arrives and the session is polled at the time it names as its next deadline
- **THEN** that deadline is the end of the notice and the title is the game's title again

#### Scenario: Failed save while a load waits
- **WHEN** a load of slot 3 waits for its answer and a failed save for slot 3 arrives
- **THEN** the title says that no state could be saved, no tick is sent, and the following loaded answer resets the host's side as usual

#### Scenario: Failed load
- **WHEN** a load of slot 3 waits for its answer and a failed load for slot 3 arrives
- **THEN** the title says that the slot holds no state and the ticks go on
