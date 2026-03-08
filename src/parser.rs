#![allow(dead_code)]

use core::panic;

use log::{debug, error};

use crate::lexer::*;

#[derive(Debug, Clone)]
pub enum TypeAnnotation {
    Num,
    Bool,
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

#[derive(Debug, Clone)]
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
        then_branch: Vec<Expr>,
        else_branch: Option<Vec<Expr>>,
    },
    Return {
        value: Box<Expr>,
    },
    While {
        condition: Box<Expr>,
        body: Vec<Expr>,
    },
    Print {
        value: Box<Expr>,
    },
    FuncDef {
        name: Token,
        params: Vec<(Token, TypeAnnotation)>,
        return_type: TypeAnnotation,
        body: Vec<Expr>,
    },
    Call {
        name: Token,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone)]
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
            if self.match_token(&[TokenType::Func]) {
                exprs.push(self.parse_func_def());
            } else {
                exprs.push(self.parse_expr());
            }
            self.skip_newlines();
        }
        exprs
    }

    fn parse_func_def(&mut self) -> Expr {
        if !self.match_token(&[TokenType::Identifier]) {
            panic!("Expected function name after 'func'");
        }
        let name = self.previous().clone();

        if !self.match_token(&[TokenType::LeftParen]) {
            panic!("Expected '(' after function name");
        }

        let mut params = vec![];
        while !self.check_token(&[TokenType::RightParen]) {
            if self.is_at_end() {
                panic!("Expected ')' in function parameter list");
            }
            if !params.is_empty() && !self.match_token(&[TokenType::Comma]) {
                panic!("Expected ',' between parameters");
            }
            if !self.match_token(&[TokenType::Identifier]) {
                panic!("Expected parameter name");
            }
            let param_name = self.previous().clone();
            if !self.match_token(&[TokenType::Colon]) {
                panic!("Expected ':' after parameter name");
            }
            let param_type = self.parse_type();
            params.push((param_name, param_type));
        }

        if !self.match_token(&[TokenType::RightParen]) {
            panic!("Expected ')' after parameters");
        }
        if !self.match_token(&[TokenType::Arrow]) {
            panic!("Expected '->' after parameters");
        }
        let return_type = self.parse_type();

        if !self.match_token(&[TokenType::LeftBrace]) {
            panic!("Expected '{{' before function body");
        }
        let body = self.parse_block();

        Expr::FuncDef {
            name,
            params,
            return_type,
            body,
        }
    }

    fn parse_type(&mut self) -> TypeAnnotation {
        if self.match_token(&[TokenType::NumType]) {
            TypeAnnotation::Num
        } else if self.match_token(&[TokenType::BoolType]) {
            TypeAnnotation::Bool
        } else {
            panic!(
                "Expected type ('num' or 'bool'), found {:?}",
                self.current()
            );
        }
    }

    fn check_token(&self, types: &[TokenType]) -> bool {
        let mut i = self.current;
        while i < self.tokens.len() && self.tokens[i].token_type == TokenType::Newline {
            i += 1;
        }
        if i >= self.tokens.len() {
            return false;
        }
        types.iter().any(|t| self.tokens[i].token_type == *t)
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

        if self.match_token(&[TokenType::Print]) {
            let value = self.parse_expr();
            return Expr::Print {
                value: Box::new(value),
            };
        }

        if self.match_token(&[TokenType::Return]) {
            let value = self.parse_expr();
            return Expr::Return {
                value: Box::new(value),
            };
        }

        if self.match_token(&[TokenType::True]) {
            return Expr::Literal(LiteralValue::Bool(true));
        }

        if self.match_token(&[TokenType::False]) {
            return Expr::Literal(LiteralValue::Bool(false));
        }

        if self.match_token(&[TokenType::Identifier]) {
            let identifier_token = self.previous().clone();
            if self.match_token(&[TokenType::LeftParen]) {
                let mut args = vec![];
                while !self.check_token(&[TokenType::RightParen]) {
                    if self.is_at_end() {
                        panic!("Expected ')' in argument list");
                    }
                    if !args.is_empty() && !self.match_token(&[TokenType::Comma]) {
                        panic!("Expected ',' between arguments");
                    }
                    args.push(self.parse_expr());
                }
                if !self.match_token(&[TokenType::RightParen]) {
                    panic!("Expected ')' after arguments");
                }
                return Expr::Call {
                    name: identifier_token,
                    args,
                };
            }
            return Expr::Variable {
                name: identifier_token,
            };
        }
        panic!("Only NUMBER can be a primary, found {:?}", self.current());
    }

    fn parse_if(&mut self) -> Expr {
        let condition = self.parse_expr();

        if !self.match_token(&[TokenType::LeftBrace]) {
            panic!("Expected '{{' after if condition");
        }
        let then_branch = self.parse_block();

        let else_branch = if self.match_token(&[TokenType::Else]) {
            if !self.match_token(&[TokenType::LeftBrace]) {
                panic!("Expected '{{' after else");
            }
            Some(self.parse_block())
        } else {
            None
        };

        Expr::If {
            condition: Box::new(condition),
            then_branch,
            else_branch,
        }
    }

    fn parse_block(&mut self) -> Vec<Expr> {
        self.skip_newlines();
        let mut exprs = vec![];
        while !self.check_token(&[TokenType::RightBrace]) {
            if self.is_at_end() {
                panic!("Expected '}}' to close block");
            }
            exprs.push(self.parse_expr());
            self.skip_newlines();
        }
        if !self.match_token(&[TokenType::RightBrace]) {
            panic!("Expected '}}' after block");
        }
        exprs
    }

    fn parse_while(&mut self) -> Expr {
        let condition = self.parse_expr();

        if !self.match_token(&[TokenType::LeftBrace]) {
            panic!("Expected '{{' after while condition");
        }
        let body = self.parse_block();

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
            Expr::Print { value } => format!("(print {})", value.as_str()),
            Expr::Return { value } => format!("(return {})", value.as_str()),
            Expr::FuncDef {
                name, params, body, ..
            } => format!(
                "(func {} ({}) {{ {} }})",
                name.lexeme,
                params
                    .iter()
                    .map(|(p, _)| p.lexeme.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                body.iter()
                    .map(|e| e.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            Expr::Call { name, args } => format!(
                "(call {} ({}))",
                name.lexeme,
                args.iter()
                    .map(|a| a.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let then_str = then_branch
                    .iter()
                    .map(|e| e.as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                match else_branch {
                    Some(else_exprs) => {
                        let else_str = else_exprs
                            .iter()
                            .map(|e| e.as_str())
                            .collect::<Vec<_>>()
                            .join("; ");
                        format!(
                            "(if {} {{ {} }} else {{ {} }})",
                            condition.as_str(),
                            then_str,
                            else_str
                        )
                    }
                    None => format!("(if {} {{ {} }})", condition.as_str(), then_str),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // #[test]
    // fn parse_primary() {
    // }
}
