//! The reports of a session: what it says about a run, and how serious each one is.

/// How serious a report is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A failure that loses something the game asked for: a frame, a freeze.
    Error,
    /// A degradation the run continues with.
    Warning,
    /// A notice of what became of something the player asked for.
    Info,
    /// Text that the game or an option asked for, which is shown at every level.
    Always,
}

/// Where a session and a headless run write their reports.
pub trait Reporter: Send {
    /// Takes one report. `text` has neither the prefix `vkrgss: ` nor an end of line.
    fn report(&mut self, severity: Severity, text: &str);
}

/// The reporter of a session that was given none: every report is a line `vkrgss: <text>` on
/// the error stream.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ErrorStream;

impl Reporter for ErrorStream {
    fn report(&mut self, _severity: Severity, text: &str) {
        eprintln!("vkrgss: {text}");
    }
}
