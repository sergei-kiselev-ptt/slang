use std::{
    collections::HashMap,
    fmt::Debug,
    io::{Write, stdin, stdout},
};

use crate::{
    jit::{JIT, JitValue},
    lexer::{TokenType, parse_into_tokens},
    parser::{Expr, LiteralValue, Parser},
};

pub fn run_repl() {
    let mut buffer = String::new();
    println!("Welcome to the slang REPL (JIT-compiled)");
    let mut jit = JIT::new();
    loop {
        buffer.clear();
        print!("> ");
        _ = stdout().flush();
        stdin()
            .read_line(&mut buffer)
            .expect("Something went wrong");

        let trimmed = buffer.trim();
        if trimmed.is_empty() {
            continue;
        }

        let tokens = parse_into_tokens(trimmed);
        if tokens.is_err() {
            println!("Can't parse the expression: {}", tokens.err().unwrap());
            continue;
        }

        let mut parser = Parser::new(tokens.unwrap());
        let expr = parser.parse();

        match jit.eval(&expr) {
            Ok(value) => println!("{}", format_jit_value(&value)),
            Err(err) => println!("Error: {}", err),
        }
    }
}

fn format_jit_value(value: &JitValue) -> String {
    match value {
        JitValue::Number(n) => n.to_string(),
        JitValue::Bool(b) => b.to_string(),
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
                LiteralValue::Bool(b) => Value::Bool(*b),
            },
            Expr::Unary { operator, right } => match operator.token_type {
                TokenType::Minus => match right.eval(env) {
                    Value::Number(num) => Value::Number(-num),
                    e @ Value::Error(_) => e,
                    Value::Bool(_) => {
                        Value::Error(format!("Can't apply minus to boolean expression"))
                    }
                },
                TokenType::Plus => right.eval(env),
                TokenType::Bang => match right.eval(env) {
                    Value::Bool(b) => Value::Bool(!b),
                    other => Value::Error(format!(
                        "Can't apply logical not operator `!`, to non boolean values: {:?}",
                        other
                    )),
                },
                other => Value::Error(format!("Can't evaluate unary {:?}", other)),
            },
            Expr::Binary {
                left,
                operator,
                right,
            } => Self::eval_binary(operator.token_type.clone(), left, right, env),
            Expr::Variable { name } => env
                .get(&name.lexeme)
                .or(Some(&Value::Error(format!(
                    "No variable named {}",
                    &name.lexeme
                ))))
                .unwrap()
                .clone(),
            Expr::Assign { name, value } => {
                let val = value.eval(env);
                env.set(name.lexeme.clone(), val.clone());
                val
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond = condition.eval(env);
                match cond {
                    Value::Bool(true) => then_branch.eval(env),
                    Value::Bool(false) => match else_branch {
                        Some(else_expr) => else_expr.eval(env),
                        None => Value::Number(0.0), // Default when no else
                    },
                    Value::Error(e) => Value::Error(e),
                    other => Value::Error(format!("if condition must be boolean, got {:?}", other)),
                }
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
                TokenType::Greater => Value::Bool(a > b),
                TokenType::GreaterEqual => Value::Bool(a >= b),
                TokenType::Less => Value::Bool(a < b),
                TokenType::LessEqual => Value::Bool(a <= b),
                TokenType::EqualEqual => Value::Bool(a == b),
                TokenType::BangEqual => Value::Bool(a != b),
                other => Value::Error(format!("Unsupported binary operator: {:?}", other)),
            },

            (Value::Bool(a), Value::Bool(b)) => match tt {
                TokenType::EqualEqual => Value::Bool(a == b),
                TokenType::BangEqual => Value::Bool(a != b),
                TokenType::LogicalOr => Value::Bool(a || b),
                TokenType::LogicalAnd => Value::Bool(a && b),
                other => Value::Error(format!("Cannot apply {:?} to boolean values", other)),
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
    Bool(bool),
    // String(String),
    Error(String),
}

impl Value {
    pub fn print(&self) -> String {
        match self {
            Value::Number(num) => num.to_string(),
            Value::Error(msg) => msg.clone(),
            Value::Bool(bool) => bool.to_string(),
        }
    }
}
