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

    // Keywords
    If,
    Else,

    // Literals
    Number,
    Identifier,
    True,
    False,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
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

    let chars = input.chars().collect::<Vec<char>>();

    while start < input.len() {
        let (token, next) = scan_next_token(&chars, start)?;
        if token.is_some() {
            tokens.push(token.unwrap());
        }
        start = next;
    }

    Ok(tokens)
}

fn scan_next_token(input: &Vec<char>, current: usize) -> Result<(Option<Token>, usize), Error> {
    match input[current] {
        ' ' | '\t' | '\r' | '\n' => Ok((None, current + 1)),
        '-' => Ok((Some(minus()), current + 1)),
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
            if current + 1 == input.len() {
                panic!("Pipe operator is not supported yet");
                // return Ok((Some(pipe()), current + 1));
            }

            if input[current + 1] == '|' {
                return Ok((Some(logical_or()), current + 2));
            }

            panic!("Pipe operator is not supported yet");
            // Ok((Some(pipe()), current + 1))
        }
        '&' => {
            if current + 1 == input.len() {
                panic!("Sinlge & operator is not supported yet");
                // return Ok((Some(ampersand()), current + 1));
            }

            if input[current + 1] == '&' {
                return Ok((Some(logical_and()), current + 2));
            }

            panic!("Single & operator is not supported yet");
            // Ok((Some(ampersand()), current + 1))
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

fn plus() -> Token {
    Token {
        token_type: TokenType::Plus,
        lexeme: "+".to_string(),
    }
}

fn star() -> Token {
    Token {
        token_type: TokenType::Star,
        lexeme: "*".to_string(),
    }
}

fn slash() -> Token {
    Token {
        token_type: TokenType::Slash,
        lexeme: "/".to_string(),
    }
}

fn left_paren() -> Token {
    Token {
        token_type: TokenType::LeftParen,
        lexeme: "(".to_string(),
    }
}

fn right_paren() -> Token {
    Token {
        token_type: TokenType::RightParen,
        lexeme: ")".to_string(),
    }
}

fn left_brace() -> Token {
    Token {
        token_type: TokenType::LeftBrace,
        lexeme: "{".to_string(),
    }
}

fn right_brace() -> Token {
    Token {
        token_type: TokenType::RightBrace,
        lexeme: "}".to_string(),
    }
}

fn if_kw() -> Token {
    Token {
        token_type: TokenType::If,
        lexeme: "if".to_string(),
    }
}

fn else_kw() -> Token {
    Token {
        token_type: TokenType::Else,
        lexeme: "else".to_string(),
    }
}

fn equal() -> Token {
    Token {
        token_type: TokenType::Equal,
        lexeme: "=".to_string(),
    }
}

fn equal_equal() -> Token {
    Token {
        token_type: TokenType::EqualEqual,
        lexeme: "==".to_string(),
    }
}

fn bang() -> Token {
    Token {
        token_type: TokenType::Bang,
        lexeme: "!".to_string(),
    }
}

fn bang_equal() -> Token {
    Token {
        token_type: TokenType::BangEqual,
        lexeme: "!=".to_string(),
    }
}

fn logical_and() -> Token {
    Token {
        token_type: TokenType::LogicalAnd,
        lexeme: "&&".to_string(),
    }
}

fn logical_or() -> Token {
    Token {
        token_type: TokenType::LogicalOr,
        lexeme: "||".to_string(),
    }
}

fn greater() -> Token {
    Token {
        token_type: TokenType::Greater,
        lexeme: ">".to_string(),
    }
}

fn greater_equal() -> Token {
    Token {
        token_type: TokenType::GreaterEqual,
        lexeme: ">=".to_string(),
    }
}

fn less() -> Token {
    Token {
        token_type: TokenType::Less,
        lexeme: "<".to_string(),
    }
}

fn less_equal() -> Token {
    Token {
        token_type: TokenType::LessEqual,
        lexeme: "<=".to_string(),
    }
}

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
    )
}

fn scan_keyword(input: &[char], start: usize) -> (Option<Token>, usize) {
    let slice = &input[start..];

    // Check for "if" (2 chars)
    if slice.len() >= 2
        && slice.starts_with(&['i', 'f'])
        && (slice.len() == 2 || is_word_boundary(slice[2]))
    {
        return (Some(if_kw()), start + 2);
    }

    // Check for "else" (4 chars)
    if slice.len() >= 4
        && slice.starts_with(&['e', 'l', 's', 'e'])
        && (slice.len() == 4 || is_word_boundary(slice[4]))
    {
        return (Some(else_kw()), start + 4);
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

fn true_l() -> Token {
    Token {
        token_type: TokenType::True,
        lexeme: "true".to_string(),
    }
}

fn false_l() -> Token {
    Token {
        token_type: TokenType::False,
        lexeme: "false".to_string(),
    }
}

fn number(acc: String) -> Token {
    Token {
        token_type: TokenType::Number,
        lexeme: acc,
    }
}

fn identifier(acc: String) -> Token {
    Token {
        token_type: TokenType::Identifier,
        lexeme: acc,
    }
}

fn minus() -> Token {
    Token {
        token_type: TokenType::Minus,
        lexeme: "-".to_string(),
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
