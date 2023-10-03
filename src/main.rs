use std::{
    fs::File,
    io::{BufRead, BufReader},
    process::exit,
};

use serde_json::Value;
use settings_guessr::{Document, FieldAccumulator};

fn main() {
    let mut reader = if let Some(path) = std::env::args().nth(1) {
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

    let mut accumulator = FieldAccumulator::new();

    let value: Value = serde_json::from_reader(&mut reader).unwrap();

    if let Some(values) = value.as_array() {
        for value in values {
            let document: &Document = value.as_object().expect("invalid document");
            accumulator.push(document);
        }
    } else if let Some(document) = value.as_object() {
        accumulator.push(document);
        while let Ok(document) = serde_json::from_reader::<_, Document>(&mut reader) {
            accumulator.push(&document);
        }
    }

    let settings = accumulator.finish();
    let settings = serde_json::to_string_pretty(&settings).unwrap();
    println!("{settings}");
}
