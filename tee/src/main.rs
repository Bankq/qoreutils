use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use anyhow::{Context, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};

struct Config {
    append: bool,
    input: Box<dyn Read>,
}

impl Config {
    pub fn from(options: &ArgMatches) -> Result<Self> {
        let input: Box<dyn Read> = match options.get_one::<String>("input") {
            Some(path) => {
                let file = File::open(path)
                    .with_context(|| format!("failed to open input file '{path}'"))?;
                Box::new(file)
            }
            None => Box::new(io::stdin()),
        };

        Ok(Self {
            append: options.get_flag("append"),
            input,
        })
    }
}

struct TeeWriters {
    writers: Vec<Box<dyn Write>>,
}

impl Write for TeeWriters {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for writer in &mut self.writers {
            if let Err(e) = writer.write_all(buf) {
                eprintln!("{e}");
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        for writer in &mut self.writers {
            if let Err(e) = writer.flush() {
                eprintln!("{e}");
            }
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let cmd = Command::new("qtee")
        .arg(
            Arg::new("append")
                .short('a')
                .action(ArgAction::SetTrue)
                .help("Append the output to the files rather than overwriting them."),
        )
        .arg(
            Arg::new("ignore_sigint")
                .short('i')
                .action(ArgAction::SetTrue)
                .help("Ignore the SIGINT signal"),
        )
        .arg(Arg::new("paths").action(ArgAction::Append));

    let matches = cmd.get_matches();
    let mut config = Config::from(&matches)?;
    let paths: Vec<&Path> = matches
        .get_many::<String>("paths")
        .map(|v| v.map(Path::new).collect())
        .unwrap_or_default();

    tee(&paths, &mut config);
    Ok(())
}

fn tee(paths: &[&Path], config: &mut Config) {
    let mut writers: Vec<Box<dyn Write>> = paths
        .iter()
        .filter_map(|p| {
            let result = if config.append {
                File::options().create(true).append(true).open(p)
            } else {
                File::create(p)
            };

            match result {
                Ok(file) => Some(Box::new(file) as Box<dyn Write>),
                Err(e) => {
                    eprintln!("{}: {e}", p.display());
                    None
                }
            }
        })
        .collect();

    writers.push(Box::new(io::stdout()));

    let mut tee_writers = TeeWriters { writers };
    if let Err(e) = io::copy(&mut config.input, &mut tee_writers) {
        eprintln!("{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[test]
    fn test_tee_single_file() {
        let test_data = String::from("test data\n");
        let buff = Cursor::new(test_data.clone());

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let mut config = Config {
            append: false,
            input: Box::new(buff),
        };

        tee(&[path], &mut config);

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, test_data);
    }

    #[test]
    fn test_tee_append_mode() {
        let test_data = String::from("test data\n");
        let buff = Cursor::new(test_data.clone());

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write initial content
        std::fs::write(path, b"initial content\n").unwrap();

        let mut config = Config {
            append: true,
            input: Box::new(buff),
        };

        tee(&[path], &mut config);

        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.starts_with("initial content\n"));
        assert!(content.ends_with(&test_data));
    }

    #[test]
    fn test_tee_multiple_files() {
        let test_data = String::from("test data\n");
        let buff = Cursor::new(test_data.clone());
        let mut config = Config {
            append: false,
            input: Box::new(buff),
        };

        let temp_file1 = NamedTempFile::new().unwrap();
        let temp_file2 = NamedTempFile::new().unwrap();
        let path1 = temp_file1.path();
        let path2 = temp_file2.path();

        tee(&[path1, path2], &mut config);

        let content1 = std::fs::read_to_string(path1).unwrap();
        let content2 = std::fs::read_to_string(path2).unwrap();

        assert_eq!(content1, content2);
    }

    #[test]
    fn test_tee_nonexistent_path() {
        let test_data = String::from("test data\n");
        let buff = Cursor::new(test_data.clone());
        let mut config = Config {
            append: false,
            input: Box::new(buff),
        };
        let nonexistent_path = PathBuf::from("/nonexistent/path");
        let temp_file = NamedTempFile::new().unwrap();
        let valid_path = temp_file.path();

        // Should not panic when one path is invalid
        tee(&[&nonexistent_path, valid_path], &mut config);

        // Valid file should still be written to
        let content = std::fs::read_to_string(valid_path).unwrap();
        assert!(!content.is_empty());
    }
}
