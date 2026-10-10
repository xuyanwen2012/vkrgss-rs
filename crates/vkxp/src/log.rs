//! The diagnostics of the program: the subscriber that writes `tracing` events to the error
//! stream, the level it shows, and the adapter that turns lines written to a stream into
//! events.
//!
//! What the game itself produces (`p`, `print`, the report of a script error) and the
//! program's usage and fatal error message are not events: they are written to the error
//! stream directly and shown at every level.

use std::fmt;

use tracing::level_filters::LevelFilter;
use tracing::{Event, Subscriber};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

/// The environment variable that sets the level or a filter.
pub(crate) const VARIABLE: &str = "VKXP_LOG";

/// What every line of a diagnostic starts with.
const PREFIX: &str = "vkxp: ";

/// What is shown, and whether a line names the level and the target of its event.
#[derive(Debug)]
pub(crate) struct Level {
    filter: EnvFilter,
    detailed: bool,
}

/// The level of a run: the value of [`VARIABLE`] if it is set and not empty, a level name or
/// filter directives such as `vkxp_host=trace`; otherwise debug with `--verbose` and warnings
/// without. The error is a message for the user that names the variable.
pub(crate) fn level(variable: Option<&str>, verbose: bool) -> Result<Level, String> {
    match variable.filter(|value| !value.is_empty()) {
        Some(value) => match EnvFilter::try_new(value) {
            Ok(filter) => Ok(Level {
                filter,
                detailed: true,
            }),
            Err(error) => Err(format!(
                "{VARIABLE}: `{value}` is neither a level nor a filter: {error}"
            )),
        },
        None => {
            let level = if verbose {
                LevelFilter::DEBUG
            } else {
                LevelFilter::WARN
            };
            Ok(Level {
                filter: EnvFilter::default().add_directive(level.into()),
                detailed: verbose,
            })
        }
    }
}

/// Writes an event as the line `vkxp: <message>`, with the level and the target in front of
/// the message when it is detailed.
struct Line {
    detailed: bool,
}

impl<S, N> FormatEvent<S, N> for Line
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        context: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        writer.write_str(PREFIX)?;
        if self.detailed {
            let metadata = event.metadata();
            write!(writer, "{} {}: ", metadata.level(), metadata.target())?;
        }
        context
            .field_format()
            .format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

/// Installs the subscriber of the program, which writes the events `level` shows to the
/// error stream. Called once, before anything emits an event.
pub(crate) fn install(level: Level) {
    let Level { filter, detailed } = level;
    let lines = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .event_format(Line { detailed });
    tracing_subscriber::registry()
        .with(filter)
        .with(lines)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The most verbose level that `level` shows of any target.
    fn most(level: &Level) -> Option<LevelFilter> {
        level.filter.max_level_hint()
    }

    #[test]
    fn warnings_and_errors_are_shown_by_default() {
        let level = level(None, false).unwrap();
        assert_eq!(most(&level), Some(LevelFilter::WARN));
        assert!(!level.detailed);
        // An empty variable is one that is not set.
        let level = super::level(Some(""), false).unwrap();
        assert_eq!(most(&level), Some(LevelFilter::WARN));
        assert!(!level.detailed);
    }

    #[test]
    fn verbose_adds_informational_and_debug_events() {
        let level = level(None, true).unwrap();
        assert_eq!(most(&level), Some(LevelFilter::DEBUG));
        assert!(level.detailed);
    }

    #[test]
    fn the_variable_comes_before_verbose() {
        let level = level(Some("error"), true).unwrap();
        assert_eq!(most(&level), Some(LevelFilter::ERROR));
    }

    #[test]
    fn the_variable_takes_a_filter_for_a_module() {
        let level = level(Some("warn,vkxp_host=trace"), false).unwrap();
        assert_eq!(most(&level), Some(LevelFilter::TRACE));
    }

    #[test]
    fn a_malformed_value_is_an_error_naming_the_variable() {
        let message = level(Some("="), true).unwrap_err();
        assert!(message.contains(VARIABLE), "{message}");
        assert!(message.contains("`=`"), "{message}");
    }
}
