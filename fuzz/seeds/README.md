# Seeds of the fuzz targets of the file readers

Small files computed for the targets `bmp` and `rgssad`; none comes from a game. The target
`jpeg` starts from the committed JPEG files of `crates/vkrgss-image/testdata/jpeg/`.

| Directory | Content |
|---|---|
| `bmp` | BMP files of 1, 4 and 8 bits with palettes, of 24 bits bottom-up and top-down and of 32 bits with alpha, each a few pixels |
| `rgssad` | an archive of version 1 with three entries and one of version 3 with two, of a few bytes each |
