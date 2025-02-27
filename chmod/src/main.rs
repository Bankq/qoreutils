use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use clap::{Arg, ArgAction, Command};

fn main() -> io::Result<()> {
    let cmd = Command::new("chmod")
        .about("Change file mode bits")
        .arg(
            Arg::new("recursive")
                .short('R')
                .long("recursive")
                .action(ArgAction::SetTrue)
                .help("Change files and directories recursively"),
        )
        .arg(
            Arg::new("mode")
                .required(true)
                .help("The file mode bits to apply (octal or symbolic)"),
        )
        .arg(
            Arg::new("files")
                .required(true)
                .action(ArgAction::Append)
                .help("File(s) to modify"),
        );

    let matches = cmd.get_matches();
    let recursive = matches.get_flag("recursive");
    let mode_str = matches.get_one::<String>("mode").unwrap();
    let files: Vec<&String> = matches.get_many("files").unwrap().collect();

    // Parse the mode - supporting only octal modes for simplicity
    let mode = match parse_mode(mode_str) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {}", e);
            return Ok(());
        }
    };

    for file in files {
        let path = Path::new(file);
        if recursive && path.is_dir() {
            chmod_recursive(path, mode)?;
        } else {
            chmod_file(path, mode)?;
        }
    }

    Ok(())
}

fn parse_mode(mode_str: &str) -> Result<u32, String> {
    // Check for symbolic mode format
    if mode_str.contains('+') || mode_str.contains('-') || mode_str.contains('=') {
        let current_mode = if mode_str.contains('=') {
            // When using '=', we start with a base of 0
            0
        } else {
            // For '+' and '-', we start with a base of current permissions (using a typical default)
            0o666
        };

        let mut result_mode = current_mode;

        // Process each symbolic operation
        for part in mode_str.split(',') {
            let mut who = 0;
            let mut permissions = 0;

            // Parse the symbolic mode (e.g. u+rw, g-x, a=r)
            let mut chars = part.chars().peekable();

            // Parse who (u, g, o, a)
            while let Some(&c) = chars.peek() {
                match c {
                    'u' => who |= 0o700,
                    'g' => who |= 0o070,
                    'o' => who |= 0o007,
                    'a' => who |= 0o777,
                    _ => break,
                }
                chars.next();
            }

            // If no who specified, default to all
            if who == 0 {
                who = 0o777;
            }

            // Parse operation (+, -, =)
            let operation = match chars.next() {
                Some(c) if c == '+' || c == '-' || c == '=' => c,
                Some(_) => return Err(format!("Invalid operation in symbolic mode: {}", mode_str)),
                None => return Err(format!("Invalid symbolic mode format: {}", mode_str)),
            };

            // Parse permissions (r, w, x)
            for c in chars {
                match c {
                    'r' => permissions |= 0o444 & who,
                    'w' => permissions |= 0o222 & who,
                    'x' => permissions |= 0o111 & who,
                    _ => return Err(format!("Invalid permission character: {}", c)),
                }
            }

            // Apply the operation
            match operation {
                '+' => result_mode |= permissions,
                '-' => result_mode &= !permissions,
                '=' => {
                    // Clear the bits for specified users
                    result_mode &= !who;
                    // Set the new permissions
                    result_mode |= permissions;
                }
                _ => unreachable!(),
            }
        }

        return Ok(result_mode);
    }

    if mode_str.starts_with('0') {
        // Octal mode
        match u32::from_str_radix(&mode_str[1..], 8) {
            Ok(mode) => Ok(mode),
            Err(_) => Err(format!("Invalid octal mode: {}", mode_str)),
        }
    } else {
        // For simplicity, just try to parse as octal without the leading 0
        match u32::from_str_radix(mode_str, 8) {
            Ok(mode) => Ok(mode),
            Err(_) => Err(format!(
                "Invalid mode: {}. This implementation only supports octal modes.",
                mode_str
            )),
        }
    }
}

fn chmod_file(path: &Path, mode: u32) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(mode);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn chmod_recursive(dir: &Path, mode: u32) -> io::Result<()> {
    // First change the permissions of the directory itself
    chmod_file(dir, mode)?;
    // Only process contents if it's a directory
    if dir.is_dir() {
        // Recursively process directory contents
        for entry in fs::read_dir(dir)? {
            chmod_recursive(&entry?.path(), mode)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    #[test]
    fn test_parse_octal_mode() {
        assert_eq!(parse_mode("644").unwrap(), 0o644);
        assert_eq!(parse_mode("0755").unwrap(), 0o755);
        assert_eq!(parse_mode("0777").unwrap(), 0o777);
        assert!(parse_mode("abc").is_err());
    }

    #[test]
    fn test_parse_symbolic_mode() {
        assert_eq!(parse_mode("u+r").unwrap() & 0o400, 0o400);
        assert_eq!(parse_mode("g+w").unwrap() & 0o020, 0o020);
        assert_eq!(parse_mode("o+x").unwrap() & 0o001, 0o001);
        assert_eq!(parse_mode("a+rwx").unwrap(), 0o777);
        assert_eq!(parse_mode("u-x").unwrap() & 0o100, 0);
        assert_eq!(parse_mode("u=rw").unwrap() & 0o700, 0o600);
    }

    #[test]
    fn test_chmod_file() -> io::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test-file.txt");

        // Create a test file
        let mut file = File::create(&file_path)?;
        file.write_all(b"test content")?;

        // Test chmod with octal mode
        chmod_file(&file_path, 0o644)?;
        let metadata = fs::metadata(&file_path)?;
        assert_eq!(metadata.permissions().mode() & 0o777, 0o644);

        // Test chmod with another mode
        chmod_file(&file_path, 0o755)?;
        let metadata = fs::metadata(&file_path)?;
        assert_eq!(metadata.permissions().mode() & 0o777, 0o755);

        Ok(())
    }

    #[test]
    fn test_chmod_recursive() -> io::Result<()> {
        let dir = tempdir()?;
        let subdir_path = dir.path().join("subdir");
        let file_path = dir.path().join("test-file.txt");
        let subfile_path = subdir_path.join("subfile.txt");

        // Create directory structure
        fs::create_dir(&subdir_path)?;
        File::create(&file_path)?.write_all(b"test content")?;
        File::create(&subfile_path)?.write_all(b"test content")?;

        // Test recursive chmod
        chmod_recursive(dir.path(), 0o755)?;

        // Check permissions
        assert_eq!(
            fs::metadata(&dir.path())?.permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            fs::metadata(&subdir_path)?.permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            fs::metadata(&file_path)?.permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            fs::metadata(&subfile_path)?.permissions().mode() & 0o777,
            0o755
        );

        Ok(())
    }
}
