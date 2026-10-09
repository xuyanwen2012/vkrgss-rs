//! The fonts of a comparison: files given with `--font` that both runtimes draw text with.
//!
//! vkxp gets a directory that holds copies of the files. The original gets the same files in
//! the font directory of its Wine prefix and a registry script that maps the font names games
//! ask for to the family of the first file.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};

/// Font names that games ask for, and an installed font to draw them with when no font file
/// is shared. Without a replacement the runtime draws no text at all for a font the prefix
/// does not have.
/// PLACEHOLDER: the list and the replacement are a first guess; text is compared for layout
/// only.
const FONT_REPLACEMENTS: [(&str, &str); 12] = [
    ("MS PGothic", "Noto Sans CJK JP"),
    (
        "\u{ff2d}\u{ff33} \u{ff30}\u{30b4}\u{30b7}\u{30c3}\u{30af}",
        "Noto Sans CJK JP",
    ),
    ("MS Gothic", "Noto Sans CJK JP"),
    (
        "\u{ff2d}\u{ff33} \u{30b4}\u{30b7}\u{30c3}\u{30af}",
        "Noto Sans CJK JP",
    ),
    ("MS UI Gothic", "Noto Sans CJK JP"),
    ("SimSun", "Noto Sans CJK SC"),
    ("\u{5b8b}\u{4f53}", "Noto Sans CJK SC"),
    ("NSimSun", "Noto Sans CJK SC"),
    ("SimHei", "Noto Sans CJK SC"),
    ("\u{9ed1}\u{4f53}", "Noto Sans CJK SC"),
    ("Microsoft YaHei", "Noto Sans CJK SC"),
    ("Arial", "Liberation Sans"),
];

/// What the file of a font that this module installed into a prefix is named with.
const INSTALLED_PREFIX: &str = "vkxp-oracle-";

/// The fonts of one run, copied below its output directory.
#[derive(Debug)]
pub struct Shared {
    /// The family name of the first file, which the game font names are mapped to.
    pub family: String,
    /// The directory that holds the copies, for vkxp.
    pub dir: PathBuf,
    /// The copies, in the order the files were given.
    pub files: Vec<PathBuf>,
}

/// Checks that every one of `fonts` is a font file and copies them into `<out>/fonts`.
/// Nothing is written unless all of them are fonts. Without fonts there is nothing to share.
pub fn share(fonts: &[PathBuf], out: &Path) -> Result<Option<Shared>> {
    let mut read: Vec<(&str, String, Vec<u8>)> = Vec::new();
    for font in fonts {
        let data = fs::read(font)
            .with_context(|| format!("{}: the font file cannot be read", font.display()))?;
        let family = family_name(&data)
            .with_context(|| format!("{}: not a font file with a family name", font.display()))?;
        let name = font
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("{}: the name of the font file", font.display()))?;
        ensure!(
            read.iter().all(|(other, ..)| *other != name),
            "{}: two font files of this name were given",
            font.display()
        );
        read.push((name, family, data));
    }
    let Some((_, family, _)) = read.first() else {
        return Ok(None);
    };
    let family = family.clone();
    let dir = out.join("fonts");
    if dir.exists() {
        // The fonts of an earlier run would be found by vkxp like those of this one.
        fs::remove_dir_all(&dir).with_context(|| dir.display().to_string())?;
    }
    fs::create_dir_all(&dir).with_context(|| dir.display().to_string())?;
    let mut files = Vec::new();
    for (name, _, data) in read {
        let file = dir.join(name);
        fs::write(&file, data).with_context(|| file.display().to_string())?;
        files.push(file);
    }
    Ok(Some(Shared { family, dir, files }))
}

/// The line of the output that says whether the text of the two sides can be compared.
pub fn notice(shared: Option<&Shared>) -> String {
    match shared {
        Some(shared) => format!("fonts: both sides draw text with {}", shared.family),
        None => "fonts: text is not comparable, because each side draws it with a font of \
                 its own; give the font file to use with --font"
            .to_owned(),
    }
}

/// Puts the files of `shared` into `fonts_dir`, the font directory of a Wine prefix, in place
/// of those an earlier run put there, and returns their names in that directory.
pub fn install(shared: Option<&Shared>, fonts_dir: &Path) -> Result<Vec<String>> {
    fs::create_dir_all(fonts_dir).with_context(|| fonts_dir.display().to_string())?;
    for entry in fs::read_dir(fonts_dir).with_context(|| fonts_dir.display().to_string())? {
        let entry = entry.with_context(|| fonts_dir.display().to_string())?;
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(INSTALLED_PREFIX)
        {
            fs::remove_file(entry.path()).with_context(|| entry.path().display().to_string())?;
        }
    }
    let mut installed = Vec::new();
    for file in shared.iter().flat_map(|shared| &shared.files) {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| file.display().to_string())?;
        let name = format!("{INSTALLED_PREFIX}{name}");
        fs::copy(file, fonts_dir.join(&name)).with_context(|| file.display().to_string())?;
        installed.push(name);
    }
    Ok(installed)
}

