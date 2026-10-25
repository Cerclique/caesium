# Lines always go to stderr, with no option to choose the stream

Every line is written to stderr, and `LoggerBuilder` has no option to change that. Lithium used to write to stdout, which loses every line in a program whose stdout is discarded or carries its own data: a waybar_gpui panel daemon runs with stdout set to `/dev/null` and stderr appended to a log file, and a CLI piped into another program or a Waybar module printing JSON needs stdout to itself. Stderr is where diagnostics conventionally go and is `env_logger`'s own default, so a fixed stream covers these cases without a setting, in keeping with lithium's fixed line shape. A program that needs its lines elsewhere redirects stderr itself. `color(true)` keeps resolving through terminal detection on the stream actually written to, so a redirected stderr gets no ANSI codes.

## Considered Options

- **Option defaulting to stdout**: rejected; it keeps the old behavior that loses lines by default, so every affected program has to know to opt out, and it adds a public enum and a builder method for a case nearly no one needs.
- **Option defaulting to stderr**: rejected for now; the only case it adds is a program whose stderr is discarded but stdout kept, which is unusual. An option can be added later without breaking anyone, whereas one shipped now could not be removed without breaking its users.
- **Always stdout**: rejected; it is the behavior that loses lines.
- **Re-export `env_logger::Target`**: rejected; its `Pipe` variant exposes `Box<dyn Write + Send>` and ties the public API to `env_logger`.
