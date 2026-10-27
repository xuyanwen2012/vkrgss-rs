## ADDED Requirements

### Requirement: Names interned by a tilemap of RGSS1
In a game of RGSS1 a tilemap and its autotiles SHALL take the symbol numbers the original XP
runtime takes: the names it interns at `Tilemap.new` and at the first use of `autotiles`, at
those moments and in its order, and no name of vkrgss's own that takes a number of the sequence.

#### Scenario: Tilemap, measured on the original
- **WHEN** a game of RGSS1 notes the number of a new name before and after `Tilemap.new`, after `t.autotiles[0]`, and after `t.autotiles[0] = nil`
- **THEN** the numbers advance as they do on the original XP runtime, which this change measured and recorded in its design and its test
