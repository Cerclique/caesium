#![doc = include_str!("../README.md")]

use std::time::{Duration, Instant};

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
/// configure it with [`level`], [`color`] and [`source_location`], then
/// initialize it with [`init`]. Each line is written to stderr as described in
/// [Format](crate#format).
///
/// # Example
///
/// ```rust
/// use caesium::{info, LoggerBuilder, LevelFilter};
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
/// [`source_location`]: LoggerBuilder::source_location
/// [`init`]: LoggerBuilder::init
#[must_use = "call .init() to enable logging"]
pub struct LoggerBuilder {
    level: LevelFilter,
    format: format::LineFormat,
}

impl Default for LoggerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LoggerBuilder {
    /// Creates a builder with the default level of [`LevelFilter::Info`],
    /// color disabled and source location shown.
    pub fn new() -> Self {
        Self {
            level: LevelFilter::Info,
            format: format::LineFormat::default(),
        }
    }

    /// Sets the minimum log level. Returns `self` for chaining.
    pub fn level(mut self, level: LevelFilter) -> Self {
        self.level = level;
        self
    }

    /// Enables or disables colorized output. Returns `self` for chaining.
    pub fn color(mut self, color: bool) -> Self {
        self.format.color = color;
        self
    }

    /// Shows or hides the source location (`file:line`) on each line. Shown
    /// by default. Returns `self` for chaining.
    pub fn source_location(mut self, source_location: bool) -> Self {
        self.format.source_location = source_location;
        self
    }

    /// Initializes the global logger, which writes lines as described in
    /// [Format](crate#format). Elapsed time on each line is measured from the
    /// moment this method runs.
    ///
    /// # Errors
    ///
    /// Only one global logger can exist per process. If one is already
    /// initialized, whether by an earlier call to this method or by another
    /// crate, this returns [`LoggerError::InitError`] and changes nothing.
    pub fn init(&self) -> Result<(), LoggerError> {
        let start = Instant::now();
        self.env_builder(move || start.elapsed()).try_init()?;
        Ok(())
    }

