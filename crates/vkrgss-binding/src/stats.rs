//! The guest's frame time statistics: where the guest spends the time of its frames.
//!
//! A frame of the guest lasts from the return of one tick wait to the start of the next.
//! When [`RunConfig::frame_stats`](crate::RunConfig::frame_stats) asks for it, the time
//! inside the bitmap-changing methods, inside the text methods and inside the extraction of
//! the frame is measured, and the rest of the frame is what the scripts took. The summary is
//! written to the error stream, by the guest itself, every [`INTERVAL`] frames and when the
//! guest ends.
//!
//! The clock is read for this report only: nothing a script can observe depends on it.
//! Without the request there is no state here and no clock is read.
//!
//! The state is one of the process, as the guest is: a measured call on a script thread of
//! Ruby 1.9.2 counts as one on the main script's thread.

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

/// The summary is written after every so many frames.
const INTERVAL: usize = 600;

/// The prefix of the program's lines on the error stream.
const PREFIX: &str = "vkrgss: ";

/// A part of a frame that is measured on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Part {
    /// The methods of `Bitmap` that change or read pixels.
    Bitmap,
    /// `Bitmap#draw_text` and `Bitmap#text_size`.
    Text,
    /// Extracting the frame from the scene and handing it to the link.
    Extraction,
}

/// The names of the rows: the rest of the frame first, then the parts in the order of
/// [`Part`].
const NAMES: [&str; 4] = [
    "guest scripts",
    "guest bitmap operations",
    "guest text",
    "guest extraction",
];

/// The durations of one row.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Row {
    count: usize,
    total: u128,
    smallest: u64,
    largest: u64,
}

impl Row {
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
}

/// The rows of the frames measured since the last summary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Rows([Row; 4]);

impl Rows {
    /// The number of frames measured.
    fn frames(&self) -> usize {
        self.0[0].count
    }

    /// A frame took `whole`, of which `parts` were spent in the parts, in the order of
    /// [`Part`]. Every row counts the frame, a part that took no time too.
    fn add_frame(&mut self, whole: Duration, parts: [Duration; 3]) {
        let measured: Duration = parts.iter().sum();
        self.0[0].add(whole.saturating_sub(measured));
        for (row, part) in self.0[1..].iter_mut().zip(parts) {
            row.add(part);
        }
    }

    /// The summary: a heading and one row per part, in the form of the host's rows.
    fn lines(&self) -> Vec<String> {
        let ms = |nanos: f64| nanos / 1e6;
        std::iter::once(format!("guest frame times over {} frame(s):", self.frames()))
            .chain(NAMES.iter().zip(&self.0).map(|(name, row)| match row.count {
                0 => format!("  {name}: 0 frame(s)"),
                count => format!(
                    "  {name}: {count} frame(s), mean {:.3} ms, smallest {:.3} ms, largest {:.3} ms",
                    ms(row.total as f64 / count as f64),
                    ms(row.smallest as f64),
                    ms(row.largest as f64)
                ),
            }))
            .collect()
    }
}

/// What a guest that measures keeps.
#[derive(Debug, Default)]
struct Measuring {
    rows: Rows,
    /// Whether a summary was written.
    reported: bool,
    /// When the frame being measured began; `None` until the first tick wait has returned.
    frame: Option<Instant>,
    /// The time spent in the parts in this frame, in the order of [`Part`].
    parts: [Duration; 3],
    /// How many measured calls are running, one inside the other.
    depth: usize,
    /// The outermost of them and when it began, or when the frame began if that was later.
    span: Option<(Part, Instant)>,
}

impl Measuring {
    /// See [`frame_ends`].
    fn frame_ends(&mut self) {
        // A measured call that is still running, because it called into a script that
        // updates the frame, gives this frame its time up to here.
        if let Some((part, began)) = self.span {
            self.parts[part as usize] += began.elapsed();
        }
        let parts = std::mem::take(&mut self.parts);
        if let Some(began) = self.frame.take() {
            self.rows.add_frame(began.elapsed(), parts);
            if self.rows.frames() >= INTERVAL {
                self.report();
            }
        }
    }

    /// See [`frame_begins`].
    fn frame_begins(&mut self) {
        let now = Instant::now();
        self.frame = Some(now);
        if let Some((_, began)) = &mut self.span {
            *began = now;
        }
    }

    /// A measured call of `part` begins; see [`Span`].
    fn enter(&mut self, part: Part) {
        self.depth += 1;
        if self.depth == 1 {
            self.span = Some((part, Instant::now()));
        }
    }

    /// A measured call ends.
    fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        if self.depth == 0
            && let Some((part, began)) = self.span.take()
        {
            self.parts[part as usize] += began.elapsed();
        }
    }

    /// Writes the summary to the error stream and starts the rows anew.
    fn report(&mut self) {
        self.reported = true;
        let mut text = String::new();
        for line in std::mem::take(&mut self.rows).lines() {
            text.push_str(PREFIX);
            text.push_str(&line);
            text.push('\n');
        }
        eprint!("{text}");
    }
}