/// The registry script of a prefix: the replacements of the game font names, by `family` if
/// fonts are shared, and the registration of the `installed` files of the font directory, so
/// that GDI enumerates them whether or not fontconfig knows them.
pub fn registry_script(family: Option<&str>, installed: &[String]) -> String {
    let quoted = |text: &str| format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""));
    let mut script = "Windows Registry Editor Version 5.00\r\n\r\n\
                      [HKEY_CURRENT_USER\\Software\\Wine\\Fonts\\Replacements]\r\n"
        .to_owned();
    for (name, replacement) in FONT_REPLACEMENTS {
        let replacement = family.unwrap_or(replacement);
        script += &format!("{}={}\r\n", quoted(name), quoted(replacement));
    }
    script +=
        "\r\n[HKEY_LOCAL_MACHINE\\Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts]\r\n";
    for (number, file) in (1..).zip(installed) {
        let name = format!("vkxp oracle font {number} (TrueType)");
        script += &format!("{}={}\r\n", quoted(&name), quoted(file));
    }
    script
}

/// A registry script as `regedit` reads it: UTF-16, little endian, with a byte order mark.
pub fn encode_script(script: &str) -> Vec<u8> {
    "\u{feff}"
        .encode_utf16()
        .chain(script.encode_utf16())
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// The family name of a TrueType or OpenType font, from the `name` table of the file, or of
/// the first face of a collection. `None` if `data` is no such font or names no family.
///
/// The family is record 1, or record 16 where there is no record 1; a record of the Windows
/// platform is preferred, the English one first, then one of the Unicode platform, then a
/// Macintosh Roman one.
pub fn family_name(data: &[u8]) -> Option<String> {
    let u16_at = |at: usize| {
        Some(usize::from(u16::from_be_bytes(
            *data.get(at..)?.first_chunk()?,
        )))
    };
    let u32_at = |at: usize| Some(u32::from_be_bytes(*data.get(at..)?.first_chunk()?) as usize);
    let tag_at = |at: usize| data.get(at..)?.first_chunk::<4>();

    let face = match tag_at(0)? {
        b"ttcf" => u32_at(12)?,
        _ => 0,
    };
    if ![b"\0\x01\0\0", b"OTTO", b"true"].contains(&tag_at(face)?) {
        return None;
    }
    let table = (0..u16_at(face + 4)?)
        .map(|index| face + 12 + 16 * index)
        .find(|&record| tag_at(record) == Some(b"name"))?;
    let name = u32_at(table + 8)?;
    let strings = name + u16_at(name + 4)?;

    let mut best: Option<((u8, u8), String)> = None;
    for record in (0..u16_at(name + 2)?).map(|index| name + 6 + 12 * index) {
        let (platform, encoding, language) =
            (u16_at(record)?, u16_at(record + 2)?, u16_at(record + 4)?);
        let by_id = match u16_at(record + 6)? {
            1 => 0,
            16 => 1,
            _ => continue,
        };
        let by_platform = match (platform, encoding, language) {
            (3, _, 0x409) => 0,
            (3, ..) => 1,
            (0, ..) => 2,
            (1, 0, _) => 3,
            _ => continue,
        };
        let start = strings + u16_at(record + 10)?;
        let Some(bytes) = data.get(start..start + u16_at(record + 8)?) else {
            continue;
        };
        let text = if platform == 1 {
            // Family names are ASCII in practice, where Macintosh Roman and Latin-1 agree.
            bytes.iter().copied().map(char::from).collect()
        } else {
            let units: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|unit| u16::from_be_bytes(*unit))
                .collect();
            let Ok(text) = String::from_utf16(&units) else {
                continue;
            };
            text
        };
        let rank = (by_platform, by_id);
        if !text.is_empty() && best.as_ref().is_none_or(|(other, _)| rank < *other) {
            best = Some((rank, text));
        }
    }
    best.map(|(_, text)| text)
}

