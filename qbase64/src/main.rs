use std::env;
use std::fs;
use std::io;

use clap::{Arg, ArgAction, ArgMatches, Command};

#[derive(Debug)]
enum Mode {
    ENCODE,
    DECODE,
}

struct Config {
    mode: Mode,
    input: Box<dyn io::Read>,
    output: Box<dyn io::Write>,
}

impl Config {
    pub fn from(options: &ArgMatches) -> Self {
        Self {
            mode: match options.get_flag("decode") {
                true => Mode::DECODE,
                false => Mode::ENCODE,
            },
            input: match options.get_one::<String>("input") {
                Some(path) => {
                    let mut file = fs::OpenOptions::new();
                    file.create(true);
                    match file.open(path) {
                        Ok(handle) => Box::new(handle) as Box<dyn io::Read>,
                        Err(e) => {
                            panic!("{}", e)
                        }
                    }
                }
                None => Box::new(io::stdin()) as Box<dyn io::Read>,
            },
            output: match options.get_one::<String>("output") {
                Some(path) => {
                    let mut file = fs::OpenOptions::new();
                    file.create(true);
                    match file.open(path) {
                        Ok(handle) => Box::new(handle) as Box<dyn io::Write>,
                        Err(e) => {
                            panic!("{}", e)
                        }
                    }
                }
                None => Box::new(io::stdout()) as Box<dyn io::Write>,
            },
        }
    }
}

fn main() {
    let cmd = Command::new("qbase64")
        .arg(
            Arg::new("decode")
                .short('d')
                .short_alias('D')
                .long("decode")
                .action(ArgAction::SetTrue)
                .help("Decode incoming Base64 stream into binary data."),
        )
        .arg(Arg::new("input").short('i').long("input"))
        .arg(Arg::new("output").short('o').long("output"));

    let args: Vec<String> = env::args().skip(1).collect();
    let matches = cmd.get_matches_from(&args);
    let config = Config::from(&matches);
    dbg!("{:?}", &config.mode);
    println!("Hello, world!");
}
