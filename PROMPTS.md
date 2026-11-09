# Prompts

The same main prompt was given to each model.

## Main prompt

> Write a small Rust command-line program called `log-analyzer`, at most 200 lines,
> using only the standard library (no external crates).
>
> It takes the path of a log file as its only argument: `cargo run -- sample.log`.
> Each valid line looks like this:
>
>     2026-11-09 10:02:10 ERROR Database connection failed
>
> That is: a date, a time (HH:MM:SS), a level (INFO, WARN or ERROR), then a free-text message.
>
> The program must:
> 1. Read the file line by line. Skip empty lines. Count lines that do not match the
>    format as "skipped" instead of crashing.
> 2. Count lines per level (INFO, WARN, ERROR).
> 3. Find the busiest minute (HH:MM) and how many lines it had.
> 4. List the 5 most common ERROR messages with their counts, most common first.
> 5. Print a clear, aligned summary.
> 6. Print a helpful message and exit with a non-zero code if the argument is missing
>    or the file cannot be read.
>
> Use a struct for a parsed log entry and an enum for the level. Keep parsing in its
> own function that returns an Option or Result. Add a few unit tests for the parser.

## Follow-up prompts

(Add any follow-up prompts you give here, e.g. fixes or changes you asked for.)
