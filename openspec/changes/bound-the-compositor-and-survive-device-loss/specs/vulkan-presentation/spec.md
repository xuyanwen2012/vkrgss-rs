## ADDED Requirements

### Requirement: A frame not shown stays unshown
A present that shows nothing SHALL leave the newest frame counted as not yet shown, whatever
the reason, so that the next present that shows it is paced as its first.

#### Scenario: Out of date twice
- **WHEN** both attempts of a present meet an out-of-date swapchain, and the next present shows the frame
- **THEN** the pacing statistics count that frame once, at the second present

### Requirement: The surface is released when idling fails
Releasing the surface SHALL destroy the swapchain and let the surface go also when waiting for
the device to be idle fails, and SHALL then report that failure.

#### Scenario: Idle fails
- **WHEN** waiting for the device to be idle fails while the surface is released
- **THEN** an error is returned and the compositor reports its surface as released
