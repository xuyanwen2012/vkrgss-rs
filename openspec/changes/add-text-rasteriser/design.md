# Design

## Context

See `proposal.md` for motivation. Facts that shape the design:

- `vkxp-core` defines `TextRasterizer` with `measure(font, text) -> (width, height)` and
  `rasterize(font, text) -> Mask`, where the mask must have exactly the measured size, and its
  `draw_text` does alignment, squeezing and blending. `vkxp-text` must not depend on
  `vkxp-core`; the binding adapts one to the other.
- The original runtime draws text with Windows GDI: grey-scale glyph outlines, whole-pixel
  advances, no shaping. `docs/decisions.md` asks for matching layout and advance widths, not
  identical glyph rasters, and for no text shaping.
- The test game requests `["楷体_GB2312", "黑体"]`, `黑体` and `Arial`, at sizes from 12 to 52.
- On the development machine `Arial` resolves to Liberation Sans, which has the same advance
  widths. The Chinese families are not installed; fontconfig's own fallback for them is a
  Latin font, so substitution must be done here. The installed Chinese font is a variable-font
  collection (`NotoSansCJK-VF.ttc`), so the font stack must handle collections and variations.
- `vkxp-text` is a guest crate: no `unsafe` of its own.

## Goals / Non-Goals

**Goals:**

- Text appears, in a sensible font, for any requested family name and any character an
  installed font covers.
- Deterministic output for a given set of font files.
- Fast enough that redrawing a message window every frame is not a concern.

**Non-Goals:**

- Shaping, kerning, ligatures, right-to-left text, colour glyphs.
- Synthesising bold or italic when a family lacks the face.
- Matching GDI's hinted advances and rasters exactly; that needs the original's fonts and its
  hinting, and is a later refinement behind the same interface.

## Decisions

### Crates

`fontdb` discovers and indexes fonts (system directories, extra directories, collections,
family names in every language a font declares). `swash` scales outlines, applies variation
defaults and renders grey-scale coverage. Both are pure Rust. If `swash` turns out unable to
load the variable collection, `skrifa` with `zeno` is the fallback; the choice is internal to
this crate.

### Engine and request

```text
TextEngine::new(sources)            sources: use system fonts or not, extra directories,
                                    extra substitutions, fallback family names
engine.is_available(family)      -> bool
engine.measure(&request, text)   -> (width, height)
engine.rasterize(&request, text) -> TextMask { width, height, coverage }
request: family names in order, size in pixels, bold, italic
```

Fonts from caller directories are loaded first and shadow system fonts of the same family.

### Lookup

For each requested name: exact family (case-insensitive, also against localised family names
such as `黑体` when a font declares them), then each replacement from the substitution table.
The built-in table maps the Windows families of `specs/text-rendering` to lists such as:

- `Arial` to Liberation Sans, Arimo, DejaVu Sans;
- `SimHei`, `黑体`, `Microsoft YaHei`, `微软雅黑` to Noto Sans CJK SC, Source Han Sans SC,
  WenQuanYi Micro Hei;
- `SimSun`, `宋体` to Noto Serif CJK SC, Source Han Serif SC;
- `KaiTi`, `楷体`, `楷体_GB2312` to AR PL UKai CN, then the Noto Serif and Sans CJK SC families;
- `MS Gothic`, `MS PGothic` to Noto Sans CJK JP, IPAGothic; `MS Mincho` to Noto Serif CJK JP.

The fallback list used when nothing matches, and per character, defaults to the Noto CJK
families followed by DejaVu Sans and Liberation Sans, filtered to what is installed; the caller
can replace it. Resolved requests are cached.

### Metrics

The selected font is scaled so that ascent plus descent equals the requested size, using the
metrics Windows uses for line height (the OS/2 `usWinAscent` and `usWinDescent`, falling back
to `hhea`). The baseline is `ascent` pixels (rounded) below the top of the mask. Fallback fonts
are scaled to the same pixels-per-em as the selected font and placed on the same baseline; what
does not fit vertically is clipped. Each character's advance is rounded to the nearest whole
pixel. The mask height is always the requested size.

### Rasterising and caching

Each character maps to one glyph by the font's character map; glyphs are rendered once per
(font face, pixels-per-em, glyph) into a cached coverage image and composed into the mask with
a per-pixel maximum where neighbours overlap. The cache is bounded by total bytes and evicts the
least recently used glyphs.

### Test fonts

Tests must not depend on what is installed, so two small fonts are committed under
`crates/vkxp-text/tests/fonts/` with the SIL Open Font License text: Liberation Sans Regular
(Latin; stands in for the "requested" font) and Noto Sans Symbols 2 (symbols absent from the
first; stands in for the fallback). Every scenario runs on an engine built from that directory
alone. One additional test builds an engine from the system's fonts and only checks that it
does not fail.

## Risks / Trade-offs

- [Substitute fonts have other advance widths than the Windows originals] → Inherent; fonts in
  a caller directory win, so a user who has the originals gets their widths.
- [The line-height rule is a guess at what the original passes to GDI] → Marked as placeholder;
  it is one function.
- [Rounded advances accumulate differently from GDI's hinted advances] → Same; refinement later.
- [1.1 MB of fonts in the repository] → Accepted for hermetic tests; both licences permit it.

## Open Questions

- Whether `Font.size` is the line height or the em height in the original runtime. To be
  measured with the oracle; it changes one scaling rule.
