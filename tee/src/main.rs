use std::env;
use std::fs;
use std::io;
use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};

#[derive(Debug)]
struct Config {
    append: bool,
    // ignore_sigint: bool,
}

impl Config {
    pub fn from(options: &ArgMatches) -> Self {
        Self {
            append: options.get_flag("append"),
            // ignore_sigint: options.get_flag("ignore_sigint"),
        }
    }
}

struct TeeWriters {
    writers: Vec<Box<dyn io::Write>>,
}

impl io::Write for TeeWriters {
    // io::Write has two methods: write and flush
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writers.iter_mut().for_each(|w| {
            w.write_all(buf).unwrap_or_else(|e| {
                eprintln!("{e}");
            })
        });
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writers.iter_mut().for_each(|w| {
            w.flush().unwrap_or_else(|e| {
                eprintln!("{e}");
            });
        });
        Ok(())
    }
}

fn main() {
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

    let args: Vec<String> = env::args().skip(1).collect();
    let matches = cmd.get_matches_from(args);
    let config = Config::from(&matches);
    dbg!(&config);
    let paths = matches
        .get_many::<String>("paths")
        .map(|v| v.map(Path::new).collect())
        .unwrap_or(vec![]);

    tee(paths, &config);
}

fn tee(paths: Vec<&Path>, config: &Config) {
    let mut reader = io::stdin();
    let mut writers: Vec<Box<dyn io::Write>> = paths
        .into_iter()
        .filter_map(|p| {
            let mut open_options = fs::OpenOptions::new();
            open_options.create(true);
            open_options.write(true);
            if config.append {
                open_options.append(true);
            }
            match open_options.open(p) {
                Ok(handle) => Some(Box::new(handle) as Box<dyn io::Write>),
                Err(e) => {
                    eprintln!("Open error for {}: {e}", p.display());
                    None
                }
            }
        })
        .collect();
    writers.push(Box::new(io::stdout()));

    let mut tee_writers = TeeWriters { writers };
    if let Err(e) = io::copy(&mut reader, &mut tee_writers) {
        eprintln!("{e}");
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[test]
    fn test_tee_single_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let config = Config { append: false };

        // Call tee with our test data
        tee(vec![path], &config);

        // Verify file contents
        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, "test data\n");
    }

    #[test]
    fn test_tee_append_mode() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write initial content
        {
            let mut file = File::create(path).unwrap();
            file.write_all(b"initial content\n").unwrap();
        }

        // Test append mode
        let config = Config { append: true };

        // Call tee with append mode
        tee(vec![path], &config);

        // Verify file contents (should contain initial content + new content)
        let content = fs::read_to_string(path).unwrap();
        assert!(content.starts_with("initial content\n"));
    }

    #[test]
    fn test_tee_multiple_files() {
        let temp_file1 = NamedTempFile::new().unwrap();
        let temp_file2 = NamedTempFile::new().unwrap();
        let path1 = temp_file1.path();
        let path2 = temp_file2.path();

        let config = Config { append: false };

        // Call tee with multiple files
        tee(vec![path1, path2], &config);

        // Verify both files have the same content
        let content1 = fs::read_to_string(path1).unwrap();
        let content2 = fs::read_to_string(path2).unwrap();

        assert_eq!(content1, content2);
    }

    #[test]
    fn test_tee_nonexistent_path() {
        let nonexistent_path = PathBuf::from("/nonexistent/path");
        let temp_file = NamedTempFile::new().unwrap();
        let valid_path = temp_file.path();

        let config = Config { append: false };

        // Should not panic when one path is invalid
        tee(vec![&nonexistent_path, valid_path], &config);

        // Valid file should still be written to
        let content = fs::read_to_string(valid_path).unwrap();
        assert!(!content.is_empty());
    }
}
