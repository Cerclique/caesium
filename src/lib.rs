use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

mod format;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors returned by [`Logger`] operations.
///
/// This crate uses [`thiserror`] for ergonomic error handling. All errors implement
/// [`std::error::Error`], [`std::fmt::Debug`], and [`std::fmt::Display`].
///
/// [`thiserror`]: https://docs.rs/thiserror
#[derive(Debug, Error)]
pub enum LoggerError {
    /// Returned when [`Logger::init`] is called on an instance that has
    /// already been initialized. A failed initialization does not mark the
    /// instance as initialized, so it may be retried.
    #[error("logger already initialized")]
    AlreadyInitialized,

    /// Returned when [`env_logger`] fails to initialize the global logger.
    ///
    /// This typically happens when another crate has already initialized a
    /// logger with different configuration.
    ///
    /// [`env_logger`]: https://docs.rs/env_logger
    #[error("failed to initialize logger: {0}")]
    InitError(#[from] log::SetLoggerError),
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Configures a [`Logger`] before building it.
///
/// `LoggerBuilder` uses a fluent builder pattern. Construct it via [`new`],
/// configure the minimum log level with [`level`], then produce the final
/// [`Logger`] by calling [`build`].
///
/// # Example
///
/// ```rust
/// use lithium::{LoggerBuilder, LevelFilter};
///
/// let logger = LoggerBuilder::new()
///     .level(LevelFilter::Debug)
///     .build();
///
/// logger.init().expect("failed to init logger");
/// ```
///
/// [`Logger`]: struct.Logger.html
/// [`new`]: #method.new
/// [`level`]: #method.level
/// [`color`]: #method.color
/// [`build`]: #method.build
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

    /// Consumes the builder and returns a [`Logger`] carrying the configured
    /// options.
    pub fn build(self) -> Logger {
        Logger {
            level: self.level,
            color: self.color,
            initialized: AtomicBool::new(false),
        }
    }
}

// ---------------------------------------------------------------------------
// Logger
// ---------------------------------------------------------------------------

/// A logger wrapper that provides elapsed time tracking and optional colorized output
/// built on top of [`env_logger`].
///
/// Create a [`Logger`] via [`LoggerBuilder::new().build()`] at application startup,
/// then initialize it exactly once via [`init`]. Do not create multiple [`Logger`]
/// instances — `env_logger` is global, so calling [`init`] on more than one instance
/// will fail for all but the first.
///
/// # stdout format
///
/// ```text
/// [      120 | ERROR | my_crate | src/lib.rs:42 ] something went wrong
/// ```
///
/// # Example
///
/// ```rust
/// use lithium::{info, LoggerBuilder};
///
/// let logger = LoggerBuilder::new().build();
/// logger.init().expect("failed to init logger");
///
/// info!("Application started");
/// ```
///
/// [`env_logger`]: https://docs.rs/env_logger
/// [`init`]: Logger::init
#[must_use = "call .init() to enable logging"]
pub struct Logger {
    level: LevelFilter,
    color: bool,
    initialized: AtomicBool,
}

impl Logger {
    /// Initializes the logger with a custom formatter that includes elapsed time,
    /// log level, target, file, and line number. Elapsed time on each formatted
    /// line is measured from the moment this method runs.
    ///
    /// The minimum log level is determined by [`LoggerBuilder::level()`],
    /// defaulting to [`LevelFilter::Info`].
    ///
    /// If `env_logger` has already been initialized globally (for example by
    /// another crate or test), this method returns [`LoggerError::InitError`].
    ///
    /// If this method is called again on the same instance after a successful
    /// initialization, it returns [`LoggerError::AlreadyInitialized`]. A failed
    /// attempt does not mark the instance as initialized, so it may be retried.
    ///
    /// [`LoggerError::AlreadyInitialized`]: LoggerError::AlreadyInitialized
    /// [`LoggerError::InitError`]: LoggerError::InitError
    pub fn init(&self) -> Result<(), LoggerError> {
        // Atomically claim initialization. If we already initialized, return error.
        if !self
            .initialized
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return Err(LoggerError::AlreadyInitialized);
        }

        if let Err(err) = self.env_builder().try_init() {
            // The global initialization failed; release the claim so the
            // instance can be retried instead of being permanently stuck
            // behind `AlreadyInitialized`.
            self.initialized.store(false, Ordering::Release);
            return Err(LoggerError::InitError(err));
        }

        Ok(())
    }

