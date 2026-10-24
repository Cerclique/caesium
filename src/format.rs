//! Formatting of log records into formatted lines.
//!
//! [`LineFormat`] is the pure core: it holds how every line is written, and
//! [`LineFormat::line`] turns the elapsed time and a [`log::Record`] into a
//! [`Display`] value that renders the exact formatted line `lithium` writes for
//! that record, without allocating. [`LineFormatter`] is a thin adapter that
//! binds a clock and a line format, and hands each formatted line to the
//! `env_logger` formatter buffer.
//!
//! A formatted line has a fixed shape:
//!
//! ```text
//! [00:02:05.120 | ERROR | my_crate | src/lib.rs:42 ] something went wrong
//! ```
//!
//! where the elapsed time is zero-padded `HH:MM:SS.sss` (hours keep growing
//! past 99) and the level is right-aligned in a 5-column field. The source
//! location segment (`| src/lib.rs:42`) is left out when it is turned off or
//! the record has no file, and shows only the file when the record has no
//! line.

use std::fmt::{self, Display};
use std::io::Write;
use std::time::Duration;

use log::Record;

/// How every line is written: whether it is colored and whether it shows
/// source location.
///
/// The default is uncolored with source location shown.
#[derive(Clone, Copy)]
pub(crate) struct LineFormat {
    pub(crate) color: bool,
    pub(crate) source_location: bool,
}

impl Default for LineFormat {
    fn default() -> Self {
        Self {
            color: false,
            source_location: true,
        }
    }
}

impl LineFormat {
    /// Renders one log record as a formatted line, without color.
    ///
    /// The elapsed time is shown as zero-padded `HH:MM:SS.sss` truncated to the
    /// millisecond. Hours are at least two digits and keep growing past 99. The
    /// level is right-aligned in a 5-column field. When source location is on
    /// and the record has a file, the line shows `file:line`, or just `file`
    /// when the record has no line; otherwise the segment is left out.
    pub(crate) fn line<'a>(self, elapsed: Duration, record: &'a Record<'a>) -> FormattedLine<'a> {
        FormattedLine {
            format: self,
            elapsed,
            record,
        }
    }
}

/// Binds a clock and the line format for a running logger, and writes
/// formatted lines into the `env_logger` formatter buffer.
///
/// The clock returns the elapsed time shown on each line. [`LoggerBuilder::init`]
/// passes one that measures from the moment `init` runs; tests pass a fixed
/// duration.
///
/// [`LoggerBuilder::init`]: crate::LoggerBuilder::init
pub(crate) struct LineFormatter<C> {
    format: LineFormat,
    elapsed: C,
}

impl<C: Fn() -> Duration> LineFormatter<C> {
    pub(crate) fn new(format: LineFormat, elapsed: C) -> Self {
        Self { format, elapsed }
    }

    /// Writes `record` as a formatted line to the `env_logger` formatter
    /// buffer, styled with the level color when color is enabled.
    pub(crate) fn format(
        &self,
        buf: &mut env_logger::fmt::Formatter,
        record: &Record<'_>,
    ) -> std::io::Result<()> {
        let line = self.format.line((self.elapsed)(), record);

        if self.format.color {
            let style = buf.default_level_style(record.level());
            writeln!(buf, "{style}{line}{style:#}")
        } else {
            writeln!(buf, "{line}")
        }
    }
}

/// A formatted line, written directly into the destination when displayed.
/// Built by [`LineFormat::line`].
pub(crate) struct FormattedLine<'a> {
    format: LineFormat,
    elapsed: Duration,
    record: &'a Record<'a>,
}

impl Display for FormattedLine<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let record = self.record;
        let secs = self.elapsed.as_secs();
        write!(
            f,
            "[{:02}:{:02}:{:02}.{:03} | {:>5} | {}",
            secs / 3600,
            secs / 60 % 60,
            secs % 60,
            self.elapsed.subsec_millis(),
            record.level(),
            record.target(),
        )?;
        if self.format.source_location {
            match (record.file(), record.line()) {
                (Some(file), Some(line)) => write!(f, " | {file}:{line}")?,
                (Some(file), None) => write!(f, " | {file}")?,
                (None, _) => {}
            }
        }
        write!(f, " ] {}", record.args())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::Level;

    #[test]
    fn test_line_full_location() {
        let record = Record::builder()
            .level(Level::Error)
            .target("my_crate")
            .file(Some("src/lib.rs"))
            .line(Some(42))
            .args(format_args!("something went wrong"))
            .build();

        assert_eq!(
            LineFormat::default()
                .line(Duration::from_millis(120), &record)
                .to_string(),
            "[00:00:00.120 | ERROR | my_crate | src/lib.rs:42 ] something went wrong"
        );
    }

    #[test]
    fn test_line_splits_elapsed_time() {
        let record = Record::builder()
            .level(Level::Warn)
            .target("my_crate")
            .file(Some("src/lib.rs"))
            .line(Some(7))
            .args(format_args!("still going"))
            .build();

        // 3h 25m 45s 678ms
        assert_eq!(
            LineFormat::default()
                .line(Duration::from_millis(12_345_678), &record)
                .to_string(),
            "[03:25:45.678 |  WARN | my_crate | src/lib.rs:7 ] still going"
        );
    }

    #[test]
    fn test_line_truncates_to_the_millisecond() {
        let record = Record::builder()
            .level(Level::Info)
            .target("my_crate")
            .args(format_args!("almost"))
            .build();

        assert_eq!(
            LineFormat::default()
                .line(Duration::from_micros(1_999), &record)
                .to_string(),
            "[00:00:00.001 |  INFO | my_crate ] almost"
        );
    }

    #[test]
    fn test_line_without_location() {
        let record = Record::builder()
            .level(Level::Info)
            .target("my_crate")
            .args(format_args!("no location"))
            .build();

        assert_eq!(
            LineFormat::default()
                .line(Duration::ZERO, &record)
                .to_string(),
            "[00:00:00.000 |  INFO | my_crate ] no location"
        );
    }

    #[test]
    fn test_line_file_without_line() {
        let record = Record::builder()
            .level(Level::Info)
            .target("my_crate")
            .file(Some("src/lib.rs"))
            .args(format_args!("no line"))
            .build();

        assert_eq!(
            LineFormat::default()
                .line(Duration::ZERO, &record)
                .to_string(),
            "[00:00:00.000 |  INFO | my_crate | src/lib.rs ] no line"
        );
    }

    #[test]
    fn test_line_source_location_off() {
        let record = Record::builder()
            .level(Level::Error)
            .target("my_crate")
            .file(Some("src/lib.rs"))
            .line(Some(42))
            .args(format_args!("something went wrong"))
            .build();

        assert_eq!(
            LineFormat {
                source_location: false,
                ..LineFormat::default()
            }
            .line(Duration::from_millis(120), &record)
            .to_string(),
            "[00:00:00.120 | ERROR | my_crate ] something went wrong"
        );
    }

    #[test]
    fn test_line_hours_past_99() {
        // Hours are a minimum of two digits: past 99 they keep growing
        // instead of wrapping.
        let record = Record::builder()
            .level(Level::Debug)
            .target("my_crate")
            .args(format_args!("late"))
            .build();

        assert_eq!(
            LineFormat::default()
                .line(Duration::from_secs(100 * 3600), &record)
                .to_string(),
            "[100:00:00.000 | DEBUG | my_crate ] late"
        );
    }
}
