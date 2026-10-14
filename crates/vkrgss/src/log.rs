//! The diagnostics of the host: the reporter that turns what a session reports into `tracing`
//! events, and the adapter that turns lines written to a stream into events. The subscriber
//! that writes events to the error stream, and the level it shows, are those of every
//! executable of a run: the module `diagnostics` of `vkrgss-guest`.
//!
//! What the game itself produces (`p`, `print`, the report of a script error) and the
//! program's usage and fatal error message are not events: they are written to the error
//! stream directly and shown at every level. So is what a session reports because the game or
//! an option asked for it.

use std::io::Write;

use vkrgss_guest::diagnostics::PREFIX;
use vkrgss_host::{Reporter, Severity};

/// The reporter of a session or a headless run: an error or a warning is an event of that
/// level with the host's target, one for each line of its text, and text that is shown at
/// every level is written to the error stream directly.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HostReporter;

impl Reporter for HostReporter {
    fn report(&mut self, severity: Severity, text: &str) {
        match severity {
            Severity::Error => {
                for line in text.split('\n') {
                    tracing::error!(target: "vkrgss_host", "{line}");
                }
            }
            Severity::Warning => {
                for line in text.split('\n') {
                    tracing::warn!(target: "vkrgss_host", "{line}");
                }
            }
            Severity::Info => {
                for line in text.split('\n') {
                    tracing::info!(target: "vkrgss_host", "{line}");
                }
            }
            Severity::Always => eprint!("{}", always_lines(text)),
        }
    }
}

/// What is written for text that is shown at every level: each of its lines, an empty one
/// too, after the prefix. A line that has the prefix already does not get a second one.
fn always_lines(text: &str) -> String {
    let mut lines = String::new();
    for (index, line) in text.split('\n').enumerate() {
        let line = match index {
            0 => line,
            _ => line.strip_prefix(PREFIX).unwrap_or(line),
        };
        lines.push_str(PREFIX);
        lines.push_str(line);
        lines.push('\n');
    }
    lines
}

/// A stream that emits each completed line written to it as a warning, in place of the error
/// stream that the mixer writes its reports to. The prefix `vkrgss: ` of a line is left out,
/// because the subscriber writes it.
#[derive(Debug)]
pub(crate) struct EventLines {
    /// What was written since the last end of a line.
    pending: Vec<u8>,
}

impl EventLines {
    /// The stream for the reports of the mixer.
    pub(crate) fn audio() -> Box<EventLines> {
        Box::new(EventLines {
            pending: Vec::new(),
        })
    }

    fn emit(&self, line: &[u8]) {
        let line = String::from_utf8_lossy(line);
        let text = line.strip_prefix(PREFIX).unwrap_or(&line);
        tracing::warn!(target: "vkrgss_audio", "{text}");
    }
}

impl Write for EventLines {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.pending.extend_from_slice(bytes);
        while let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=end).collect();
            self.emit(&line[..end]);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing_subscriber::fmt::MakeWriter;
    use tracing_subscriber::layer::SubscriberExt;
    use vkrgss_guest::diagnostics::{Level, Line, level};

    use super::*;

    /// The lines a subscriber of the program's format wrote.
    #[derive(Debug, Clone, Default)]
    struct Written(Arc<Mutex<Vec<u8>>>);

    impl Write for Written {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Written {
        type Writer = Written;

        fn make_writer(&'a self) -> Written {
            self.clone()
        }
    }

    /// What the events of `emit` look like on the error stream.
    fn written(detailed: bool, emit: impl FnOnce()) -> String {
        let written = Written::default();
        let lines = tracing_subscriber::fmt::layer()
            .with_writer(written.clone())
            .event_format(Line { detailed });
        let subscriber = tracing_subscriber::registry().with(lines);
        tracing::subscriber::with_default(subscriber, emit);
        let bytes = written.0.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn written_lines_become_events_without_a_second_prefix() {
        let text = written(false, || {
            let mut lines = EventLines::audio();
            write!(lines, "vkrgss: frame 3 ").unwrap();
            writeln!(lines, "was rejected").unwrap();
            writeln!(lines, "vkrgss: first\nsecond").unwrap();
            write!(lines, "not completed").unwrap();
        });
        assert_eq!(
            text,
            "vkrgss: frame 3 was rejected\nvkrgss: first\nvkrgss: second\n"
        );
    }

    #[test]
    fn the_lines_of_a_side_have_its_target() {
        let text = written(true, || {
            writeln!(EventLines::audio(), "vkrgss: audio: silent").unwrap();
        });
        assert_eq!(text, "vkrgss: WARN vkrgss_audio: audio: silent\n");
    }

    #[test]
    fn a_report_is_an_event_of_its_severity_with_the_target_of_the_host() {
        let text = written(true, || {
            HostReporter.report(Severity::Error, "frame 3 was rejected: why");
            HostReporter.report(Severity::Warning, "the mask does not exist");
        });
        assert_eq!(
            text,
            "vkrgss: ERROR vkrgss_host: frame 3 was rejected: why\n\
             vkrgss: WARN vkrgss_host: the mask does not exist\n"
        );
    }

    #[test]
    fn a_report_of_several_lines_is_an_event_for_each() {
        let text = written(false, || {
            HostReporter.report(Severity::Error, "first\nsecond");
        });
        assert_eq!(text, "vkrgss: first\nvkrgss: second\n");
    }

    #[test]
    fn text_shown_at_every_level_has_the_prefix_on_each_line() {
        assert_eq!(always_lines("one"), "vkrgss: one\n");
        assert_eq!(
            always_lines("first\n\nthird\nvkrgss: fourth"),
            "vkrgss: first\nvkrgss: \nvkrgss: third\nvkrgss: fourth\n"
        );
    }

    /// Scenario "Rejected frame at error level" of `specs/host-session/spec.md`, for what the
    /// program does with the report; that the session reports a rejected frame as an error
    /// is tested in `vkrgss-host`.
    #[test]
    fn at_the_level_of_errors_a_rejected_frame_is_shown_and_a_degradation_is_not() {
        let written = Written::default();
        let Level { filter, detailed } = level(Some("error"), false).unwrap();
        let lines = tracing_subscriber::fmt::layer()
            .with_writer(written.clone())
            .event_format(Line { detailed });
        let subscriber = tracing_subscriber::registry().with(filter).with(lines);
        tracing::subscriber::with_default(subscriber, || {
            HostReporter.report(Severity::Error, "frame 3 was rejected: why");
            HostReporter.report(Severity::Warning, "the mask does not exist");
        });
        let bytes = written.0.lock().unwrap().clone();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "vkrgss: ERROR vkrgss_host: frame 3 was rejected: why\n"
        );
    }
}
