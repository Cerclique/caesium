//! Formatting of log records into formatted lines.
//!
//! [`format_line`] is the pure core: given the elapsed time and a
//! [`log::Record`], it returns a [`Display`] value that renders the exact
//! formatted line `lithium` writes for that record, without allocating.
//! [`LineFormatter`] is a thin adapter that binds the origin instant and the
//! color flag, and hands each formatted line to the `env_logger` formatter
//! buffer.
//!
//! A formatted line has a fixed shape:
//!
//! ```text
//! [00:02:05.120 | ERROR | my_crate | src/lib.rs:42 ] something went wrong
//! ```
//!
//! where the elapsed time is zero-padded `HH:MM:SS.sss` (hours keep growing
//! past 99) and the level is right-aligned in a 5-column field. Records
//! without source location render as `unknown:0`.

use std::fmt::{self, Display};
use std::io::Write;
use std::time::{Duration, Instant};

use log::Record;

/// Binds the origin instant and the color flag for a running logger, and
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
    /// buffer, styled with the level color when color is enabled.
    pub(crate) fn format(
        &self,
        buf: &mut env_logger::fmt::Formatter,
        record: &Record<'_>,
    ) -> std::io::Result<()> {
        let line = format_line(self.start.elapsed(), record);

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
/// The elapsed time is the time since [`LineFormatter::new`] ran, as
/// zero-padded `HH:MM:SS.sss` truncated to the millisecond. Hours are at least
/// two digits and keep growing past 99. The level is right-aligned in a
/// 5-column field. A record without source location renders as `unknown:0`.
pub(crate) fn format_line<'a>(elapsed: Duration, record: &'a Record<'a>) -> FormattedLine<'a> {
    FormattedLine { elapsed, record }
}

/// A formatted line, written directly into the destination when displayed.
/// Built by [`format_line`].
pub(crate) struct FormattedLine<'a> {
    elapsed: Duration,
    record: &'a Record<'a>,
}

impl Display for FormattedLine<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let record = self.record;
        let secs = self.elapsed.as_secs();
        write!(
            f,
            "[{:02}:{:02}:{:02}.{:03} | {:>5} | {} | {}:{} ] {}",
            secs / 3600,
            secs / 60 % 60,
            secs % 60,
            self.elapsed.subsec_millis(),
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
            format_line(Duration::from_millis(120), &record).to_string(),
            "[00:00:00.120 | ERROR | my_crate | src/lib.rs:42 ] something went wrong"
        );
    }

    #[test]
    fn test_format_line_splits_elapsed_time() {
        let record = Record::builder()
            .level(Level::Warn)
            .target("my_crate")
            .file(Some("src/lib.rs"))
            .line(Some(7))
            .args(format_args!("still going"))
            .build();

        // 3h 25m 45s 678ms
        assert_eq!(
            format_line(Duration::from_millis(12_345_678), &record).to_string(),
            "[03:25:45.678 |  WARN | my_crate | src/lib.rs:7 ] still going"
        );
    }

    #[test]
    fn test_format_line_truncates_to_the_millisecond() {
        let record = Record::builder()
            .level(Level::Info)
            .target("my_crate")
            .args(format_args!("almost"))
            .build();

        assert_eq!(
            format_line(Duration::from_micros(1_999), &record).to_string(),
            "[00:00:00.001 |  INFO | my_crate | unknown:0 ] almost"
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
            format_line(Duration::ZERO, &record).to_string(),
            "[00:00:00.000 |  INFO | my_crate | unknown:0 ] no location"
        );
    }

    #[test]
    fn test_format_line_hours_past_99() {
        // Hours are a minimum of two digits: past 99 they keep growing
        // instead of wrapping.
        let record = Record::builder()
            .level(Level::Debug)
            .target("my_crate")
            .args(format_args!("late"))
            .build();

        assert_eq!(
            format_line(Duration::from_secs(100 * 3600), &record).to_string(),
            "[100:00:00.000 | DEBUG | my_crate | unknown:0 ] late"
        );
    }
}
