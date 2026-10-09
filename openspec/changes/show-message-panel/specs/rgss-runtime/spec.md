# Spec Delta

## ADDED Requirements

### Requirement: Message panel
While `print`, `p` or the report of an uncaught error waits for its dismissal, the guest SHALL
keep sending frames that show the message in a panel drawn above everything else, with the
scene underneath unchanged, and SHALL remove the panel when the dismissal arrives. Text SHALL
be wrapped to the panel's width, and lines beyond the panel's height SHALL be cut with an
ellipsis. When the caller makes `p` log-only, `p` SHALL show no panel and wait for nothing.

#### Scenario: Panel is visible
- **WHEN** a script that shows a red sprite calls `print("hello")` against a host that dismisses after three frames
- **THEN** those frames differ from the frame before the call in the panel's area, are unchanged outside it, and the frame after the dismissal equals the frame before the call

#### Scenario: Error report
- **WHEN** a script raises an uncaught error
- **THEN** frames with a panel are sent before the shutdown message

#### Scenario: Log-only p
- **WHEN** `p 1` is evaluated with `p` made log-only
- **THEN** the text is on the error stream, no modal message is sent and no frame is sent by the call
