use std::fs;
use std::io;

use anyhow::{anyhow, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};

const B64TABLE: &'static [char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', '+', '/',
];

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
                    file.read(true);
                    match file.open(path) {
                        Ok(handle) => Box::new(handle) as Box<dyn io::Read>,
                        Err(e) => {
                            panic!("{path}:{e}")
                        }
                    }
                }
                None => Box::new(io::stdin()) as Box<dyn io::Read>,
            },
            output: match options.get_one::<String>("output") {
                Some(path) => {
                    let mut file = fs::OpenOptions::new();
                    file.create(true);
                    file.write(true);
                    match file.open(path) {
                        Ok(handle) => Box::new(handle) as Box<dyn io::Write>,
                        Err(e) => {
                            panic!("{path}:{e}")
                        }
                    }
                }
                None => Box::new(io::stdout()) as Box<dyn io::Write>,
            },
        }
    }
}

fn main() {
    let matches = Command::new("qbase64")
        .arg(
            Arg::new("decode")
                .short('d')
                .short_alias('D')
                .long("decode")
                .action(ArgAction::SetTrue)
                .help("Decode incoming Base64 stream into binary data."),
        )
        .arg(Arg::new("input").short('i').long("input"))
        .arg(Arg::new("output").short('o').long("output"))
        .get_matches();
    let config = Config::from(&matches);

    match config.mode {
        Mode::ENCODE => encode(config.input, config.output),
        Mode::DECODE => decode(config.input, config.output),
    }
    .unwrap();
}

fn decode(mut input: impl io::Read, mut output: impl io::Write) -> Result<()> {
    let mut buf = Vec::new();
    input.read_to_end(&mut buf)?;
    let chunks = &buf[..].chunks(4);
    chunks.to_owned().try_for_each(|chunk| {
        let mut encoded: u32 = 0;
        let mut pad_count = 0;
        for (i, c) in chunk.iter().enumerate() {
            if *c == ('=' as u8) {
                pad_count += 1;
                continue;
            }

            if let Some(v) = B64TABLE.iter().position(|&x| (x as u8) == *c) {
                encoded = encoded | (v << (18 - i * 6)) as u32;
            } else {
                return Err(anyhow!("invalid input"));
            }
        }

        for i in 0..(3 - pad_count) {
            let shift = 16 - i * 8;
            let mask: u32 = 255 << shift;
            let v = (encoded & mask) >> shift;
            output.write(&[v as u8])?;
        }
        Ok(())
    })?;
    Ok(())
}

fn encode(mut input: impl io::Read, mut output: impl io::Write) -> Result<()> {
    let mask: u32 = 63;
    let mut buf = Vec::new();
    input.read_to_end(&mut buf)?;
    let chunks = &buf[..].chunks(3);
    chunks.to_owned().for_each(|chunk| {
        let l = chunk.len();
        let mut b3: u32 = 0; // higher 8bits ignored
        for i in 0..l {
            let shift = 16 - i * 8;
            b3 = b3 | (chunk[i] as u32) << shift;
        }
        let mut encoded = Vec::new();
        for i in 0..=l {
            let shift = 18 - i * 6;
            let sextet = (b3 & (mask << shift)) >> shift;
            encoded.push(B64TABLE[sextet as usize] as u8);
        }
        for _ in l..3 {
            encoded.push('=' as u8);
        }

        if let Err(e) = output.write(&encoded) {
            eprintln!("{e}");
        }
    });

    Ok(())
}