    /// Returns the `env_logger` builder that [`init`] initializes globally,
    /// configured to write to stderr with the level filter and the line
    /// formatter. `elapsed` returns the elapsed time shown on each line.
    ///
    /// Kept separate from [`init`] so tests can redirect it to a pipe, fix the
    /// elapsed time and exercise the whole pipeline without touching the
    /// global logger.
    ///
    /// [`init`]: LoggerBuilder::init
    fn env_builder(
        &self,
        elapsed: impl Fn() -> Duration + Send + Sync + 'static,
    ) -> env_logger::Builder {
        let formatter = format::LineFormatter::new(self.format, elapsed);

        let mut builder = env_logger::Builder::new();
        builder.target(env_logger::Target::Stderr);
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
/// These macros and types can be used directly after importing from the `caesium` crate:
///
/// ```rust
/// use caesium::{info, debug, warn, error, trace, LevelFilter};
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
    /// buffer instead of becoming the global logger, with every line showing
    /// `elapsed`.
    fn piped(
        builder: &LoggerBuilder,
        write_style: WriteStyle,
        elapsed: Duration,
    ) -> (env_logger::Logger, Capture) {
        let capture = Capture::default();
        let env_logger = builder
            .env_builder(move || elapsed)
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

    fn lines(capture: &Capture) -> Vec<String> {
        capture.contents().lines().map(str::to_owned).collect()
    }

    #[test]
    fn test_default_level_filters_below_info() {
        let (log, capture) = piped(&LoggerBuilder::new(), WriteStyle::Never, Duration::ZERO);

        emit(&log, Level::Trace, "dropped");
        emit(&log, Level::Debug, "dropped");
        emit(&log, Level::Info, "kept");
        emit(&log, Level::Error, "kept too");

        assert_eq!(
            lines(&capture),
            [
                "[00:00:00.000 |  INFO | my_crate | src/lib.rs:42 ] kept",
                "[00:00:00.000 | ERROR | my_crate | src/lib.rs:42 ] kept too",
            ]
        );
    }

    #[test]
    fn test_explicit_level_is_applied() {
        let builder = LoggerBuilder::new().level(LevelFilter::Warn);
        let (log, capture) = piped(&builder, WriteStyle::Never, Duration::ZERO);

        emit(&log, Level::Info, "dropped");
        emit(&log, Level::Warn, "kept");

        assert_eq!(
            lines(&capture),
            ["[00:00:00.000 |  WARN | my_crate | src/lib.rs:42 ] kept"]
        );
    }

    #[test]
    fn test_trace_level_emits_everything() {
        let builder = LoggerBuilder::new().level(LevelFilter::Trace);
        let (log, capture) = piped(&builder, WriteStyle::Never, Duration::ZERO);

        emit(&log, Level::Trace, "finest");

        assert_eq!(
            lines(&capture),
            ["[00:00:00.000 | TRACE | my_crate | src/lib.rs:42 ] finest"]
        );
    }

    #[test]
    fn test_source_location_off_omits_the_segment() {
        let builder = LoggerBuilder::new().source_location(false);
        let (log, capture) = piped(&builder, WriteStyle::Never, Duration::ZERO);

        emit(&log, Level::Info, "short");

        assert_eq!(
            lines(&capture),
            ["[00:00:00.000 |  INFO | my_crate ] short"]
        );
    }

    #[test]
    fn test_elapsed_time_comes_from_the_clock() {
        let (log, capture) = piped(
            &LoggerBuilder::new(),
            WriteStyle::Never,
            Duration::from_millis(125_120),
        );

        emit(&log, Level::Info, "later");

        assert_eq!(
            lines(&capture),
            ["[00:02:05.120 |  INFO | my_crate | src/lib.rs:42 ] later"]
        );
    }

    #[test]
    fn test_color_enabled_styles_the_line() {
        let builder = LoggerBuilder::new().color(true);
        let (log, capture) = piped(&builder, WriteStyle::Always, Duration::ZERO);

        emit(&log, Level::Error, "styled");

        let output = capture.contents();
        assert!(
            output.starts_with('\x1b'),
            "expected ANSI styling, got {output:?}"
        );
        assert!(output.contains("[00:00:00.000 | ERROR | my_crate | src/lib.rs:42 ] styled"));
    }

    #[test]
    fn test_color_disabled_writes_plain_text() {
        // Even when the terminal allows styling, color(false) must not emit it.
        let (log, capture) = piped(&LoggerBuilder::new(), WriteStyle::Always, Duration::ZERO);

        emit(&log, Level::Error, "plain");

        let output = capture.contents();
        assert!(
            !output.contains('\x1b'),
            "unexpected ANSI styling in {output:?}"
        );
    }

    /// Set in the child process started by
    /// `test_init_writes_lines_to_stderr_not_stdout`.
    const STDERR_CHILD: &str = "CAESIUM_STDERR_CHILD";

    /// Runs only inside the child process: initializes the real global logger
    /// and logs one record. Does nothing in a normal test run.
    #[test]
    fn stderr_child() {
        if std::env::var_os(STDERR_CHILD).is_none() {
            return;
        }
        LoggerBuilder::new().source_location(false).init().unwrap();
        info!("to stderr");
    }

    #[test]
    fn test_init_writes_lines_to_stderr_not_stdout() {
        // Rerun this test binary with only `stderr_child`, so `init` sets up
        // the global logger of a fresh process whose streams we can read.
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["tests::stderr_child", "--exact", "--nocapture"])
            .env(STDERR_CHILD, "1")
            .output()
            .unwrap();
        assert!(output.status.success(), "child failed: {output:?}");

        // libtest writes its own report to stdout, so check for the line
        // itself rather than an empty stdout.
        let line = "|  INFO | caesium::tests ] to stderr";
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains(line),
            "line missing from stderr: {stderr:?}"
        );
        assert!(!stdout.contains(line), "line written to stdout: {stdout:?}");
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
