## MODIFIED Requirements

### Requirement: Extension search
When the path as given names no existing file, the lookup SHALL try it with each extension of
the requested kind appended, in a fixed order: `png`, `jpg`, `jpeg`, `bmp` for images; `mid`,
`ogg`, `wav`, `mp3`, `wma` for audio, and `ogg`, `mid`, `mp3`, `wav`, `wma` for the audio of
a game of RGSS3; none for data. Within one directory a file matching the path as given SHALL
win over one found by appending an extension.

#### Scenario: Extension omitted
- **WHEN** the image `Graphics/Titles/001-Title01` is looked up and only `001-Title01.jpg` exists
- **THEN** that file is returned

#### Scenario: Order of extensions
- **WHEN** both `Fog.png` and `Fog.jpg` exist and the image `Fog` is looked up
- **THEN** `Fog.png` is returned

#### Scenario: Name that contains a dot
- **WHEN** the image `Ver1.5` is looked up and `Ver1.5.png` exists
- **THEN** `Ver1.5.png` is returned

#### Scenario: Audio of RGSS3, measured on the original
- **WHEN** `Theme` exists with all five audio extensions, and `Pair` in turn with each two of `mid`, `mp3`, `wav` and `wma`, and each name is looked up as audio of a game of RGSS3
- **THEN** `Theme.ogg` is returned, and for each pair the file whose extension comes first in `mid`, `mp3`, `wav`, `wma`

#### Scenario: Audio of the other runtimes
- **WHEN** `Theme` exists with all five audio extensions and is looked up as audio
- **THEN** `Theme.mid` is returned

#### Scenario: Audio of RGSS3 in a package and in the archive
- **WHEN** only the run-time package holds `Audio/BGM/Theme.ogg`, and in another game only the encrypted archive holds `Audio\SE\Hit.ogg`, and both names are read without extension as audio of a game of RGSS3
- **THEN** the bytes of each are returned

#### Scenario: Other kinds with the audio of RGSS3
- **WHEN** `Fog.png` exists and `Fog` is looked up as audio of a game of RGSS3
- **THEN** the lookup fails
