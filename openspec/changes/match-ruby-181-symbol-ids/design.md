# Design

## Context

- Measured with `Symbol.all_symbols` in the original runtime (RGSS103J, Ruby 1.8.1): 2002
  symbols exist before the first game script; the list with numbers is in this clone at
  `local/measurements/symbols.startup.txt` (ignored by git; it is committed by this change as
  data, being a list of measured numbers and of names that are Ruby and RGSS API names). The
  highest number is 16185 (`s`, a local of the runtime's script loader), that is counter 2023
  with scope bits; the first local variable a game script creates gets 16193, then 16201.
  The parser's internal identifiers (`internal_id`) draw from the same counter in both
  versions and need no special treatment.
- A symbol number is `(counter << 3) | scope`, with the scope derived from the form of the
  name, in both Ruby versions; operator symbols have fixed small numbers and already agree.
  Attribute-setter numbers are derived from the base name's number.
- In vkxp the first script-created symbol was measured as 17489: 1.8.7 interns more names at
  start-up, and the binding's own Ruby prelude interns names too. The built-in `RPG` scripts are
  evaluated by vkxp from an extracted file and intern the same names as in the original.
- `rb_intern` in `parse.y` (the build compiles the pre-generated `parse.c`; both files must be
  changed consistently, see how the existing patches treat generated files) assigns
  `last_id++` to new names.

## Decisions

- Generate a C table sorted by name from the data file at build time (or commit the generated
  table next to the data file, whichever fits the manifest rules), and look a new name up in it
  before a number is assigned.
- Two counters: a provisional one starting far above the table (for names outside the table
  before the mark) and the sequence counter starting at 2024, used after the mark. A flag set
  per provisional name records whether it has been "seen" after the mark; the first
  `rb_intern` of it after the mark advances the sequence counter once and keeps the name's
  number.
- `vkxp-ruby-sys` exposes the mark as one function; `vkxp-binding` calls it once, after the
  prelude and the built-in scripts and before preload scripts and the game's scripts, and not
  again on reset (the original does not renumber either).
- Record the patch like the others. The coordinator verifies against the original with a
  symbol dump and a save file after the merge.

## Risks / Trade-offs

- [A table entry whose scope bits disagree with what 1.8.7 derives from the name] → Assert at
  build or start-up that every table number's scope equals the derived one, and fail loudly.
