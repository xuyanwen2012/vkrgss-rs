## ADDED Requirements

### Requirement: Mask threshold is not negative
The threshold of a step of a masked transition SHALL NOT be below 0.

#### Scenario: Long transition
- **WHEN** step 1 of a masked transition of 257 frames is shown
- **THEN** its threshold is 0
