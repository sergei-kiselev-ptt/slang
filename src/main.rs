mod parser;
mod lexer;

use std::env;
use std::fs::read_to_string;
use std::io::Error;
use std::process::exit;

use lexer::parse_into_tokens;
use log::{debug, error, info};
use parser::parse;
use simple_logger::SimpleLogger;
use time::macros::format_description;

fn main() {
    SimpleLogger::new()
        .with_timestamp_format(format_description!("[hour]:[minute]:[second]"))
        .init()
        .unwrap();

    // let expr_str = "1+2/3";
    // let expr = parse(parse_into_tokens(expr_str));

    // info!("Expression {}, parsed: {}", expr_str, expr.print());

    // exit(0);
    
    let args = env::args().collect::<Vec<String>>();
    debug!("Args: [{:?}]", &args);

    let filename = &args[1];

    match process_file(filename) {
        Ok(_) => exit(0),
        Err(_) => exit(1),
    }
}

fn process_file(file_path: &str) -> Result<(), Error> {
    let content = read_to_string(file_path);
    if content.is_err() {
        let error = content.err().unwrap();
        error!("Couldn't read {} file: {}", file_path, error);
        return Err(error);
    }

    info!(
        "File content {}\n------------\n{}------------",
        file_path,
        content.unwrap()
    );

    return Ok(());
}
