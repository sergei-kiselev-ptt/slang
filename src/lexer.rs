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
    Print,
    Func,
    Return,
    Let,

    // Type keywords
    NumType,
    BoolType,

    // Punctuation
    Arrow, // ->
    Colon, // :
    Comma, // ,

    // Literals
    Number,
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

// #[derive(Debug, Clone)]
// pub struct LexerError;

// impl Display for LexerError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "Couldn't parse for input for lexems")
//     }
// }

pub fn parse_into_tokens(input: &str) -> Result<Vec<Token>, Error> {
    let mut tokens = vec![];

    let mut start = 0;
    let mut line = 1usize;
    let mut col = 1usize;

    let chars = input.chars().collect::<Vec<char>>();

    while start < input.len() {
        let token_line = line;
        let token_col = col;
        let (token, next) = scan_next_token(&chars, start)?;
        let len = next - start;

        // Update line/col tracking
        for &ch in &chars[start..next] {
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

fn scan_next_token(input: &Vec<char>, current: usize) -> Result<(Option<Token>, usize), Error> {
    match input[current] {
        ' ' | '\t' | '\r' => Ok((None, current + 1)),
        '\n' => Ok((
            Some(tok(TokenType::Newline, "\n")),
            current + 1,
        )),
        ':' => Ok((Some(colon()), current + 1)),
        ',' => Ok((Some(comma()), current + 1)),
        '-' => {
            if current + 1 < input.len() && input[current + 1] == '>' {
                return Ok((Some(arrow()), current + 2));
            }
            Ok((Some(minus()), current + 1))
        }
        '+' => Ok((Some(plus()), current + 1)),
        '*' => Ok((Some(star()), current + 1)),
        '/' => Ok((Some(slash()), current + 1)),
        '(' => Ok((Some(left_paren()), current + 1)),
        ')' => Ok((Some(right_paren()), current + 1)),
        '{' => Ok((Some(left_brace()), current + 1)),
        '}' => Ok((Some(right_brace()), current + 1)),
        '=' => {
            if current + 1 == input.len() {
                return Ok((Some(equal()), current + 1));
            }

            if input[current + 1] == '=' {
                return Ok((Some(equal_equal()), current + 2));
            }

            Ok((Some(equal()), current + 1))
        }
        '!' => {
            if current + 1 == input.len() {
                return Ok((Some(bang()), current + 1));
            }

            if input[current + 1] == '=' {
                return Ok((Some(bang_equal()), current + 2));
            }

            Ok((Some(bang()), current + 1))
        }
        '>' => {
            if current + 1 == input.len() {
                return Ok((Some(greater()), current + 1));
            }

            if input[current + 1] == '=' {
                return Ok((Some(greater_equal()), current + 2));
            }

            Ok((Some(greater()), current + 1))
        }
        '<' => {
            if current + 1 == input.len() {
                return Ok((Some(less()), current + 1));
            }

            if input[current + 1] == '=' {
                return Ok((Some(less_equal()), current + 2));
            }

            Ok((Some(less()), current + 1))
        }
        '|' => {
            if current + 1 < input.len() && input[current + 1] == '|' {
                return Ok((Some(logical_or()), current + 2));
            }
            return Err(Error::new(ErrorKind::Other, "single '|' operator is not supported"));
        }
        '&' => {
            if current + 1 < input.len() && input[current + 1] == '&' {
                return Ok((Some(logical_and()), current + 2));
            }
            return Err(Error::new(ErrorKind::Other, "single '&' operator is not supported"));
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

            return Err(Error::new(ErrorKind::Other, "lexer error"));
        }
    }
}

fn log_lexer_error(input: &Vec<char>, current: usize, other: char) {
    let shift = 5;
    let left = current.saturating_sub(shift);
    let right = (current + shift).min(input.len() - 1);

    error!(
        "Couldn't parse \'{}\' symbol: {}",
        other,
        input[left..=right].iter().collect::<String>()
    );
}

fn tok(token_type: TokenType, lexeme: &str) -> Token {
    Token {
        token_type,
        lexeme: lexeme.to_string(),
        span: Span::default(),
    }
}

fn plus() -> Token { tok(TokenType::Plus, "+") }
fn star() -> Token { tok(TokenType::Star, "*") }
fn slash() -> Token { tok(TokenType::Slash, "/") }
fn left_paren() -> Token { tok(TokenType::LeftParen, "(") }
fn right_paren() -> Token { tok(TokenType::RightParen, ")") }
fn left_brace() -> Token { tok(TokenType::LeftBrace, "{") }
fn right_brace() -> Token { tok(TokenType::RightBrace, "}") }
fn if_kw() -> Token { tok(TokenType::If, "if") }
fn else_kw() -> Token { tok(TokenType::Else, "else") }
fn while_kw() -> Token { tok(TokenType::While, "while") }
fn print_kw() -> Token { tok(TokenType::Print, "print") }
fn func_kw() -> Token { tok(TokenType::Func, "func") }
fn return_kw() -> Token { tok(TokenType::Return, "return") }
fn let_kw() -> Token { tok(TokenType::Let, "let") }
fn num_type_kw() -> Token { tok(TokenType::NumType, "num") }
fn bool_type_kw() -> Token { tok(TokenType::BoolType, "bool") }
fn arrow() -> Token { tok(TokenType::Arrow, "->") }
fn colon() -> Token { tok(TokenType::Colon, ":") }
fn comma() -> Token { tok(TokenType::Comma, ",") }
fn equal() -> Token { tok(TokenType::Equal, "=") }
fn equal_equal() -> Token { tok(TokenType::EqualEqual, "==") }
fn bang() -> Token { tok(TokenType::Bang, "!") }
fn bang_equal() -> Token { tok(TokenType::BangEqual, "!=") }
fn logical_and() -> Token { tok(TokenType::LogicalAnd, "&&") }
fn logical_or() -> Token { tok(TokenType::LogicalOr, "||") }
fn greater() -> Token { tok(TokenType::Greater, ">") }
fn greater_equal() -> Token { tok(TokenType::GreaterEqual, ">=") }
fn less() -> Token { tok(TokenType::Less, "<") }
fn less_equal() -> Token { tok(TokenType::LessEqual, "<=") }

fn is_valid_identifier_character(char: char) -> bool {
    char.is_alphabetic() || char.is_numeric() || char == '_'
}

fn scan_identifier(input: &[char], start: usize) -> (Option<Token>, usize) {
    let mut current = start;
    let mut acc = String::with_capacity(16);

    if !(input[current].is_alphabetic() || input[current] == '_') {
        return (None, current);
    }

    while current < input.len() {
        if is_valid_identifier_character(input[current]) {
            acc.push(input[current]);
            current += 1;
            continue;
        }

        break;
    }

    (Some(identifier(acc)), current)
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

fn scan_keyword(input: &[char], start: usize) -> (Option<Token>, usize) {
    let slice = &input[start..];

    if slice.len() >= 2
        && slice.starts_with(&['i', 'f'])
        && (slice.len() == 2 || is_word_boundary(slice[2]))
    {
        return (Some(if_kw()), start + 2);
    }

    if slice.len() >= 4
        && slice.starts_with(&['e', 'l', 's', 'e'])
        && (slice.len() == 4 || is_word_boundary(slice[4]))
    {
        return (Some(else_kw()), start + 4);
    }

    if slice.len() >= 5
        && slice.starts_with(&['w', 'h', 'i', 'l', 'e'])
        && (slice.len() == 5 || is_word_boundary(slice[5]))
    {
        return (Some(while_kw()), start + 5);
    }

    if slice.len() >= 5
        && slice.starts_with(&['p', 'r', 'i', 'n', 't'])
        && (slice.len() == 5 || is_word_boundary(slice[5]))
    {
        return (Some(print_kw()), start + 5);
    }

    if slice.len() >= 4
        && slice.starts_with(&['f', 'u', 'n', 'c'])
        && (slice.len() == 4 || is_word_boundary(slice[4]))
    {
        return (Some(func_kw()), start + 4);
    }

    if slice.len() >= 3
        && slice.starts_with(&['n', 'u', 'm'])
        && (slice.len() == 3 || is_word_boundary(slice[3]))
    {
        return (Some(num_type_kw()), start + 3);
    }

    if slice.len() >= 4
        && slice.starts_with(&['b', 'o', 'o', 'l'])
        && (slice.len() == 4 || is_word_boundary(slice[4]))
    {
        return (Some(bool_type_kw()), start + 4);
    }

    if slice.len() >= 6
        && slice.starts_with(&['r', 'e', 't', 'u', 'r', 'n'])
        && (slice.len() == 6 || is_word_boundary(slice[6]))
    {
        return (Some(return_kw()), start + 6);
    }

    if slice.len() >= 3
        && slice.starts_with(&['l', 'e', 't'])
        && (slice.len() == 3 || is_word_boundary(slice[3]))
    {
        return (Some(let_kw()), start + 3);
    }

    (None, 0)
}

fn scan_boolean_literal(input: &[char], start: usize) -> (Option<Token>, usize) {
    let slice = &input[start..];
    if slice.len() < 4 {
        return (None, 0);
    }

    let mut checked_size = 4;

    if slice.starts_with(&['t', 'r', 'u', 'e'])
        && (slice.len() == checked_size || is_word_boundary(slice[checked_size]))
    {
        return (Some(true_l()), start + checked_size);
    }

    checked_size = 5;

    if slice.len() >= checked_size
        && slice.starts_with(&['f', 'a', 'l', 's', 'e'])
        && (slice.len() == checked_size || is_word_boundary(slice[checked_size]))
    {
        return (Some(false_l()), start + checked_size);
    }

    (None, 0)
}

fn scan_number(input: &[char], start: usize) -> (Option<Token>, usize) {
    let mut current = start;
    let mut acc = String::with_capacity(16);
    while current < input.len() {
        if input[current].is_numeric() {
            acc.push(input[current]);
            current += 1;
            continue;
        }

        break;
    }

    (Some(number(acc)), current)
}

fn true_l() -> Token { tok(TokenType::True, "true") }
fn false_l() -> Token { tok(TokenType::False, "false") }
fn minus() -> Token { tok(TokenType::Minus, "-") }

fn number(acc: String) -> Token {
    Token {
        token_type: TokenType::Number,
        lexeme: acc,
        span: Span::default(),
    }
}

fn identifier(acc: String) -> Token {
    Token {
        token_type: TokenType::Identifier,
        lexeme: acc,
        span: Span::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::TokenType;
    use test_case::test_case;

    #[test]
    fn scan_next_token_number() {
        let input = vec!['2', '3', '7'];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 3);
        assert_eq!(token.lexeme, "237");
        assert_eq!(token.token_type, TokenType::Number);
    }

    #[test]
    fn scan_next_token_minus() {
        let input = vec!['-'];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "-");
        assert_eq!(token.token_type, TokenType::Minus);
    }

    #[test]
    fn scan_next_token_plus() {
        let input = vec!['+'];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "+");
        assert_eq!(token.token_type, TokenType::Plus);
    }

    #[test]
    fn scan_next_token_star() {
        let input = vec!['*'];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "*");
        assert_eq!(token.token_type, TokenType::Star);
    }

    #[test]
    fn scan_next_token_slash() {
        let input = vec!['/'];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "/");
        assert_eq!(token.token_type, TokenType::Slash);
    }

    #[test]
    fn scan_next_token_equal() {
        let input = vec!['='];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "=");
        assert_eq!(token.token_type, TokenType::Equal);
    }

    #[test]
    fn scan_next_token_equal_equal() {
        let input = vec!['=', '='];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 2);
        assert_eq!(token.lexeme, "==");
        assert_eq!(token.token_type, TokenType::EqualEqual);
    }

    #[test]
    fn scan_next_token_bang() {
        let input = vec!['!'];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "!");
        assert_eq!(token.token_type, TokenType::Bang);
    }

    #[test]
    fn scan_next_token_bang_equal() {
        let input = vec!['!', '='];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 2);
        assert_eq!(token.lexeme, "!=");
        assert_eq!(token.token_type, TokenType::BangEqual);
    }

    #[test]
    fn scan_next_token_logical_and() {
        let input = vec!['&', '&'];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 2);
        assert_eq!(token.lexeme, "&&");
        assert_eq!(token.token_type, TokenType::LogicalAnd);
    }

    #[test]
    fn scan_next_token_logical_or() {
        let input = vec!['|', '|'];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 2);
        assert_eq!(token.lexeme, "||");
        assert_eq!(token.token_type, TokenType::LogicalOr);
    }

    #[test]
    fn scan_next_token_greater() {
        let input = vec!['>'];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, ">");
        assert_eq!(token.token_type, TokenType::Greater);
    }

    #[test]
    fn scan_next_token_greater_equal() {
        let input = vec!['>', '='];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 2);
        assert_eq!(token.lexeme, ">=");
        assert_eq!(token.token_type, TokenType::GreaterEqual);
    }

    #[test]
    fn scan_next_token_less() {
        let input = vec!['<'];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "<");
        assert_eq!(token.token_type, TokenType::Less);
    }

    #[test]
    fn scan_next_token_less_equal() {
        let input = vec!['<', '='];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 2);
        assert_eq!(token.lexeme, "<=");
        assert_eq!(token.token_type, TokenType::LessEqual);
    }

    #[test]
    fn scan_next_token_true() {
        let input = vec!['t', 'r', 'u', 'e'];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 4);
        assert_eq!(token.lexeme, "true");
        assert_eq!(token.token_type, TokenType::True);
    }

    #[test]
    fn scan_next_token_false() {
        let input = vec!['f', 'a', 'l', 's', 'e'];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 5);
        assert_eq!(token.lexeme, "false");
        assert_eq!(token.token_type, TokenType::False);
    }

    #[test]
    fn scan_next_token_left_paren() {
        let input = vec!['('];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "(");
        assert_eq!(token.token_type, TokenType::LeftParen);
    }

    #[test]
    fn scan_next_token_right_paren() {
        let input = vec![')'];
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

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
        let input = input_str.chars().collect();
        let (_token, end) = super::scan_next_token(&input, 0).unwrap();

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, input_str.len());
        assert_eq!(token.lexeme, input_str);
        assert_eq!(token.token_type, TokenType::Identifier);
    }

    #[test]
    fn scan_next_token_invalid_identifier() {
        let input = vec!['?', 'b', 'c', '_'];
        let (scan_res, end) = super::scan_identifier(&input, 0);

        assert!(scan_res.is_none());
        assert_eq!(end, 0);
    }

    #[test]
    fn parse_into_tokens_math_expr() {
        let tokens = super::parse_into_tokens("2-1+3*4/5").unwrap();

        assert_eq!(tokens.len(), 9);
        assert_eq!(tokens[0].lexeme, "2");
        assert_eq!(tokens[0].token_type, TokenType::Number);
        assert_eq!(tokens[1].lexeme, "-");
        assert_eq!(tokens[1].token_type, TokenType::Minus);
        assert_eq!(tokens[2].lexeme, "1");
        assert_eq!(tokens[2].token_type, TokenType::Number);
        assert_eq!(tokens[3].lexeme, "+");
        assert_eq!(tokens[3].token_type, TokenType::Plus);
        assert_eq!(tokens[4].lexeme, "3");
        assert_eq!(tokens[4].token_type, TokenType::Number);
        assert_eq!(tokens[5].lexeme, "*");
        assert_eq!(tokens[5].token_type, TokenType::Star);
        assert_eq!(tokens[6].lexeme, "4");
        assert_eq!(tokens[6].token_type, TokenType::Number);
        assert_eq!(tokens[7].lexeme, "/");
        assert_eq!(tokens[7].token_type, TokenType::Slash);
        assert_eq!(tokens[8].lexeme, "5");
        assert_eq!(tokens[8].token_type, TokenType::Number);
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
        assert_eq!(tokens[0].token_type, TokenType::Number);
        assert_eq!(tokens[1].token_type, TokenType::Minus);
        assert_eq!(tokens[2].token_type, TokenType::Number);
    }
}
