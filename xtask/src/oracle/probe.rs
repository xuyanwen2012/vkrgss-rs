//! The probe script and the input schedule it answers `Input` from.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail, ensure};

use super::runtime::Runtime;

/// The name of the probe in the script archive.
pub(super) const NAME: &str = "vkrgss oracle probe";

/// The seed both runtimes start their random numbers from.
const SEED: u32 = 20_261_009;

const TEMPLATE: &str = include_str!("probe.rb");

/// The probe for games of RGSS2: its frame counting and the repeat rule of its input differ.
const TEMPLATE_RGSS2: &str = include_str!("probe_rgss2.rb");

/// The probe for games of RGSS3: the probe of RGSS2 for Ruby 1.9.2, with buttons as Symbols
/// and the size of the screen in the request to hold a frame.
const TEMPLATE_RGSS3: &str = include_str!("probe_rgss3.rb");

/// What `--no-threads` adds to every probe.
const THREADS: &str = include_str!("probe_threads.rb");

/// The buttons a schedule may name, as `vkrgss --input` names them.
const BUTTONS: [&str; 20] = [
    "DOWN", "LEFT", "RIGHT", "UP", "A", "B", "C", "X", "Y", "Z", "L", "R", "SHIFT", "CTRL", "ALT",
    "F5", "F6", "F7", "F8", "F9",
];

/// The runtime a probe is generated for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    /// The original runtime: the probe holds the game at the capture frames and ends it
    /// after the last one.
    Original,
    /// vkrgss: the frames are dumped by the host, and the frame limit ends the run.
    Vkrgss,
}

/// The buttons held from given frames on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Schedule {
    entries: BTreeMap<u64, Vec<&'static str>>,
}

impl Schedule {
    /// Reads the syntax of `vkrgss --input`: `frame:BUTTONS` entries separated by commas or
    /// line breaks, `BUTTONS` being button names joined by `+`, or nothing.
    pub(crate) fn parse(text: &str) -> Result<Schedule> {
        let mut entries = BTreeMap::new();
        for entry in text.split([',', '\n']).map(str::trim) {
            if entry.is_empty() {
                continue;
            }
            let (frame, buttons) = entry.split_once(':').with_context(|| {
                format!("input entry `{entry}` is not of the form FRAME:BUTTONS")
            })?;
            let frame: u64 = frame.trim().parse().ok().with_context(|| {
                format!("input entry `{entry}` does not start with a frame number")
            })?;
            let mut held = Vec::new();
            for name in buttons.split('+').map(str::trim) {
                if name.is_empty() && buttons.trim().is_empty() {
                    break;
                }
                let button = BUTTONS
                    .iter()
                    .find(|button| button.eq_ignore_ascii_case(name))
                    .with_context(|| format!("input entry `{entry}`: `{name}` is not a button"))?;
                held.push(*button);
            }
            if entries.insert(frame, held).is_some() {
                bail!("the input has two entries for frame {frame}");
            }
        }
        Ok(Schedule { entries })
    }
}

/// The source of the probe of `runtime` for one side. `captures` are the frames to capture.
/// With `no_threads` the probe also replaces `Thread.new`.
pub(super) fn generate(
    runtime: Runtime,
    side: Side,
    schedule: &Schedule,
    captures: &[u64],
    no_threads: bool,
) -> String {
    let side = match side {
        Side::Original => "original",
        Side::Vkrgss => "vkrgss",
    };
    let schedule: Vec<String> = schedule
        .entries
        .iter()
        .map(|(frame, buttons)| {
            let buttons: Vec<String> = buttons.iter().map(|name| format!(":{name}")).collect();
            format!("[{frame}, [{}]]", buttons.join(", "))
        })
        .collect();
    let captures: Vec<String> = captures.iter().map(u64::to_string).collect();
    let template = match runtime {
        Runtime::Rgss1 => TEMPLATE,
        Runtime::Rgss2 => TEMPLATE_RGSS2,
        Runtime::Rgss3 => TEMPLATE_RGSS3,
    };
    let probe = template
        .replace("__SIDE__", side)
        .replace("__SCHEDULE__", &schedule.join(", "))
        .replace("__CAPTURES__", &captures.join(", "))
        .replace("__SEED__", &SEED.to_string());
    if no_threads { probe + THREADS } else { probe }
}

