use std::env;
use std::fs;
use std::path::Path;

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

fn main() {
    let cmd = Command::new("qls")
        .arg(
            Arg::new("include_dot_files")
                .short('a')
                .action(ArgAction::SetTrue)
                .help("Do not ingore hidden files (files with names that start with '.'). "),
        )
        .arg(Arg::new("paths").action(ArgAction::Append));

    let args: Vec<String> = env::args().skip(1).collect();
    let matches = cmd.get_matches_from(&args);
    let config = Config::from(&matches);
    dbg!(&config);
    let dirs = matches
        .get_many::<String>("paths")
        .map(|v| v.map(Path::new).collect())
        .unwrap_or(vec![Path::new(".")]);

    list(dirs, &config);
}

fn list(dirs: Vec<&Path>, config: &Config) {
    let multiple_input = dirs.len() > 1;
    for d in dirs {
        if multiple_input {
            println!("{}:", d.display());
        }

        match fs::read_dir(d) {
            Ok(res) => {
                res.for_each(|f| {
                    let fname = f.unwrap().file_name();
                    let fname = fname.to_str().unwrap();
                    if !fname.starts_with(".") || config.include_dot_files {
                        print!("{}\t", fname);
                    }
                });
                println!();
                if multiple_input {
                    println!();
                };
            }
            Err(e) => eprintln!("error {}", e),
        }
    }
}
