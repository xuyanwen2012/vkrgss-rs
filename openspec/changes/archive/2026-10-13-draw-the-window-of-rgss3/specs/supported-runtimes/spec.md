## ADDED Requirements

### Requirement: Recorded decisions on the window of RGSS3
`docs/decisions.md` SHALL record, under "VX Ace", that vkxp goes on where the window of the
original runtime of RGSS3 ends with a page fault, as a known difference, that the rule for
a window without area rests on few measured pixels, and what of the window of RGSS2 stays
unmeasured. `docs/compatibility.md` SHALL say, for the window of VX, what is now measured of
skins smaller than the layout and what is not.

#### Scenario: The decisions are there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is compared with the decisions 4 to 7 of this change's `design.md`
- **THEN** it names the calls and sizes at which the original faults and what vkxp does there, the 14 pixels behind the window without area, and, for RGSS2, the cursor from a skin that cuts its block, the list of calls that decide the arrows and disposed bitmaps as not measured, with the date of the measurement

#### Scenario: The window of VX
- **WHEN** the section "The window of VX" of `docs/compatibility.md` is read
- **THEN** it no longer lists the background from a skin narrower or lower than 64 pixels as a difference, and it says that the cursor from a skin that cuts the cursor block was not measured on VX

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and `docs/compatibility.md` are read after this change
- **THEN** they do not say that a game of VX Ace runs, and `docs/compatibility.md` differs from before only in the section "The window of VX"
