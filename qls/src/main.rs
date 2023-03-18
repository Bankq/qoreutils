use std::fs;

fn main() {
    match fs::read_dir("./") {
        Ok(res) => res.for_each(|f| {
            println!("{}", f.unwrap().path().display());
        }),
        Err(e) => eprintln!("error {}", e),
    };
}
