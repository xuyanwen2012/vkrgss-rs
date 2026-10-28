## ADDED Requirements

### Requirement: Fuzz targets of the file readers
`cargo xtask fuzz` SHALL also run three targets, `jpeg`, `bmp` and `rgssad`, that feed
arbitrary bytes to the JPEG decoder, the BMP decoder and the RGSSAD reader of both versions.
A target SHALL fail on a panic, on an allocation beyond the stated bounds, or on a decode that
returns part of an image. The `fuzz` job of `nightly.yml` SHALL run them.

#### Scenario: A short run
- **WHEN** `cargo xtask fuzz jpeg --seconds 30` is run
- **THEN** the target runs for about 30 seconds and ends with status 0 when nothing failed

#### Scenario: Nightly
- **WHEN** `.github/workflows/nightly.yml` is read
- **THEN** the matrix of the `fuzz` job names `jpeg`, `bmp` and `rgssad`
