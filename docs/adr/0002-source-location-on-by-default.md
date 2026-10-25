# Source location is shown by default, behind a toggle

Every line shows the record's source location (`file:line`) unless `LoggerBuilder::source_location(false)` turns it off. This departs from `env_logger` and `tracing-subscriber`, which both hide file and line by default, because caesium's main reader is a developer watching a terminal while running their own program, who uses the location to jump to the code; the secondary reader, someone going through a saved or CI log from a possibly older build, gains little from it and can turn it off. Records from dependencies keep the full, often absolute, path the compiler saw rather than a shortened one, since a full path is the one a terminal or IDE can open. A record with no file leaves the segment out, so a missing location and a turned-off one render the same.

## Considered Options

- **Always shown, no toggle**: rejected; the full path is long, especially for dependencies, and a reader who doesn't need it had no way out.
- **Toggle, off by default** (the `env_logger` and `tracing-subscriber` choice): rejected; it hides the field the main reader uses most, and changes the output of existing users.
- **Drop source location, rely on target**: rejected; target gives the module but not the line, so the reader can't jump to the code.
- **Shorten dependency paths** (from the crate folder, or to the file name): rejected; it needs platform-specific path guessing and produces paths that can no longer be opened, trading the field's purpose for looks.
- **`unknown:0` for records without a location**: replaced; it takes space to say nothing, and `:0` reads like a real line number.
