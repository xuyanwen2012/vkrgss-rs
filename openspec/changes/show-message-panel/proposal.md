# Proposal

## Why

`print`, `p` and the report of an uncaught script error only reach the error stream; a player
in a window sees nothing and the game goes on or ends without a word. The original shows a
message box and waits. The design record asks for a blocking panel inside the window.

## What Changes

- The guest draws a message panel over the current picture while a modal message is pending:
  a dark box with the text, wrapped and centred, above every sprite and viewport.
- The windowed host dismisses a modal message when the player presses a confirm or cancel key
  or clicks, not at once; that press does not reach the game. Headless runs still dismiss at
  once.
- A setting makes `p` log-only (no panel), as the design record states.

Out of scope: buttons, scrolling of very long messages (they are cut with an ellipsis), a
native dialog.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-runtime`: the message panel.
- `host-session`: when a modal message is dismissed.

## Impact

- Crates `vkxp-binding` (panel drawing and waiting), `vkxp-host` (modal state of the session),
  `vkxp` (the `p` setting as option and configuration key).
