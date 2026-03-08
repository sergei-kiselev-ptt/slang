use std::{
    collections::HashMap,
    fmt::Debug,
    io::{Write, stdin, stdout},
};

use crate::{
    lexer::{TokenType, parse_into_tokens},
    parser::{Expr, LiteralValue, Parser},
};

struct StoredFunc {
    params: Vec<String>,
    body: Vec<Expr>,
}

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
        println!("{}", expr.as_str());

        println!("{}", expr.eval(&mut env).print());
    }
}

fn eval_block(exprs: &[Expr], env: &mut Environment) -> Value {
    let mut result = Value::Number(0.0);
    for expr in exprs {
        result = expr.eval(env);
        if matches!(result, Value::Return(_)) {
            return result;
        }
    }
    result
}

impl Expr {
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
                    r @ Value::Return(_) => r,
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
            Expr::While { condition, body } => {
                loop {
                    match condition.eval(env) {
                        Value::Bool(true) => {
                            for expr in body {
                                let val = expr.eval(env);
                                if matches!(val, Value::Return(_)) {
                                    return val;
                                }
                            }
                        }
                        Value::Bool(false) => break,
                        other => {
                            return Value::Error(format!(
                                "while condition must be boolean, got {:?}",
                                other
                            ));
                        }
                    }
                }
                Value::Number(0.0)
            }
            Expr::Print { value } => {
                let val = value.eval(env);
                println!("{}", val.print());
                val
            }
            Expr::FuncDef {
                name, params, body, ..
            } => {
                let param_names = params.iter().map(|(tok, _)| tok.lexeme.clone()).collect();
                env.define_func(name.lexeme.clone(), param_names, body.clone());
                Value::Number(0.0)
            }
            Expr::Call { name, args } => {
                let func_name = &name.lexeme;
                let func = match env.functions.get(func_name) {
                    Some(f) => StoredFunc {
                        params: f.params.clone(),
                        body: f.body.clone(),
                    },
                    None => return Value::Error(format!("undefined function '{}'", func_name)),
                };

                if args.len() != func.params.len() {
                    return Value::Error(format!(
                        "function '{}' expects {} arguments, got {}",
                        func_name,
                        func.params.len(),
                        args.len()
                    ));
                }

                let mut call_env = Environment::new();
                // Copy functions so the callee can call other functions
                for (fname, f) in &env.functions {
                    call_env.functions.insert(
                        fname.clone(),
                        StoredFunc {
                            params: f.params.clone(),
                            body: f.body.clone(),
                        },
                    );
                }
                for (param_name, arg_expr) in func.params.iter().zip(args.iter()) {
                    let val = arg_expr.eval(env);
                    call_env.set(param_name.clone(), val);
                }

                let result = eval_block(&func.body, &mut call_env);
                match result {
                    Value::Return(inner) => *inner,
                    _ => Value::Error(format!("function '{}' has no return statement", func_name)),
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond = condition.eval(env);
                match cond {
                    Value::Bool(true) => eval_block(then_branch, env),
                    Value::Bool(false) => match else_branch {
                        Some(else_exprs) => eval_block(else_exprs, env),
                        None => Value::Number(0.0),
                    },
                    Value::Error(e) => Value::Error(e),
                    other => Value::Error(format!("if condition must be boolean, got {:?}", other)),
                }
            }
            Expr::Return { value } => {
                let val = value.eval(env);
                Value::Return(Box::new(val))
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
        if matches!(l, Value::Return(_)) {
            return l;
        }
        let r = right.eval(env);

        match (&l, &r) {
            (_, Value::Return(_)) => return r,
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
    functions: HashMap<String, StoredFunc>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }

    pub fn define_func(&mut self, name: String, params: Vec<String>, body: Vec<Expr>) {
        self.functions.insert(name, StoredFunc { params, body });
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool),
    // String(String),
    Error(String),
    Return(Box<Value>),
}

impl Value {
    pub fn print(&self) -> String {
        match self {
            Value::Number(num) => num.to_string(),
            Value::Error(msg) => msg.clone(),
            Value::Bool(bool) => bool.to_string(),
            Value::Return(inner) => inner.print(),
        }
    }
}
