use anyhow::Result;
use std::fmt::Debug;
use thiserror::Error;

use crate::{
    lexer::TokenType,
    parser::{Expr, LiteralValue},
};

pub struct Compiler {
    counter: usize,
}

#[derive(Debug, PartialEq)]
enum ResType {
    Number, // QBE type 'd' (double)
    Bool,   // QBE type 'w' (word)
}

impl Compiler {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    fn next_tmp(&mut self) -> String {
        let tmp = format!("%t{}", self.counter);
        self.counter += 1;
        tmp
    }

    pub fn compile(&mut self, expr: Expr) -> Vec<String> {
        let mut out = vec![];
        out.push("export function w $main() {".to_string());
        out.push("@start".to_string());
        let (_tmp, _ty, instructions) = self.compile_expr(expr).unwrap();
        for line in instructions {
            out.push(format!("  {}", line));
        }
        out.push("  ret 0".to_string());
        out.push("}".to_string());
        out
    }

    fn compile_expr(&mut self, expr: Expr) -> Result<(String, ResType, Vec<String>)> {
        match expr {
            Expr::Literal(literal_value) => {
                let tmp = self.next_tmp();
                match literal_value {
                    LiteralValue::Number(n) => Ok((
                        tmp.clone(),
                        ResType::Number,
                        vec![format!("{} =d copy d_{}", tmp, n)],
                    )),
                    LiteralValue::Bool(b) => {
                        let val = if b { 1 } else { 0 };
                        Ok((
                            tmp.clone(),
                            ResType::Bool,
                            vec![format!("{} =w copy {}", tmp, val)],
                        ))
                    }
                    LiteralValue::String(_) => todo!("strings not yet supported"),
                }
            }

            Expr::Unary { operator, right } => {
                let (right_tmp, _right_ty, mut instructions) = self.compile_expr(*right)?;
                let tmp = self.next_tmp();
                match operator.token_type {
                    TokenType::Minus => {
                        instructions.push(format!("{} =d neg {}", tmp, right_tmp));
                        Ok((tmp, ResType::Number, instructions))
                    }
                    TokenType::Plus => {
                        instructions.push(format!("{} =d copy {}", tmp, right_tmp));
                        Ok((tmp, ResType::Number, instructions))
                    }
                    TokenType::Bang => {
                        instructions.push(format!("{} =w ceqw {}, 0", tmp, right_tmp));
                        Ok((tmp, ResType::Bool, instructions))
                    }
                    _ => panic!("Unknown unary operator: {:?}", operator.token_type),
                }
            }

            Expr::Binary {
                left,
                operator,
                right,
            } => {
                let (l_tmp, l_type, l_instructions) = self.compile_expr(*left)?;
                let (r_tmp, r_type, r_instructions) = self.compile_expr(*right)?;
                let mut instructions = l_instructions.clone();
                instructions.extend_from_slice(&r_instructions);
                if l_type != r_type {
                    println!(
                        "operands in binary expression should be of the same type, got {:?}, {:?}",
                        l_type, r_type
                    )
                }
                let tmp = self.next_tmp();
                match operator.token_type {
                    TokenType::Minus => {
                        instructions.push(format!("{} =d sub {} {}", tmp, l_tmp, r_tmp));
                        Ok((tmp, ResType::Number, instructions))
                    }
                    TokenType::Plus => {
                        instructions.push(format!("{} =d add {} {}", tmp, l_tmp, r_tmp));
                        Ok((tmp, ResType::Number, instructions))
                    }
                    TokenType::Star => {
                        instructions.push(format!("{} =d mul {} {}", tmp, l_tmp, r_tmp));
                        Ok((tmp, ResType::Number, instructions))
                    }
                    TokenType::Slash => {
                        instructions.push(format!("{} =d div {} {}", tmp, l_tmp, r_tmp));
                        Ok((tmp, ResType::Number, instructions))
                    }

                    TokenType::EqualEqual => todo!(),
                    TokenType::BangEqual => todo!(),
                    TokenType::Greater => todo!(),
                    TokenType::GreaterEqual => todo!(),
                    TokenType::Less => todo!(),
                    TokenType::LessEqual => todo!(),

                    TokenType::LogicalOr => todo!(),
                    TokenType::LogicalAnd => todo!(),

                    _ => {
                        let err_message =
                            format!("{} cannot be a binary operator", operator.lexeme);
                        eprint!("{}", err_message);
                        Err(QbeError::CompilationError(err_message).into())
                    }
                }
            }
            Expr::Variable { .. } => todo!(),
            Expr::Assign { .. } => todo!(),
            Expr::If { .. } => todo!(),
        }
    }
}

#[derive(Error, Debug)]
enum QbeError {
    #[error("failed  to compile")]
    CompilationError(String),
}
