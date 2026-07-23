use crate::lexer::{
    cursor::{Cursor, EOF_CHAR},
    token::{Keyword, Token, TokenKind, Trivia},
};

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        tokens.push(token);
        if token.kind == TokenKind::EndOfFile {
            break;
        }
    }
    tokens
}

struct Lexer<'a> {
    input: &'a str,
    pos: u32,
    cursor: Cursor<'a>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            cursor: Cursor::new(input),
        }
    }

    fn next_token(&mut self) -> Token {
        self.cursor.reset_len_consumed();
        let start = self.pos;

        let Some(c) = self.cursor.bump() else {
            return Token {
                kind: TokenKind::EndOfFile,
                len: 0,
            };
        };

        let kind = match c {
            c if is_whitespace(c) => {
                self.cursor.eat_while(is_whitespace);
                TokenKind::Trivia(Trivia::Whitespace)
            }

            '/' if self.cursor.peek() == '/' => self.line_comment(),
            '/' if self.cursor.peek() == '*' => self.block_comment(),

            c if is_ident_start(c) => self.ident(),
            c if c.is_ascii_digit() => self.number(),
            '"' => self.string(),

            '+' => self.one_or_two('+', TokenKind::PlusPlus, TokenKind::Plus),
            '-' => match self.cursor.peek() {
                '-' => {
                    self.cursor.bump();
                    TokenKind::MinusMinus
                }
                '>' => {
                    self.cursor.bump();
                    TokenKind::ArrowRight
                }
                _ => TokenKind::Minus,
            },
            '*' => self.one_or_two('*', TokenKind::StarStar, TokenKind::Star),
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '~' => TokenKind::Tilde,
            '^' => TokenKind::Caret,

            '&' => self.one_or_two('&', TokenKind::AmpersandAmpersand, TokenKind::Ampersand),
            '|' => self.one_or_two('|', TokenKind::PipePipe, TokenKind::Pipe),

            '!' => self.one_or_two('=', TokenKind::BangEqual, TokenKind::Bang),
            '=' => self.one_or_two('=', TokenKind::EqualEqual, TokenKind::Equal),

            '<' => match self.cursor.peek() {
                '<' => {
                    self.cursor.bump();
                    TokenKind::ShiftLeft
                }
                '=' => {
                    self.cursor.bump();
                    TokenKind::LessThanOrEqual
                }
                '-' => {
                    self.cursor.bump();
                    TokenKind::ArrowLeft
                }
                _ => TokenKind::LessThan,
            },
            '>' => match self.cursor.peek() {
                '>' if self.cursor.peek_second() == '>' => {
                    self.cursor.bump();
                    self.cursor.bump();
                    TokenKind::ShiftRightShiftRight
                }
                '>' => {
                    self.cursor.bump();
                    TokenKind::ShiftRight
                }
                '=' => {
                    self.cursor.bump();
                    TokenKind::GreaterThanOrEqual
                }
                _ => TokenKind::GreaterThan,
            },

            ':' => self.one_or_two(':', TokenKind::ColonColon, TokenKind::Colon),
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            '.' => TokenKind::Dot,
            '?' => TokenKind::Question,
            '(' => TokenKind::ParenthesisLeft,
            ')' => TokenKind::ParenthesisRight,
            '[' => TokenKind::BracketLeft,
            ']' => TokenKind::BracketRight,
            '{' => TokenKind::BraceLeft,
            '}' => TokenKind::BraceRight,
            _ => TokenKind::Error,
        };

        let len = self.cursor.len_consumed();
        self.pos += len;

        let kind = if kind == TokenKind::Ident {
            let text = &self.input[start as usize..(start + len) as usize];
            Keyword::try_from(text)
                .map(TokenKind::from)
                .unwrap_or(TokenKind::Ident)
        } else {
            kind
        };

        Token { kind, len }
    }

    fn one_or_two(&mut self, next: char, two: TokenKind, one: TokenKind) -> TokenKind {
        if self.cursor.peek() == next {
            self.cursor.bump();
            two
        } else {
            one
        }
    }

    fn line_comment(&mut self) -> TokenKind {
        debug_assert_eq!(self.cursor.peek(), '/');
        self.cursor.bump();
        self.cursor.eat_while(|c| c != '\n');
        TokenKind::Trivia(Trivia::LineComment)
    }

    fn block_comment(&mut self) -> TokenKind {
        debug_assert_eq!(self.cursor.peek(), '*');
        self.cursor.bump();

        loop {
            match self.cursor.bump() {
                None => return TokenKind::Error,
                Some('*') if self.cursor.peek() == '/' => {
                    self.cursor.bump();
                    return TokenKind::Trivia(Trivia::BlockComment);
                }
                _ => {}
            }
        }
    }

    fn ident(&mut self) -> TokenKind {
        self.cursor.eat_while(is_ident_continue);
        TokenKind::Ident
    }

    fn number(&mut self) -> TokenKind {
        let mut kind = TokenKind::IntLiteral;

        if self.cursor.peek() == 'x' || self.cursor.peek() == 'X' {
            self.cursor.bump();
            self.cursor.eat_while(|c| c.is_ascii_hexdigit());
        } else if self.cursor.peek() == 'b' || self.cursor.peek() == 'B' {
            self.cursor.bump();
            self.cursor.eat_while(|c| c == '0' || c == '1');
        } else {
            self.cursor.eat_while(|c| c.is_ascii_digit());

            if self.cursor.peek() == '.' && self.cursor.peek_second().is_ascii_digit() {
                kind = TokenKind::FloatLiteral;
                self.cursor.bump();
                self.cursor.eat_while(|c| c.is_ascii_digit());
            }
        }

        self.cursor
            .eat_while(|c| c.is_ascii_alphanumeric() || c == '_');

        kind
    }

    fn string(&mut self) -> TokenKind {
        loop {
            match self.cursor.bump() {
                None | Some('\n') => return TokenKind::Error,
                Some('"') => return TokenKind::StringLiteral,
                Some('\\') if self.cursor.peek() != EOF_CHAR => {
                    self.cursor.bump();
                }
                _ => {}
            }
        }
    }
}

