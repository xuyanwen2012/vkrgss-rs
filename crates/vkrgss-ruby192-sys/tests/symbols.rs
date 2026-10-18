//! The interpreter's symbols must not be exported by an executable that links it.
//!
//! The interpreter defines general names such as `st_*` and those of its regular expression
//! engine, `onig*`, which a system library may define too. Were they in the dynamic symbol
//! table of the executable, a shared library loaded into the process would bind to them
//! instead of to its own. The build compiles with hidden visibility; this test links an
//! executable with `-rdynamic`, which exports everything that is not hidden, and reads its
//! dynamic symbol table.
//!
//! Kept to Linux and Android: the test reads an ELF file, and macOS links Mach-O.
#![cfg(not(vkrgss_interpreter_absent))]
#![cfg(any(target_os = "linux", target_os = "android"))]

mod common;

use std::fs;

const SHT_DYNSYM: u32 = 11;
const SHN_UNDEF: u16 = 0;
/// `STT_SECTION`, the type of a symbol that stands for a section.
const STT_SECTION: u8 = 3;

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn u64_at(bytes: &[u8], offset: usize) -> usize {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize
}

/// The names of the symbols that a little-endian ELF64 file defines in its dynamic symbol
/// table, which is what `nm -D --defined-only` lists.
fn defined_dynamic_symbols(elf: &[u8]) -> Vec<String> {
    assert_eq!(&elf[..4], b"\x7fELF", "an ELF file");
    assert_eq!((elf[4], elf[5]), (2, 1), "64-bit, little-endian");
    let section_headers = u64_at(elf, 0x28);
    let header_size = usize::from(u16_at(elf, 0x3a));
    let header_count = usize::from(u16_at(elf, 0x3c));
    let header = |index: usize| section_headers + index * header_size;

    let mut names = Vec::new();
    for index in 0..header_count {
        let section = header(index);
        if u32_at(elf, section + 0x04) != SHT_DYNSYM {
            continue;
        }
        let (offset, size) = (u64_at(elf, section + 0x18), u64_at(elf, section + 0x20));
        let entry_size = u64_at(elf, section + 0x38);
        // The section's link field names its string table.
        let strings = u64_at(elf, header(u32_at(elf, section + 0x28) as usize) + 0x18);
        for symbol in (offset..offset + size).step_by(entry_size) {
            if u16_at(elf, symbol + 6) == SHN_UNDEF {
                continue;
            }
            // A section symbol has no name; the linker of aarch64 puts some into the table,
            // and `nm -D` does not list them.
            if elf[symbol + 4] & 0xf == STT_SECTION {
                continue;
            }
            let name = strings + u32_at(elf, symbol) as usize;
            let end = name
                + elf[name..]
                    .iter()
                    .position(|&byte| byte == 0)
                    .expect("a NUL");
            names.push(String::from_utf8_lossy(&elf[name..end]).into_owned());
        }
    }
    names
}

#[test]
fn interpreter_symbols_are_not_exported_by_a_linked_executable() {
    let scratch = common::scratch_dir("symbols");
    let executable = common::build_ruby_executable(&scratch);
    let symbols = defined_dynamic_symbols(&fs::read(&executable).expect("the executable"));

    // The probe works: `-rdynamic` does export what is not hidden.
    assert!(
        symbols.iter().any(|name| name == "vkrgss_export_probe"),
        "the symbol of default visibility is missing, so the table shows nothing: {symbols:?}"
    );

    let leaked: Vec<&String> = symbols
        .iter()
        .filter(|name| {
            ["rb_", "ruby_", "st_", "Init_", "onig", "Onig"]
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
        .collect();
    assert!(
        leaked.is_empty(),
        "{} interpreter symbol(s) are exported, for example {:?}",
        leaked.len(),
        &leaked[..leaked.len().min(12)]
    );

    // Nothing else of the interpreter or of its extension leaks either: what is left is the
    // C start-up code and the probe.
    let unexpected: Vec<&String> = symbols
        .iter()
        .filter(|name| {
            !name.starts_with('_')
                && !["vkrgss_export_probe", "data_start"].contains(&name.as_str())
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "unexpected exported symbols: {unexpected:?}"
    );
}
