# Lithium

[![CI](https://github.com/Cerclique/lithium/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/Cerclique/lithium/actions/workflows/rust.yml)
[![MSRV](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2FCerclique%2Flithium%2Fmain%2FCargo.toml&query=%24.package.rust-version&label=MSRV)](./Cargo.toml)
[![License](https://img.shields.io/github/license/Cerclique/lithium)](./LICENSE)

A logger wrapper with elapsed time tracking and colorized output built on [env_logger], [log], and [thiserror].

## Features

- Elapsed time tracking per line
- Custom format: `[elapsed_ms | LEVEL | target | file:line ] message`
- Programmatic log level configuration via `LoggerBuilder`
- Colorized output toggle via `color`, disabled by default

## Installation

`lithium` is not published on crates.io (the `lithium` crate there is an
unrelated project), so depend on it from Git, pinned to a release tag:

```toml
[dependencies]
lithium = { git = "https://github.com/Cerclique/lithium", tag = "v0.2.0" }
```

The minimum supported Rust version is the `rust-version` in
[`Cargo.toml`](./Cargo.toml), shown in the MSRV badge above.

## Usage

Configure a `LoggerBuilder`, then call `init` once to initialize it as
the global logger:

```rust
use lithium::{LevelFilter, LoggerBuilder, debug, info};

fn main() {
    LoggerBuilder::new()
        .level(LevelFilter::Debug)
        .init()
        .expect("failed to init logger");

    info!("logger ready");
    debug!("this is shown because the minimum level is Debug");
}
```

`lithium` re-exports the `log` macros (`trace!`, `debug!`, `info!`, `warn!`,
`error!`) and `LevelFilter`, so you don't need a direct `log` dependency.
Records logged through `log` by your dependencies go through the same logger.

The default minimum level is `Info`. Override it with `level`.
Colorized output is disabled by default. Enable it with `color`.

### Errors

Only one global logger can exist per process. If one is already initialized,
whether by an earlier call or by another crate, `init` returns
`LoggerError::InitError` (wrapping `log::SetLoggerError`) and changes nothing:

```text
failed to initialize logger: attempted to set a logger after the logging system was already initialized
```

## Example

[`examples/main.rs`](./examples/main.rs) initializes the logger, logs at every
level and then tries to initialize it a second time:

```sh
cargo run --example main
```

```text
[        0 |  INFO | main | examples/main.rs:8 ] Logger initialized successfully!
[        0 | DEBUG | main | examples/main.rs:9 ] This is a debug message
[        0 |  WARN | main | examples/main.rs:11 ] This is a warning message
[        0 | ERROR | main | examples/main.rs:12 ] This is an error message
[      150 |  INFO | main | examples/main.rs:16 ] After a short delay
[      400 |  WARN | main | examples/main.rs:19 ] Another warning after more time has passed
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

### stdout

Every line is written to stdout in a fixed shape:

```text
[      120 | ERROR | my_crate | src/lib.rs:42 ] something went wrong
```

- **Elapsed time**: milliseconds since `init` ran, right-aligned in a field at
  least 9 columns wide.
- **Level**: right-aligned in a 5-column field.
- **Target**: the record's target, usually the module path.
- **Source location**: `file:line`, or `unknown:0` when the record has none.

## Notes

- `LoggerBuilder::new` reads no environment variables, so
  `RUST_LOG` has no effect. The minimum level is set only via `level`.
- `color(true)` does not force color. It resolves through
  `env_logger`'s terminal detection, so styling is stripped when the output is
  not a TTY (for example, when piped or redirected). It means "allow color
  when the terminal supports it".

## License

[MIT](./LICENSE)

[env_logger]: https://docs.rs/env_logger
[log]: https://docs.rs/log
[thiserror]: https://docs.rs/thiserror