/// The measuring of the guest, if it was asked for.
static MEASURING: Mutex<Option<Measuring>> = Mutex::new(None);

/// Runs `access` on the measuring, if there is one.
fn measuring(access: impl FnOnce(&mut Measuring)) {
    if let Some(measuring) = MEASURING
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
    {
        access(measuring);
    }
}

/// Starts the statistics of a guest that begins to play, or turns them off.
pub(crate) fn start(requested: bool) {
    *MEASURING.lock().unwrap_or_else(PoisonError::into_inner) = requested.then(Measuring::default);
}

/// The guest is about to wait for a tick: the frame that began when the last wait returned
/// is over. What came before the first wait is not a frame and is not counted.
pub(crate) fn frame_ends() {
    measuring(Measuring::frame_ends);
}

/// The tick wait has returned: a frame begins. A measured call that is still running goes on
/// from here, so the wait belongs to no part.
pub(crate) fn frame_begins() {
    measuring(Measuring::frame_begins);
}

/// The guest has ended: the summary of a run that had none yet, or of the frames since the
/// last one, is written, and the measuring stops.
pub(crate) fn finish() {
    measuring(|measuring| {
        if !measuring.reported || measuring.rows.frames() > 0 {
            measuring.report();
        }
    });
    start(false);
}

/// A measured call: from [`Span::enter`] until the value is dropped, the time belongs to a
/// part. A call inside another measured call belongs to the outer one, so no time is counted
/// twice.
#[derive(Debug)]
pub(crate) struct Span;

impl Span {
    pub(crate) fn enter(part: Part) -> Span {
        measuring(|measuring| measuring.enter(part));
        Span
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        measuring(Measuring::leave);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn scripts_are_the_frame_without_its_parts() {
        let mut rows = Rows::default();
        rows.add_frame(10 * MS, [2 * MS, 3 * MS, MS]);
        rows.add_frame(6 * MS, [Duration::ZERO, MS, MS]);
        assert_eq!(
            rows.lines(),
            [
                "guest frame times over 2 frame(s):",
                "  guest scripts: 2 frame(s), mean 4.000 ms, smallest 4.000 ms, largest 4.000 ms",
                "  guest bitmap operations: 2 frame(s), mean 1.000 ms, smallest 0.000 ms, \
                 largest 2.000 ms",
                "  guest text: 2 frame(s), mean 2.000 ms, smallest 1.000 ms, largest 3.000 ms",
                "  guest extraction: 2 frame(s), mean 1.000 ms, smallest 1.000 ms, largest \
                 1.000 ms",
            ]
        );
        assert_eq!(Rows::default().lines()[1], "  guest scripts: 0 frame(s)");
    }

    // The measuring is one of the process, which other tests reach through the bitmap
    // methods; these tests use a value of their own.

    #[test]
    fn nested_calls_are_counted_once_for_the_outer_part() {
        let mut measuring = Measuring::default();
        measuring.frame_begins();
        measuring.enter(Part::Bitmap);
        measuring.enter(Part::Text);
        std::thread::sleep(MS);
        measuring.leave();
        measuring.leave();
        assert!(measuring.parts[Part::Bitmap as usize] >= MS);
        assert_eq!(measuring.parts[Part::Text as usize], Duration::ZERO);
        assert_eq!(measuring.depth, 0);
        measuring.frame_ends();
        assert_eq!(measuring.rows.frames(), 1);
    }

    /// A measured call can run a script, through the conversion of an argument, that updates
    /// the frame: each frame gets the part of the call that lies in it, and the tick wait
    /// belongs to none.
    #[test]
    fn call_across_a_tick_wait_is_split_at_the_frames_and_leaves_the_wait_out() {
        const WAIT: Duration = Duration::from_millis(60);
        let mut measuring = Measuring::default();
        // Before the first frame: not counted.
        measuring.enter(Part::Bitmap);
        std::thread::sleep(2 * MS);
        measuring.frame_ends();
        measuring.frame_begins();
        for _ in 0..2 {
            std::thread::sleep(2 * MS);
            measuring.enter(Part::Text);
            measuring.leave();
            measuring.frame_ends();
            std::thread::sleep(WAIT);
            measuring.frame_begins();
        }
        std::thread::sleep(2 * MS);
        measuring.leave();
        measuring.frame_ends();
        let [scripts, bitmap, text, _] = measuring.rows.0;
        assert_eq!(measuring.rows.frames(), 3);
        assert!(bitmap.smallest >= 2_000_000, "{bitmap:?}");
        assert!(Duration::from_nanos(bitmap.largest) < WAIT, "{bitmap:?}");
        // The whole of each frame lay inside the call.
        assert!(Duration::from_nanos(scripts.largest) < MS, "{scripts:?}");
        assert_eq!(text.total, 0);
        assert_eq!(measuring.depth, 0);
        assert!(measuring.span.is_none());
    }

    #[test]
    fn nothing_is_kept_without_the_request() {
        start(false);
        frame_begins();
        drop(Span::enter(Part::Bitmap));
        frame_ends();
        assert!(
            MEASURING
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_none()
        );
    }
}
