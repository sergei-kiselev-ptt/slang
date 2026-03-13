#![allow(dead_code)]

use crate::lexer::*;

#[derive(Debug, Clone)]
pub enum TypeAnnotation {
    Num,
    Bool,
    Int,
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.span, self.message)
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(LiteralValue, Span),
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
    Let {
        name: Token,
        type_ann: TypeAnnotation,
        value: Box<Expr>,
        mutable: bool,
    },
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Number(f64),
    Int(i64),
    String(String),
    Bool(bool),
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        let span = if self.is_at_end() {
            if self.current > 0 {
                self.tokens[self.current - 1].span
            } else {
                Span::default()
            }
        } else {
            self.current().span
        };
        ParseError {
            message: message.into(),
            span,
        }
    }

    fn expect(&mut self, token_type: TokenType, msg: &str) -> Result<&Token, ParseError> {
        if !self.match_token(&[token_type]) {
            return Err(self.error(msg));
        }
        Ok(self.previous())
    }

    pub fn parse(&mut self) -> Result<Expr, ParseError> {
        self.skip_newlines();
        let expr = self.parse_expr()?;
        self.skip_newlines();
        if !self.is_at_end() {
            return Err(self.error(format!("unexpected token '{}'", self.current().lexeme)));
        }
        Ok(expr)
    }

    pub fn parse_program(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut exprs = vec![];
        self.skip_newlines();
        while !self.is_at_end() {
            if self.match_token(&[TokenType::Func]) {
                exprs.push(self.parse_func_def()?);
            } else {
                exprs.push(self.parse_expr()?);
            }
            self.skip_newlines();
        }
        Ok(exprs)
    }

    fn parse_func_def(&mut self) -> Result<Expr, ParseError> {
        self.expect(TokenType::Identifier, "expected function name after 'func'")?;
        let name = self.previous().clone();

        self.expect(TokenType::LeftParen, "expected '(' after function name")?;

        let mut params = vec![];
        while !self.check_token(&[TokenType::RightParen]) {
            if self.is_at_end() {
                return Err(self.error("expected ')' in function parameter list"));
            }
            if !params.is_empty() {
                self.expect(TokenType::Comma, "expected ',' between parameters")?;
            }
            self.expect(TokenType::Identifier, "expected parameter name")?;
            let param_name = self.previous().clone();
            self.expect(TokenType::Colon, "expected ':' after parameter name")?;
            let param_type = self.parse_type()?;
            params.push((param_name, param_type));
        }

        self.expect(TokenType::RightParen, "expected ')' after parameters")?;
        self.expect(TokenType::Arrow, "expected '->' after parameters")?;
        let return_type = self.parse_type()?;

        self.expect(TokenType::LeftBrace, "expected '{' before function body")?;
        let body = self.parse_block()?;

        Ok(Expr::FuncDef {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_type(&mut self) -> Result<TypeAnnotation, ParseError> {
        if self.match_token(&[TokenType::NumType]) {
            Ok(TypeAnnotation::Num)
        } else if self.match_token(&[TokenType::BoolType]) {
            Ok(TypeAnnotation::Bool)
        } else if self.match_token(&[TokenType::IntType]) {
            Ok(TypeAnnotation::Int)
        } else {
            Err(self.error(format!(
                "expected type ('num', 'int', or 'bool'), found '{}'",
                if self.is_at_end() {
                    "end of input"
                } else {
                    &self.current().lexeme
                }
            )))
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

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_logical_or()?;

        if self.match_token(&[TokenType::Equal]) {
            if let Expr::Variable { name } = expr {
                let value = self.parse_assignment()?;
                return Ok(Expr::Assign {
                    name,
                    value: Box::new(value),
                });
            }
            return Err(self.error("invalid assignment target"));
        }

        while self.match_token(&[TokenType::Plus, TokenType::Minus]) {
            let operator = self.previous().clone();
            let right = self.parse_term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_logical_and()?;

        while self.match_token(&[TokenType::LogicalOr]) {
            let operator = self.previous().clone();
            let right = self.parse_logical_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            }
        }

        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_equality()?;

        while self.match_token(&[TokenType::LogicalAnd]) {
            let operator = self.previous().clone();
            let right = self.parse_equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            }
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;

        while self.match_token(&[TokenType::EqualEqual, TokenType::BangEqual]) {
            let operator = self.previous().clone();
            let right = self.parse_comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            }
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_term()?;

        while self.match_token(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            let operator = self.previous().clone();
            let right = self.parse_term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            }
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_factor()?;

        while self.match_token(&[TokenType::Star, TokenType::Slash]) {
            let operator = self.previous().clone();
            let right = self.parse_factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            }
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        self.parse_unary()
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_token(&[TokenType::Plus, TokenType::Minus, TokenType::Bang]) {
            let operator = self.previous().clone();
            let right = self.parse_unary()?;
            return Ok(Expr::Unary {
                operator,
                right: Box::new(right),
            });
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        if self.is_at_end() {
            return Err(self.error("unexpected end of input"));
        }

        if self.match_token(&[TokenType::Integer]) {
            let span = self.previous().span;
            if let Ok(parsed) = self.previous().clone().lexeme.parse::<i64>() {
                return Ok(Expr::Literal(LiteralValue::Int(parsed), span));
            }
            return Err(self.error("invalid integer literal"));
        }

        if self.match_token(&[TokenType::Number]) {
            let span = self.previous().span;
            if let Ok(parsed) = self.previous().clone().lexeme.parse::<f64>() {
                return Ok(Expr::Literal(LiteralValue::Number(parsed), span));
            }
            return Err(self.error("invalid float literal"));
        }

        if self.match_token(&[TokenType::LeftParen]) {
            let expr = self.parse_expr()?;
            self.expect(TokenType::RightParen, "expected ')' after expression")?;
            return Ok(expr);
        }

        if self.match_token(&[TokenType::If]) {
            return self.parse_if();
        }

        if self.match_token(&[TokenType::While]) {
            return self.parse_while();
        }

        if self.match_token(&[TokenType::Print]) {
            let value = self.parse_expr()?;
            return Ok(Expr::Print {
                value: Box::new(value),
            });
        }

        if self.match_token(&[TokenType::Return]) {
            let value = self.parse_expr()?;
            return Ok(Expr::Return {
                value: Box::new(value),
            });
        }

        if self.match_token(&[TokenType::Let]) {
            let mutable = self.match_token(&[TokenType::Mut]);
            self.expect(TokenType::Identifier, "expected variable name after 'let'")?;
            let name = self.previous().clone();
            self.expect(
                TokenType::Colon,
                "expected ': <type>' after variable name in 'let'",
            )?;
            let type_ann = self.parse_type()?;
            self.expect(TokenType::Equal, "expected '=' after type in 'let'")?;
            let value = self.parse_expr()?;
            return Ok(Expr::Let {
                name,
                type_ann,
                value: Box::new(value),
                mutable,
            });
        }

        if self.match_token(&[TokenType::True]) {
            return Ok(Expr::Literal(
                LiteralValue::Bool(true),
                self.previous().span,
            ));
        }

        if self.match_token(&[TokenType::False]) {
            return Ok(Expr::Literal(
                LiteralValue::Bool(false),
                self.previous().span,
            ));
        }

        if self.match_token(&[TokenType::Identifier]) {
            let identifier_token = self.previous().clone();
            if self.match_token(&[TokenType::LeftParen]) {
                let mut args = vec![];
                while !self.check_token(&[TokenType::RightParen]) {
                    if self.is_at_end() {
                        return Err(self.error("expected ')' in argument list"));
                    }
                    if !args.is_empty() {
                        self.expect(TokenType::Comma, "expected ',' between arguments")?;
                    }
                    args.push(self.parse_expr()?);
                }
                self.expect(TokenType::RightParen, "expected ')' after arguments")?;
                return Ok(Expr::Call {
                    name: identifier_token,
                    args,
                });
            }
            return Ok(Expr::Variable {
                name: identifier_token,
            });
        }

        Err(self.error(format!("unexpected token '{}'", self.current().lexeme)))
    }

    fn parse_if(&mut self) -> Result<Expr, ParseError> {
        let condition = self.parse_expr()?;

        self.expect(TokenType::LeftBrace, "expected '{' after if condition")?;
        let then_branch = self.parse_block()?;

        let else_branch = if self.match_token(&[TokenType::Else]) {
            self.expect(TokenType::LeftBrace, "expected '{' after else")?;
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Expr::If {
            condition: Box::new(condition),
            then_branch,
            else_branch,
        })
    }

    fn parse_block(&mut self) -> Result<Vec<Expr>, ParseError> {
        self.skip_newlines();
        let mut exprs = vec![];
        while !self.check_token(&[TokenType::RightBrace]) {
            if self.is_at_end() {
                return Err(self.error("expected '}' to close block"));
            }
            exprs.push(self.parse_expr()?);
            self.skip_newlines();
        }
        self.expect(TokenType::RightBrace, "expected '}' after block")?;
        Ok(exprs)
    }

    fn parse_while(&mut self) -> Result<Expr, ParseError> {
        let condition = self.parse_expr()?;

        self.expect(TokenType::LeftBrace, "expected '{' after while condition")?;
        let body = self.parse_block()?;

        Ok(Expr::While {
            condition: Box::new(condition),
            body,
        })
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn current(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
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
            Expr::Literal(val, _) => match val {
                LiteralValue::Number(n) => n.to_string(),
                LiteralValue::Int(n) => n.to_string(),
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
            Expr::Let {
                name,
                value,
                mutable,
                ..
            } => {
                let kw = if *mutable { "let mut" } else { "let" };
                format!("({} {} = {})", kw, name.lexeme, value.as_str())
            }
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

    pub fn span(&self) -> Option<Span> {
        match self {
            Expr::Unary { operator, .. } => Some(operator.span),
            Expr::Binary { operator, .. } => Some(operator.span),
            Expr::Variable { name } => Some(name.span),
            Expr::Assign { name, .. } => Some(name.span),
            Expr::FuncDef { name, .. } => Some(name.span),
            Expr::Call { name, .. } => Some(name.span),
            Expr::Let { name, .. } => Some(name.span),
            Expr::Print { value } => value.span(),
            Expr::Return { value } => value.span(),
            Expr::If { condition, .. } => condition.span(),
            Expr::While { condition, .. } => condition.span(),
            Expr::Literal(_, span) => Some(*span),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::parse_into_tokens;

    fn parse(source: &str) -> Result<Expr, ParseError> {
        let tokens = parse_into_tokens(source).unwrap();
        Parser::new(tokens).parse()
    }

    fn parse_program(source: &str) -> Result<Vec<Expr>, ParseError> {
        let tokens = parse_into_tokens(source).unwrap();
        Parser::new(tokens).parse_program()
    }

    #[test]
    fn parse_number_literal() {
        let expr = parse("42").unwrap();
        assert_eq!(expr.as_str(), "42");
    }

    #[test]
    fn parse_binary_expr() {
        let expr = parse("1 + 2").unwrap();
        assert_eq!(expr.as_str(), "(+ 1 2)");
    }

    #[test]
    fn error_unclosed_paren() {
        let err = parse("(1 + 2").unwrap_err();
        assert!(err.message.contains("expected ')'"));
    }

    #[test]
    fn error_unclosed_block() {
        let err = parse("if true { 42").unwrap_err();
        assert!(err.message.contains("expected '}'"));
    }

    #[test]
    fn error_let_missing_type() {
        let err = parse("let x = 5").unwrap_err();
        assert!(err.message.contains("expected ': <type>'"));
    }

    #[test]
    fn error_unexpected_token() {
        let err = parse("1 + + +").unwrap_err();
        assert!(err.message.contains("unexpected"));
    }

    #[test]
    fn error_has_span() {
        let err = parse("if true { 42").unwrap_err();
        assert!(err.span.line > 0);
    }

    #[test]
    fn parse_program_func() {
        let exprs = parse_program("func add(a: num, b: num) -> num {\n a + b\n}").unwrap();
        assert_eq!(exprs.len(), 1);
    }

    #[test]
    fn error_func_missing_arrow() {
        let err = parse_program("func foo() { 1 }").unwrap_err();
        assert!(err.message.contains("expected '->'"));
    }
}
