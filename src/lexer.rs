use log::error;

#[derive(Debug, PartialEq)]
pub enum TokenType {
    // Single character tokens
    // LeftPar,
    // RightPar,
    Minus,
    Plus,
    Star,
    Slash,

    // Literals
    Number,
}

#[derive(Debug, PartialEq)]
pub struct Token {
    token_type: TokenType,
    lexeme: String,
}

fn parse_into_tokens(input: &str) -> Vec<Token> {
    let mut tokens = vec![];

    let mut start = 0;

    let chars = input.chars().collect::<Vec<char>>();

    while start < input.len() {
        let (token, next) = scan_next_token(&chars, start);
        if token.is_some() {
            tokens.push(token.unwrap());
        }
        start = next;
    }

    tokens
}

fn scan_next_token(input: &Vec<char>, current: usize) -> (Option<Token>, usize) {
    match input[current] {
        ' ' | '\t' | '\r' => (None, current + 1),
        // what to do with \n ???
        '-' => (Some(minus()), current + 1),
        '+' => (Some(plus()), current + 1),
        '*' => (Some(star()), current + 1),
        '/' => (Some(slash()), current + 1),
        other => {
            if other.is_numeric() {
                return scan_number(input, current);
            }

            log_lexer_error(input, current, other);

            panic!();
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

fn scan_number(input: &[char], start: usize) -> (Option<Token>, usize) {
    let mut current = start;
    let mut acc = String::with_capacity(input.len());
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

fn number(acc: String) -> Token {
    Token {
        token_type: TokenType::Number,
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
        let (_token, end) = super::scan_next_token(&input, 0);

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 3);
        assert_eq!(token.lexeme, "237");
        assert_eq!(token.token_type, TokenType::Number);
    }

    #[test]
    fn scan_next_token_minus() {
        let input = vec!['-'];
        let (_token, end) = super::scan_next_token(&input, 0);

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "-");
        assert_eq!(token.token_type, TokenType::Minus);
    }

    #[test]
    fn scan_next_token_plus() {
        let input = vec!['+'];
        let (_token, end) = super::scan_next_token(&input, 0);

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "+");
        assert_eq!(token.token_type, TokenType::Plus);
    }

    #[test]
    fn scan_next_token_star() {
        let input = vec!['*'];
        let (_token, end) = super::scan_next_token(&input, 0);

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "*");
        assert_eq!(token.token_type, TokenType::Star);
    }

    #[test]
    fn scan_next_token_slash() {
        let input = vec!['/'];
        let (_token, end) = super::scan_next_token(&input, 0);

        assert!(_token.is_some());
        let token = _token.unwrap();
        assert_eq!(end, 1);
        assert_eq!(token.lexeme, "/");
        assert_eq!(token.token_type, TokenType::Slash);
    }

    #[test]
    fn parse_into_tokens_math_expr() {
        let tokens = super::parse_into_tokens("2-1+3*4/5");

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
        let tokens = super::parse_into_tokens(input);

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].token_type, TokenType::Number);
        assert_eq!(tokens[1].token_type, TokenType::Minus);
        assert_eq!(tokens[2].token_type, TokenType::Number);
    }
}
