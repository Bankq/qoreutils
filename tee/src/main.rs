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
            let mut file = fs::OpenOptions::new();
            file.create(true);
            if config.append {
                file.append(true);
            }
            match file.open(p) {
                Ok(handle) => Some(Box::new(handle) as Box<dyn io::Write>),
                Err(e) => {
                    eprintln!("{}: {e}", p.display());
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
