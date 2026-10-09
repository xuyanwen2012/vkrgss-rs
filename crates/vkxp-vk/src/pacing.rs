//! Pacing of presents: the schedule of target times and the statistics of what was achieved.
//!
//! Both are pure: times are integer nanoseconds of whatever clock the caller reads, and
//! nothing here touches Vulkan.

use std::collections::VecDeque;

const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// How many periods a target may lie behind the clock before the schedule starts over.
const MAX_PERIODS_BEHIND: u128 = 4;

/// The target times of consecutive presents: the first from the clock, each following one a
/// frame period after the one before.
///
/// The period is one second over the frame rate and is kept as that fraction: a target is
/// the time the current run of frames started at plus the whole nanoseconds of
/// `frames / rate` seconds, so nothing accumulates however long the run is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PresentSchedule {
    /// The target the current run counts from, and the rate it counts at. `None` before the
    /// first target and after a restart.
    run: Option<Run>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Run {
    base: u64,
    /// Frames per second, not zero.
    rate: u32,
    /// Periods from `base` to the newest target.
    frames: u64,
}

impl Run {
    /// The target `frames` periods after the base.
    fn target(&self, frames: u64) -> u64 {
        let offset = u128::from(frames) * NANOS_PER_SECOND / u128::from(self.rate);
        self.base
            .saturating_add(u64::try_from(offset).unwrap_or(u64::MAX))
    }
}

impl PresentSchedule {
    pub fn new() -> PresentSchedule {
        PresentSchedule::default()
    }

    /// Makes the next target start from the clock again.
    pub fn restart(&mut self) {
        self.run = None;
    }

    /// The target of the next present, at `rate` frames per second, when the clock reads
    /// `now`: the previous target plus one period, or `now` for the first present, after a
    /// [`restart`](PresentSchedule::restart) and when the clock is more than four periods
    /// past the target. A rate of 0 counts as 1.
    pub fn next(&mut self, now: u64, rate: u32) -> u64 {
        let rate = rate.max(1);
        let mut run = match self.run {
            Some(run) if run.rate == rate => run,
            // Another period from here on: it is counted from the previous target.
            Some(run) => Run {
                base: run.target(run.frames),
                rate,
                frames: 0,
            },
            None => return self.start(now, rate),
        };
        run.frames += 1;
        let target = run.target(run.frames);
        let allowed = MAX_PERIODS_BEHIND * NANOS_PER_SECOND / u128::from(rate);
        if u128::from(now.saturating_sub(target)) > allowed {
            return self.start(now, rate);
        }
        self.run = Some(run);
        target
    }

    /// The newest target, or `None` before the first one and after a restart.
    pub fn current(&self) -> Option<u64> {
        self.run.map(|run| run.target(run.frames))
    }

    fn start(&mut self, now: u64, rate: u32) -> u64 {
        self.run = Some(Run {
            base: now,
            rate,
            frames: 0,
        });
        now
    }
}

/// How many intervals the statistics keep: the newest ones.
pub const STATS_INTERVALS: usize = 600;

/// The intervals between the times at which consecutive frames became visible, the newest
/// [`STATS_INTERVALS`] of them.
#[derive(Debug, Clone, Default)]
pub struct PresentStats {
    /// The time the newest frame became visible.
    last: Option<u64>,
    intervals: VecDeque<u64>,
}

impl PresentStats {
    pub fn new() -> PresentStats {
        PresentStats::default()
    }

    /// A frame became visible at `time`, in nanoseconds of the clock every call uses.
    pub fn record(&mut self, time: u64) {
        if let Some(last) = self.last {
            if self.intervals.len() == STATS_INTERVALS {
                self.intervals.pop_front();
            }
            self.intervals.push_back(time.saturating_sub(last));
        }
        self.last = Some(time);
    }

    /// The next time recorded is not measured against the one before it, as when the clock
    /// the times come from changes.
    pub fn interrupt(&mut self) {
        self.last = None;
    }

    /// The summary of the intervals kept, or `None` while there is none.
    pub fn summary(&self) -> Option<IntervalSummary> {
        let smallest = *self.intervals.iter().min()?;
        let largest = *self.intervals.iter().max()?;
        let total: u128 = self
            .intervals
            .iter()
            .map(|interval| u128::from(*interval))
            .sum();
        Some(IntervalSummary {
            count: self.intervals.len(),
            mean: total as f64 / self.intervals.len() as f64,
            smallest,
            largest,
        })
    }
}

/// A summary of intervals between visible frames, in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntervalSummary {
    pub count: usize,
    pub mean: f64,
    pub smallest: u64,
    pub largest: u64,
}