/// Reads the list of frames of `--frames`: numbers separated by commas. The result is in
/// ascending order without repetitions.
pub(super) fn parse_frames(text: &str) -> Result<Vec<u64>> {
    let mut frames = Vec::new();
    for part in text.split(',').map(str::trim) {
        let frame = part
            .parse()
            .ok()
            .with_context(|| format!("`{part}` is not a frame number"))?;
        frames.push(frame);
    }
    frames.sort_unstable();
    frames.dedup();
    ensure!(!frames.is_empty(), "no frame to capture");
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_schedule_is_read_as_vkrgss_reads_an_input_script() {
        let schedule = Schedule::parse("60:RIGHT, 100:\n120: c + Shift").unwrap();
        let entries: Vec<_> = schedule.entries.into_iter().collect();
        assert_eq!(
            entries,
            [
                (60, vec!["RIGHT"]),
                (100, vec![]),
                (120, vec!["C", "SHIFT"])
            ]
        );
        assert_eq!(Schedule::parse("").unwrap(), Schedule::default());
        for wrong in ["5", "x:C", "5:FIRE", "5:C+", "5:C, 5:"] {
            assert!(Schedule::parse(wrong).is_err(), "{wrong}");
        }
    }

    #[test]
    fn the_probe_holds_the_schedule_the_frames_and_the_side() {
        let schedule = Schedule::parse("60:RIGHT+C, 100:").unwrap();
        let original = generate(Runtime::Rgss1, Side::Original, &schedule, &[10, 20], false);
        assert!(original.contains("SIDE = \"original\"\n"), "{original}");
        assert!(
            original.contains("SCHEDULE = [[60, [:RIGHT, :C]], [100, []]]\n"),
            "{original}"
        );
        assert!(original.contains("CAPTURES = [10, 20]\n"), "{original}");
        assert!(original.contains("srand(20261009)\n"), "{original}");
        assert!(!original.contains("__"), "{original}");

        let vkrgss = generate(
            Runtime::Rgss1,
            Side::Vkrgss,
            &Schedule::default(),
            &[3],
            false,
        );
        assert!(vkrgss.contains("SIDE = \"vkrgss\"\n"), "{vkrgss}");
        assert!(vkrgss.contains("SCHEDULE = []\n"), "{vkrgss}");
    }

    #[test]
    fn the_probe_of_rgss2_is_another_text_with_the_same_values() {
        let schedule = Schedule::parse("0:DOWN").unwrap();
        let probe = generate(Runtime::Rgss2, Side::Original, &schedule, &[10], false);
        assert!(probe.contains("SIDE = \"original\"\n"), "{probe}");
        assert!(probe.contains("SCHEDULE = [[0, [:DOWN]]]\n"), "{probe}");
        assert!(probe.contains("CAPTURES = [10]\n"), "{probe}");
        assert!(probe.contains("srand(20261009)\n"), "{probe}");
        assert!(probe.contains("oracle_probe_fadeout"), "{probe}");
        assert!(!probe.contains("__"), "{probe}");
        let xp = generate(Runtime::Rgss1, Side::Original, &schedule, &[10], false);
        assert!(!xp.contains("oracle_probe_fadeout"), "{xp}");
    }

    /// The without-the-option scenario of `oracle-harness`, and what the option adds.
    #[test]
    fn the_probe_redefines_threads_only_with_the_option() {
        for runtime in [Runtime::Rgss1, Runtime::Rgss2, Runtime::Rgss3] {
            for side in [Side::Original, Side::Vkrgss] {
                let plain = generate(runtime, side, &Schedule::default(), &[3], false);
                assert!(!plain.contains("Thread"), "{plain}");
                let with = generate(runtime, side, &Schedule::default(), &[3], true);
                assert!(with.starts_with(&plain));
                assert!(with.contains("class Thread\n"), "{with}");
            }
        }
    }

    /// The one-text scenario of `oracle-harness`.
    #[test]
    fn the_probe_of_rgss3_is_one_ascii_text_for_both_sides() {
        let schedule = Schedule::parse("0:DOWN, 60:RIGHT+C, 100:").unwrap();
        for no_threads in [false, true] {
            let [original, vkrgss] = [Side::Original, Side::Vkrgss]
                .map(|side| generate(Runtime::Rgss3, side, &schedule, &[2, 5], no_threads));
            assert!(original.contains("SIDE = \"original\"\n"), "{original}");
            assert!(
                original.contains("SCHEDULE = [[0, [:DOWN]], [60, [:RIGHT, :C]], [100, []]]\n")
            );
            assert!(original.contains("CAPTURES = [2, 5]\n"), "{original}");
            assert!(!original.contains("__"), "{original}");
            let differing: Vec<_> = original
                .lines()
                .zip(vkrgss.lines())
                .filter(|(one, two)| one != two)
                .collect();
            assert_eq!(
                differing,
                [("  SIDE = \"original\"", "  SIDE = \"vkrgss\"")]
            );
            assert_eq!(original.lines().count(), vkrgss.lines().count());
            assert!(
                original
                    .bytes()
                    .chain(vkrgss.bytes())
                    .all(|byte| byte < 128)
            );
        }
        let rgss2 = generate(Runtime::Rgss2, Side::Original, &schedule, &[2], false);
        let rgss3 = generate(Runtime::Rgss3, Side::Original, &schedule, &[2], false);
        assert_ne!(rgss2, rgss3);
    }

    #[test]
    fn frames_are_sorted_and_unique() {
        assert_eq!(parse_frames("20, 10,20").unwrap(), [10, 20]);
        assert!(parse_frames("").is_err());
        assert!(parse_frames("1,two").is_err());
    }
}
