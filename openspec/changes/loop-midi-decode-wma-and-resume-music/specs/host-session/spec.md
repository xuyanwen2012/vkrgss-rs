## ADDED Requirements

### Requirement: Music goes on after a load
The session SHALL have the audio sink remember the slot when a state was saved into it,
and recall the slot when its state was loaded, after the channels were stopped.

#### Scenario: Save, then load
- **WHEN** the saved answer for slot 2 arrives, and later the loaded answer for slot 2
- **THEN** the sink was asked to remember slot 2 at the first, and at the second to stop the four channels and then to recall slot 2
