#![allow(dead_code)]

use core::panic;

use log::{debug, error};

use crate::lexer::*;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

#[derive(Debug)]
pub enum Expr {
    Literal(LiteralValue),
    Unary {
        operator: Token,
        right: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },
    Variable {
        name: Token,
    },
    Assign {
        name: Token,
        value: Box<Expr>,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    While {
        condition: Box<Expr>,
        body: Vec<Expr>,
    },
}

#[derive(Debug)]
pub enum LiteralValue {
    Number(f64),
    String(String),
    Bool(bool),
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Expr {
        self.skip_newlines();
        let expr = self.parse_expr();
        self.skip_newlines();
        if !self.is_at_end() {
            error!(
                "Parsed the expression, but there are unprocessed tokens: {:?}",
                self.current()
            );
            panic!();
        }

        return expr;
    }

    pub fn parse_program(&mut self) -> Vec<Expr> {
        let mut exprs = vec![];
        self.skip_newlines();
        while !self.is_at_end() {
            exprs.push(self.parse_expr());
            self.skip_newlines();
        }
        exprs
    }

    fn skip_newlines(&mut self) {
        while !self.is_at_end() && self.current().token_type == TokenType::Newline {
            self.advance();
        }
    }

    fn parse_expr(&mut self) -> Expr {
        debug!("Parsing assignment, lexeme is {}", self.current().lexeme);
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Expr {
        debug!("Parsing assignment, lexeme is {}", self.current().lexeme);
        let mut expr = self.parse_logical_or();

        if self.match_token(&[TokenType::Equal]) {
            let _ = self.previous();
            if let Expr::Variable { name } = expr {
                let value = self.parse_assignment();
                return Expr::Assign {
                    name,
                    value: Box::new(value),
                };
            }
        }

        while self.match_token(&[TokenType::Plus, TokenType::Minus]) {
            let operator = self.previous().clone();
            let right = self.parse_term();
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        expr
    }

    fn parse_logical_or(&mut self) -> Expr {
        debug!("Parsing logical or, lexeme is {}", self.current().lexeme);
        let mut expr = self.parse_logical_and();

        while self.match_token(&[TokenType::LogicalOr]) {
            let operator = self.previous().clone();
            let right = self.parse_logical_and();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            }
        }

        expr
    }

    fn parse_logical_and(&mut self) -> Expr {
        debug!("Parsing logical and, lexeme is {}", self.current().lexeme);
        let mut expr = self.parse_equality();

        while self.match_token(&[TokenType::LogicalAnd]) {
            let operator = self.previous().clone();
            let right = self.parse_equality();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            }
        }

        expr
    }

    fn parse_equality(&mut self) -> Expr {
        debug!("Parsing equality, lexeme is {}", self.current().lexeme);
        let mut expr = self.parse_comparison();

        while self.match_token(&[TokenType::EqualEqual, TokenType::BangEqual]) {
            let operator = self.previous().clone();
            let right = self.parse_comparison();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            }
        }

        expr
    }

    fn parse_comparison(&mut self) -> Expr {
        let mut expr = self.parse_term();

        while self.match_token(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            let operator = self.previous().clone();
            let right = self.parse_term();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            }
        }

