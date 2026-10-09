# Spec Delta

## ADDED Requirements

### Requirement: Symbol numbers as in the original runtime
A name that the original runtime has interned before the first game script SHALL have the same
symbol number in vkxp, taken from the committed table. Once the host has marked the start of
the game's scripts, a name seen for the first time SHALL get the next number of the sequence
the original continues with. A name interned before the mark that is not in the table SHALL
get a number outside that sequence, and its first use after the mark SHALL consume one number
of the sequence.

#### Scenario: Table names
- **WHEN** `:name.to_i`, `:id.to_i` and `:s.to_i` are evaluated
- **THEN** they equal the numbers the table lists for those names

#### Scenario: First new symbols, measured on the original
- **WHEN** the first script after the mark is `zz_first = 1; zz_second = 2` and it evaluates `[:zz_first.to_i, :zz_second.to_i]`
- **THEN** the result is `[16193, 16201]`

#### Scenario: Instance variable order
- **WHEN** an object is given the instance variables `@torch_keep`, `@message_frame`, `@magic_number` and `@map_interpreter` in an order of the test's choosing and `instance_variables` is evaluated twice in separate processes
- **THEN** both results are equal, and equal to the order computed in the test from the symbol numbers and the interpreter's table layout

#### Scenario: Name outside the table before the mark
- **WHEN** the host interns a name that is not in the table before the mark, and the first game script then creates a new local variable
- **THEN** that variable's symbol number is the first of the sequence, as if the earlier name did not exist
