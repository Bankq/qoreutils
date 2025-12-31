use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};

#[derive(Debug)]
struct Config {
    include_dot_files: bool,
}

impl Config {
    pub fn from(options: &ArgMatches) -> Self {
        Self {
            include_dot_files: options.get_flag("include_dot_files"),
        }
    }
}

fn main() -> Result<()> {
    let cmd = Command::new("ls")
        .arg(
            Arg::new("include_dot_files")
                .short('a')
                .action(ArgAction::SetTrue)
                .help("Do not ignore hidden files (files with names that start with '.')."),
        )
        .arg(Arg::new("paths").action(ArgAction::Append));

    let matches = cmd.get_matches();
    let config = Config::from(&matches);
    let dirs: Vec<&Path> = matches
        .get_many::<String>("paths")
        .map(|v| v.map(Path::new).collect())
        .unwrap_or_else(|| vec![Path::new(".")]);

    for dir in dirs {
        let entries = list_dir(dir, &config)?;
        println!("{}:", dir.display());
        for entry in entries {
            println!("{}", entry);
        }
    }

    Ok(())
}

fn list_dir(dir: &Path, config: &Config) -> Result<Vec<String>> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("cannot access '{}'", dir.display()))?;

    let mut names: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with('.') || config.include_dot_files {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_list_dir_basic() -> Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join("file1.txt"))?;
        File::create(dir.path().join("file2.txt"))?;
        File::create(dir.path().join("file3.txt"))?;

        let config = Config { include_dot_files: false };
        let entries = list_dir(dir.path(), &config)?;

        assert_eq!(entries.len(), 3);
        assert!(entries.contains(&"file1.txt".to_string()));
        assert!(entries.contains(&"file2.txt".to_string()));
        assert!(entries.contains(&"file3.txt".to_string()));

        Ok(())
    }

    #[test]
    fn test_list_dir_hides_dotfiles_by_default() -> Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join("visible.txt"))?;
        File::create(dir.path().join(".hidden"))?;

        let config = Config { include_dot_files: false };
        let entries = list_dir(dir.path(), &config)?;

        assert_eq!(entries.len(), 1);
        assert!(entries.contains(&"visible.txt".to_string()));
        assert!(!entries.contains(&".hidden".to_string()));

        Ok(())
    }

    #[test]
    fn test_list_dir_shows_dotfiles_with_flag() -> Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join("visible.txt"))?;
        File::create(dir.path().join(".hidden"))?;

        let config = Config { include_dot_files: true };
        let entries = list_dir(dir.path(), &config)?;

        assert_eq!(entries.len(), 2);
        assert!(entries.contains(&"visible.txt".to_string()));
        assert!(entries.contains(&".hidden".to_string()));

        Ok(())
    }

    #[test]
    fn test_list_dir_includes_subdirectories() -> Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join("file.txt"))?;
        fs::create_dir(dir.path().join("subdir"))?;

        let config = Config { include_dot_files: false };
        let entries = list_dir(dir.path(), &config)?;

        assert_eq!(entries.len(), 2);
        assert!(entries.contains(&"file.txt".to_string()));
        assert!(entries.contains(&"subdir".to_string()));

        Ok(())
    }

    #[test]
    fn test_list_dir_sorted() -> Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join("zebra.txt"))?;
        File::create(dir.path().join("apple.txt"))?;
        File::create(dir.path().join("mango.txt"))?;

        let config = Config { include_dot_files: false };
        let entries = list_dir(dir.path(), &config)?;

        assert_eq!(entries, vec!["apple.txt", "mango.txt", "zebra.txt"]);

        Ok(())
    }

    #[test]
    fn test_list_dir_nonexistent() {
        let config = Config { include_dot_files: false };
        let result = list_dir(Path::new("/nonexistent/path"), &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_list_dir_empty() -> Result<()> {
        let dir = tempdir()?;

        let config = Config { include_dot_files: false };
        let entries = list_dir(dir.path(), &config)?;

        assert!(entries.is_empty());

        Ok(())
    }
}
