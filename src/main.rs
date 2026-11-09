use std::collections::HashMap;
use std::env;
use std::fs;
use std::process;

/// Severity of a log line. Only these three are accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn parse(s: &str) -> Option<Level> {
        match s {
            "INFO" => Some(Level::Info),
            "WARN" => Some(Level::Warn),
            "ERROR" => Some(Level::Error),
            _ => None,
        }
    }
}

/// One successfully parsed log line.
#[derive(Debug, PartialEq)]
struct LogEntry {
    date: String,
    time: String,
    level: Level,
    message: String,
}

impl LogEntry {
    /// "HH:MM" part of the time, used to group lines per minute.
    fn minute(&self) -> &str {
        &self.time[..5]
    }
}

/// Checks that `s` has exactly the given shape, where '9' means "any digit"
/// and every other character must match literally.
fn matches_pattern(s: &str, pattern: &str) -> bool {
    s.len() == pattern.len()
        && s.chars().zip(pattern.chars()).all(|(c, p)| match p {
            '9' => c.is_ascii_digit(),
            _ => c == p,
        })
}

/// Splits off the first whitespace-separated word, returning (word, rest).
fn next_word(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    match s.find(char::is_whitespace) {
        Some(i) => Some((&s[..i], &s[i..])),
        None => Some((s, "")),
    }
}

/// Parses "YYYY-MM-DD HH:MM:SS LEVEL message". Returns None if the line
/// does not have that shape, so the caller can count it as skipped.
fn parse_line(line: &str) -> Option<LogEntry> {
    let (date, rest) = next_word(line)?;
    let (time, rest) = next_word(rest)?;
    let (level, rest) = next_word(rest)?;
    let message = rest.trim();

    if !matches_pattern(date, "9999-99-99") || !matches_pattern(time, "99:99:99") {
        return None;
    }
    if message.is_empty() {
        return None;
    }
    Some(LogEntry {
        date: date.to_string(),
        time: time.to_string(),
        level: Level::parse(level)?,
        message: message.to_string(),
    })
}

/// Everything we count while reading the file.
#[derive(Default)]
struct Stats {
    total: usize,
    skipped: usize,
    per_level: HashMap<Level, usize>,
    per_minute: HashMap<String, usize>,
    errors: HashMap<String, usize>,
}

fn analyze(contents: &str) -> Stats {
    let mut stats = Stats::default();
    for line in contents.lines().filter(|l| !l.trim().is_empty()) {
        stats.total += 1;
        let Some(entry) = parse_line(line) else {
            stats.skipped += 1;
            continue;
        };
        *stats.per_level.entry(entry.level).or_insert(0) += 1;
        let minute = format!("{} {}", entry.date, entry.minute());
        *stats.per_minute.entry(minute).or_insert(0) += 1;
        if entry.level == Level::Error {
            *stats.errors.entry(entry.message).or_insert(0) += 1;
        }
    }
    stats
}

/// Sorts a count map by count (highest first), then by key so ties are stable.
fn sorted_by_count(map: &HashMap<String, usize>) -> Vec<(&String, &usize)> {
    let mut items: Vec<_> = map.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    items
}

fn print_report(stats: &Stats) {
    println!("Lines read:      {}  ({} skipped as malformed)", stats.total, stats.skipped);

    println!("\nBy level:");
    for (name, level) in [("INFO", Level::Info), ("WARN", Level::Warn), ("ERROR", Level::Error)] {
        let count = stats.per_level.get(&level).copied().unwrap_or(0);
        println!("  {:<6} {:>5}", name, count);
    }

    match sorted_by_count(&stats.per_minute).first() {
        Some((minute, count)) => println!("\nBusiest minute:  {}  ({} lines)", minute, count),
        None => println!("\nBusiest minute:  none"),
    }

    println!("\nTop errors:");
    let errors = sorted_by_count(&stats.errors);
    if errors.is_empty() {
        println!("  none");
    }
    for (message, count) in errors.into_iter().take(5) {
        println!("  {:>3}x  {}", count, message);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <path-to-log-file>", args[0]);
        process::exit(2);
    }
    let contents = match fs::read_to_string(&args[1]) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: cannot read '{}': {}", args[1], e);
            process::exit(1);
        }
    };
    print_report(&analyze(&contents));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_valid_line() {
        let e = parse_line("2026-11-09 10:02:10 ERROR Database connection failed").unwrap();
        assert_eq!(e.level, Level::Error);
        assert_eq!(e.minute(), "10:02");
        assert_eq!(e.message, "Database connection failed");
    }

    #[test]
    fn tolerates_extra_spaces() {
        let e = parse_line("2026-11-09 10:01:12 INFO  Server started").unwrap();
        assert_eq!(e.message, "Server started");
    }

    #[test]
    fn rejects_malformed_lines() {
        assert!(parse_line("this line is broken").is_none());
        assert!(parse_line("2026-11-09 10:02:44 DEBUG unknown level").is_none());
        assert!(parse_line("2026-11-09 10:02 INFO bad time").is_none());
        assert!(parse_line("2026-11-09 10:02:10 ERROR").is_none());
    }

    #[test]
    fn counts_sample_log() {
        let stats = analyze(include_str!("../sample.log"));
        assert_eq!(stats.total, 12);
        assert_eq!(stats.skipped, 2);
        assert_eq!(stats.errors["Database connection failed"], 3);
    }
}
