use crate::lexer::{
    cursor::{Cursor, EOF_CHAR},
    token::{Keyword, SyntaxKind, Token, Trivia},
};

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        let is_eof = token.kind == SyntaxKind::EndOfFile;
        tokens.push(token);
        if is_eof {
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
                kind: SyntaxKind::EndOfFile,
                len: 0,
            };
        };

        let kind = match c {
            c if is_whitespace(c) => {
                self.cursor.eat_while(is_whitespace);
                SyntaxKind::Trivia(Trivia::Whitespace)
            }

            '/' if self.cursor.peek() == '/' => self.line_comment(),
            '/' if self.cursor.peek() == '*' => self.block_comment(),

            c if is_ident_start(c) => self.ident(),
            c if c.is_ascii_digit() => self.number(),
            '"' => self.string(),

            '+' => self.one_or_two('+', SyntaxKind::PlusPlus, SyntaxKind::Plus),
            '-' => match self.cursor.peek() {
                '-' => {
                    self.cursor.bump();
                    SyntaxKind::MinusMinus
                }
                '>' => {
                    self.cursor.bump();
                    SyntaxKind::ArrowRight
                }
                _ => SyntaxKind::Minus,
            },
            '*' => self.one_or_two('*', SyntaxKind::StarStar, SyntaxKind::Star),
            '/' => SyntaxKind::Slash,
            '%' => SyntaxKind::Percent,
            '~' => SyntaxKind::Tilde,
            '^' => SyntaxKind::Caret,

            '&' => self.one_or_two('&', SyntaxKind::AmpersandAmpersand, SyntaxKind::Ampersand),
            '|' => self.one_or_two('|', SyntaxKind::PipePipe, SyntaxKind::Pipe),

            '!' => self.one_or_two('=', SyntaxKind::BangEqual, SyntaxKind::Bang),
            '=' => self.one_or_two('=', SyntaxKind::EqualEqual, SyntaxKind::Equal),

            '<' => match self.cursor.peek() {
                '<' => {
                    self.cursor.bump();
                    SyntaxKind::ShiftLeft
                }
                '=' => {
                    self.cursor.bump();
                    SyntaxKind::LessThanOrEqual
                }
                '-' => {
                    self.cursor.bump();
                    SyntaxKind::ArrowLeft
                }
                _ => SyntaxKind::LessThan,
            },
            '>' => match self.cursor.peek() {
                '>' if self.cursor.peek_second() == '>' => {
                    self.cursor.bump();
                    self.cursor.bump();
                    SyntaxKind::ShiftRightShiftRight
                }
                '>' => {
                    self.cursor.bump();
                    SyntaxKind::ShiftRight
                }
                '=' => {
                    self.cursor.bump();
                    SyntaxKind::GreaterThanOrEqual
                }
                _ => SyntaxKind::GreaterThan,
            },

            ':' => self.one_or_two(':', SyntaxKind::ColonColon, SyntaxKind::Colon),
            ',' => SyntaxKind::Comma,
            ';' => SyntaxKind::Semicolon,
            '.' => SyntaxKind::Dot,
            '?' => SyntaxKind::Question,
            '(' => SyntaxKind::ParenthesisLeft,
            ')' => SyntaxKind::ParenthesisRight,
            '[' => SyntaxKind::BracketLeft,
            ']' => SyntaxKind::BracketRight,
            '{' => SyntaxKind::BraceLeft,
            '}' => SyntaxKind::BraceRight,

            _ => SyntaxKind::Error,
        };

        let len = self.cursor.len_consumed();
        self.pos += len;

        let kind = if kind == SyntaxKind::Ident {
            let text = &self.input[start as usize..(start + len) as usize];
            Keyword::try_from(text)
                .map(SyntaxKind::from)
                .unwrap_or(SyntaxKind::Ident)
        } else {
            kind
        };

        Token { kind, len }
    }

    fn one_or_two(&mut self, next: char, two: SyntaxKind, one: SyntaxKind) -> SyntaxKind {
        if self.cursor.peek() == next {
            self.cursor.bump();
            two
        } else {
            one
        }
    }

    fn line_comment(&mut self) -> SyntaxKind {
        debug_assert_eq!(self.cursor.peek(), '/');
        self.cursor.bump();
        self.cursor.eat_while(|c| c != '\n');
        SyntaxKind::Trivia(Trivia::LineComment)
    }

    fn block_comment(&mut self) -> SyntaxKind {
        debug_assert_eq!(self.cursor.peek(), '*');
        self.cursor.bump();

        loop {
            match self.cursor.bump() {
                None => return SyntaxKind::Error,
                Some('*') if self.cursor.peek() == '/' => {
                    self.cursor.bump();
                    return SyntaxKind::Trivia(Trivia::BlockComment);
                }
                _ => {}
            }
        }
    }

    fn ident(&mut self) -> SyntaxKind {
        self.cursor.eat_while(is_ident_continue);
        SyntaxKind::Ident
    }

    fn number(&mut self) -> SyntaxKind {
        let mut kind = SyntaxKind::IntLiteral;

        if self.cursor.peek() == 'x' || self.cursor.peek() == 'X' {
            self.cursor.bump();
            self.cursor.eat_while(|c| c.is_ascii_hexdigit());
        } else if self.cursor.peek() == 'b' || self.cursor.peek() == 'B' {
            self.cursor.bump();
            self.cursor.eat_while(|c| c == '0' || c == '1');
        } else {
            self.cursor.eat_while(|c| c.is_ascii_digit());

            if self.cursor.peek() == '.' && self.cursor.peek_second().is_ascii_digit() {
                kind = SyntaxKind::FloatLiteral;
                self.cursor.bump();
                self.cursor.eat_while(|c| c.is_ascii_digit());
            }
        }

        self.cursor
            .eat_while(|c| c.is_ascii_alphanumeric() || c == '_');

        kind
    }

    fn string(&mut self) -> SyntaxKind {
        loop {
            match self.cursor.bump() {
                None | Some('\n') => return SyntaxKind::Error,
                Some('"') => return SyntaxKind::StringLiteral,
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

    fn lex(input: &str) -> Vec<(SyntaxKind, &str)> {
        let mut pos = 0usize;
        tokenize(input)
            .into_iter()
            .filter(|token| token.kind != SyntaxKind::EndOfFile)
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
                kind: SyntaxKind::EndOfFile,
                len: 0
            }]
        );
    }

    #[test]
    fn keywords_vs_identifiers() {
        assert_eq!(
            lex("processor foo"),
            vec![
                (SyntaxKind::Keyword(Keyword::Processor), "processor"),
                (SyntaxKind::Trivia(Trivia::Whitespace), " "),
                (SyntaxKind::Ident, "foo"),
            ]
        );
    }

    #[test]
    fn wrap_and_clamp_are_keywords() {
        assert_eq!(
            lex("wrap<4>"),
            vec![
                (SyntaxKind::Keyword(Keyword::Wrap), "wrap"),
                (SyntaxKind::LessThan, "<"),
                (SyntaxKind::IntLiteral, "4"),
                (SyntaxKind::GreaterThan, ">"),
            ]
        );
    }

    #[test]
    fn integer_literals() {
        assert_eq!(
            lex("-12345").last(),
            Some(&(SyntaxKind::IntLiteral, "12345"))
        );
        assert_eq!(lex("0x12345")[0], (SyntaxKind::IntLiteral, "0x12345"));
        assert_eq!(lex("0b101101")[0], (SyntaxKind::IntLiteral, "0b101101"));
        assert_eq!(lex("12345L")[0], (SyntaxKind::IntLiteral, "12345L"));
        assert_eq!(lex("12345_i64")[0], (SyntaxKind::IntLiteral, "12345_i64"));
        assert_eq!(lex("0x12345_L")[0], (SyntaxKind::IntLiteral, "0x12345_L"));
    }

    #[test]
    fn float_literals() {
        assert_eq!(lex("1234.0")[0], (SyntaxKind::FloatLiteral, "1234.0"));
        assert_eq!(
            lex("1234.0_f64")[0],
            (SyntaxKind::FloatLiteral, "1234.0_f64")
        );
        assert_eq!(lex("1234.0f")[0], (SyntaxKind::FloatLiteral, "1234.0f"));
        assert_eq!(lex("123.0i")[0], (SyntaxKind::FloatLiteral, "123.0i"));
        assert_eq!(lex("123.0fi")[0], (SyntaxKind::FloatLiteral, "123.0fi"));
    }

    #[test]
    fn string_literal_with_escapes() {
        assert_eq!(
            lex(r#""Hello\n World\n 😀""#),
            vec![(SyntaxKind::StringLiteral, r#""Hello\n World\n 😀""#)]
        );
    }

    #[test]
    fn unterminated_string_is_an_error() {
        assert_eq!(lex("\"abc")[0].0, SyntaxKind::Error);
    }

    #[test]
    fn line_and_block_comments() {
        assert_eq!(
            lex("// comment\n/* block */"),
            vec![
                (SyntaxKind::Trivia(Trivia::LineComment), "// comment"),
                (SyntaxKind::Trivia(Trivia::Whitespace), "\n"),
                (SyntaxKind::Trivia(Trivia::BlockComment), "/* block */"),
            ]
        );
    }

    #[test]
    fn unterminated_block_comment_is_an_error() {
        assert_eq!(lex("/* comment")[0].0, SyntaxKind::Error);
    }

    #[test]
    fn operators_use_maximal_munch() {
        assert_eq!(
            lex("a->b<-c>>>d"),
            vec![
                (SyntaxKind::Ident, "a"),
                (SyntaxKind::ArrowRight, "->"),
                (SyntaxKind::Ident, "b"),
                (SyntaxKind::ArrowLeft, "<-"),
                (SyntaxKind::Ident, "c"),
                (SyntaxKind::ShiftRightShiftRight, ">>>"),
                (SyntaxKind::Ident, "d"),
            ]
        );
    }

    #[test]
    fn ternary_and_range_slice_tokens() {
        assert_eq!(
            lex("b ? x[3:5] : y"),
            vec![
                (SyntaxKind::Ident, "b"),
                (SyntaxKind::Trivia(Trivia::Whitespace), " "),
                (SyntaxKind::Question, "?"),
                (SyntaxKind::Trivia(Trivia::Whitespace), " "),
                (SyntaxKind::Ident, "x"),
                (SyntaxKind::BracketLeft, "["),
                (SyntaxKind::IntLiteral, "3"),
                (SyntaxKind::Colon, ":"),
                (SyntaxKind::IntLiteral, "5"),
                (SyntaxKind::BracketRight, "]"),
                (SyntaxKind::Trivia(Trivia::Whitespace), " "),
                (SyntaxKind::Colon, ":"),
                (SyntaxKind::Trivia(Trivia::Whitespace), " "),
                (SyntaxKind::Ident, "y"),
            ]
        );
    }

    #[test]
    fn namespace_path() {
        assert_eq!(
            lex("std::intrinsics::sin"),
            vec![
                (SyntaxKind::Ident, "std"),
                (SyntaxKind::ColonColon, "::"),
                (SyntaxKind::Ident, "intrinsics"),
                (SyntaxKind::ColonColon, "::"),
                (SyntaxKind::Ident, "sin"),
            ]
        );
    }
}
