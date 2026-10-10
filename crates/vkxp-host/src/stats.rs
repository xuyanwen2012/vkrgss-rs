//! The frame time statistics: where frames spend their time, summed up per stage.

use std::time::Duration;

use vkxp_vk::{FrameTiming, IntervalSummary};

/// The name of the row for the time the host waits for the guest between frames, which is
/// what the game's scripts and the extraction of the frame take.
pub(crate) const GUEST: &str = "guest";

/// The durations of one stage.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Stage {
    count: usize,
    total: u128,
    smallest: u64,
    largest: u64,
}

impl Stage {
    fn add(&mut self, duration: Duration) {
        let nanos = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
        self.smallest = match self.count {
            0 => nanos,
            _ => self.smallest.min(nanos),
        };
        self.largest = self.largest.max(nanos);
        self.total += u128::from(nanos);
        self.count += 1;
    }

    /// The summary of the durations, or `None` while there is none.
    fn summary(&self) -> Option<IntervalSummary> {
        (self.count > 0).then(|| IntervalSummary {
            count: self.count,
            mean: self.total as f64 / self.count as f64,
            smallest: self.smallest,
            largest: self.largest,
        })
    }
}

/// The durations of the stages of the frames measured so far: the wait for the guest and
/// every stage the backend measures.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FrameStats {
    /// Frames rendered.
    frames: usize,
    guest: Stage,
    /// In the order of [`FrameTiming::stages`].
    stages: [Stage; 7],
}

impl FrameStats {
    /// The number of frames rendered since the statistics started.
    pub(crate) fn frames(&self) -> usize {
        self.frames
    }

    /// The host waited this long for a frame of the guest.
    pub(crate) fn add_guest(&mut self, wait: Duration) {
        self.guest.add(wait);
    }

    /// A frame was rendered, with what the backend measured of it, if it measures.
    pub(crate) fn add_frame(&mut self, timing: Option<FrameTiming>) {
        self.frames += 1;
        let durations = timing.unwrap_or_default().stages();
        for (stage, (_, duration)) in self.stages.iter_mut().zip(durations) {
            if let Some(duration) = duration {
                stage.add(duration);
            }
        }
    }

    /// The summary: a heading and one row per stage, the guest first, each with the number
    /// of frames it was measured for and the mean, smallest and largest duration in
    /// milliseconds.
    pub(crate) fn lines(&self) -> Vec<String> {
        let names = FrameTiming::default().stages().map(|(name, _)| name);
        let rows = std::iter::once((GUEST, &self.guest)).chain(names.into_iter().zip(&self.stages));
        let ms = |nanos: f64| nanos / 1e6;
        std::iter::once(format!("frame times over {} frame(s):", self.frames))
            .chain(rows.map(|(name, stage)| match stage.summary() {
                Some(summary) => format!(
                    "  {name}: {} frame(s), mean {:.3} ms, smallest {:.3} ms, largest {:.3} ms",
                    summary.count,
                    ms(summary.mean),
                    ms(summary.smallest as f64),
                    ms(summary.largest as f64)
                ),
                None => format!("  {name}: 0 frame(s)"),
            }))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn summary_has_a_row_for_the_guest_and_one_for_every_stage() {
        let mut stats = FrameStats::default();
        stats.add_frame(Some(FrameTiming {
            encode: Some(MS),
            bin: Some(2 * MS),
            upload: Some(3 * MS),
            submit: Some(4 * MS),
            ..FrameTiming::default()
        }));
        stats.add_guest(10 * MS);
        stats.add_frame(Some(FrameTiming {
            encode: Some(3 * MS),
            bin: Some(2 * MS),
            upload: Some(3 * MS),
            submit: Some(5 * MS),
            compose_device: Some(MS / 2),
            ..FrameTiming::default()
        }));
        assert_eq!(stats.frames(), 2);
        assert_eq!(
            stats.lines(),
            [
                "frame times over 2 frame(s):",
                "  guest: 1 frame(s), mean 10.000 ms, smallest 10.000 ms, largest 10.000 ms",
                "  encode: 2 frame(s), mean 2.000 ms, smallest 1.000 ms, largest 3.000 ms",
                "  bin: 2 frame(s), mean 2.000 ms, smallest 2.000 ms, largest 2.000 ms",
                "  upload: 2 frame(s), mean 3.000 ms, smallest 3.000 ms, largest 3.000 ms",
                "  submit: 2 frame(s), mean 4.500 ms, smallest 4.000 ms, largest 5.000 ms",
                "  present: 0 frame(s)",
                "  compose (device): 1 frame(s), mean 0.500 ms, smallest 0.500 ms, largest \
                 0.500 ms",
                "  present (device): 0 frame(s)",
            ]
        );
    }

    #[test]
    fn frame_of_a_display_that_measures_nothing_is_counted() {
        let mut stats = FrameStats::default();
        stats.add_frame(None);
        assert_eq!(stats.frames(), 1);
        assert_eq!(stats.lines()[0], "frame times over 1 frame(s):");
        assert_eq!(stats.lines()[2], "  encode: 0 frame(s)");
    }
}
