//! Helpers shared by the test files: engines built from the committed test fonts.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use swash::FontRef;
use vkrgss_text::{FontSources, TextEngine, TextMask, TextRequest};

pub(crate) const LIBERATION: &str = "Liberation Sans";
pub(crate) const LIBERATION_FILE: &str = "LiberationSans-Regular.ttf";
pub(crate) const SYMBOLS: &str = "Noto Sans Symbols 2";
pub(crate) const SYMBOLS_FILE: &str = "NotoSansSymbols2-Regular.ttf";

/// A character Noto Sans Symbols 2 has and Liberation Sans lacks.
pub(crate) const SYMBOL: char = '\u{2BC5}';
/// A character neither test font has.
pub(crate) const NOWHERE: char = '\u{4E2D}';

/// The directory of the committed test fonts.
pub(crate) fn fonts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fonts")
}

/// The sources of an engine that knows the two test fonts and nothing else.
pub(crate) fn test_sources() -> FontSources {
    FontSources::directories_only([fonts_dir()])
}

/// An engine that knows the two test fonts and nothing else.
pub(crate) fn engine() -> TextEngine {
    TextEngine::new(&test_sources())
}

/// Copies one of the test fonts into `directory` and returns the copy's path.
pub(crate) fn copy_font(directory: &Path, file: &str) -> PathBuf {
    let target = directory.join(file);
    std::fs::copy(fonts_dir().join(file), &target).expect("copy test font");
    target.canonicalize().expect("canonical path of the copy")
}

pub(crate) fn request(family: &str, size: u32) -> TextRequest {
    TextRequest::new([family], size)
}

/// The family a request for `families` resolves to.
pub(crate) fn resolved_family(engine: &mut TextEngine, families: &[&str]) -> String {
    engine
        .resolve(&TextRequest::new(families.iter().copied(), 22))
        .expect("the engine has fonts")
        .family
}

/// Values read straight from a test font file, independently of the engine.
pub(crate) struct RawFont {
    data: Vec<u8>,
}

impl RawFont {
    pub(crate) fn load(file: &str) -> Self {
        Self {
            data: std::fs::read(fonts_dir().join(file)).expect("read test font"),
        }
    }

    fn font(&self) -> FontRef<'_> {
        FontRef::from_index(&self.data, 0).expect("test font parses")
    }

    pub(crate) fn has(&self, ch: char) -> bool {
        self.font().charmap().map(ch) != 0
    }

    pub(crate) fn units_per_em(&self) -> u32 {
        u32::from(self.font().metrics(&[]).units_per_em)
    }

    /// The OS/2 `usWinAscent` and `usWinDescent`.
    pub(crate) fn win_metrics(&self) -> (u32, u32) {
        let table = self
            .font()
            .table(swash::tag_from_bytes(b"OS/2"))
            .expect("OS/2 table");
        let value = |at: usize| u32::from(u16::from_be_bytes([table[at], table[at + 1]]));
        (value(74), value(76))
    }

    /// The whole pixels per em at which this font is set for `size`: the line box scaled to
    /// the size, one less when the box would then be higher than the size.
    pub(crate) fn pixels_per_em(&self, size: u32) -> u32 {
        let (ascent, descent) = self.win_metrics();
        let (units, em) = (ascent + descent, self.units_per_em());
        let pixels_per_em = (em * size + units / 2) / units;
        if pixels_per_em > 1 && (units * pixels_per_em + em / 2) / em > size {
            pixels_per_em - 1
        } else {
            pixels_per_em
        }
    }

    /// The rounded ascent and descent of this font at `size`.
    pub(crate) fn cell(&self, size: u32) -> (u32, u32) {
        let (ascent, descent) = self.win_metrics();
        let (pixels_per_em, em) = (self.pixels_per_em(size), self.units_per_em());
        let scaled = |units: u32| (units * pixels_per_em + em / 2) / em;
        (scaled(ascent), scaled(descent))
    }

    /// The advance of `ch` (of the missing-glyph box when the font lacks it) at
    /// `pixels_per_em`, in whole pixels: rounded to 1/64 pixel and then to a pixel.
    pub(crate) fn advance(&self, ch: char, pixels_per_em: u32) -> u32 {
        let font = self.font();
        let glyph = font.charmap().map(ch);
        let units = font.glyph_metrics(&[]).advance_width(glyph) as u64;
        let em = u64::from(self.units_per_em());
        let scale = (u64::from(pixels_per_em) * 64 * 65536 * 2 + em) / (2 * em);
        ((((units * scale + 0x8000) >> 16) + 32) >> 6) as u32
    }
}

/// The largest coverage value in the columns `columns` of `mask`.
pub(crate) fn max_coverage(mask: &TextMask, columns: std::ops::Range<u32>) -> u8 {
    let mut max = 0;
    for y in 0..mask.height {
        for x in columns.clone() {
            max = max.max(mask.coverage[(y * mask.width + x) as usize]);
        }
    }
    max
}

/// Windows language IDs for names added by [`derive_font`].
pub(crate) const ENGLISH_US: u16 = 0x0409;
pub(crate) const CHINESE_PRC: u16 = 0x0804;

/// Name IDs of the legacy and the typographic family name.
pub(crate) const NAME_FAMILY: u16 = 1;
pub(crate) const NAME_TYPOGRAPHIC_FAMILY: u16 = 16;

