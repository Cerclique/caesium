# Color is automatic by default, as a three-way choice

`LoggerBuilder::color` takes a `ColorChoice` of `Never`, `Auto` or `Always`, and the default is `Auto`: each line is colored with its level's color when stderr supports it. It replaced `color(bool)`, which defaulted to off and whose `true` only allowed color, since `env_logger`'s terminal detection still stripped it from a redirected stderr; the name promised a switch it was not. As in the source location decision, caesium's main reader is a developer watching a terminal while running their own program, and color is what lets them pick out warnings and errors at a glance. `Auto` costs the other readers nothing: it falls back to plain text when stderr is redirected to a file or a pipe, or when `NO_COLOR` is set, and `env_logger` and most CLI loggers already default to it. `Always` covers the case `bool` could not, such as a pager or CI log viewer that renders ANSI codes from a pipe. `ColorChoice` is caesium's own type, so no third-party type sits in the public API.

## Considered Options

- **Keep `color(bool)` and fix its docs**: rejected; the name still doesn't say that `true` only allows color, and color still can't be forced.
- **Rename the `bool` to `allow_color`**: rejected; the name becomes honest, but color still can't be forced.
- **Three-way choice, default `Never`**: rejected; it hides color from the main reader, and the change to the `color` signature is breaking anyway, so keeping the old default saves no one a migration.
- **Re-export `env_logger::WriteStyle`**: rejected; it puts a third-party type back into the public API, and its variants follow `env_logger`'s releases rather than caesium's.
