mod lexer;
mod parser;

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
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

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
        content.as_ref().unwrap()
    );
    let content_str = &content.unwrap();
    let tokens = parse_into_tokens(content_str);
    let expr = parse(tokens);
    info!(
        "Expression {}; parsed: {}; result={}",
        content_str.trim(),
        expr.print(),
        expr.eval()
    );
    return Ok(());
}