/// How a font derived from a committed test font differs from it.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Derivation<'a> {
    /// Records added to the `name` table: name ID, Windows language ID, text.
    pub names: &'a [(u16, u16, &'a str)],
    /// The OS/2 weight class; 600 and above also sets the bold flags.
    pub weight: Option<u16>,
    /// Sets the italic flags.
    pub italic: bool,
    /// Font units added to every glyph's advance width.
    pub wider_by: u16,
    /// Replaces the character map by one that maps no character at all.
    pub empty_cmap: bool,
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn put_u16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
}

/// Writes a variant of the test font `file` to `target` and returns its path. The variant is
/// made in memory from the committed file, so no further font needs to be committed.
pub(crate) fn derive_font(file: &str, target: &Path, change: Derivation<'_>) -> PathBuf {
    let source = std::fs::read(fonts_dir().join(file)).expect("read test font");
    let table_count = usize::from(u16_at(&source, 4));
    let mut tables: Vec<([u8; 4], Vec<u8>)> = (0..table_count)
        .map(|position| {
            let record = 12 + 16 * position;
            let tag = source[record..record + 4].try_into().unwrap();
            let offset = u32_at(&source, record + 8) as usize;
            let length = u32_at(&source, record + 12) as usize;
            (tag, source[offset..offset + length].to_vec())
        })
        .collect();
    let metric_count = {
        let hhea = &tables.iter().find(|(tag, _)| tag == b"hhea").unwrap().1;
        usize::from(u16_at(hhea, 34))
    };
    let bold = change.weight.is_some_and(|weight| weight >= 600);

    for (tag, data) in &mut tables {
        match &*tag {
            b"name" if !change.names.is_empty() => *data = with_names(data, change.names),
            b"OS/2" => {
                if let Some(weight) = change.weight {
                    put_u16(data, 4, weight);
                }
                // fsSelection: bit 0 italic, bit 5 bold, bit 6 regular.
                let mut selection = u16_at(data, 62);
                selection |= u16::from(change.italic) | u16::from(bold) << 5;
                if change.italic || bold {
                    selection &= !(1 << 6);
                }
                put_u16(data, 62, selection);
            }
            b"head" => {
                // macStyle: bit 0 bold, bit 1 italic.
                let style = u16_at(data, 44) | u16::from(bold) | u16::from(change.italic) << 1;
                put_u16(data, 44, style);
            }
            b"cmap" if change.empty_cmap => {
                // One Windows Unicode subtable of format 4 holding only the closing segment.
                let cmap: [u16; 18] = [
                    0, 1, 3, 1, 0, 12, 4, 24, 0, 2, 2, 0, 0, 0xFFFF, 0, 0xFFFF, 1, 0,
                ];
                *data = cmap.into_iter().flat_map(u16::to_be_bytes).collect();
            }
            b"hmtx" => {
                for metric in 0..metric_count {
                    let advance = u16_at(data, 4 * metric) + change.wider_by;
                    put_u16(data, 4 * metric, advance);
                }
            }
            _ => {}
        }
    }

    // Header and table directory, then the tables, each padded to four bytes.
    let mut font = source[..12 + 16 * table_count].to_vec();
    for (position, (_, data)) in tables.iter().enumerate() {
        let offset = font.len() as u32;
        font.extend_from_slice(data);
        font.resize(font.len().next_multiple_of(4), 0);
        let (words, _) = font[offset as usize..].as_chunks::<4>();
        let checksum = words.iter().fold(0u32, |sum, word| {
            sum.wrapping_add(u32::from_be_bytes(*word))
        });
        let record = 12 + 16 * position;
        font[record + 4..record + 8].copy_from_slice(&checksum.to_be_bytes());
        font[record + 8..record + 12].copy_from_slice(&offset.to_be_bytes());
        font[record + 12..record + 16].copy_from_slice(&(data.len() as u32).to_be_bytes());
    }
    std::fs::write(target, font).expect("write derived font");
    target
        .canonicalize()
        .expect("canonical path of the derived font")
}

/// The `name` table `table` with Windows Unicode records added.
fn with_names(table: &[u8], added: &[(u16, u16, &str)]) -> Vec<u8> {
    let count = usize::from(u16_at(table, 2));
    let strings = usize::from(u16_at(table, 4));
    // Platform, encoding, language and name ID, then the string's bytes.
    let mut records: Vec<([u16; 4], Vec<u8>)> = (0..count)
        .map(|position| {
            let record = 6 + 12 * position;
            let field = |number: usize| u16_at(table, record + 2 * number);
            let start = strings + usize::from(field(5));
            let text = table[start..start + usize::from(field(4))].to_vec();
            ([field(0), field(1), field(2), field(3)], text)
        })
        .collect();
    for &(name_id, language, text) in added {
        let text = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
        records.push(([3, 1, language, name_id], text));
    }
    records.sort();

    let mut result = vec![0; 6 + 12 * records.len()];
    put_u16(&mut result, 2, records.len() as u16);
    let strings = result.len() as u16;
    put_u16(&mut result, 4, strings);
    let mut offset = 0;
    for (position, (key, text)) in records.iter().enumerate() {
        let record = 6 + 12 * position;
        for (number, value) in key.iter().enumerate() {
            put_u16(&mut result, record + 2 * number, *value);
        }
        put_u16(&mut result, record + 8, text.len() as u16);
        put_u16(&mut result, record + 10, offset);
        offset += text.len() as u16;
    }
    for (_, text) in &records {
        result.extend_from_slice(text);
    }
    result
}
