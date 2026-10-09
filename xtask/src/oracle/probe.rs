//! The probe script and the input schedule it answers `Input` from.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail, ensure};

/// The name of the probe in the script archive.
pub const NAME: &str = "vkxp oracle probe";

/// The seed both runtimes start their random numbers from.
const SEED: u32 = 20_261_009;

const TEMPLATE: &str = include_str!("probe.rb");

/// The buttons a schedule may name, as `vkxp --input` names them.
const BUTTONS: [&str; 20] = [
    "DOWN", "LEFT", "RIGHT", "UP", "A", "B", "C", "X", "Y", "Z", "L", "R", "SHIFT", "CTRL", "ALT",
    "F5", "F6", "F7", "F8", "F9",
];

/// The runtime a probe is generated for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The original runtime: the probe holds the game at the capture frames and ends it
    /// after the last one.
    Original,
    /// vkxp: the frames are dumped by the host, and the frame limit ends the run.
    Vkxp,
}

/// The buttons held from given frames on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule {
    entries: BTreeMap<u64, Vec<&'static str>>,
}

impl Schedule {
    /// Reads the syntax of `vkxp --input`: `frame:BUTTONS` entries separated by commas or
    /// line breaks, `BUTTONS` being button names joined by `+`, or nothing.
    pub fn parse(text: &str) -> Result<Schedule> {
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

/// The source of the probe for one side. `captures` are the frames to capture.
pub fn generate(side: Side, schedule: &Schedule, captures: &[u64]) -> String {
    let side = match side {
        Side::Original => "original",
        Side::Vkxp => "vkxp",
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
    TEMPLATE
        .replace("__SIDE__", side)
        .replace("__SCHEDULE__", &schedule.join(", "))
        .replace("__CAPTURES__", &captures.join(", "))
        .replace("__SEED__", &SEED.to_string())
}

/// Reads the list of frames of `--frames`: numbers separated by commas. The result is in
/// ascending order without repetitions.
pub fn parse_frames(text: &str) -> Result<Vec<u64>> {
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
    fn a_schedule_is_read_as_vkxp_reads_an_input_script() {
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
        let original = generate(Side::Original, &schedule, &[10, 20]);
        assert!(original.contains("SIDE = :original\n"), "{original}");
        assert!(
            original.contains("SCHEDULE = [[60, [:RIGHT, :C]], [100, []]]\n"),
            "{original}"
        );
        assert!(original.contains("CAPTURES = [10, 20]\n"), "{original}");
        assert!(original.contains("srand(20261009)\n"), "{original}");
        assert!(!original.contains("__"), "{original}");

        let vkxp = generate(Side::Vkxp, &Schedule::default(), &[3]);
        assert!(vkxp.contains("SIDE = :vkxp\n"), "{vkxp}");
        assert!(vkxp.contains("SCHEDULE = []\n"), "{vkxp}");
    }

    #[test]
    fn frames_are_sorted_and_unique() {
        assert_eq!(parse_frames("20, 10,20").unwrap(), [10, 20]);
        assert!(parse_frames("").is_err());
        assert!(parse_frames("1,two").is_err());
    }
}