impl std::fmt::Display for IntervalSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ms = |nanos: f64| nanos / 1e6;
        write!(
            f,
            "{} intervals, mean {:.2} ms, smallest {:.2} ms, largest {:.2} ms",
            self.count,
            ms(self.mean),
            ms(self.smallest as f64),
            ms(self.largest as f64)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    /// Scenario "Schedule".
    #[test]
    fn targets_follow_the_first_time_one_period_apart() {
        let first = 123_456_789;
        let mut schedule = PresentSchedule::new();
        // The clock keeps up, a little late each time.
        let targets: Vec<u64> = (0..5)
            .map(|frame| schedule.next(first + frame * 25 * MS + 3 * MS, 40))
            .collect();
        let first = first + 3 * MS;
        assert_eq!(
            targets,
            [0, 25, 50, 75, 100].map(|offset| first + offset * MS)
        );
    }

    /// Scenario "Schedule": no rounding error accumulates, at a rate whose period is a whole
    /// number of nanoseconds and at one whose period is not.
    #[test]
    fn targets_do_not_drift_over_many_frames() {
        for (rate, frames) in [(40_u32, 100_000_u64), (60, 100_000), (7, 100_000)] {
            let mut schedule = PresentSchedule::new();
            let first = schedule.next(1000, rate);
            let mut last = first;
            for _ in 0..frames {
                // The clock reads the target itself: never behind.
                last = schedule.next(last, rate);
            }
            let expected = u128::from(frames) * NANOS_PER_SECOND / u128::from(rate);
            assert_eq!(u128::from(last - first), expected, "{rate} fps");
        }
        let mut schedule = PresentSchedule::new();
        schedule.next(0, 40);
        let mut last = 0;
        for _ in 0..100_000 {
            last = schedule.next(last, 40);
        }
        assert_eq!(last, 100_000 * 25 * MS);
    }

    /// Scenario "Falling behind".
    #[test]
    fn schedule_restarts_at_the_clock_when_more_than_four_periods_behind() {
        let mut schedule = PresentSchedule::new();
        assert_eq!(schedule.next(0, 40), 0);
        // The next target is 25 ms. Exactly four periods past it is still kept.
        assert_eq!(schedule.next(125 * MS, 40), 25 * MS);
        // The next is 50 ms; the clock is one nanosecond more than four periods past it.
        let now = 150 * MS + 1;
        assert_eq!(schedule.next(now, 40), now);
        assert_eq!(schedule.next(now, 40), now + 25 * MS, "counted from there");
    }

    /// Scenario "Period change".
    #[test]
    fn new_rate_counts_from_the_previous_target() {
        let mut schedule = PresentSchedule::new();
        assert_eq!(schedule.next(0, 40), 0);
        assert_eq!(schedule.next(25 * MS, 40), 25 * MS);
        assert_eq!(schedule.next(50 * MS, 40), 50 * MS);
        assert_eq!(schedule.next(75 * MS, 20), 100 * MS);
        assert_eq!(schedule.next(125 * MS, 20), 150 * MS);
        assert_eq!(schedule.next(175 * MS, 40), 175 * MS);
    }

    #[test]
    fn restart_takes_the_next_target_from_the_clock() {
        let mut schedule = PresentSchedule::new();
        schedule.next(0, 40);
        schedule.next(0, 40);
        schedule.restart();
        assert_eq!(schedule.current(), None);
        assert_eq!(schedule.next(7 * MS, 40), 7 * MS);
        assert_eq!(schedule.current(), Some(7 * MS));
        assert_eq!(schedule.next(7 * MS, 40), 32 * MS);
        // A target ahead of the clock is kept however far ahead it is.
        assert_eq!(schedule.next(0, 40), 57 * MS);
        assert_eq!(schedule.next(u64::MAX, 0), u64::MAX, "no division by zero");
    }

    /// Scenario "Summary".
    #[test]
    fn summary_counts_the_intervals_with_mean_smallest_and_largest() {
        let mut stats = PresentStats::new();
        assert_eq!(stats.summary(), None);
        stats.record(0);
        assert_eq!(stats.summary(), None, "one frame is no interval");
        for time in [25, 50, 80] {
            stats.record(time * MS);
        }
        let summary = stats.summary().unwrap();
        assert_eq!(summary.count, 3);
        assert_eq!(summary.smallest, 25 * MS);
        assert_eq!(summary.largest, 30 * MS);
        assert!((summary.mean - 80.0 * MS as f64 / 3.0).abs() < 1e-3);
        assert_eq!(
            summary.to_string(),
            "3 intervals, mean 26.67 ms, smallest 25.00 ms, largest 30.00 ms"
        );
    }

    #[test]
    fn statistics_keep_the_newest_intervals_and_skip_an_interruption() {
        let mut stats = PresentStats::new();
        stats.record(0);
        stats.record(1000 * MS);
        for frame in 0..STATS_INTERVALS as u64 {
            stats.record(1000 * MS + (frame + 1) * 25 * MS);
        }
        let summary = stats.summary().unwrap();
        assert_eq!(
            (summary.count, summary.smallest, summary.largest),
            (STATS_INTERVALS, 25 * MS, 25 * MS),
            "the long first interval has left"
        );
        stats.interrupt();
        stats.record(5);
        assert_eq!(stats.summary().unwrap().count, STATS_INTERVALS);
        stats.record(40 * MS + 5);
        assert_eq!(stats.summary().unwrap().largest, 40 * MS);
    }
}
