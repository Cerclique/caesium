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
#[non_exhaustive]
pub enum LoggerError {
    /// The global logger was already initialized, whether by an earlier call
    /// to [`LoggerBuilder::init`] or by another crate.
    #[error("a global logger is already initialized")]
    AlreadyInitialized,
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// When lines are styled with their level's color. Passed to
/// [`LoggerBuilder::color`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColorChoice {
    /// Never styles lines, even when stderr is a terminal.
    Never,
    /// Styles lines only when stderr supports it: it is a terminal, or
    /// `CLICOLOR_FORCE` is set, and `NO_COLOR` is not. The default.
    #[default]
    Auto,
    /// Always styles lines, even when stderr is redirected to a file or a
    /// pipe.
    Always,
}

impl From<ColorChoice> for env_logger::WriteStyle {
    fn from(choice: ColorChoice) -> Self {
        match choice {
            ColorChoice::Never => Self::Never,
            ColorChoice::Auto => Self::Auto,
            ColorChoice::Always => Self::Always,
        }
    }
}

/// Configures and initializes the global logger, which writes elapsed time,
/// styled with each level's color when stderr supports it, on top of
/// [`env_logger`].
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
#[derive(Debug)]
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
    /// color set to [`ColorChoice::Auto`] and source location shown.
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

    /// Sets when lines are styled with their level's color. Defaults to
    /// [`ColorChoice::Auto`]. Returns `self` for chaining.
    pub fn color(mut self, choice: ColorChoice) -> Self {
        self.format.color = choice;
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
    /// crate, this returns [`LoggerError::AlreadyInitialized`] and changes
    /// nothing.
    pub fn init(&self) -> Result<(), LoggerError> {
        let start = Instant::now();
        self.env_builder(move || start.elapsed())
            .try_init()
            .map_err(|_| LoggerError::AlreadyInitialized)
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
        builder.write_style(self.format.color.into());
        builder.format(move |buf, record| formatter.format(buf, record));
        builder
    }
}

// ---------------------------------------------------------------------------
// Re-exports
// ---------------------------------------------------------------------------

