# log-analyzer

A small Rust command-line tool that reads a log file and summarizes it: how many
lines per level (INFO / WARN / ERROR), the busiest minute, and the most common
error messages. Lines that don't match the expected format are counted as
skipped instead of crashing the program.

Standard library only, no external crates.

## Run it

```bash
cargo build
cargo run -- sample.log
cargo test
```

Expected log line format:

```
2026-11-09 10:02:10 ERROR Database connection failed
```

Example output for `sample.log`:

```
Lines read:      12  (2 skipped as malformed)

By level:
  INFO       4
  WARN       2
  ERROR      4

Busiest minute:  2026-11-09 10:02  (4 lines)

Top errors:
    3x  Database connection failed
    1x  Timeout calling payment service
```

## Why I built this

I wanted something in the middle: not as trivial as a number guessing game, but
not so big that I couldn't explain every line. A log analyzer felt right because
every server, and also a Solana validator, writes huge log files, and the first
thing you do when something breaks is read the logs. Doing it by hand is slow,
so a small tool that counts and summarizes them is actually useful.

It is also a nice problem for learning Rust: you have to read a file, split
text, decide what is a "valid" line, and count things with `HashMap`, without
needing any external crates.

## How I described it to the agents

The main prompt is in [PROMPTS.md](PROMPTS.md). Both models got exactly the
same prompt.

I first described the idea to Claude Code in simple words: read a log file and
show counts per level, the busiest minute and the most common errors. Then we
turned that into one precise prompt, and I sent that same prompt to both models
so the comparison would be fair.

Things I put in the prompt on purpose:

- **The exact line format with an example.** Without it, each model would guess
  a different format.
- **What to do with bad lines.** I said "count them as skipped instead of
  crashing", because real log files always have some junk lines.
- **Standard library only and at most 200 lines**, so the code stays small
  enough for me to read fully.
- **A struct for the entry, an enum for the level, parsing in its own function,
  and unit tests.** I asked for this so the code would have a clear structure,
  and so the parser could be tested on its own.

## Models I tried and how they differed

Sonnet's version is kept for reference in
[comparison/sonnet_main.rs](comparison/sonnet_main.rs) (not compiled).

| | Claude Opus 5.5 (kept, `src/main.rs`) | Claude Sonnet 5.5 |
|---|---|---|
| Lines of Rust | 193 | 169 |
| Builds first try | yes | yes (one cleanup edit) |
| Tests | 4, all pass, incl. one over the whole `sample.log` | 4, all pass, parser only |
| Structure | `parse_line` → `analyze` → `print_report`, counts in a `Stats` struct | parsing in a function, everything else inside `main` |
| Reading the file | reads it all into memory | streams it line by line (`BufReader`) |
| Time validation | shape only (`99:99:99`) | also checks ranges (hour < 24, etc.) |
| Busiest minute | keyed by date + HH:MM | keyed by HH:MM only, so 10:02 on two different days is merged |
| Extra spaces between fields | handled everywhere | only after the level |
| Usage error exit code | 2 | 1 |

Both versions compiled on the first try and gave the same counts for
`sample.log`, so at first they looked the same. The differences only showed up
when I read the code side by side.

- **Structure:** Opus split the work into small functions (`parse_line`,
  `analyze`, `print_report`) and kept all counters in one `Stats` struct. Sonnet
  put almost everything inside `main`. Opus's version is longer but easier to
  follow and easier to test.
- **A real bug in Sonnet's version:** it groups the busiest minute by `HH:MM`
  only. If the log covers two days, `10:02` on Monday and `10:02` on Tuesday
  get counted together. Opus uses date + minute, which is correct.
- **Where Sonnet was better:** it reads the file line by line with `BufReader`
  instead of loading the whole file into memory, which is better for very big
  logs. It also checks that the time makes sense (hour below 24, minutes below
  60), while Opus only checks that the shape is `99:99:99`.

I kept the Opus version because of its clearer structure and because it has no
bug in the busiest-minute logic. The two things Sonnet did better are on my
list for the next version.

## How the program works

The program goes through four steps: read the file, parse each line, count,
and print.

**1. `main()`: start and error handling.** It reads the command-line
arguments. If there isn't exactly one file path, it prints a usage message and
exits with code 2. Then it reads the whole file into a `String` with
`fs::read_to_string`. If that fails, for example because the file doesn't
exist, it prints the error and exits with code 1. Non-zero exit codes matter
because scripts can then tell that something went wrong.

**2. `parse_line()`: turning text into a `LogEntry`.** This is the most
important function. It uses a small helper, `next_word`, to take the first
word off the line three times: date, time, level. Whatever is left is the
message. Then it checks everything:

- the date must look like `9999-99-99` and the time like `99:99:99`, where `9`
  means "any digit" (`matches_pattern` does this check);
- the level must be `INFO`, `WARN` or `ERROR`, and `Level::parse` returns
  `None` for anything else, like `DEBUG`;
- the message must not be empty.

If any check fails, the function returns `None`. It never panics, which is why
broken lines are just counted as skipped. Because `next_word` skips any amount
of whitespace, extra spaces like `INFO  Server started` still work.

I used an enum for the level instead of a plain string, so the compiler only
allows the three valid levels, and a struct, `LogEntry`, so each parsed line is
one clear value.

**3. `analyze()`: counting.** It loops over the lines, ignores empty ones, and
puts the results in a `Stats` struct:

- `total` and `skipped` counters;
- `per_level`: a `HashMap<Level, usize>` (that's why `Level` derives `Hash`);
- `per_minute`: a `HashMap` keyed by date + `HH:MM`;
- `errors`: a `HashMap` from error message to how many times it appeared.

`*map.entry(key).or_insert(0) += 1` is the usual Rust way to count: it creates
the key with 0 if it's new, then adds 1.

**4. `sorted_by_count()` and `print_report()`: output.** A `HashMap` has no
order, so `sorted_by_count` turns it into a `Vec` and sorts by count, highest
first. When two counts are equal, it sorts by the text, so the output is always
the same. The first item gives the busiest minute, and the first five give the
top errors. `print_report` uses format specifiers like `{:<6}` (left-align in 6
characters) and `{:>5}` (right-align in 5) to line up the columns.

## How I tested it

- Unit tests for the parser: valid line, extra spaces, malformed lines
  (garbage, unknown level, bad time, missing message), and a full count over
  `sample.log`.
- `sample.log` deliberately contains a broken line, an unknown `DEBUG` level and
  an empty line.
- Manually: no argument (exit code 2) and a missing file (exit code 1).

- The numbers are easy to check by hand: `sample.log` has 4 INFO, 2 WARN and
  4 ERROR lines, and 3 of the errors are "Database connection failed", which
  matches what the program prints.
- `cargo clippy` reports no warnings.
