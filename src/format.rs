//! Formatting of log records into formatted lines.
//!
//! [`format_line`] is the pure core: given the elapsed milliseconds and a
//! [`log::Record`], it returns a [`Display`] value that renders the exact
//! formatted line `lithium` writes for that record, without allocating. [`LineFormatter`] is a thin adapter that binds the origin
//! instant and the colour flag, and hands each formatted line to the
//! `env_logger` formatter buffer.
//!
//! A formatted line has a fixed shape:
//!
//! ```text
//! [      120 | ERROR | my_crate | src/lib.rs:42 ] something went wrong
//! ```
//!
//! where the elapsed time is right-aligned in a 9-column minimum-width field
//! and the level is right-aligned in a 5-column field. Records without source
//! location render as `unknown:0`.

use std::fmt::{self, Display};
use std::io::Write;
use std::time::Instant;

use log::Record;

/// Binds the origin instant and the colour flag for a running logger, and
/// writes formatted lines into the `env_logger` formatter buffer.
///
/// The origin instant is captured in [`new`], which [`LoggerBuilder::init`] calls,
/// so every formatted line measures time from the moment `init` runs.
///
/// [`LoggerBuilder::init`]: crate::LoggerBuilder::init
pub(crate) struct LineFormatter {
    start: Instant,
    color: bool,
}

impl LineFormatter {
    pub(crate) fn new(color: bool) -> Self {
        Self {
            start: Instant::now(),
            color,
        }
    }

    /// Writes `record` as a formatted line to the `env_logger` formatter
    /// buffer, styled with the level colour when colour is enabled.
    pub(crate) fn format(
        &self,
        buf: &mut env_logger::fmt::Formatter,
        record: &Record<'_>,
    ) -> std::io::Result<()> {
        let line = format_line(self.start.elapsed().as_millis(), record);

        if self.color {
            let style = buf.default_level_style(record.level());
            writeln!(buf, "{style}{line}{style:#}")
        } else {
            writeln!(buf, "{line}")
        }
    }
}

/// Renders one log record as a formatted line.
///
/// The elapsed time is the number of milliseconds since
/// [`LineFormatter::new`] ran, right-aligned in a 9-column minimum-width
/// field. The level is right-aligned in a 5-column field. A record without
/// source location renders as `unknown:0`.
pub(crate) fn format_line<'a>(elapsed_ms: u128, record: &'a Record<'a>) -> FormattedLine<'a> {
    FormattedLine { elapsed_ms, record }
}

/// A formatted line, written directly into the destination when displayed.
/// Built by [`format_line`].
pub(crate) struct FormattedLine<'a> {
    elapsed_ms: u128,
    record: &'a Record<'a>,
}

impl Display for FormattedLine<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let record = self.record;
        write!(
            f,
            "[{:>9} | {:>5} | {} | {}:{} ] {}",
            self.elapsed_ms,
            record.level(),
            record.target(),
            record.file().unwrap_or("unknown"),
            record.line().unwrap_or(0),
            record.args(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::Level;

    #[test]
    fn test_format_line_full_location() {
        let record = Record::builder()
            .level(Level::Error)
            .target("my_crate")
            .file(Some("src/lib.rs"))
            .line(Some(42))
            .args(format_args!("something went wrong"))
            .build();

        assert_eq!(
            format_line(120, &record).to_string(),
            "[      120 | ERROR | my_crate | src/lib.rs:42 ] something went wrong"
        );
    }

    #[test]
    fn test_format_line_pads_elapsed_time() {
        let record = Record::builder()
            .level(Level::Warn)
            .target("my_crate")
            .file(Some("src/lib.rs"))
            .line(Some(7))
            .args(format_args!("still going"))
            .build();

        assert_eq!(
            format_line(987654, &record).to_string(),
            "[   987654 |  WARN | my_crate | src/lib.rs:7 ] still going"
        );
    }

    #[test]
    fn test_format_line_without_location() {
        let record = Record::builder()
            .level(Level::Info)
            .target("my_crate")
            .args(format_args!("no location"))
            .build();

        assert_eq!(
            format_line(0, &record).to_string(),
            "[        0 |  INFO | my_crate | unknown:0 ] no location"
        );
    }

    #[test]
    fn test_format_line_elapsed_wider_than_field() {
        // The 9-column field is a minimum width: a longer value is not
        // truncated, it simply stops the padding.
        let record = Record::builder()
            .level(Level::Debug)
            .target("my_crate")
            .args(format_args!("late"))
            .build();

        assert_eq!(
            format_line(1_234_567_890, &record).to_string(),
            "[1234567890 | DEBUG | my_crate | unknown:0 ] late"
        );
    }
}