/// A font file that holds nothing but a `name` table with these records of the Windows
/// platform, each a name identifier, a language and a text.
#[cfg(test)]
pub fn minimal_font(records: &[(u16, u16, &str)]) -> Vec<u8> {
    let mut entries = Vec::new();
    let mut strings = Vec::new();
    for &(id, language, text) in records {
        let bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
        for field in [3, 1, language, id, bytes.len() as u16, strings.len() as u16] {
            entries.extend(field.to_be_bytes());
        }
        strings.extend(bytes);
    }
    let mut name = Vec::new();
    for field in [0, records.len() as u16, 6 + entries.len() as u16] {
        name.extend(field.to_be_bytes());
    }
    name.extend(entries);
    name.extend(strings);

    // The offset table with one table, then the directory entry of `name`, then the table.
    let mut font = b"\0\x01\0\0\0\x01\0\0\0\0\0\0name\0\0\0\0".to_vec();
    font.extend(28u32.to_be_bytes());
    font.extend((name.len() as u32).to_be_bytes());
    font.extend(name);
    font
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_family_is_read_from_the_name_table() {
        let font = minimal_font(&[(4, 0x409, "Test Sans Regular"), (1, 0x409, "Test Sans")]);
        assert_eq!(family_name(&font).as_deref(), Some("Test Sans"));

        // The English record 1 comes before a localised one and before record 16.
        let font = minimal_font(&[
            (16, 0x409, "Test"),
            (1, 0x804, "\u{6d4b}\u{8bd5}"),
            (1, 0x409, "Test Sans Light"),
        ]);
        assert_eq!(family_name(&font).as_deref(), Some("Test Sans Light"));

        let font = minimal_font(&[(16, 0x409, "Test")]);
        assert_eq!(family_name(&font).as_deref(), Some("Test"));
    }

    #[test]
    fn a_collection_names_the_family_of_its_first_face() {
        let face = minimal_font(&[(1, 0x409, "Test Sans")]);
        let mut collection = b"ttcf\0\x01\0\0\0\0\0\x01\0\0\0\x10".to_vec();
        // The offsets of a face count from the start of the file.
        let mut face = face;
        face[20..24].copy_from_slice(&(28u32 + 16).to_be_bytes());
        collection.extend(face);
        assert_eq!(family_name(&collection).as_deref(), Some("Test Sans"));
    }

    #[test]
    fn what_is_no_font_has_no_family() {
        let font = minimal_font(&[(1, 0x409, "Test Sans")]);
        for wrong in [
            &b""[..],
            b"A text file.\n",
            &font[..font.len() - 4],
            &font[..20],
            &minimal_font(&[(4, 0x409, "Test Sans Regular")]),
        ] {
            assert_eq!(family_name(wrong), None, "{wrong:?}");
        }
    }

    /// The family-name scenario of `oracle-harness`, on the side of the original.
    #[test]
    fn the_registry_script_maps_the_game_fonts_to_the_family_of_the_first_file() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("TestSans.ttf");
        let second = dir.path().join("Other.otf");
        fs::write(&first, minimal_font(&[(1, 0x409, "Test Sans")])).unwrap();
        fs::write(&second, minimal_font(&[(1, 0x409, "Other")])).unwrap();
        let shared = share(&[first, second], &dir.path().join("out"))
            .unwrap()
            .unwrap();
        let fonts_dir = dir.path().join("prefix/drive_c/windows/Fonts");
        fs::create_dir_all(&fonts_dir).unwrap();
        fs::write(fonts_dir.join("vkxp-oracle-Earlier.ttf"), b"").unwrap();
        fs::write(fonts_dir.join("arial.ttf"), b"").unwrap();

        let installed = install(Some(&shared), &fonts_dir).unwrap();
        let script = registry_script(Some(&shared.family), &installed);

        assert_eq!(
            installed,
            ["vkxp-oracle-TestSans.ttf", "vkxp-oracle-Other.otf"]
        );
        let mut found: Vec<_> = fs::read_dir(&fonts_dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        found.sort();
        assert_eq!(
            found,
            [
                "arial.ttf",
                "vkxp-oracle-Other.otf",
                "vkxp-oracle-TestSans.ttf"
            ]
        );
        let lines: Vec<&str> = script.lines().collect();
        assert!(lines.contains(&"\"SimHei\"=\"Test Sans\""), "{script}");
        assert!(
            lines.contains(&"\"\u{9ed1}\u{4f53}\"=\"Test Sans\""),
            "{script}"
        );
        assert!(
            lines.contains(&"\"vkxp oracle font 1 (TrueType)\"=\"vkxp-oracle-TestSans.ttf\""),
            "{script}"
        );
        assert!(!script.contains("Noto"), "{script}");
        assert_eq!(
            encode_script("[\u{9ed1}]"),
            [0xff, 0xfe, b'[', 0, 0xd1, 0x9e, b']', 0]
        );
    }

    #[test]
    fn without_fonts_the_replacements_are_those_of_the_system() {
        let dir = tempfile::tempdir().unwrap();
        assert!(share(&[], &dir.path().join("out")).unwrap().is_none());
        assert!(!dir.path().join("out").exists());
        let script = registry_script(None, &[]);
        assert!(
            script.contains("\"SimHei\"=\"Noto Sans CJK SC\"\r\n"),
            "{script}"
        );
    }
}
