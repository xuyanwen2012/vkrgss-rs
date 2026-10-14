## Context

`prepare` read the built-in scripts file and made its source with `rb::text_new`, a UTF-8
string over Ruby 1.9.2. The original evaluates the built-in scripts it carries as a C string
(`rb_eval_string` makes a string of bytes of it), so the literals of `RPG::AudioFile#initialize`
(`name = ""`) and the rest are ASCII-8BIT. This was inferred from the save and is the reading
that explains it; the original's code was not read.

## Goals / Non-Goals

Goal: the saves of a game of VX Ace equal the original's where they hold a literal of the
built-in scripts. Non-goal: the encodings of preload files, which are vkxp's own.

## Decisions

- `rb::bytes_new` in place of `rb::text_new` for the built-in scripts, for every runtime:
  over Ruby 1.8.7 the two make the same string.

## Measured

After the fix, the vkxp side of the same oracle run (`--vkxp-only`, the same schedule and the
harness's seed) wrote `Save01.rvdata2` (19018 bytes) and `AutoSave.rvdata2` (21992 bytes)
byte for byte equal to the original's of the earlier run.
