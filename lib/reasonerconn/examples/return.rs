use std::fs;
use eflint_json::json;

fn main() {
    let result = fs::read_to_string("./lib/reasonerconn/examples/example-result.json").unwrap();

    let result : json::Result = serde_json::from_str(&result).unwrap();

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}