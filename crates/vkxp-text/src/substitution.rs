//! The table that replaces family names a system does not have.

use std::collections::HashMap;

/// Built-in substitutions: the names that share a replacement list, then the list. All
/// replacements are openly licensed families commonly packaged for Linux.
const BUILT_IN: &[(&[&str], &[&str])] = &[
    (
        &["Arial", "Helvetica"],
        &["Liberation Sans", "Arimo", "DejaVu Sans"],
    ),
    (
        &["Tahoma", "Verdana", "Segoe UI", "Trebuchet MS"],
        &["DejaVu Sans", "Liberation Sans", "Noto Sans"],
    ),
    (
        &["Times New Roman", "Times", "Georgia"],
        &["Liberation Serif", "Tinos", "DejaVu Serif"],
    ),
    (
        &["Courier New", "Courier", "Consolas", "Lucida Console"],
        &["Liberation Mono", "Cousine", "DejaVu Sans Mono"],
    ),
    (
        &["MS Gothic", "ＭＳ ゴシック", "MS UI Gothic"],
        &[
            "Noto Sans CJK JP",
            "IPAGothic",
            "Source Han Sans JP",
            "VL Gothic",
        ],
    ),
    (
        &["MS PGothic", "ＭＳ Ｐゴシック", "Meiryo", "メイリオ"],
        &[
            "Noto Sans CJK JP",
            "IPAGothic",
            "IPAPGothic",
            "Source Han Sans JP",
            "VL PGothic",
        ],
    ),
    (
        &["MS Mincho", "ＭＳ 明朝", "MS PMincho", "ＭＳ Ｐ明朝"],
        &["Noto Serif CJK JP", "IPAMincho", "Source Han Serif JP"],
    ),
    (
        &["SimSun", "宋体", "NSimSun", "新宋体"],
        &[
            "Noto Serif CJK SC",
            "Source Han Serif SC",
            "AR PL UMing CN",
            "Noto Sans CJK SC",
            "WenQuanYi Micro Hei",
        ],
    ),
    (
        &["SimHei", "黑体", "Microsoft YaHei", "微软雅黑"],
        &[
            "Noto Sans CJK SC",
            "Source Han Sans SC",
            "WenQuanYi Micro Hei",
            "WenQuanYi Zen Hei",
        ],
    ),
    (
        &["KaiTi", "楷体", "KaiTi_GB2312", "楷体_GB2312"],
        &[
            "AR PL UKai CN",
            "LXGW WenKai",
            "Noto Serif CJK SC",
            "Noto Sans CJK SC",
        ],
    ),
    (
        &["FangSong", "仿宋", "FangSong_GB2312", "仿宋_GB2312"],
        &[
            "Noto Serif CJK SC",
            "Source Han Serif SC",
            "Noto Sans CJK SC",
        ],
    ),
    (
        &["MingLiU", "細明體", "PMingLiU", "新細明體"],
        &["Noto Serif CJK TC", "Source Han Serif TC", "AR PL UMing TW"],
    ),
    (
        &["Microsoft JhengHei", "微軟正黑體"],
        &["Noto Sans CJK TC", "Source Han Sans TC"],
    ),
];

/// The form in which family names are compared: without regard to letter case or surrounding
/// white space.
pub(crate) fn family_key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Maps a family name to the families that may replace it.
#[derive(Debug, Clone)]
pub(crate) struct SubstitutionTable {
    /// Family key to replacement family keys, in order of preference.
    entries: HashMap<String, Vec<String>>,
}

impl SubstitutionTable {
    /// The built-in table with `extra` entries placed before the built-in replacements of the
    /// same name.
    pub(crate) fn new(extra: &[(String, Vec<String>)]) -> Self {
        let mut entries: HashMap<String, Vec<String>> = HashMap::new();
        let mut add = |name: &str, replacements: &mut dyn Iterator<Item = &str>| {
            let list = entries.entry(family_key(name)).or_default();
            for replacement in replacements {
                let key = family_key(replacement);
                if !list.contains(&key) {
                    list.push(key);
                }
            }
        };
        for (name, replacements) in extra {
            add(name, &mut replacements.iter().map(String::as_str));
        }
        for (names, replacements) in BUILT_IN {
            for name in *names {
                add(name, &mut replacements.iter().copied());
            }
        }
        Self { entries }
    }

    /// The replacement family keys for the family key `key`, in order of preference.
    pub(crate) fn replacements(&self, key: &str) -> &[String] {
        self.entries.get(key).map_or(&[], Vec::as_slice)
    }
}

/// The built-in replacements for `family`, in order of preference and as written in the
/// table. Empty when the table has no entry for the name.
pub fn default_substitutions(family: &str) -> &'static [&'static str] {
    let key = family_key(family);
    BUILT_IN
        .iter()
        .find(|(names, _)| names.iter().any(|name| family_key(name) == key))
        .map_or(&[], |(_, replacements)| replacements)
}