// The crate docs (README "Usage") explain why these are re-exported; a doc
// comment here would be prepended to `LevelFilter`'s own page.
pub use log::LevelFilter;
pub use log::{debug, error, info, trace, warn};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use env_logger::Target;
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
    fn piped(builder: &LoggerBuilder, elapsed: Duration) -> (env_logger::Logger, Capture) {
        let capture = Capture::default();
        let env_logger = builder
            .env_builder(move || elapsed)
            .target(Target::Pipe(Box::new(capture.clone())))
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
        let (log, capture) = piped(&LoggerBuilder::new(), Duration::ZERO);

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
        let (log, capture) = piped(&builder, Duration::ZERO);

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
        let (log, capture) = piped(&builder, Duration::ZERO);

        emit(&log, Level::Trace, "finest");

        assert_eq!(
            lines(&capture),
            ["[00:00:00.000 | TRACE | my_crate | src/lib.rs:42 ] finest"]
        );
    }

    #[test]
    fn test_source_location_off_omits_the_segment() {
        let builder = LoggerBuilder::new().source_location(false);
        let (log, capture) = piped(&builder, Duration::ZERO);

        emit(&log, Level::Info, "short");

        assert_eq!(
            lines(&capture),
            ["[00:00:00.000 |  INFO | my_crate ] short"]
        );
    }

    #[test]
    fn test_elapsed_time_comes_from_the_clock() {
        let (log, capture) = piped(&LoggerBuilder::new(), Duration::from_millis(125_120));

        emit(&log, Level::Info, "later");

        assert_eq!(
            lines(&capture),
            ["[00:02:05.120 |  INFO | my_crate | src/lib.rs:42 ] later"]
        );
    }

    #[test]
    fn test_color_always_styles_a_line_that_is_not_going_to_a_terminal() {
        // A pipe is never a terminal, so only a forced choice styles it.
        let builder = LoggerBuilder::new().color(ColorChoice::Always);
        let (log, capture) = piped(&builder, Duration::ZERO);

        emit(&log, Level::Error, "styled");

        let output = capture.contents();
        assert!(
            output.starts_with('\x1b'),
            "expected ANSI styling, got {output:?}"
        );
        assert!(output.contains("[00:00:00.000 | ERROR | my_crate | src/lib.rs:42 ] styled"));
    }

    #[test]
    fn test_color_auto_leaves_a_line_that_is_not_going_to_a_terminal_plain() {
        let (log, capture) = piped(&LoggerBuilder::new(), Duration::ZERO);

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

    /// Runs only inside the child process started by `run_child`: initializes
    /// the real global logger with the color choice named by `STDERR_CHILD`
    /// and logs one record. Does nothing if run by hand with `--ignored`.
    #[test]
    #[ignore = "helper run by test_init_writes_lines_to_stderr_not_stdout and the color tests"]
    fn stderr_child() {
        let Some(color) = std::env::var_os(STDERR_CHILD) else {
            return;
        };
        let builder = LoggerBuilder::new().source_location(false);
        let builder = match color.to_str() {
            Some("unset") => builder,
            Some("never") => builder.color(ColorChoice::Never),
            Some("auto") => builder.color(ColorChoice::Auto),
            Some("always") => builder.color(ColorChoice::Always),
            other => panic!("unknown color choice {other:?}"),
        };
        builder.init().unwrap();
        info!("to stderr");
    }

    /// The streams of a child process that initialized the real global logger
    /// and logged one record.
    struct ChildOutput {
        stdout: String,
        stderr: String,
    }

    impl ChildOutput {
        /// The line the child logged, as written to stderr with any styling.
        fn stderr_line(&self) -> &str {
            self.stderr
                .lines()
                .find(|line| line.contains("to stderr"))
                .unwrap_or_else(|| panic!("line missing from stderr: {:?}", self.stderr))
        }
    }

    /// Reruns this test binary with only `stderr_child`, so `init` sets up the
    /// global logger of a fresh process whose streams we can read.
    ///
    /// `color` is the child's color choice, `None` to leave it unset. The
    /// child's stderr is a pipe, never a terminal; `env` adds variables on top
    /// of a parent environment cleared of every variable that affects color
    /// detection, so `CLICOLOR_FORCE` stands in for a terminal that supports
    /// color.
    fn run_child(color: Option<ColorChoice>, env: &[(&str, &str)]) -> ChildOutput {
        let color = match color {
            None => "unset",
            Some(ColorChoice::Never) => "never",
            Some(ColorChoice::Auto) => "auto",
            Some(ColorChoice::Always) => "always",
        };
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["tests::stderr_child", "--exact", "--ignored", "--nocapture"])
            .env_remove("NO_COLOR")
            .env_remove("CLICOLOR")
            .env_remove("CLICOLOR_FORCE")
            .env(STDERR_CHILD, color)
            .envs(env.iter().copied())
            .output()
            .unwrap();
        assert!(output.status.success(), "child failed: {output:?}");
        ChildOutput {
            stdout: String::from_utf8(output.stdout).unwrap(),
            stderr: String::from_utf8(output.stderr).unwrap(),
        }
    }

    #[test]
    fn test_init_writes_lines_to_stderr_not_stdout() {
        let output = run_child(None, &[]);

        // libtest writes its own report to stdout, so check for the line
        // itself rather than an empty stdout.
        assert!(
            output
                .stderr_line()
                .ends_with("|  INFO | caesium::tests ] to stderr"),
            "{:?}",
            output.stderr
        );
        assert!(
            !output.stdout.contains("to stderr"),
            "line written to stdout: {:?}",
            output.stdout
        );
    }

    #[test]
    fn test_color_auto_styles_lines_when_stderr_supports_color() {
        let output = run_child(Some(ColorChoice::Auto), &[("CLICOLOR_FORCE", "1")]);

        assert!(output.stderr_line().contains('\x1b'), "{:?}", output.stderr);
    }

    #[test]
    fn test_color_auto_leaves_lines_plain_when_stderr_does_not_support_color() {
        let output = run_child(Some(ColorChoice::Auto), &[]);

        assert!(
            !output.stderr_line().contains('\x1b'),
            "{:?}",
            output.stderr
        );
    }

    #[test]
    fn test_color_auto_respects_no_color() {
        let output = run_child(
            Some(ColorChoice::Auto),
            &[("CLICOLOR_FORCE", "1"), ("NO_COLOR", "1")],
        );

        assert!(
            !output.stderr_line().contains('\x1b'),
            "{:?}",
            output.stderr
        );
    }

    #[test]
    fn test_color_never_leaves_lines_plain_even_when_stderr_supports_color() {
        let output = run_child(Some(ColorChoice::Never), &[("CLICOLOR_FORCE", "1")]);

        assert!(
            !output.stderr_line().contains('\x1b'),
            "{:?}",
            output.stderr
        );
    }

    #[test]
    fn test_color_defaults_to_auto() {
        let output = run_child(None, &[("CLICOLOR_FORCE", "1")]);

        assert!(output.stderr_line().contains('\x1b'), "{:?}", output.stderr);
    }

    #[test]
    fn test_init_fails_once_a_logger_is_initialized() {
        let builder = LoggerBuilder::new();

        // The first call may fail if a parallel test initialized a logger
        // first; either way, a global logger exists afterwards.
        let _ = builder.init();
        let second = builder.init();

        assert!(
            matches!(second, Err(LoggerError::AlreadyInitialized)),
            "expected AlreadyInitialized, got {second:?}"
        );
    }

    #[test]
    fn test_already_initialized_has_a_message_and_no_source() {
        let error = LoggerError::AlreadyInitialized;

        assert_eq!(error.to_string(), "a global logger is already initialized");
        assert!(std::error::Error::source(&error).is_none());
    }

    #[test]
    fn test_builder_implements_debug() {
        let debug = format!("{:?}", LoggerBuilder::new());

        assert!(debug.starts_with("LoggerBuilder"), "got {debug:?}");
    }
}
