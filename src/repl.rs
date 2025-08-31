use std::io::{Write, stdin, stdout};

use crate::{lexer::parse_into_tokens, parser::Parser};

pub fn run_repl() {
    let mut buffer = String::new();
    println!("Welcome to the slang REPL");
    loop {
        buffer.clear();
        print!("> ");
        _ = stdout().flush();
        stdin()
            .read_line(&mut buffer)
            .expect("Something went wrong");
        let tokens = parse_into_tokens(&buffer);
        if tokens.is_err() {
            println!("Error: {}", tokens.err().unwrap());
            continue;
        }
        let mut parser = Parser::new(tokens.unwrap());
        let expr = parser.parse();

        println!("{}", expr.eval());
    }
}
