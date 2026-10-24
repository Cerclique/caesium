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
The configuration of the global logger (minimum level and color) before
initialization. It is not itself a logger and never handles a record.
_Avoid_: Logger, config

**Line**:
The text written for one log record: elapsed time, level, target, source
location and message, in a fixed shape.
_Avoid_: Entry, log entry, output

**Elapsed time**:
The milliseconds between initialization and a record being written, shown at
the start of every line.
_Avoid_: Timestamp, uptime, duration

**Minimum level**:
The least severe level the global logger writes; records below it are dropped.
_Avoid_: Level filter, log level, verbosity

**Color**:
Styling a whole line with its level's color, applied only when the destination
supports it.
_Avoid_: Colour, colorized output, styling
