# Lithium

A logger that prefixes every log record with the time elapsed since
initialization, plus its level, target and source location, with optional color.

## Language

**Global logger**:
The single process-wide logger that receives every log record. A process has
at most one, and once initialized it cannot be replaced.
_Avoid_: Logger instance, env logger

**Initialization**:
Turning a logger builder into the global logger. It succeeds at most once per
process, whoever attempts it.
_Avoid_: Install, setup, registration

**Logger builder**:
The configuration of the global logger (minimum level and line format)
before initialization. It is not itself a logger and never handles a record.
_Avoid_: Logger, config

**Line**:
The text written to standard error for one log record: elapsed time, level,
target, source location and message, in a fixed shape. Source location is shown by default; it
is left out when turned off or when the record has none, and nothing else in
the shape changes.
_Avoid_: Entry, log entry, output

**Line format**:
How every line is written: whether it shows source location, and whether it
is colored. It decides how a record is written, never whether it is; that is
the minimum level's job.
_Avoid_: Line shape, line options, line style

**Source location**:
The file and line in the code that emitted a log record. It is there for a
developer to jump to that code, so it is optional in a line.
_Avoid_: Location, file:line, callsite

**Elapsed time**:
The time between initialization and a record being written, shown at the
start of every line as hours, minutes, seconds and milliseconds. It is a
duration, not a time of day.
_Avoid_: Timestamp, uptime, duration

**Minimum level**:
The least severe level the global logger writes; records below it are dropped.
_Avoid_: Level filter, log level, verbosity

**Color**:
Styling a whole line with its level's color, applied only when standard error
supports it.
_Avoid_: Colour, colorized output, styling
