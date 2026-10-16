## MODIFIED Requirements

### Requirement: Lookups see changes made during the run
A lookup in the game directory or a run-time package SHALL find a file or directory that was
made, and SHALL NOT find one that was removed or renamed away, since an earlier lookup in the
same directory, whoever made the change and without a call that announces it, also when the
change falls within one tick of the clock the file system takes its times from.

#### Scenario: New file in a directory already looked into
- **WHEN** a lookup of `Graphics/Pictures/a` has succeeded, a file `Graphics/Pictures/b.png` is then written by other code, and `Graphics/Pictures/b` is looked up
- **THEN** it is found

#### Scenario: New directory
- **WHEN** a lookup of `Save/1` has failed because `Save` does not exist, the directory and a file `Save/1.cdat` are then made, and `Save/1.cdat` is looked up
- **THEN** it is found

#### Scenario: Removed file
- **WHEN** a file that a lookup found is deleted and looked up again
- **THEN** the lookup fails with the error for a missing file

#### Scenario: A picture a script wrote
- **WHEN** a script of a generated game evaluates `Dir.mkdir("Save")`, writes the bytes of a PNG picture to `Save/1.cdat` with `File.open("Save/1.cdat", "wb")`, and then `Bitmap.new("Save/1.cdat")`
- **THEN** a bitmap of the picture's size is returned

#### Scenario: Change within one tick
- **WHEN** a directory is listed by a lookup, a file is added to it, and the directory's modification time is then set back to the value it had when it was listed
- **THEN** a lookup of the new file finds it
