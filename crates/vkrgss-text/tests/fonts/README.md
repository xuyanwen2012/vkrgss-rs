# Test fonts

Inputs for the tests of `vkrgss-text`. Nothing here is linked into a build; the tests read the
files from this directory so that they do not depend on the fonts installed on the machine.

| File | Family | Version | Origin |
|---|---|---|---|
| `LiberationSans-Regular.ttf` | Liberation Sans | 2.1.5 | <https://github.com/liberationfonts/liberation-fonts> |
| `NotoSansSymbols2-Regular.ttf` | Noto Sans Symbols 2 | 2.008 | <https://github.com/notofonts/symbols> |

Both files are unmodified. In the tests Liberation Sans stands in for the font a game asks for
and Noto Sans Symbols 2 for a fallback font that has characters the first one lacks.

## A CFF font

`../fonts-cff/SourceCodePro-Regular.otf` is Source Code Pro 2.042 (Regular), unmodified, from
<https://github.com/adobe-fonts/source-code-pro>. It is a font with CFF outlines, which
FreeType hints with its PostScript hinter, for the tests of hinted glyph images. It lies in a
directory of its own so that the tests that load this directory keep seeing the two fonts
above alone. It declares the SIL Open Font License, Version 1.1; `../fonts-cff/LICENSE.SourceCodePro.md`
is the licence file distributed with it.

## Licences

Both fonts declare the SIL Open Font License, Version 1.1, in their `name` table.

- `LICENSE.LiberationSans.txt` is the licence file distributed with Liberation Sans. It holds
  the copyright notices of that font and the full text of the SIL Open Font License 1.1.
- `LICENSE.NotoSansSymbols2.txt` is the licence file that accompanied the copy of Noto Sans
  Symbols 2 used here, kept as received. It holds the text of the Apache License 2.0, under
  which earlier Noto releases were published; version 2.008 of the font itself names the SIL
  Open Font License 1.1, whose text is in the other file.
