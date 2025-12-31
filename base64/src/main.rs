use std::fs::File;
use std::io::{self, Read, Write};

use anyhow::{bail, Context, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};

const B64TABLE: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', '+', '/',
];

#[derive(Debug)]
enum Mode {
    Encode,
    Decode,
}

struct Config {
    mode: Mode,
    input: Box<dyn io::Read>,
    output: Box<dyn io::Write>,
}

impl Config {
    pub fn from(options: &ArgMatches) -> Result<Self> {
        let input: Box<dyn io::Read> = match options.get_one::<String>("input") {
            Some(path) => {
                let file = File::open(path)
                    .with_context(|| format!("failed to open input file '{path}'"))?;
                Box::new(file)
            }
            None => Box::new(io::stdin()),
        };

        let output: Box<dyn io::Write> = match options.get_one::<String>("output") {
            Some(path) => {
                let file = File::create(path)
                    .with_context(|| format!("failed to create output file '{path}'"))?;
                Box::new(file)
            }
            None => Box::new(io::stdout()),
        };

        let mode = if options.get_flag("decode") {
            Mode::Decode
        } else {
            Mode::Encode
        };

        Ok(Self { mode, input, output })
    }
}

fn main() -> Result<()> {
    let matches = Command::new("base64")
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

    let mut config = Config::from(&matches)?;

    let mut input = Vec::new();
    config
        .input
        .read_to_end(&mut input)
        .context("failed to read input")?;

    let output = match config.mode {
        Mode::Encode => encode(&input),
        Mode::Decode => decode(&input),
    }?;

    config
        .output
        .write_all(&output)
        .context("failed to write output")?;

    Ok(())
}

fn decode(input: &[u8]) -> Result<Vec<u8>> {
    if input.len() % 4 != 0 {
        bail!("input length is not a multiple of 4");
    }

    let mut decoded = Vec::with_capacity((input.len() / 4) * 3);
    for chunk in input.chunks(4) {
        let mut encoded: u32 = 0;
        let mut pad_count = 0;
        for (i, c) in chunk.iter().enumerate() {
            if *c == b'=' {
                pad_count += 1;
                continue;
            }

            if let Some(v) = B64TABLE.iter().position(|&x| (x as u8) == *c) {
                encoded |= (v << (18 - i * 6)) as u32;
            } else {
                bail!("invalid character '{}' in input", *c as char);
            }
        }

        for i in 0..(3 - pad_count) {
            let shift = 16 - i * 8;
            let mask: u32 = 255 << shift;
            let v = (encoded & mask) >> shift;
            decoded.push(v as u8);
        }
    }
    Ok(decoded)
}

fn encode(input: &[u8]) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    for chunk in input.chunks(3) {
        let l = chunk.len();
        let mut b3: u32 = 0; // higher 8bits ignored
        for (i, &c) in chunk.iter().enumerate() {
            b3 |= (c as u32) << (16 - i * 8);
        }
        for i in 0..=l {
            let shift = 18 - i * 6;
            let sextet = (b3 >> shift) & 0x3F;
            encoded.push(B64TABLE[sextet as usize] as u8);
        }
        encoded.resize(encoded.len() + 3 - l, b'=');
    }
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() -> Result<()> {
        let input = "HELLO".as_bytes().to_vec();
        let expected = "SEVMTE8=".as_bytes().to_vec();
        assert_eq!(expected, encode(&input)?);
        Ok(())
    }

    #[test]
    fn test_decode() -> Result<()> {
        let expected = "HELLO".as_bytes().to_vec();
        let input = "SEVMTE8=".as_bytes().to_vec();
        assert_eq!(expected, decode(&input)?);
        Ok(())
    }
}
