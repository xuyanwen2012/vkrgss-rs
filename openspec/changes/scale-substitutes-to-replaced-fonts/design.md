# Design

## Context

See `proposal.md` for motivation. What this change starts from:

- `vkxp-text` resolves a request by trying each requested name as an installed family and then
  through the substitution table (caller entries before built-in ones), and otherwise falls
  back. The selected face is scaled so that its own line box, taken from the OS/2
  `usWinAscent` and `usWinDescent` (falling back to `hhea`), is as high as the requested size;
  `line_metrics` in `face.rs` is the one place that knows this, and it is marked as a
  placeholder for what the original runtime asks of GDI.
- Characters the selected face lacks are drawn from fallback fonts at the selected face's
  pixels per em and on its baseline.
- `FontSources` has `substitutions: Vec<(String, Vec<String>)>` and derives `Eq`.
- The test fonts are Liberation Sans Regular and Noto Sans Symbols 2. Liberation Sans is
  metric-compatible with Arial: same advances, and a line box of 1854 + 434 units of a
  2048-unit em. Its em dash is one em wide.
- Asking GDI for a font by cell height, as the placeholder assumes, makes the size the ascent
  plus descent of the font that was asked for. The classic CJK fonts of Windows are drawn on a
  256-unit em with an ascent of 220 and a descent of 36 units, so their line box is exactly one
  em and a full-width character at size 22 is 22 pixels wide. These numbers are supplied from
  memory of those fonts' metric tables; no copy of the fonts is available to this project.

## Goals / Non-Goals

**Goals:**

- Text set in a replacement has the em size the named font would have had.
- Nothing changes for fonts that are installed.

**Non-Goals:**

- Equal advance widths for every character. A replacement that is not metric-compatible keeps
  its own proportions; for full-width CJK characters, whose advance is one em in every such
  font, the widths do come out equal.
- Deciding whether `Font.size` is a cell height or an em height. That placeholder stays where
  it is; this change only makes replacements follow whatever the named font would do.

## Decisions

### A line box is three integers

`LineBox { ascent, descent, units_per_em }` holds font units as the font files do, so the
values are exact, comparable and hashable, and `FontSources` keeps `Eq`. `FontSources` gains
`line_boxes: Vec<(String, LineBox)>`, empty by default. Lookup is by the same case-insensitive
family key the substitution table uses, caller entries first, then the built-in list;
`default_line_box(family)` exposes the built-in list like `default_substitutions` does.

### Where it applies

Resolution already knows how a face was found. It now also returns the line box to set the
text in: the replaced family's when the face came through a substitution entry for a requested
name whose line box is known, otherwise none. `pixels_per_em` and `baseline` take that line box
when present and the face's own otherwise. Everything downstream (advances, the glyph cache
key, per-character fallback at the selected face's pixels per em) follows from the pixels per
em and needs no other change.

The line box belongs to the requested name, not to the replacement list: `SimHei` and
`Microsoft YaHei` share replacements but not metrics.

### The built-in list

| Families | ascent | descent | em | Status |
|---|---|---|---|---|
| Arial | 1854 | 434 | 2048 | checked against Liberation Sans in a test |
| Times New Roman | 1825 | 443 | 2048 | from memory |
| Courier New | 1705 | 615 | 2048 | from memory |
| Tahoma | 2049 | 423 | 2048 | from memory |
| Verdana | 2059 | 430 | 2048 | from memory |
| SimSun, 宋体, NSimSun, 新宋体, SimHei, 黑体, KaiTi_GB2312, 楷体_GB2312, FangSong_GB2312, 仿宋_GB2312 | 220 | 36 | 256 | from memory |
| MS Gothic, ＭＳ ゴシック, MS PGothic, ＭＳ Ｐゴシック, MS UI Gothic, MS Mincho, ＭＳ 明朝, MS PMincho, ＭＳ Ｐ明朝 | 220 | 36 | 256 | from memory |

Every row marked "from memory" is a `PLACEHOLDER` in the code, to be confirmed by reading the
font files or by measuring the original runtime. Families not in the list (`Microsoft YaHei`,
`Meiryo`, `KaiTi` of later Windows versions and others) keep today's behaviour until their
numbers are established.

### Ink outside the line box

A replacement scaled up to the replaced family's em can have ink above or below the line box,
most visibly the descenders of Latin letters in a CJK replacement. The mask stays as high as
the size and such ink is clipped, as today's rule already says; the height a script measures
must not change.

## Risks / Trade-offs

- [A number in the list is wrong] → The text of that family is off by the same ratio; the row
  is a marked placeholder and one line to correct.
- [Clipped descenders] → Accepted for now; noted for the comparison with the original, whose
  own fonts fit their boxes.
- [A caller relied on replacements being scaled by their own box] → No such caller exists yet.