fn is_whitespace(c: char) -> bool {
    c.is_whitespace()
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use {super::*, crate::lexer::token::Keyword};

    fn lex(input: &str) -> Vec<(TokenKind, &str)> {
        let mut pos = 0usize;
        tokenize(input)
            .into_iter()
            .filter(|token| token.kind != TokenKind::EndOfFile)
            .map(|token| {
                let start = pos;
                pos += token.len as usize;
                (token.kind, &input[start..pos])
            })
            .collect()
    }

    #[test]
    fn empty_input_is_just_eof() {
        assert_eq!(
            tokenize(""),
            vec![Token {
                kind: TokenKind::EndOfFile,
                len: 0
            }]
        );
    }

    #[test]
    fn keywords_vs_identifiers() {
        assert_eq!(
            lex("processor foo"),
            vec![
                (TokenKind::Keyword(Keyword::Processor), "processor"),
                (TokenKind::Trivia(Trivia::Whitespace), " "),
                (TokenKind::Ident, "foo"),
            ]
        );
    }

    #[test]
    fn wrap_and_clamp_are_keywords() {
        assert_eq!(
            lex("wrap<4>"),
            vec![
                (TokenKind::Keyword(Keyword::Wrap), "wrap"),
                (TokenKind::LessThan, "<"),
                (TokenKind::IntLiteral, "4"),
                (TokenKind::GreaterThan, ">"),
            ]
        );
    }

    #[test]
    fn integer_literals() {
        assert_eq!(
            lex("-12345").last(),
            Some(&(TokenKind::IntLiteral, "12345"))
        );
        assert_eq!(lex("0x12345")[0], (TokenKind::IntLiteral, "0x12345"));
        assert_eq!(lex("0b101101")[0], (TokenKind::IntLiteral, "0b101101"));
        assert_eq!(lex("12345L")[0], (TokenKind::IntLiteral, "12345L"));
        assert_eq!(lex("12345_i64")[0], (TokenKind::IntLiteral, "12345_i64"));
        assert_eq!(lex("0x12345_L")[0], (TokenKind::IntLiteral, "0x12345_L"));
    }

    #[test]
    fn float_literals() {
        assert_eq!(lex("1234.0")[0], (TokenKind::FloatLiteral, "1234.0"));
        assert_eq!(
            lex("1234.0_f64")[0],
            (TokenKind::FloatLiteral, "1234.0_f64")
        );
        assert_eq!(lex("1234.0f")[0], (TokenKind::FloatLiteral, "1234.0f"));
        assert_eq!(lex("123.0i")[0], (TokenKind::FloatLiteral, "123.0i"));
        assert_eq!(lex("123.0fi")[0], (TokenKind::FloatLiteral, "123.0fi"));
    }

    #[test]
    fn string_literal_with_escapes() {
        assert_eq!(
            lex(r#""Hello\n World\n 😀""#),
            vec![(TokenKind::StringLiteral, r#""Hello\n World\n 😀""#)]
        );
    }

    #[test]
    fn unterminated_string_is_an_error() {
        assert_eq!(lex("\"abc")[0].0, TokenKind::Error);
    }

    #[test]
    fn line_and_block_comments() {
        assert_eq!(
            lex("// comment\n/* block */"),
            vec![
                (TokenKind::Trivia(Trivia::LineComment), "// comment"),
                (TokenKind::Trivia(Trivia::Whitespace), "\n"),
                (TokenKind::Trivia(Trivia::BlockComment), "/* block */"),
            ]
        );
    }

    #[test]
    fn unterminated_block_comment_is_an_error() {
        assert_eq!(lex("/* comment")[0].0, TokenKind::Error);
    }

    #[test]
    fn operators_use_maximal_munch() {
        assert_eq!(
            lex("a->b<-c>>>d"),
            vec![
                (TokenKind::Ident, "a"),
                (TokenKind::ArrowRight, "->"),
                (TokenKind::Ident, "b"),
                (TokenKind::ArrowLeft, "<-"),
                (TokenKind::Ident, "c"),
                (TokenKind::ShiftRightShiftRight, ">>>"),
                (TokenKind::Ident, "d"),
            ]
        );
    }

    #[test]
    fn ternary_and_range_slice_tokens() {
        assert_eq!(
            lex("b ? x[3:5] : y"),
            vec![
                (TokenKind::Ident, "b"),
                (TokenKind::Trivia(Trivia::Whitespace), " "),
                (TokenKind::Question, "?"),
                (TokenKind::Trivia(Trivia::Whitespace), " "),
                (TokenKind::Ident, "x"),
                (TokenKind::BracketLeft, "["),
                (TokenKind::IntLiteral, "3"),
                (TokenKind::Colon, ":"),
                (TokenKind::IntLiteral, "5"),
                (TokenKind::BracketRight, "]"),
                (TokenKind::Trivia(Trivia::Whitespace), " "),
                (TokenKind::Colon, ":"),
                (TokenKind::Trivia(Trivia::Whitespace), " "),
                (TokenKind::Ident, "y"),
            ]
        );
    }

    #[test]
    fn namespace_path() {
        assert_eq!(
            lex("std::intrinsics::sin"),
            vec![
                (TokenKind::Ident, "std"),
                (TokenKind::ColonColon, "::"),
                (TokenKind::Ident, "intrinsics"),
                (TokenKind::ColonColon, "::"),
                (TokenKind::Ident, "sin"),
            ]
        );
    }
}
