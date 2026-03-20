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

    let content_str = content.unwrap();
    info!(
        "File content {}\n------------\n{}------------",
        file_path, &content_str
    );
    let tokens = parse_into_tokens(&content_str)?;
    let mut parser = Parser::new(tokens);
    let exprs = parser.parse_program().map_err(|e| {
        print_parse_error(file_path, &content_str, &e);
        Error::other(e.to_string())
    })?;

    let mut compiler = Compiler::new();
    let text = compiler.compile(exprs).map_err(|e| {
        if let Some(qbe_err) = e.downcast_ref::<qbe::QbeError>() {
            print_compile_error(file_path, &content_str, qbe_err.message(), qbe_err.span());
        } else {
            eprintln!("error: {}", e);
        }
        Error::other(e.to_string())
    })?;
    println!("------------------");
    let mut file = File::create(".build/main.qbe")?;
    for line in text {
        println!("{}", line);
        writeln!(file, "{}", line)?;
    }
    println!("------------------");
    Ok(())
}

fn print_parse_error(file_path: &str, source: &str, err: &parser::ParseError) {
    print_compile_error(file_path, source, &err.message, Some(err.span));
}

fn print_compile_error(file_path: &str, source: &str, message: &str, span: Option<lexer::Span>) {
    if let Some(span) = span {
        eprintln!(
            "error: {} [{}:{}:{}]",
            message, file_path, span.line, span.col
        );
        if span.line > 0
            && let Some(line) = source.lines().nth(span.line - 1)
        {
            eprintln!("  {} | {}", span.line, line);
            let padding = format!("{}", span.line).len() + 3 + span.col.saturating_sub(1);
            let underline_len = span.len.max(1);
            eprintln!("{}{}", " ".repeat(padding), "^".repeat(underline_len));
        }
    } else {
        eprintln!("error: {}", message);
    }
}
