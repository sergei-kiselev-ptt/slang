#![allow(dead_code)]
use std::io::{Error, ErrorKind};

use log::error;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenType {
    // Single character tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Minus,
    Plus,
    Star,
    Slash,
    Equal,

    // Comparison
    EqualEqual,
    BangEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Logical operators
    LogicalOr,
    LogicalAnd,
    Bang,

    Newline,

    // Keywords
    If,
    Else,
    While,
    For,
    In,
    Print,
    Func,
    Return,
    Let,
    Mut,

    // Type keywords
    NumType,
    BoolType,
    IntType,

    // Punctuation
    Arrow,
    Colon,
    Comma,
    DotDot,

    // Literals
    Number,
    Integer,
    Identifier,
    True,
    False,
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Span {
    pub line: usize, // 1-based
    pub col: usize,  // 1-based
    pub len: usize,
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub span: Span,
}

pub fn parse_into_tokens(input: &str) -> Result<Vec<Token>, Error> {
    let mut tokens = vec![];

    let mut start = 0;
    let mut line = 1usize;
    let mut col = 1usize;

    while start < input.len() {
        let token_line = line;
        let token_col = col;
        let (token, next) = scan_next_token(input, start)?;
        let len = next - start;

        for ch in input[start..next].chars() {
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        if let Some(mut tok) = token {
            tok.span = Span {
                line: token_line,
                col: token_col,
                len,
            };
            tokens.push(tok);
        }
        start = next;
    }

    Ok(tokens)
}

fn scan_next_token(input: &str, current: usize) -> Result<(Option<Token>, usize), Error> {
    let slice = &input[current..];
    let first = slice.chars().next().unwrap();

    match first {
        ' ' | '\t' | '\r' => Ok((None, current + 1)),
        '\n' => Ok((Some(tok(TokenType::Newline, "\n")), current + 1)),
        ':' => Ok((Some(colon()), current + 1)),
        ',' => Ok((Some(comma()), current + 1)),
        '-' => {
            if slice.starts_with("->") {
                return Ok((Some(arrow()), current + 2));
            }
            Ok((Some(minus()), current + 1))
        }
        '+' => Ok((Some(plus()), current + 1)),
        '*' => Ok((Some(star()), current + 1)),
        '/' => Ok((Some(slash()), current + 1)),
        '.' => {
            if slice.starts_with("..") {
                return Ok((Some(dot_dot()), current + 2));
            }
            Err(Error::new(ErrorKind::Other, "single '.' is not supported"))
        }
        '(' => Ok((Some(left_paren()), current + 1)),
        ')' => Ok((Some(right_paren()), current + 1)),
        '{' => Ok((Some(left_brace()), current + 1)),
        '}' => Ok((Some(right_brace()), current + 1)),
        '=' => {
            if slice.starts_with("==") {
                return Ok((Some(equal_equal()), current + 2));
            }
            Ok((Some(equal()), current + 1))
        }
        '!' => {
            if slice.starts_with("!=") {
                return Ok((Some(bang_equal()), current + 2));
            }
            Ok((Some(bang()), current + 1))
        }
        '>' => {
            if slice.starts_with(">=") {
                return Ok((Some(greater_equal()), current + 2));
            }
            Ok((Some(greater()), current + 1))
        }
        '<' => {
            if slice.starts_with("<=") {
                return Ok((Some(less_equal()), current + 2));
            }
            Ok((Some(less()), current + 1))
        }
        '|' => {
            if slice.starts_with("||") {
                return Ok((Some(logical_or()), current + 2));
            }
            Err(Error::new(
                ErrorKind::Other,
                "single '|' operator is not supported",
            ))
        }
        '&' => {
            if slice.starts_with("&&") {
                return Ok((Some(logical_and()), current + 2));
            }
            Err(Error::new(
                ErrorKind::Other,
                "single '&' operator is not supported",
            ))
        }
        other => {
            if let (Some(keyword), current) = scan_keyword(input, current) {
                return Ok((Some(keyword), current));
            }

            if let (Some(bool_literal), current) = scan_boolean_literal(input, current) {
                return Ok((Some(bool_literal), current));
            }

            if let (Some(identifier), current) = scan_identifier(input, current) {
                return Ok((Some(identifier), current));
            }

            if other.is_numeric() {
                return Ok(scan_number(input, current));
            }

            log_lexer_error(input, current, other);

            Err(Error::new(ErrorKind::Other, "lexer error"))
        }
    }
}

fn log_lexer_error(input: &str, current: usize, ch: char) {
    let left = (current.saturating_sub(5)..=current)
        .find(|&i| input.is_char_boundary(i))
        .unwrap_or(current);
    let right_base = (current + ch.len_utf8() + 5).min(input.len());
    let right = (current + ch.len_utf8()..=right_base)
        .rev()
        .find(|&i| input.is_char_boundary(i))
        .unwrap_or(current + ch.len_utf8());
    error!("Couldn't parse \'{}\' symbol: {}", ch, &input[left..right]);
}

fn tok(token_type: TokenType, lexeme: &str) -> Token {
    Token {
        token_type,
        lexeme: lexeme.to_string(),
        span: Span::default(),
    }
}

fn plus() -> Token {
    tok(TokenType::Plus, "+")
}
fn star() -> Token {
    tok(TokenType::Star, "*")
}
fn slash() -> Token {
    tok(TokenType::Slash, "/")
}
fn left_paren() -> Token {
    tok(TokenType::LeftParen, "(")
}
fn right_paren() -> Token {
    tok(TokenType::RightParen, ")")
}
fn left_brace() -> Token {
    tok(TokenType::LeftBrace, "{")
}
fn right_brace() -> Token {
    tok(TokenType::RightBrace, "}")
}
fn if_kw() -> Token {
    tok(TokenType::If, "if")
}
fn else_kw() -> Token {
    tok(TokenType::Else, "else")
}
fn while_kw() -> Token {
    tok(TokenType::While, "while")
}
fn for_kw() -> Token {
    tok(TokenType::For, "for")
}
fn in_kw() -> Token {
    tok(TokenType::In, "in")
}
fn print_kw() -> Token {
    tok(TokenType::Print, "print")
}
fn func_kw() -> Token {
    tok(TokenType::Func, "func")
}
fn return_kw() -> Token {
    tok(TokenType::Return, "return")
}
fn let_kw() -> Token {
    tok(TokenType::Let, "let")
}
fn mut_kw() -> Token {
    tok(TokenType::Mut, "mut")
}
fn num_type_kw() -> Token {
    tok(TokenType::NumType, "num")
}
fn bool_type_kw() -> Token {
    tok(TokenType::BoolType, "bool")
}
fn int_type_kw() -> Token {
    tok(TokenType::IntType, "int")
}
fn arrow() -> Token {
    tok(TokenType::Arrow, "->")
}
fn colon() -> Token {
    tok(TokenType::Colon, ":")
}
fn comma() -> Token {
    tok(TokenType::Comma, ",")
}
fn dot_dot() -> Token {
    tok(TokenType::DotDot, "..")
}
fn equal() -> Token {
    tok(TokenType::Equal, "=")
}
fn equal_equal() -> Token {
    tok(TokenType::EqualEqual, "==")
}
fn bang() -> Token {
    tok(TokenType::Bang, "!")
}
fn bang_equal() -> Token {
    tok(TokenType::BangEqual, "!=")
}
fn logical_and() -> Token {
    tok(TokenType::LogicalAnd, "&&")
}
fn logical_or() -> Token {
    tok(TokenType::LogicalOr, "||")
}
fn greater() -> Token {
    tok(TokenType::Greater, ">")
}
fn greater_equal() -> Token {
    tok(TokenType::GreaterEqual, ">=")
}
fn less() -> Token {
    tok(TokenType::Less, "<")
}
fn less_equal() -> Token {
    tok(TokenType::LessEqual, "<=")
}
fn true_l() -> Token {
    tok(TokenType::True, "true")
}
fn false_l() -> Token {
    tok(TokenType::False, "false")
}
fn minus() -> Token {
    tok(TokenType::Minus, "-")
}
fn number(acc: &str) -> Token {
    tok(TokenType::Number, acc)
}
fn integer(acc: &str) -> Token {
    tok(TokenType::Integer, acc)
}

fn identifier(acc: &str) -> Token {
    tok(TokenType::Identifier, acc)
}

fn is_valid_identifier_character(c: char) -> bool {
    c.is_alphabetic() || c.is_numeric() || c == '_'
}

fn is_word_boundary(c: char) -> bool {
    matches!(
        c,
        ' ' | '\n'
            | '\t'
            | '\r'
            | ')'
            | '('
            | '{'
            | '}'
            | '+'
            | '-'
            | '*'
            | '/'
            | '='
            | '!'
            | '<'
            | '>'
            | '&'
            | '|'
            | ','
            | ':'
    )
}

/// Returns the byte length of `word` if `slice` starts with `word` followed by a word boundary
/// (or end of input), otherwise `None`.
fn keyword_len(slice: &str, word: &str) -> Option<usize> {
    if !slice.starts_with(word) {
        return None;
    }
    let rest = &slice[word.len()..];
    if rest.is_empty() || rest.chars().next().map_or(false, is_word_boundary) {
        return Some(word.len());
    }

    None
}

fn scan_identifier(input: &str, start: usize) -> (Option<Token>, usize) {
    let slice = &input[start..];

    match slice.chars().next() {
        Some(c) if c.is_alphabetic() || c == '_' => {}
        _ => return (None, start),
    }

    let end = slice
        .find(|c: char| !is_valid_identifier_character(c))
        .unwrap_or(slice.len());

    (Some(identifier(&slice[..end])), start + end)
}

fn scan_keyword(input: &str, start: usize) -> (Option<Token>, usize) {
    let slice = &input[start..];

    if let Some(n) = keyword_len(slice, "if") {
        return (Some(if_kw()), start + n);
    }
    if let Some(n) = keyword_len(slice, "else") {
        return (Some(else_kw()), start + n);
    }
    if let Some(n) = keyword_len(slice, "while") {
        return (Some(while_kw()), start + n);
    }
    if let Some(n) = keyword_len(slice, "for") {
        return (Some(for_kw()), start + n);
    }
    if let Some(n) = keyword_len(slice, "in") {
        return (Some(in_kw()), start + n);
    }
    if let Some(n) = keyword_len(slice, "print") {
        return (Some(print_kw()), start + n);
    }
    if let Some(n) = keyword_len(slice, "func") {
        return (Some(func_kw()), start + n);
    }
    if let Some(n) = keyword_len(slice, "return") {
        return (Some(return_kw()), start + n);
    }
    if let Some(n) = keyword_len(slice, "num") {
        return (Some(num_type_kw()), start + n);
    }
    if let Some(n) = keyword_len(slice, "bool") {
        return (Some(bool_type_kw()), start + n);
    }
    if let Some(n) = keyword_len(slice, "let") {
        return (Some(let_kw()), start + n);
    }
    if let Some(n) = keyword_len(slice, "mut") {
        return (Some(mut_kw()), start + n);
    }
    if let Some(n) = keyword_len(slice, "int") {
        return (Some(int_type_kw()), start + n);
    }

    (None, 0)
}

fn scan_boolean_literal(input: &str, start: usize) -> (Option<Token>, usize) {
    let slice = &input[start..];

    if let Some(n) = keyword_len(slice, "true") {
        return (Some(true_l()), start + n);
    }
    if let Some(n) = keyword_len(slice, "false") {
        return (Some(false_l()), start + n);
    }

    (None, 0)
}

fn scan_number(input: &str, start: usize) -> (Option<Token>, usize) {
    let slice = &input[start..];
    let int_end = slice.find(|c: char| !c.is_numeric()).unwrap_or(slice.len());
    let after_int = &slice[int_end..];

    // '.' followed by a digit (but not '..') means a float literal
    if after_int.starts_with('.') && after_int[1..].starts_with(|c: char| c.is_numeric()) {
        let after_dot = &after_int[1..];
        let frac_end = after_dot
            .find(|c: char| !c.is_numeric())
            .unwrap_or(after_dot.len());
        let total = int_end + 1 + frac_end;
        return (Some(number(&slice[..total])), start + total);
    }

    (Some(integer(&slice[..int_end])), start + int_end)
}

#[cfg(test)]
mod tests {
    use super::TokenType;
    use test_case::test_case;

    #[test]
    fn scan_next_token_integer() {
        let (_token, end) = super::scan_next_token("237", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 3);
        assert_eq!(token.lexeme, "237");
        assert_eq!(token.token_type, TokenType::Integer);
    }

    #[test]
    fn scan_next_token_float() {
        let (_token, end) = super::scan_next_token("3.14", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 4);
        assert_eq!(token.lexeme, "3.14");
        assert_eq!(token.token_type, TokenType::Number);
    }

    #[test]
    fn scan_next_token_minus() {
        let (_token, end) = super::scan_next_token("-", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "-");
        assert_eq!(token.token_type, TokenType::Minus);
    }

    #[test]
    fn scan_next_token_plus() {
        let (_token, end) = super::scan_next_token("+", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "+");
        assert_eq!(token.token_type, TokenType::Plus);
    }

    #[test]
    fn scan_next_token_star() {
        let (_token, end) = super::scan_next_token("*", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "*");
        assert_eq!(token.token_type, TokenType::Star);
    }

    #[test]
    fn scan_next_token_slash() {
        let (_token, end) = super::scan_next_token("/", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "/");
        assert_eq!(token.token_type, TokenType::Slash);
    }

    #[test]
    fn scan_next_token_equal() {
        let (_token, end) = super::scan_next_token("=", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "=");
        assert_eq!(token.token_type, TokenType::Equal);
    }

    #[test]
    fn scan_next_token_equal_equal() {
        let (_token, end) = super::scan_next_token("==", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 2);
        assert_eq!(token.lexeme, "==");
        assert_eq!(token.token_type, TokenType::EqualEqual);
    }

    #[test]
    fn scan_next_token_bang() {
        let (_token, end) = super::scan_next_token("!", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "!");
        assert_eq!(token.token_type, TokenType::Bang);
    }

    #[test]
    fn scan_next_token_bang_equal() {
        let (_token, end) = super::scan_next_token("!=", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 2);
        assert_eq!(token.lexeme, "!=");
        assert_eq!(token.token_type, TokenType::BangEqual);
    }

    #[test]
    fn scan_next_token_logical_and() {
        let (_token, end) = super::scan_next_token("&&", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 2);
        assert_eq!(token.lexeme, "&&");
        assert_eq!(token.token_type, TokenType::LogicalAnd);
    }

    #[test]
    fn scan_next_token_logical_or() {
        let (_token, end) = super::scan_next_token("||", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 2);
        assert_eq!(token.lexeme, "||");
        assert_eq!(token.token_type, TokenType::LogicalOr);
    }

    #[test]
    fn scan_next_token_greater() {
        let (_token, end) = super::scan_next_token(">", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, ">");
        assert_eq!(token.token_type, TokenType::Greater);
    }

    #[test]
    fn scan_next_token_greater_equal() {
        let (_token, end) = super::scan_next_token(">=", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 2);
        assert_eq!(token.lexeme, ">=");
        assert_eq!(token.token_type, TokenType::GreaterEqual);
    }

    #[test]
    fn scan_next_token_less() {
        let (_token, end) = super::scan_next_token("<", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "<");
        assert_eq!(token.token_type, TokenType::Less);
    }

    #[test]
    fn scan_next_token_less_equal() {
        let (_token, end) = super::scan_next_token("<=", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 2);
        assert_eq!(token.lexeme, "<=");
        assert_eq!(token.token_type, TokenType::LessEqual);
    }

    #[test]
    fn scan_next_token_true() {
        let (_token, end) = super::scan_next_token("true", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 4);
        assert_eq!(token.lexeme, "true");
        assert_eq!(token.token_type, TokenType::True);
    }

    #[test]
    fn scan_next_token_false() {
        let (_token, end) = super::scan_next_token("false", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 5);
        assert_eq!(token.lexeme, "false");
        assert_eq!(token.token_type, TokenType::False);
    }

    #[test]
    fn scan_next_token_left_paren() {
        let (_token, end) = super::scan_next_token("(", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "(");
        assert_eq!(token.token_type, TokenType::LeftParen);
    }

    #[test]
    fn scan_next_token_right_paren() {
        let (_token, end) = super::scan_next_token(")", 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, ")");
        assert_eq!(token.token_type, TokenType::RightParen);
    }

    #[test_case("abc")]
    #[test_case("ab1")]
    #[test_case("a1b")]
    #[test_case("a_b")]
    #[test_case("_ab")]
    #[test_case("_1av_")]
    fn scan_next_token_identifier(input_str: &str) {
        let (_token, end) = super::scan_next_token(input_str, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, input_str.len());
        assert_eq!(token.lexeme, input_str);
        assert_eq!(token.token_type, TokenType::Identifier);
    }

    #[test]
    fn scan_next_token_invalid_identifier() {
        let (scan_res, end) = super::scan_identifier("?bc_", 0);

        assert!(scan_res.is_none());
        assert_eq!(end, 0);
    }

    #[test]
    fn parse_into_tokens_math_expr() {
        let tokens = super::parse_into_tokens("2-1+3*4/5").unwrap();

        assert_eq!(tokens.len(), 9);
        assert_eq!(tokens[0].lexeme, "2");
        assert_eq!(tokens[0].token_type, TokenType::Integer);
        assert_eq!(tokens[1].lexeme, "-");
        assert_eq!(tokens[1].token_type, TokenType::Minus);
        assert_eq!(tokens[2].lexeme, "1");
        assert_eq!(tokens[2].token_type, TokenType::Integer);
        assert_eq!(tokens[3].lexeme, "+");
        assert_eq!(tokens[3].token_type, TokenType::Plus);
        assert_eq!(tokens[4].lexeme, "3");
        assert_eq!(tokens[4].token_type, TokenType::Integer);
        assert_eq!(tokens[5].lexeme, "*");
        assert_eq!(tokens[5].token_type, TokenType::Star);
        assert_eq!(tokens[6].lexeme, "4");
        assert_eq!(tokens[6].token_type, TokenType::Integer);
        assert_eq!(tokens[7].lexeme, "/");
        assert_eq!(tokens[7].token_type, TokenType::Slash);
        assert_eq!(tokens[8].lexeme, "5");
        assert_eq!(tokens[8].token_type, TokenType::Integer);
    }

    #[test_case("2 -1")]
    #[test_case("2 \t-1")]
    #[test_case("2 \r-1")]
    #[test_case("3- 1")]
    #[test_case(" 4- 1")]
    #[test_case(" 5 - 1")]
    fn parse_into_tokens_skip_insignificant_symbols(input: &str) {
        let tokens = super::parse_into_tokens(input).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].token_type, TokenType::Integer);
        assert_eq!(tokens[1].token_type, TokenType::Minus);
        assert_eq!(tokens[2].token_type, TokenType::Integer);
    }
}
