mod lexer;
mod parser;
mod qbe;

use std::fs::read_to_string;
use std::io::{Error, Write};
use std::process::exit;
use std::{env, fs::File};

use lexer::parse_into_tokens;
use log::{debug, error, info};
use parser::Parser;
use qbe::Compiler;
use simple_logger::SimpleLogger;
use time::macros::format_description;

fn main() {
    SimpleLogger::new()
        .with_timestamp_format(format_description!("[hour]:[minute]:[second]"))
        .with_level(log::LevelFilter::Info)
        .with_module_level("cranelift_jit", log::LevelFilter::Warn)
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
    let tokens = parse_into_tokens(content_str)?;
    let mut parser = Parser::new(tokens);
    let exprs = parser.parse_program();

    let mut compiler = Compiler::new();
    let text = compiler.compile(exprs).map_err(|e| {
        eprintln!("Compilation error: {}", e);
        Error::new(std::io::ErrorKind::Other, e.to_string())
    })?;
    println!("------------------");
    let mut file = File::create(".build/main.qbe")?;
    for line in text {
        println!("{}", line);
        write!(file, "{}\n", line)?;
    }
    println!("------------------");
    return Ok(());
}
