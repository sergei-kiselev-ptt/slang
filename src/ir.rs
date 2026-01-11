use crate::lexer::TokenType;
use crate::parser::{Expr, LiteralValue};

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // Stack manipulation
    Push(Literal),
    Load(String),
    Store(String),
    Dup,

    // Binary ops (pop 2, push 1)
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,

    // Unary ops (pop 1, push 1)
    Neg,
    Not,
}

pub fn compile(expr: &Expr) -> Vec<Instruction> {
    let mut instructions = Vec::new();
    compile_expr(expr, &mut instructions);
    instructions
}

fn compile_expr(expr: &Expr, out: &mut Vec<Instruction>) {
    match expr {
        Expr::Literal(lit) => {
            let literal = match lit {
                LiteralValue::Number(n) => Literal::Number(*n),
                LiteralValue::Bool(b) => Literal::Bool(*b),
                LiteralValue::String(s) => Literal::String(s.clone()),
            };
            out.push(Instruction::Push(literal));
        }

        Expr::Variable { name } => {
            out.push(Instruction::Load(name.lexeme.clone()));
        }

        Expr::Assign { name, value } => {
            compile_expr(value, out);
            out.push(Instruction::Dup);
            out.push(Instruction::Store(name.lexeme.clone()));
        }

        Expr::Unary { operator, right } => {
            compile_expr(right, out);
            let op = match operator.token_type {
                TokenType::Minus => Instruction::Neg,
                TokenType::Bang => Instruction::Not,
                TokenType::Plus => return,
                _ => panic!("Unknown unary operator: {:?}", operator),
            };
            out.push(op);
        }

        Expr::Binary {
            left,
            operator,
            right,
        } => {
            compile_expr(left, out);
            compile_expr(right, out);
            let op = match operator.token_type {
                TokenType::Plus => Instruction::Add,
                TokenType::Minus => Instruction::Sub,
                TokenType::Star => Instruction::Mul,
                TokenType::Slash => Instruction::Div,
                TokenType::EqualEqual => Instruction::Eq,
                TokenType::BangEqual => Instruction::Neq,
                TokenType::Less => Instruction::Lt,
                TokenType::LessEqual => Instruction::Lte,
                TokenType::Greater => Instruction::Gt,
                TokenType::GreaterEqual => Instruction::Gte,
                TokenType::LogicalAnd => Instruction::And,
                TokenType::LogicalOr => Instruction::Or,
                _ => panic!("Unknown binary operator: {:?}", operator),
            };
            out.push(op);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::parse_into_tokens;
    use crate::parser::Parser;

    fn compile_source(source: &str) -> Vec<Instruction> {
        let tokens = parse_into_tokens(source).unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse();
        compile(&expr)
    }

    #[test]
    fn compile_number() {
        let instructions = compile_source("42");
        assert_eq!(instructions, vec![Instruction::Push(Literal::Number(42.0))]);
    }

    #[test]
    fn compile_bool() {
        let instructions = compile_source("true");
        assert_eq!(instructions, vec![Instruction::Push(Literal::Bool(true))]);
    }

    #[test]
    fn compile_addition() {
        let instructions = compile_source("2 + 3");
        assert_eq!(
            instructions,
            vec![
                Instruction::Push(Literal::Number(2.0)),
                Instruction::Push(Literal::Number(3.0)),
                Instruction::Add,
            ]
        );
    }

    #[test]
    fn compile_precedence() {
        let instructions = compile_source("2 + 3 * 4");
        assert_eq!(
            instructions,
            vec![
                Instruction::Push(Literal::Number(2.0)),
                Instruction::Push(Literal::Number(3.0)),
                Instruction::Push(Literal::Number(4.0)),
                Instruction::Mul,
                Instruction::Add,
            ]
        );
    }

    #[test]
    fn compile_precedence_parenthesis() {
        let instructions = compile_source("(2 + 3) * 4");
        assert_eq!(
            instructions,
            vec![
                Instruction::Push(Literal::Number(2.0)),
                Instruction::Push(Literal::Number(3.0)),
                Instruction::Add,
                Instruction::Push(Literal::Number(4.0)),
                Instruction::Mul,
            ]
        );
    }

    #[test]
    fn compile_assignment() {
        let instructions = compile_source("x = 5");
        assert_eq!(
            instructions,
            vec![
                Instruction::Push(Literal::Number(5.0)),
                Instruction::Dup,
                Instruction::Store("x".to_string()),
            ]
        );
    }

    #[test]
    fn compile_chained_assignment() {
        let instructions = compile_source("x = y = 1");
        assert_eq!(
            instructions,
            vec![
                Instruction::Push(Literal::Number(1.0)),
                Instruction::Dup,
                Instruction::Store("y".to_string()),
                Instruction::Dup,
                Instruction::Store("x".to_string()),
            ]
        );
    }

    #[test]
    fn compile_unary_negation() {
        let instructions = compile_source("-5");
        assert_eq!(
            instructions,
            vec![Instruction::Push(Literal::Number(5.0)), Instruction::Neg,]
        );
    }

    #[test]
    fn compile_comparison() {
        let instructions = compile_source("a > b");
        assert_eq!(
            instructions,
            vec![
                Instruction::Load("a".to_string()),
                Instruction::Load("b".to_string()),
                Instruction::Gt,
            ]
        );
    }

    #[test]
    fn compile_logical() {
        let instructions = compile_source("true && false");
        assert_eq!(
            instructions,
            vec![
                Instruction::Push(Literal::Bool(true)),
                Instruction::Push(Literal::Bool(false)),
                Instruction::And,
            ]
        );
    }

    #[test]
    fn compile_logical_with_precendence() {
        let instructions = compile_source("false || true && false");
        assert_eq!(
            instructions,
            vec![
                Instruction::Push(Literal::Bool(false)),
                Instruction::Push(Literal::Bool(true)),
                Instruction::Push(Literal::Bool(false)),
                Instruction::And,
                Instruction::Or,
            ]
        );
    }

    #[test]
    fn compile_logical_with_precendence_parenthesis() {
        let instructions = compile_source("(false || true) && false");
        assert_eq!(
            instructions,
            vec![
                Instruction::Push(Literal::Bool(false)),
                Instruction::Push(Literal::Bool(true)),
                Instruction::Or,
                Instruction::Push(Literal::Bool(false)),
                Instruction::And,
            ]
        );
    }
}