        expr
    }

    fn parse_term(&mut self) -> Expr {
        debug!("Parsing term, lexeme is {}", self.current().lexeme);
        let mut expr = self.parse_factor();

        while self.match_token(&[TokenType::Star, TokenType::Slash]) {
            let operator = self.previous().clone();
            let right = self.parse_factor();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            }
        }

        return expr;
    }

    fn parse_factor(&mut self) -> Expr {
        debug!("Parsing factor, lexeme is {}", self.current().lexeme);
        self.parse_unary()
    }

    fn parse_unary(&mut self) -> Expr {
        debug!("Parsing factor, lexeme is {}", self.current().lexeme);
        if self.match_token(&[TokenType::Plus, TokenType::Minus, TokenType::Bang]) {
            let operator = self.previous().clone();
            let right = self.parse_unary();
            return Expr::Unary {
                operator,
                right: Box::new(right),
            };
        }

        return self.parse_primary();
    }

    fn parse_primary(&mut self) -> Expr {
        debug!("parsing primary, lexeme is {}", self.current().lexeme);
        if self.match_token(&[TokenType::Number]) {
            if let Ok(parsed) = self.previous().clone().lexeme.parse() {
                return Expr::Literal(LiteralValue::Number(parsed));
            }
        }

        if self.match_token(&[TokenType::LeftParen]) {
            debug!("Processing parenthesis...");

            let expr = self.parse_expr();
            if !self.match_token(&[TokenType::RightParen]) {
                panic!("Expected ')' after expresion");
            }
            debug!("Processed parenthesis");
            return expr;
        }

        if self.match_token(&[TokenType::If]) {
            debug!("Processing if expression...");
            return self.parse_if();
        }

        if self.match_token(&[TokenType::While]) {
            return self.parse_while();
        }

        if self.match_token(&[TokenType::True]) {
            return Expr::Literal(LiteralValue::Bool(true));
        }

        if self.match_token(&[TokenType::False]) {
            return Expr::Literal(LiteralValue::Bool(false));
        }

        if self.match_token(&[TokenType::Identifier]) {
            let identifier_token = self.previous();
            return Expr::Variable {
                name: identifier_token.clone(),
            };
        }
        panic!("Only NUMBER can be a primary, found {:?}", self.current());
    }

    fn parse_if(&mut self) -> Expr {
        // Parse condition (no braces around it)
        let condition = self.parse_expr();

        // Expect opening brace for then branch
        if !self.match_token(&[TokenType::LeftBrace]) {
            panic!("Expected '{{' after if condition");
        }

        // Parse then branch expression
        let then_branch = self.parse_expr();

        // Expect closing brace for then branch
        if !self.match_token(&[TokenType::RightBrace]) {
            panic!("Expected '}}' after then branch");
        }

        // Optional else branch
        let else_branch = if self.match_token(&[TokenType::Else]) {
            // Expect opening brace for else branch
            if !self.match_token(&[TokenType::LeftBrace]) {
                panic!("Expected '{{' after else");
            }

            // Parse else branch expression
            let else_expr = self.parse_expr();

            // Expect closing brace for else branch
            if !self.match_token(&[TokenType::RightBrace]) {
                panic!("Expected '}}' after else branch");
            }

            Some(Box::new(else_expr))
        } else {
            None
        };

        Expr::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch,
        }
    }

    fn parse_while(&mut self) -> Expr {
        let condition = self.parse_expr();

        if !self.match_token(&[TokenType::LeftBrace]) {
            panic!("Expected '{{' after while condition");
        }

        let mut body = vec![];
        while !self.match_token(&[TokenType::RightBrace]) {
            if self.is_at_end() {
                panic!("Expected '}}' to close while body");
            }
            body.push(self.parse_expr());
        }

        Expr::While {
            condition: Box::new(condition),
            body,
        }
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn current(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn advance(&mut self) -> () {
        if !self.is_at_end() {
            debug!("Advancing parser");
            self.current += 1
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    fn match_token(&mut self, types: &[TokenType]) -> bool {
        let saved = self.current;
        while !self.is_at_end() && self.current().token_type == TokenType::Newline {
            self.current += 1;
        }
        for t_type in types {
            if !self.is_at_end() && self.current().token_type == *t_type {
                self.advance();
                return true;
            }
        }

        self.current = saved;
        false
    }
}

impl Expr {
    pub fn as_str(&self) -> String {
        match self {
            Expr::Literal(val) => match val {
                LiteralValue::Number(n) => n.to_string(),
                LiteralValue::String(s) => s.clone(),
                LiteralValue::Bool(b) => b.to_string(),
            },
            Expr::Unary { operator, right } => format!("({} {})", operator.lexeme, right.as_str()),
            Expr::Binary {
                left,
                operator,
                right,
            } => format!("({} {} {})", operator.lexeme, left.as_str(), right.as_str()),
            Expr::Variable { name } => name.lexeme.clone(),
            Expr::Assign { name, value } => format!("({} {} {})", "=", name.lexeme, value.as_str()),
            Expr::While { condition, body } => format!(
                "(while {} {{ {} }})",
                condition.as_str(),
                body.iter()
                    .map(|e| e.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => match else_branch {
                Some(else_expr) => format!(
                    "(if {} {{ {} }} else {{ {} }})",
                    condition.as_str(),
                    then_branch.as_str(),
                    else_expr.as_str()
                ),
                None => format!("(if {} {{ {} }})", condition.as_str(), then_branch.as_str()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    // #[test]
    // fn parse_primary() {
    // }
}
