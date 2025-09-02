use std::{
    collections::HashMap,
    fmt::Debug,
    io::{Write, stdin, stdout},
};

use crate::{
    lexer::{TokenType, parse_into_tokens},
    parser::{Expr, LiteralValue, Parser},
};

pub fn run_repl() {
    let mut buffer = String::new();
    println!("Welcome to the slang REPL");
    let mut env = Environment::new();
    loop {
        buffer.clear();
        print!("> ");
        _ = stdout().flush();
        stdin()
            .read_line(&mut buffer)
            .expect("Something went wrong");
        let tokens = parse_into_tokens(&buffer);
        if tokens.is_err() {
            println!("Can't parse the expression: {}", tokens.err().unwrap());
            continue;
        }
        let mut parser = Parser::new(tokens.unwrap());
        let expr = parser.parse();

        println!("{}", expr.eval(&mut env).print());
    }
}

impl Expr {
    pub fn eval_no_env(&self) -> Value {
        self.eval(&mut Environment::new())
    }

    fn eval(&self, env: &mut Environment) -> Value {
        match self {
            Expr::Literal(literal_value) => match literal_value {
                LiteralValue::Number(num) => Value::Number(*num),
                LiteralValue::String(_) => panic!("Can't process strings atm"),
            },
            Expr::Unary { operator, right } => match operator.token_type {
                TokenType::Minus => match right.eval(env) {
                    Value::Number(num) => Value::Number(-num),
                    e @ Value::Error(_) => e,
                },
                TokenType::Plus => right.eval(env),
                other => Value::Error(format!("Can't evaluate unary {:?}", other)),
            },
            Expr::Binary {
                left,
                operator,
                right,
            } => Self::eval_binary(operator.token_type.clone(), left, right, env),
            Expr::Variable { name } => env.get(&name.lexeme).unwrap().clone(),
            Expr::Assign { name, value } => {
                let val = value.eval(env);
                env.set(name.lexeme.clone(), val.clone());
                val
            }
        }
    }

    fn eval_binary(
        tt: TokenType,
        left: &Box<Expr>,
        right: &Box<Expr>,
        env: &mut Environment,
    ) -> Value {
        let l = left.eval(env);
        let r = right.eval(env);

        match (&l, &r) {
            (Value::Error(l_msg), Value::Error(r_msg)) => {
                return Value::Error(format!(
                    "Error when evaluating left part: {}\nError when evaluating right part: {}",
                    l_msg, r_msg
                ));
            }
            (Value::Error(msg), r) => {
                return Value::Error(format!(
                    "Error when evaluating left part: {}\nRight part is: {}",
                    msg,
                    r.print()
                ));
            }
            (l, Value::Error(msg)) => {
                return Value::Error(format!(
                    "Error when evaluating right part: {}\nLeft part is: {}",
                    msg,
                    l.print()
                ));
            }
            _ => {}
        }

        match (l, r) {
            (Value::Number(a), Value::Number(b)) => match tt {
                TokenType::Plus => Value::Number(a + b),
                TokenType::Minus => Value::Number(a - b),
                TokenType::Star => Value::Number(a * b),
                TokenType::Slash => {
                    if b == 0.0 {
                        Value::Error("Division by zero".to_string())
                    } else {
                        Value::Number(a / b)
                    }
                }
                other => Value::Error(format!("Unsupported binary operator: {:?}", other)),
            },

            (left_val, right_val) => Value::Error(format!(
                "Cannot apply {:?} to values {:?} and {:?}",
                tt, left_val, right_val
            )),
        }
    }
}

struct Environment {
    values: HashMap<String, Value>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    // Bool(bool),
    // String(String),
    Error(String),
}

impl Value {
    pub fn print(&self) -> String {
        match self {
            Value::Number(num) => num.to_string(),
            Value::Error(msg) => msg.clone(),
        }
    }
}