    /// Returns the `env_logger` builder that [`init`] installs globally,
    /// configured with the level filter and the line formatter.
    ///
    /// Kept separate from [`init`] so tests can redirect it to a pipe and
    /// exercise the whole pipeline without touching the global logger.
    ///
    /// [`init`]: Logger::init
    fn env_builder(&self) -> env_logger::Builder {
        let formatter = format::LineFormatter::new(self.color);

        let mut builder = env_logger::Builder::new();
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

    /// Builds the same pipeline `init` installs, but writing to a capture
    /// buffer instead of being installed as the global logger.
    fn piped(logger: &Logger, write_style: WriteStyle) -> (env_logger::Logger, Capture) {
        let capture = Capture::default();
        let env_logger = logger
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

    /// Strips the elapsed-time field, which depends on wall-clock time.
    fn without_elapsed(output: &str) -> Vec<&str> {
        output
            .lines()
            .map(|line| {
                let (elapsed, rest) = line.split_once(" | ").expect("missing elapsed field");
                elapsed
                    .trim_start_matches('[')
                    .trim()
                    .parse::<u128>()
                    .expect("elapsed field is not a number");
                rest
            })
            .collect()
    }

    #[test]
    fn test_default_level_filters_below_info() {
        let (log, capture) = piped(&LoggerBuilder::new().build(), WriteStyle::Never);

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
        let logger = LoggerBuilder::new().level(LevelFilter::Warn).build();
        let (log, capture) = piped(&logger, WriteStyle::Never);

        emit(&log, Level::Info, "dropped");
        emit(&log, Level::Warn, "kept");

        assert_eq!(
            without_elapsed(&capture.contents()),
            [" WARN | my_crate | src/lib.rs:42 ] kept"]
        );
    }

    #[test]
    fn test_trace_level_emits_everything() {
        let logger = LoggerBuilder::new().level(LevelFilter::Trace).build();
        let (log, capture) = piped(&logger, WriteStyle::Never);

        emit(&log, Level::Trace, "finest");

        assert_eq!(
            without_elapsed(&capture.contents()),
            ["TRACE | my_crate | src/lib.rs:42 ] finest"]
        );
    }

    #[test]
    fn test_color_enabled_styles_the_line() {
        let logger = LoggerBuilder::new().color(true).build();
        let (log, capture) = piped(&logger, WriteStyle::Always);

        emit(&log, Level::Error, "styled");

        let output = capture.contents();
        assert!(output.starts_with('\x1b'), "expected ANSI styling, got {output:?}");
        assert!(output.contains("ERROR | my_crate | src/lib.rs:42 ] styled"));
    }

    #[test]
    fn test_color_disabled_writes_plain_text() {
        // Even when the terminal allows styling, color(false) must not emit it.
        let (log, capture) = piped(&LoggerBuilder::new().build(), WriteStyle::Always);

        emit(&log, Level::Error, "plain");

        let output = capture.contents();
        assert!(!output.contains('\x1b'), "unexpected ANSI styling in {output:?}");
    }

    #[test]
    fn test_multiple_initializations() {
        let logger = LoggerBuilder::new().build();

        let first = logger.init();
        let result = logger.init();

        if first.is_ok() {
            // A successful init claims the instance permanently.
            assert!(
                matches!(result, Err(LoggerError::AlreadyInitialized)),
                "expected AlreadyInitialized, got {result:?}"
            );
        } else {
            // A failed init leaves the instance retryable.
            assert!(
                matches!(result, Err(LoggerError::InitError(_))),
                "expected InitError, got {result:?}"
            );
        }
    }

    #[test]
    fn test_failed_init_is_retryable() {
        // Claim the global logger first so the init below deterministically
        // fails, regardless of what parallel tests have done.
        let claimant = LoggerBuilder::new().build();
        let _ = claimant.init();

        let logger = LoggerBuilder::new().build();
        let first = logger.init();
        assert!(
            matches!(first, Err(LoggerError::InitError(_))),
            "expected InitError, got {first:?}"
        );

        // A failed init must not leave the instance stuck behind
        // `AlreadyInitialized`; the retry should surface the original error.
        let second = logger.init();
        assert!(
            matches!(second, Err(LoggerError::InitError(_))),
            "failed init should be retryable, got {second:?}"
        );
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;

        let logger = Arc::new(LoggerBuilder::new().build());
        let ok_count = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0));

        let mut handles = vec![];
        for _ in 0..10 {
            let logger_clone = Arc::clone(&logger);
            let ok_count_clone = Arc::clone(&ok_count);
            handles.push(std::thread::spawn(move || {
                if logger_clone.init().is_ok() {
                    ok_count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let count = ok_count.load(std::sync::atomic::Ordering::Relaxed);
        assert!(
            count <= 1,
            "at most one thread should init successfully (got {count})"
        );
    }

    #[test]
    fn test_two_loggers_init_error() {
        // env_logger is global, so only one Logger can initialize successfully.
        let l1 = LoggerBuilder::new().build();
        let l2 = LoggerBuilder::new().build();

        let r1 = l1.init();
        let r2 = l2.init();

        let ok_count = r1.is_ok() as u8 + r2.is_ok() as u8;
        assert!(
            ok_count <= 1,
            "only one Logger should init successfully (got {ok_count})"
        );
    }
}
