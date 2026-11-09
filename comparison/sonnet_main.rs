use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, PartialEq)]
struct Entry {
    date: String,
    time: String,
    level: Level,
    message: String,
}

impl Entry {
    /// Minute bucket in HH:MM form.
    fn minute(&self) -> &str {
        &self.time[..5]
    }
}

fn is_digits(s: &str, n: usize) -> bool {
    s.len() == n && s.bytes().all(|b| b.is_ascii_digit())
}

fn valid_date(s: &str) -> bool {
    let p: Vec<&str> = s.split('-').collect();
    p.len() == 3 && is_digits(p[0], 4) && is_digits(p[1], 2) && is_digits(p[2], 2)
}

fn valid_time(s: &str) -> bool {
    let p: Vec<&str> = s.split(':').collect();
    p.len() == 3 && p.iter().all(|x| is_digits(x, 2)) && p[0] < "24" && p[1] < "60" && p[2] < "60"
}

/// Parse a line like `2026-11-09 10:02:10 ERROR Database connection failed`.
fn parse_line(line: &str) -> Option<Entry> {
    let mut parts = line.trim().splitn(4, char::is_whitespace);
    let date = parts.next()?;
    let time = parts.next()?;
    let level = Level::parse(parts.next()?)?;
    let message = parts.next()?.trim();
    if !valid_date(date) || !valid_time(time) || message.is_empty() {
        return None;
    }
    Some(Entry { date: date.to_string(), time: time.to_string(), level, message: message.to_string() })
}

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("Usage: log-analyzer <logfile>\nExample: cargo run -- sample.log");
            process::exit(1);
        }
    };
    let file = File::open(&path).unwrap_or_else(|e| {
        eprintln!("Error: cannot open '{}': {}", path, e);
        process::exit(1);
    });

    let (mut info, mut warn, mut error, mut skipped, mut total) = (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut minutes: HashMap<String, usize> = HashMap::new();
    let mut errors: HashMap<String, usize> = HashMap::new();

    for line in BufReader::new(file).lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error: failed reading '{}': {}", path, e);
                process::exit(1);
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let Some(entry) = parse_line(&line) else {
            skipped += 1;
            continue;
        };
        total += 1;
        match entry.level {
            Level::Info => info += 1,
            Level::Warn => warn += 1,
            Level::Error => {
                error += 1;
                *errors.entry(entry.message.clone()).or_insert(0) += 1;
            }
        }
        *minutes.entry(entry.minute().to_string()).or_insert(0) += 1;
    }

    // Busiest minute (HH:MM); ties broken by earliest key.
    let busiest = minutes.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)));

    let mut top: Vec<(&String, &usize)> = errors.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

    println!("Log summary: {}", path);
    println!("{}", "=".repeat(40));
    println!("{:<18}{:>6}", "Valid lines:", total);
    println!("{:<18}{:>6}", "Skipped lines:", skipped);
    println!("\nLines per level");
    println!("  {:<16}{:>6}", "INFO", info);
    println!("  {:<16}{:>6}", "WARN", warn);
    println!("  {:<16}{:>6}", "ERROR", error);
    println!("\nBusiest minute");
    match busiest {
        Some((m, c)) => println!("  {} ({} lines)", m, c),
        None => println!("  n/a"),
    }
    println!("\nTop 5 ERROR messages");
    if top.is_empty() {
        println!("  none");
    }
    for (i, (msg, c)) in top.iter().take(5).enumerate() {
        println!("  {}. {:>3}x  {}", i + 1, c, msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_line() {
        let e = parse_line("2026-11-09 10:02:10 ERROR Database connection failed").unwrap();
        assert_eq!(e.level, Level::Error);
        assert_eq!(e.minute(), "10:02");
        assert_eq!(e.message, "Database connection failed");
    }

    #[test]
    fn parses_extra_spaces_after_level() {
        let e = parse_line("2026-11-09 10:01:12 INFO  Server started").unwrap();
        assert_eq!(e.level, Level::Info);
        assert_eq!(e.message, "Server started");
    }

    #[test]
    fn rejects_bad_level() {
        assert!(parse_line("2026-11-09 10:02:10 DEBUG hi").is_none());
    }

    #[test]
    fn rejects_bad_time_and_garbage() {
        assert!(parse_line("2026-11-09 25:02:10 INFO hi").is_none());
        assert!(parse_line("garbage").is_none());
        assert!(parse_line("2026-11-09 10:02:10 INFO").is_none());
    }
}
