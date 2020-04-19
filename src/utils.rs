use serde_json::{Result, Value};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn lines_from_file(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("Unable to open file");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

pub fn get_parsed_json_file(filename: &str) -> Result<Value> {
    let lines = lines_from_file(filename);
    let config: Value = serde_json::from_str(&lines.join("\n"))?;
    Ok(config)
}
