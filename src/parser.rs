#![allow(dead_code)]

use log::debug;

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
}

#[derive(Debug)]
pub enum LiteralValue {
    Number(f64),
    String(String),
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse_expr(&mut self) -> Expr {
        debug!("Parsing expression");
        let mut expr = self.parse_term();

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

    fn parse_term(&mut self) -> Expr {
        debug!("Parsing term");
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
        debug!("Parsing factor");
        self.parse_unary()
    }

    fn parse_unary(&mut self) -> Expr {
        debug!("Parsing factor");
        if self.match_token(&[TokenType::Plus, TokenType::Minus]) {
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
        debug!("parsing primary");
        if self.match_token(&[TokenType::Number]){
            if let Ok(parsed) = self.previous().clone().lexeme.parse() {
                return Expr::Literal(LiteralValue::Number(parsed));
            }
        }
        panic!("Only NUMBER can be a primary, found {:?}", self.current());
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn current(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn advance(&mut self) -> () {
        debug!("Advancing parser");
        if !self.is_at_end() {
            self.current += 1
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    fn match_token(&mut self, types: &[TokenType]) -> bool {
        for t_type in types {
            if !self.is_at_end() && self.current().token_type == *t_type {
                self.advance();
                return true;
            }
        }

        false
    }
}

impl Expr {
    pub fn print(&self) -> String {
        match self {
            Expr::Literal(val) => match val {
                LiteralValue::Number(n) => n.to_string(),
                LiteralValue::String(s) => s.clone(),
            },
            Expr::Unary { operator, right } => format!("({} {})", operator.lexeme, right.print()),
            Expr::Binary {
                left,
                operator,
                right,
            } => format!("({} {} {})", operator.lexeme, left.print(), right.print()),
        }
    }
}

pub fn parse(tokens: Vec<Token>) -> Expr {
    let mut parser = Parser::new(tokens);
    parser.parse_expr()
}

#[cfg(test)]
mod tests {
    // #[test]
    // fn parse_primary() {
    // }
}
