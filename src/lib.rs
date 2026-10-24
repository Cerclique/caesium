#![doc = include_str!("../README.md")]

use thiserror::Error;

mod format;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors returned by [`LoggerBuilder::init`].
///
/// This crate uses [`thiserror`] for ergonomic error handling. All errors implement
/// [`std::error::Error`], [`std::fmt::Debug`], and [`std::fmt::Display`].
///
/// [`thiserror`]: https://docs.rs/thiserror
#[derive(Debug, Error)]
pub enum LoggerError {
    /// Returned when the global logger is already initialized, whether by an
    /// earlier call to [`LoggerBuilder::init`] or by another crate.
    #[error("failed to initialize logger: {0}")]
    InitError(#[from] log::SetLoggerError),
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Configures and initializes the global logger, which writes elapsed time and
/// optional colorized output on top of [`env_logger`].
///
/// `LoggerBuilder` uses a fluent builder pattern. Construct it via [`new`],
/// configure it with [`level`] and [`color`], then initialize it with [`init`].
///
/// # stdout format
///
/// ```text
/// [00:02:05.120 | ERROR | my_crate | src/lib.rs:42 ] something went wrong
/// ```
///
/// # Example
///
/// ```rust
/// use lithium::{info, LoggerBuilder, LevelFilter};
///
/// LoggerBuilder::new()
///     .level(LevelFilter::Debug)
///     .init()
///     .expect("failed to init logger");
///
/// info!("Application started");
/// ```
///
/// [`env_logger`]: https://docs.rs/env_logger
/// [`new`]: LoggerBuilder::new
/// [`level`]: LoggerBuilder::level
/// [`color`]: LoggerBuilder::color
/// [`init`]: LoggerBuilder::init
#[must_use = "call .init() to enable logging"]
pub struct LoggerBuilder {
    level: LevelFilter,
    color: bool,
}

impl Default for LoggerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LoggerBuilder {
    /// Creates a builder with the default level of [`LevelFilter::Info`] and
    /// color disabled.
    pub fn new() -> Self {
        Self {
            level: LevelFilter::Info,
            color: false,
        }
    }

    /// Sets the minimum log level. Returns `self` for chaining.
    pub fn level(mut self, level: LevelFilter) -> Self {
        self.level = level;
        self
    }

    /// Enables or disables colorized output. Returns `self` for chaining.
    pub fn color(mut self, color: bool) -> Self {
        self.color = color;
        self
    }

    /// Initializes the global logger with a custom formatter that includes
    /// elapsed time, log level, target, file, and line number. Elapsed time on
    /// each formatted line is measured from the moment this method runs.
    ///
    /// # Errors
    ///
    /// Only one global logger can exist per process. If one is already
    /// initialized, whether by an earlier call to this method or by another
    /// crate, this returns [`LoggerError::InitError`] and changes nothing.
    pub fn init(&self) -> Result<(), LoggerError> {
        self.env_builder().try_init()?;
        Ok(())
    }

    /// Returns the `env_logger` builder that [`init`] initializes globally,
    /// configured to write to stdout with the level filter and the line
    /// formatter.
    ///
    /// Kept separate from [`init`] so tests can redirect it to a pipe and
    /// exercise the whole pipeline without touching the global logger.
    ///
    /// [`init`]: LoggerBuilder::init
    fn env_builder(&self) -> env_logger::Builder {
        let formatter = format::LineFormatter::new(self.color);

        let mut builder = env_logger::Builder::new();
        builder.target(env_logger::Target::Stdout);
        builder.filter_level(self.level);
        builder.format(move |buf, record| formatter.format(buf, record));
        builder
    }
}

// ---------------------------------------------------------------------------
// Re-exports
// ---------------------------------------------------------------------------

/// Re-exports the `log` crate macros and `LevelFilter` for convenience.
///
/// These macros and types can be used directly after importing from the `lithium` crate:
///
/// ```rust
/// use lithium::{info, debug, warn, error, trace, LevelFilter};
///
/// info!("This is an info message");
/// ```
pub use log::LevelFilter;
pub use log::{debug, error, info, trace, warn};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use env_logger::{Target, WriteStyle};
    use log::{Level, Log, Record};
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    /// A cloneable in-memory sink handed to `env_logger` as a pipe target.
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    impl Capture {
        fn contents(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl Write for Capture {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(data)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Builds the same pipeline `init` initializes, but writing to a capture
    /// buffer instead of becoming the global logger.
    fn piped(builder: &LoggerBuilder, write_style: WriteStyle) -> (env_logger::Logger, Capture) {
        let capture = Capture::default();
        let env_logger = builder
            .env_builder()
            .target(Target::Pipe(Box::new(capture.clone())))
            .write_style(write_style)
            .build();
        (env_logger, capture)
    }

    fn emit(log: &impl Log, level: Level, message: &str) {
        log.log(
            &Record::builder()
                .level(level)
                .target("my_crate")
                .file(Some("src/lib.rs"))
                .line(Some(42))
                .args(format_args!("{message}"))
                .build(),
        );
    }

    /// Strips the elapsed-time field, which depends on wall-clock time,
    /// after checking it has the `[HH:MM:SS.sss` shape.
    fn without_elapsed(output: &str) -> Vec<&str> {
        output
            .lines()
            .map(|line| {
                let (elapsed, rest) = line.split_once(" | ").expect("missing elapsed field");
                let shape: String = elapsed
                    .chars()
                    .map(|c| if c.is_ascii_digit() { '0' } else { c })
                    .collect();
                assert_eq!(
                    shape, "[00:00:00.000",
                    "unexpected elapsed field {elapsed:?}"
                );
                rest
            })
            .collect()
    }

    #[test]
    fn test_default_level_filters_below_info() {
        let (log, capture) = piped(&LoggerBuilder::new(), WriteStyle::Never);

        emit(&log, Level::Trace, "dropped");
        emit(&log, Level::Debug, "dropped");
        emit(&log, Level::Info, "kept");
        emit(&log, Level::Error, "kept too");

        assert_eq!(
            without_elapsed(&capture.contents()),
            [
                " INFO | my_crate | src/lib.rs:42 ] kept",
                "ERROR | my_crate | src/lib.rs:42 ] kept too",
            ]
        );
    }

    #[test]
    fn test_explicit_level_is_applied() {
        let builder = LoggerBuilder::new().level(LevelFilter::Warn);
        let (log, capture) = piped(&builder, WriteStyle::Never);

        emit(&log, Level::Info, "dropped");
        emit(&log, Level::Warn, "kept");

        assert_eq!(
            without_elapsed(&capture.contents()),
            [" WARN | my_crate | src/lib.rs:42 ] kept"]
        );
    }

    #[test]
    fn test_trace_level_emits_everything() {
        let builder = LoggerBuilder::new().level(LevelFilter::Trace);
        let (log, capture) = piped(&builder, WriteStyle::Never);

        emit(&log, Level::Trace, "finest");

        assert_eq!(
            without_elapsed(&capture.contents()),
            ["TRACE | my_crate | src/lib.rs:42 ] finest"]
        );
    }

    #[test]
    fn test_color_enabled_styles_the_line() {
        let builder = LoggerBuilder::new().color(true);
        let (log, capture) = piped(&builder, WriteStyle::Always);

        emit(&log, Level::Error, "styled");

        let output = capture.contents();
        assert!(
            output.starts_with('\x1b'),
            "expected ANSI styling, got {output:?}"
        );
        assert!(output.contains("ERROR | my_crate | src/lib.rs:42 ] styled"));
    }

    #[test]
    fn test_color_disabled_writes_plain_text() {
        // Even when the terminal allows styling, color(false) must not emit it.
        let (log, capture) = piped(&LoggerBuilder::new(), WriteStyle::Always);

        emit(&log, Level::Error, "plain");

        let output = capture.contents();
        assert!(
            !output.contains('\x1b'),
            "unexpected ANSI styling in {output:?}"
        );
    }

    #[test]
    fn test_init_fails_once_a_logger_is_initialized() {
        let builder = LoggerBuilder::new();

        // The first call may fail if a parallel test initialized a logger
        // first; either way, a global logger exists afterwards.
        let _ = builder.init();
        let second = builder.init();

        assert!(
            matches!(second, Err(LoggerError::InitError(_))),
            "expected InitError, got {second:?}"
        );
    }
}
