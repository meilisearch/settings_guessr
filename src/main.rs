use std::{
    fs::File,
    io::{BufRead, BufReader},
    process::exit,
};

fn main() {
    let reader = if let Some(path) = std::env::args().nth(1) {
        let file = File::open(path).unwrap();
        Box::new(BufReader::new(file)) as Box<dyn BufRead>
    } else if atty::isnt(atty::Stream::Stdin) {
        let stdin = std::io::stdin();
        Box::new(BufReader::new(stdin)) as Box<dyn BufRead>
    } else {
        eprintln!(
            "Usage: pipe your documents in the command or give the path to a file as argument."
        );
        exit(2);
    };
}
