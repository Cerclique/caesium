# Caesium

[![CI](https://github.com/Cerclique/caesium/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/Cerclique/caesium/actions/workflows/rust.yml)
[![MSRV](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2FCerclique%2Fcaesium%2Fmain%2FCargo.toml&query=%24.package.rust-version&label=MSRV)](https://github.com/Cerclique/caesium/blob/main/Cargo.toml)
[![Crates.io](https://img.shields.io/crates/v/caesium-log)](https://crates.io/crates/caesium-log)
[![Docs](https://img.shields.io/docsrs/caesium-log)](https://docs.rs/caesium-log)
[![License](https://img.shields.io/github/license/Cerclique/caesium)](https://github.com/Cerclique/caesium/blob/main/LICENSE)

A logger wrapper with elapsed time tracking and colorized output built on [env_logger], [log], and [thiserror].

## Features

- Fixed line format: elapsed time, level, target and optional source
  location (see [Format](#format))
- Programmatic log level configuration via `LoggerBuilder`
- Colorized output toggle via `color`, disabled by default
- Source location toggle via `source_location`, enabled by default

## Installation

```sh
cargo add caesium-log
```

The package is `caesium-log` on crates.io, and the library is imported as
`caesium`. Up to 0.4.0 the crate was called `lithium` and was only available
from Git.

The minimum supported Rust version is the `rust-version` in
[`Cargo.toml`](https://github.com/Cerclique/caesium/blob/main/Cargo.toml), shown in the MSRV badge above.

## Usage

Configure a `LoggerBuilder`, then call `init` once to initialize it as
the global logger:

```rust
use caesium::{LevelFilter, LoggerBuilder, debug, info};

fn main() {
    LoggerBuilder::new()
        .level(LevelFilter::Debug)
        .init()
        .expect("failed to init logger");

    info!("logger ready");
    debug!("this is shown because the minimum level is Debug");
}
```

`caesium` re-exports the `log` macros (`trace!`, `debug!`, `info!`, `warn!`,
`error!`) and `LevelFilter`, so you don't need a direct `log` dependency.
Records logged through `log` by your dependencies go through the same logger.

The default minimum level is `Info`. Override it with `level`.
Colorized output is disabled by default. Enable it with `color`.
Source location is shown by default. Hide it with `source_location(false)`.

### Errors

Only one global logger can exist per process. If one is already initialized,
whether by an earlier call or by another crate, `init` returns
`LoggerError::InitError` (wrapping `log::SetLoggerError`) and changes nothing:

```text
failed to initialize logger: attempted to set a logger after the logging system was already initialized
```

## Example

[`examples/main.rs`](https://github.com/Cerclique/caesium/blob/main/examples/main.rs) initializes the logger, logs at every
level and then tries to initialize it a second time:

```sh
cargo run --example main
```

```text
[00:00:00.000 |  INFO | main | examples/main.rs:11 ] Logger initialized successfully!
[00:00:00.000 | DEBUG | main | examples/main.rs:12 ] This is a debug message
[00:00:00.000 |  WARN | main | examples/main.rs:14 ] This is a warning message
[00:00:00.000 | ERROR | main | examples/main.rs:15 ] This is an error message
[00:00:00.150 |  INFO | main | examples/main.rs:19 ] After a short delay
[00:00:00.400 |  WARN | main | examples/main.rs:22 ] Another warning after more time has passed
Expected init error: failed to initialize logger: attempted to set a logger after the logging system was already initialized
```

The `trace!` call is filtered out because the minimum level is `Debug`.

## Log Levels

The following log levels are available, listed from least to most severe:

| Level   | Description                                      |
| ------- | ------------------------------------------------ |
| `Trace` | Finest-grained diagnostic events                 |
| `Debug` | Debugging information                            |
| `Info`  | General informational messages (default minimum) |
| `Warn`  | Warning conditions                               |
| `Error` | Critical errors and failures                     |

Set the minimum level via `LoggerBuilder::level(LevelFilter::Level)`. Messages at the minimum severity or higher are emitted.

## Format

Every line is written to stderr, never stdout, so your program's stdout stays
free for its own output. Each line has a fixed shape:

```text
[00:02:05.120 | ERROR | my_crate | src/lib.rs:42 ] something went wrong
```

- **Elapsed time**: time since `init` ran, as zero-padded `HH:MM:SS.sss`
  truncated to the millisecond. Hours are at least two digits and keep
  growing past 99 (`100:00:00.000`) rather than wrapping.
- **Level**: right-aligned in a 5-column field.
- **Target**: the record's target, usually the module path.
- **Source location**: `file:line`, or just `file` when the record has no
  line. Records from dependencies show the full path the compiler saw, often
  an absolute path into the Cargo registry. The segment is left out when
  `source_location(false)` is set or the record has no file:

  ```text
  [00:02:05.120 | ERROR | my_crate ] something went wrong
  ```

Key-value fields attached through the `log` crate's `kv` feature
(`info!(user_id = 42; "logged in")`) are not shown; only the message is
written.

## Notes

- `LoggerBuilder::new` reads no environment variables, so
  `RUST_LOG` has no effect. The minimum level is set only via `level`.
- `color(true)` does not force color. It resolves through
  `env_logger`'s terminal detection, so styling is stripped when stderr is
  not a TTY (for example, when redirected to a file). It means "allow color
  when the terminal supports it".

## Changelog

See [CHANGELOG.md](https://github.com/Cerclique/caesium/blob/main/CHANGELOG.md).

## Contributing

See [CONTRIBUTING.md](https://github.com/Cerclique/caesium/blob/main/docs/CONTRIBUTING.md).
To start with every tool preinstalled, open the dev container in GitHub Codespaces:

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/Cerclique/caesium)

To report a vulnerability, see [SECURITY.md](https://github.com/Cerclique/caesium/blob/main/docs/SECURITY.md).
This project follows a [Code of Conduct](https://github.com/Cerclique/caesium/blob/main/docs/CODE_OF_CONDUCT.md).

## License

[MIT](https://github.com/Cerclique/caesium/blob/main/LICENSE)

[env_logger]: https://docs.rs/env_logger
[log]: https://docs.rs/log
[thiserror]: https://docs.rs/thiserror
