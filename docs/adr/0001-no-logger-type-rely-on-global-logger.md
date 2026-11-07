# No Logger type: initialization relies on the one-global-logger rule

There is no `Logger` type: `LoggerBuilder::init(&self)` initializes the global logger directly, and a repeated or competing initialization surfaces only as `LoggerError::AlreadyInitialized`. An earlier design built a `Logger` that guarded `init` with its own per-instance flag and an `AlreadyInitialized` error, but the `log` crate already guarantees at most one global logger per process, so the guard duplicated that rule and only distinguished "this instance already initialized" from "something else did", which no caller acts on differently.

The current `LoggerError::AlreadyInitialized` (which replaced `LoggerError::InitError` in #44) shares that rejected error's name but not its meaning: it reports the `log` crate's one-global-logger rule, whoever initialized first, rather than a per-instance flag.

## Considered Options

- **Keep `Logger` with a per-instance guard**: rejected; it added a type, an error variant and claim/release logic without giving callers any decision to make.
- **Rename `LoggerBuilder` to `Logger`**: rejected; the type configures the global logger but never handles a record itself, and `Logger` would clash with `env_logger::Logger`. `env_logger::Builder` sets the precedent of a builder finished by `init` rather than `build`.
