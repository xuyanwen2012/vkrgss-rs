## MODIFIED Requirements

### Requirement: Symbol numbers as in the original runtime
The interpreter SHALL be started with the committed start-up table of RGSS1 or of RGSS2,
RGSS1 unless the caller names the other. A name of that table SHALL have the number listed
there. After the host has marked the start of the game's scripts, a new name SHALL get the
next number of the sequence that runtime continues with. A name interned before the mark that
is not in the table SHALL get a number outside the sequence, and its first use after the mark
SHALL consume one number of it.

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

#### Scenario: Table names of RGSS2
- **WHEN** the interpreter is started with the table of RGSS2 and `:s.to_i`, `:$TEST.to_i` and `:brightness.to_i` are evaluated
- **THEN** they equal the numbers the table of RGSS2 lists, and every other name of that table has its listed number too

#### Scenario: First new symbols of RGSS2, measured on the original
- **WHEN** the interpreter is started with the table of RGSS2 and the first script after the mark is `zz_first = 1; zz_second = 2`
- **THEN** `[:zz_first.to_i, :zz_second.to_i]` is `[16089, 16097]`

#### Scenario: A name of the other table
- **WHEN** the interpreter is started with the table of RGSS2 and a name that only the table of RGSS1 lists is first used after the mark
- **THEN** it gets the next number of the sequence, like any new name
