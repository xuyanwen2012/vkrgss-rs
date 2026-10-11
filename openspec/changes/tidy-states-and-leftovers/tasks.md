# Tasks

## 1. Leftovers in `vkxp-core`

- [x] 1.1 `Tilemap::strips` without its parameter; the comment on `Sprite::bush_depth` as in `design.md`. Verify: `cargo test -p vkxp-core --locked` passes.

## 2. Protocol and its users

- [x] 2.1 `StateFailed { slot, load }`, version 9, every sender and the session as in `design.md`; fixtures and expected files that carry the version are regenerated. Verify: the scenarios of `specs/guest-host-protocol/spec.md` and "Failed save while a load waits" and "Failed load"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-proto -p vkxp-compose-ref -p vkxp-binding -p vkxp-host -p vkxp --locked` passes.

## 3. Notices

- [ ] 3.1 The session's next deadline and its use in the window loop. Verify: scenario "No frames"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-host --locked` passes.

## 4. Socket directories

- [ ] 4.1 The sweep in `GuestProcess::start`. Verify: scenario "Left by a killed run" as a test with the base directory in a temporary directory; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` passes.

## 5. Documentation

- [ ] 5.1 README: protocol version 9 and what it added; the sentence about the socket directory mentions the removal at the next start. Verify: the README names version 9 and no longer says the protocol is at version 8.

## 6. Integration check

- [ ] 6.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate tidy-states-and-leftovers --strict` passes.
