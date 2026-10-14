## MODIFIED Requirements

### Requirement: Strings the binding creates are text or bytes
Every Ruby string the binding creates SHALL be made through one of two functions: one for
bytes, which over Ruby 1.9.2 gives an ASCII-8BIT string, and one for text, which gives a
UTF-8 string. Over Ruby 1.8.7 the two SHALL give the same string. Text SHALL be only the source of
game scripts and preload files handed to the interpreter and the path of the script archive;
the built-in scripts file SHALL be handed over as bytes. Everything else SHALL
be bytes: serialized forms, `to_s`, font names, data for `Marshal`, buffers, file names.

#### Scenario: Serialized values
- **WHEN** over Ruby 1.9.2 a script evaluates `_dump(0).encoding.name` of a `Color`, a `Tone`, a `Rect` and a `Table`, and the third byte of `Marshal.dump` of each
- **THEN** every name is `"ASCII-8BIT"` and every third byte is `u`, the type of a user-defined dump without instance variables

#### Scenario: Values as strings
- **WHEN** over Ruby 1.9.2 a script evaluates `Color.new(1, 2, 3, 255).to_s` and `Rect.new(1, 2, 3, 4).to_s` with the name of the encoding of each
- **THEN** the strings are `(1.000000, 2.000000, 3.000000, 255.000000)` and `(1, 2, 3, 4)` and both names are `"ASCII-8BIT"`

#### Scenario: Font name
- **WHEN** over Ruby 1.9.2 a script evaluates `Font.new.name.encoding.name`
- **THEN** the result is `"ASCII-8BIT"`

#### Scenario: Message of an error of RGSS
- **WHEN** over Ruby 1.9.2 a script disposes a bitmap, calls `clear` on it and rescues the error
- **THEN** the message is `disposed bitmap` and the name of its encoding is `"ASCII-8BIT"`

#### Scenario: Script source is text
- **WHEN** over Ruby 1.9.2 a game is run whose built-in scripts file and whose preload script each store the name of the encoding and the size of a literal of one character outside ASCII
- **THEN** the built-in scripts store `"ASCII-8BIT"` and 2, and the preload script `"UTF-8"` and 1

#### Scenario: Round trip
- **WHEN** over Ruby 1.9.2 a script evaluates `Marshal.load(Marshal.dump(value)) == value` for a `Color`, a `Tone` and a `Rect` with values of its own, and for a `Table`, which defines no `==`, compares `_dump(0)` of the loaded table with that of the table dumped
- **THEN** every result is true
