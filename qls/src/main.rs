use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    for arg in args.iter() {
        let p = PathBuf::from(arg);
        println!("{}:", p.canonicalize().unwrap().display());
        match fs::read_dir(arg) {
            Ok(res) => res.for_each(|f| {
                println!("{}", f.unwrap().file_name().to_str().unwrap());
            }),
            Err(e) => eprintln!("error {}", e),
        };
        println!();
    }
}
