use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

use anyhow::{Context, Result};
use clap::{Arg, ArgAction, Command};

#[derive(Debug)]
struct Config {
    lines: bool,
    words: bool,
    bytes: bool,
    chars: bool,
}

impl Config {
    fn from_matches(matches: &clap::ArgMatches) -> Self {
        let lines = matches.get_flag("lines");
        let words = matches.get_flag("words");
        let bytes = matches.get_flag("bytes");
        let chars = matches.get_flag("chars");

        // If no flags specified, default to lines + words + bytes
        if !lines && !words && !bytes && !chars {
            Self {
                lines: true,
                words: true,
                bytes: true,
                chars: false,
            }
        } else {
            Self {
                lines,
                words,
                bytes,
                chars,
            }
        }
    }
}

#[derive(Debug, Default)]
struct Counts {
    lines: usize,
    words: usize,
    bytes: usize,
    chars: usize,
}

fn main() -> Result<()> {
    let matches = Command::new("wc")
        .about("Print newline, word, and byte counts for each file")
        .arg(
            Arg::new("lines")
                .short('l')
                .long("lines")
                .action(ArgAction::SetTrue)
                .help("Print the newline counts"),
        )
        .arg(
            Arg::new("words")
                .short('w')
                .long("words")
                .action(ArgAction::SetTrue)
                .help("Print the word counts"),
        )
        .arg(
            Arg::new("bytes")
                .short('c')
                .long("bytes")
                .action(ArgAction::SetTrue)
                .help("Print the byte counts"),
        )
        .arg(
            Arg::new("chars")
                .short('m')
                .long("chars")
                .action(ArgAction::SetTrue)
                .help("Print the character counts"),
        )
        .arg(Arg::new("files").action(ArgAction::Append))
        .get_matches();

    let config = Config::from_matches(&matches);
    let files: Vec<&str> = matches
        .get_many::<String>("files")
        .map(|v| v.map(String::as_str).collect())
        .unwrap_or_else(|| vec!["-"]); // "-" means stdin

    let mut total = Counts::default();
    let show_total = files.len() > 1;

    for file in &files {
        let counts = if *file == "-" {
            count_stdin()?
        } else {
            count_file(Path::new(file))?
        };

        print_counts(&counts, file, &config);

        // Accumulate totals
        total.lines += counts.lines;
        total.words += counts.words;
        total.bytes += counts.bytes;
        total.chars += counts.chars;
    }

    if show_total {
        print_counts(&total, "total", &config);
    }

    Ok(())
}

fn count_file(path: &Path) -> Result<Counts> {
    let file = File::open(path).with_context(|| format!("cannot open '{}'", path.display()))?;
    let reader = BufReader::new(file);
    count_reader(reader)
}

fn count_stdin() -> Result<Counts> {
    let reader = BufReader::new(io::stdin());
    count_reader(reader)
}

fn count_reader<R: Read>(reader: BufReader<R>) -> Result<Counts> {
    let mut in_word = false;
    let mut counts = Counts::default();
    for byte in reader.bytes() {
        let b = byte?;
        counts.bytes += 1;
        if b == b'\n' {
            counts.lines += 1;
        }

        if b.is_ascii_whitespace() {
            in_word = false
        } else {
            if !in_word {
                counts.words += 1
            }
            in_word = true
        }

        // A byte is a continuation byte if top 2 bits are "10"
        if !(128u8..192u8).contains(&b) {
            counts.chars += 1;
        }
    }

    Ok(counts)
}

fn print_counts(counts: &Counts, name: &str, config: &Config) {
    if config.lines {
        print!("{:8}", counts.lines);
    }
    if config.words {
        print!("{:8}", counts.words);
    }
    if config.bytes {
        print!("{:8}", counts.bytes);
    }
    if config.chars {
        print!("{:8}", counts.chars);
    }
    println!(" {}", name);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn count_str(s: &str) -> Counts {
        let reader = BufReader::new(Cursor::new(s));
        count_reader(reader).unwrap()
    }

    #[test]
    fn test_empty_input() {
        let counts = count_str("");
        assert_eq!(counts.lines, 0);
        assert_eq!(counts.words, 0);
        assert_eq!(counts.bytes, 0);
        assert_eq!(counts.chars, 0);
    }

    #[test]
    fn test_single_word_no_newline() {
        let counts = count_str("hello");
        assert_eq!(counts.lines, 0);
        assert_eq!(counts.words, 1);
        assert_eq!(counts.bytes, 5);
        assert_eq!(counts.chars, 5);
    }

    #[test]
    fn test_single_word_with_newline() {
        let counts = count_str("hello\n");
        assert_eq!(counts.lines, 1);
        assert_eq!(counts.words, 1);
        assert_eq!(counts.bytes, 6);
        assert_eq!(counts.chars, 6);
    }

    #[test]
    fn test_multiple_words() {
        let counts = count_str("hello world\n");
        assert_eq!(counts.lines, 1);
        assert_eq!(counts.words, 2);
        assert_eq!(counts.bytes, 12);
        assert_eq!(counts.chars, 12);
    }

    #[test]
    fn test_multiple_lines() {
        let counts = count_str("line one\nline two\nline three\n");
        assert_eq!(counts.lines, 3);
        assert_eq!(counts.words, 6);
        assert_eq!(counts.bytes, 29);
        assert_eq!(counts.chars, 29);
    }

    #[test]
    fn test_varying_whitespace() {
        let counts = count_str("  hello   world  \n");
        assert_eq!(counts.lines, 1);
        assert_eq!(counts.words, 2); // multiple spaces don't create extra words
        assert_eq!(counts.bytes, 18);
        assert_eq!(counts.chars, 18);
    }

    #[test]
    fn test_only_whitespace() {
        let counts = count_str("   \n\t\n  ");
        assert_eq!(counts.lines, 2);
        assert_eq!(counts.words, 0);
        assert_eq!(counts.bytes, 8);
        assert_eq!(counts.chars, 8);
    }

    #[test]
    fn test_unicode_chars() {
        // 你好 = 2 characters, 6 bytes (3 bytes each)
        let counts = count_str("你好\n");
        assert_eq!(counts.lines, 1);
        assert_eq!(counts.words, 1);
        assert_eq!(counts.bytes, 7);  // 6 + newline
        assert_eq!(counts.chars, 3);  // 2 + newline
    }

    #[test]
    fn test_mixed_ascii_unicode() {
        // "hi 你好" = 5 chars (h, i, space, 你, 好), but more bytes
        let counts = count_str("hi 你好\n");
        assert_eq!(counts.lines, 1);
        assert_eq!(counts.words, 2);
        assert_eq!(counts.bytes, 10); // 2 + 1 + 6 + 1
        assert_eq!(counts.chars, 6);  // h, i, space, 你, 好, newline
    }

    #[test]
    fn test_emoji() {
        // 😀 = 1 character, 4 bytes
        let counts = count_str("😀\n");
        assert_eq!(counts.lines, 1);
        assert_eq!(counts.words, 1);
        assert_eq!(counts.bytes, 5);  // 4 + newline
        assert_eq!(counts.chars, 2);  // emoji + newline
    }

    #[test]
    fn test_tabs_count_as_whitespace() {
        let counts = count_str("a\tb\tc\n");
        assert_eq!(counts.words, 3);
    }

    #[test]
    fn test_no_trailing_newline() {
        let counts = count_str("no newline");
        assert_eq!(counts.lines, 0);
        assert_eq!(counts.words, 2);
        assert_eq!(counts.bytes, 10);
    }
}
