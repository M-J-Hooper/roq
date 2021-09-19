use rq::query::{Executable, Query};
use serde_json::Value;
use std::{
    env,
    io::{self, Read},
};

fn main() {
    let mut value_input = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    if let Err(e) = handle.read_to_string(&mut value_input) {
        eprintln!("Failed to read stdin: {:?}", e.kind());
    };

    let value: Value = match serde_json::from_str(&value_input) {
        Ok(v) => v,
        Err(e) => {
            return eprintln!(
                "Failed to parse document: {:?} at line {} column {}",
                e.classify(),
                e.line(),
                e.column()
            )
        }
    };

    let query_input = match env::args().nth(1) {
        Some(q) => q,
        None => return eprintln!("No query string provided"),
    };

    let query: Query = match query_input.parse() {
        Ok(q) => q,
        Err(e) => return eprintln!("Failed to parse query string: {}", e),
    };

    let results = match query.execute(&value) {
        Ok(r) => r,
        Err(e) => return eprintln!("Failed to execute query: {}", e),
    };

    if results.is_empty() {
        println!("No results")
    }
    for result in results {
        let pretty = serde_json::to_string_pretty(&result).unwrap();
        println!("{}", pretty);
    }
}
