//! The `extract-rpg` command: copies the built-in `RPG` scripts out of an RGSS DLL.
//!
//! The original runtime keeps the Ruby source of its built-in scripts (the `RPG` data classes,
//! `RPG::Cache`, `RPG::Sprite`, `RPG::Weather`) in its data section as NUL-terminated strings
//! that each start with `module RPG;`. The DLLs of versions 1.00 and 1.02 store that section
//! as it is; later ones are compressed by their protector and show the strings only once
//! unpacked. That source is Enterbrain's and is never committed: this tool writes it to a
//! file of the user's, which the `vkxp` program loads.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

/// What every built-in script block starts with.
const MARKER: &[u8] = b"module RPG;";

/// Whether a byte belongs to Ruby source as the DLLs store it: printable ASCII, a tab or a
/// line break.
fn is_text(byte: u8) -> bool {
    matches!(byte, b' '..=b'~' | b'\t' | b'\n' | b'\r')
}

/// The built-in script blocks in `bytes`, in the order they appear: every NUL-terminated
/// string that starts with [`MARKER`] and consists of text.
fn blocks(bytes: &[u8]) -> Vec<&[u8]> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(offset) = bytes[at..]
        .windows(MARKER.len())
        .position(|window| window == MARKER)
    {
        let start = at + offset;
        let length = bytes[start..]
            .iter()
            .position(|byte| !is_text(*byte))
            .unwrap_or(bytes.len() - start);
        let end = start + length;
        if bytes.get(end) == Some(&0) {
            found.push(&bytes[start..end]);
        }
        // A string that ends otherwise is not a block; the search goes on behind its marker.
        at = end.max(start + MARKER.len());
    }
    found
}

/// The `extract-rpg` command: writes the blocks found in `dll` to `output`, each followed by
/// a line break. Fails without writing anything when there is none.
pub fn run(dll: &Path, output: &Path) -> Result<()> {
    let bytes = fs::read(dll).with_context(|| format!("cannot read {}", dll.display()))?;
    let blocks = blocks(&bytes);
    if blocks.is_empty() {
        bail!(
            "extract-rpg: {} holds no built-in script block (no text starting with `module \
             RPG;`). The RGSS DLLs of version 1.03 and later are compressed by their \
             protector; use a DLL of version 1.00 or 1.02, or an unpacked one",
            dll.display()
        );
    }
    let mut source = Vec::new();
    for block in &blocks {
        source.extend_from_slice(block);
        source.push(b'\n');
    }
    fs::write(output, source).with_context(|| format!("cannot write {}", output.display()))?;
    eprintln!(
        "extract-rpg: wrote {} block(s) to {}",
        blocks.len(),
        output.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIRST: &str = "module RPG;class Map;attr_accessor :width;end;end;";
    const SECOND: &str = "module RPG;module Cache;@cache = {};end;\r\nend;";

    /// A file like a DLL's data section: binary data around NUL-terminated strings.
    fn image(strings: &[&str]) -> Vec<u8> {
        let mut bytes = vec![0x4d, 0x5a, 0x90, 0x00, 0xff, 0x01];
        for string in strings {
            bytes.extend_from_slice(string.as_bytes());
            bytes.extend_from_slice(&[0, 0, 0x80, 0x07]);
        }
        bytes
    }

    #[test]
    fn blocks_found_are_written_in_file_order() {
        let dir = tempfile::tempdir().unwrap();
        let dll = dir.path().join("RGSS.dll");
        let output = dir.path().join("rpg_builtin.rb");
        fs::write(&dll, image(&[FIRST, "module Other;end", SECOND])).unwrap();

        run(&dll, &output).unwrap();

        let written = fs::read_to_string(&output).unwrap();
        assert_eq!(written, format!("{FIRST}\n{SECOND}\n"));
    }

    #[test]
    fn nothing_found_fails_and_writes_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let dll = dir.path().join("RGSS.dll");
        let output = dir.path().join("rpg_builtin.rb");
        fs::write(&dll, image(&["module Other;end", "class RPG;end"])).unwrap();

        let error = run(&dll, &output).unwrap_err();

        assert!(error.to_string().contains("no built-in script block"));
        assert!(!output.exists());
    }

    #[test]
    fn a_marker_in_binary_data_is_not_a_block() {
        // Not terminated by a NUL: cut off by a control byte, and by the end of the file.
        let mut bytes = b"module RPG;x\x01\0".to_vec();
        bytes.extend_from_slice(FIRST.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(b"module RPG;unterminated");
        assert_eq!(blocks(&bytes), [FIRST.as_bytes()]);
    }

    #[test]
    fn an_unreadable_file_is_named() {
        let dir = tempfile::tempdir().unwrap();
        let dll = dir.path().join("absent.dll");
        let error = run(&dll, &dir.path().join("out.rb")).unwrap_err();
        assert!(error.to_string().contains("absent.dll"));
    }
}
