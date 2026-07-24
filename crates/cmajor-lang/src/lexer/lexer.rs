use crate::lexer::{
    cursor::Cursor,
    token::{Keyword, Literal, Token, TokenKind, Trivia},
};

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token() {
        tokens.push(token);
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

    fn next_token(&mut self) -> Option<Token> {
        self.cursor.reset_bytes_taken();
        let start = self.pos;

        let c = self.cursor.take()?;

        let kind = match c {
            c if is_whitespace(c) => {
                self.cursor.take_while(is_whitespace);
                TokenKind::Trivia(Trivia::Whitespace)
            }
            '/' if self.cursor.peek() == Some('/') => self.line_comment(),
            '/' if self.cursor.peek() == Some('*') => self.block_comment(),
            c if is_ident_start(c) => self.ident(),
            c if c.is_ascii_digit() => self.number(),
            '"' => self.string(),
            '+' => match self.cursor.peek() {
                Some('+') => {
                    self.cursor.take();
                    TokenKind::PlusPlus
                }
                Some('=') => {
                    self.cursor.take();
                    TokenKind::PlusEqual
                }
                _ => TokenKind::Plus,
            },
            '-' => match self.cursor.peek() {
                Some('-') => {
                    self.cursor.take();
                    TokenKind::MinusMinus
                }
                Some('>') => {
                    self.cursor.take();
                    TokenKind::ArrowRight
                }
                Some('=') => {
                    self.cursor.take();
                    TokenKind::MinusEqual
                }
                _ => TokenKind::Minus,
            },
            '*' => match self.cursor.peek() {
                Some('*') => {
                    self.cursor.take();
                    TokenKind::StarStar
                }
                Some('=') => {
                    self.cursor.take();
                    TokenKind::StarEqual
                }
                _ => TokenKind::Star,
            },
            '/' => self.one_or_two('=', TokenKind::SlashEqual, TokenKind::Slash),
            '%' => self.one_or_two('=', TokenKind::PercentEqual, TokenKind::Percent),
            '~' => TokenKind::Tilde,
            '^' => self.one_or_two('=', TokenKind::CaretEqual, TokenKind::Caret),
            '&' if self.cursor.peek_at(0) == Some('&') && self.cursor.peek_at(1) == Some('=') => {
                self.cursor.take();
                self.cursor.take();
                TokenKind::AmpersandAmpersandEqual
            }
            '&' => match self.cursor.peek() {
                Some('&') => {
                    self.cursor.take();
                    TokenKind::AmpersandAmpersand
                }
                Some('=') => {
                    self.cursor.take();
                    TokenKind::AmpersandEqual
                }
                _ => TokenKind::Ampersand,
            },
            '|' if self.cursor.peek_at(0) == Some('|') && self.cursor.peek_at(1) == Some('=') => {
                self.cursor.take();
                self.cursor.take();
                TokenKind::PipePipeEqual
            }
            '|' => match self.cursor.peek() {
                Some('|') => {
                    self.cursor.take();
                    TokenKind::PipePipe
                }
                Some('=') => {
                    self.cursor.take();
                    TokenKind::PipeEqual
                }
                _ => TokenKind::Pipe,
            },
            '!' => self.one_or_two('=', TokenKind::BangEqual, TokenKind::Bang),
            '=' => self.one_or_two('=', TokenKind::EqualEqual, TokenKind::Equal),
            '<' => match self.cursor.peek() {
                Some('<') if self.cursor.peek_two() == (Some('<'), Some('=')) => {
                    self.cursor.take();
                    self.cursor.take();
                    TokenKind::ShiftLeftEqual
                }
                Some('<') => {
                    self.cursor.take();
                    TokenKind::ShiftLeft
                }
                Some('=') => {
                    self.cursor.take();
                    TokenKind::LessThanOrEqual
                }
                Some('-') => {
                    self.cursor.take();
                    TokenKind::ArrowLeft
                }
                _ => TokenKind::LessThan,
            },
            '>' if self.cursor.peek_at(0) == Some('>')
                && self.cursor.peek_at(1) == Some('>')
                && self.cursor.peek_at(2) == Some('=') =>
            {
                self.cursor.take();
                self.cursor.take();
                self.cursor.take();
                TokenKind::ShiftRightShiftRightEqual
            }
            '>' if self.cursor.peek_at(0) == Some('>') && self.cursor.peek_at(1) == Some('>') => {
                self.cursor.take();
                self.cursor.take();
                TokenKind::ShiftRightShiftRight
            }
            '>' if self.cursor.peek_at(0) == Some('>') && self.cursor.peek_at(1) == Some('=') => {
                self.cursor.take();
                self.cursor.take();
                TokenKind::ShiftRightEqual
            }
            '>' => match self.cursor.peek() {
                Some('>') => {
                    self.cursor.take();
                    TokenKind::ShiftRight
                }
                Some('=') => {
                    self.cursor.take();
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
            '[' => self.one_or_two('[', TokenKind::DoubleBracketLeft, TokenKind::BracketLeft),
            ']' => self.one_or_two(']', TokenKind::DoubleBracketRight, TokenKind::BracketRight),
            '{' => TokenKind::BraceLeft,
            '}' => TokenKind::BraceRight,
            _ => TokenKind::Error,
        };

        let len = self.cursor.bytes_taken();
        self.pos += len;

        let kind = if kind == TokenKind::Identifier {
            let text = &self.input[start as usize..(start + len) as usize];
            Keyword::try_from(text)
                .map(TokenKind::from)
                .unwrap_or(TokenKind::Identifier)
        } else {
            kind
        };

        Some(Token { kind, len })
    }

    fn one_or_two(&mut self, next: char, two: TokenKind, one: TokenKind) -> TokenKind {
        if self.cursor.peek() == Some(next) {
            self.cursor.take();
            two
        } else {
            one
        }
    }

    fn line_comment(&mut self) -> TokenKind {
        debug_assert_eq!(self.cursor.peek(), Some('/'));
        self.cursor.take();
        self.cursor.take_while(|c| c != '\n');
        TokenKind::Trivia(Trivia::LineComment)
    }

    fn block_comment(&mut self) -> TokenKind {
        debug_assert_eq!(self.cursor.peek(), Some('*'));
        self.cursor.take();

        loop {
            match self.cursor.take() {
                None => return TokenKind::Error,
                Some('*') if self.cursor.peek() == Some('/') => {
                    self.cursor.take();
                    return TokenKind::Trivia(Trivia::BlockComment);
                }
                _ => {}
            }
        }
    }

    fn ident(&mut self) -> TokenKind {
        self.cursor.take_while(is_ident_continue);
        TokenKind::Identifier
    }

    fn number(&mut self) -> TokenKind {
        let kind = match self.cursor.peek() {
            Some('x' | 'X') => {
                self.cursor.take();
                self.cursor.take_while(|c| c.is_ascii_hexdigit());
                Literal::Int32
            }
            Some('b' | 'B') => {
                self.cursor.take();
                self.cursor.take_while(is_binary_digit);
                Literal::Int32
            }
            _ => {
                self.cursor.take_while(|c| c.is_ascii_digit());

                if self.cursor.peek() == Some('.') {
                    self.cursor.take();
                    self.cursor.take_while(|c| c.is_ascii_digit());
                    Literal::Float64
                } else {
                    Literal::Int32
                }
            }
        };

        if let Some('_') = self.cursor.peek() {
            self.cursor.take();
        }

        let kind = match self.cursor.peek_three() {
            (Some('f'), Some('3'), Some('2')) => {
                self.cursor.take_n(3);
                Literal::Float32
            }
            (Some('f'), Some('6'), Some('4')) => {
                self.cursor.take_n(3);
                Literal::Float64
            }
            (Some('f'), Some('i'), _) => {
                self.cursor.take_n(2);
                Literal::Imaginary32
            }
            (Some('f'), _, _) => {
                self.cursor.take();
                Literal::Float32
            }
            (Some('i'), Some('6'), Some('4')) => {
                self.cursor.take_n(3);
                Literal::Int64
            }
            (Some('i'), _, _) => {
                self.cursor.take();
                Literal::Imaginary64
            }
            (Some('L'), _, _) => {
                self.cursor.take();
                Literal::Int64
            }
            _ => kind,
        };

        TokenKind::Literal(kind)
    }

    fn string(&mut self) -> TokenKind {
        loop {
            match self.cursor.take() {
                None | Some('\n') => return TokenKind::Error,
                Some('"') => return TokenKind::Literal(Literal::String),
                Some('\\') if self.cursor.peek().is_some() => {
                    self.cursor.take();
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

fn is_binary_digit(c: char) -> bool {
    c == '0' || c == '1'
}

#[cfg(test)]
mod tests {
    use {super::*, crate::lexer::token::Keyword};

    fn lex(input: &str) -> Vec<(TokenKind, &str)> {
        let mut pos = 0usize;
        tokenize(input)
            .into_iter()
            .map(|token| {
                let start = pos;
                pos += token.len as usize;
                (token.kind, &input[start..pos])
            })
            .collect()
    }

    #[test]
    fn empty_input_produces_no_tokens() {
        assert_eq!(tokenize(""), vec![]);
    }

    #[test]
    fn keywords_vs_identifiers() {
        assert_eq!(
            lex("processor foo"),
            vec![
                (TokenKind::Keyword(Keyword::Processor), "processor"),
                (TokenKind::Trivia(Trivia::Whitespace), " "),
                (TokenKind::Identifier, "foo"),
            ]
        );
    }

    #[test]
    fn wrap_and_clamp_are_not_keywords() {
        assert_eq!(
            lex("wrap<4>"),
            vec![
                (TokenKind::Identifier, "wrap"),
                (TokenKind::LessThan, "<"),
                (TokenKind::Literal(Literal::Int32), "4"),
                (TokenKind::GreaterThan, ">"),
            ]
        );
    }

    #[test]
    fn integer_literals() {
        assert_eq!(
            lex("12345"),
            vec![(TokenKind::Literal(Literal::Int32), "12345")]
        );
        assert_eq!(
            lex("-12345"),
            vec![
                (TokenKind::Minus, "-"),
                (TokenKind::Literal(Literal::Int32), "12345")
            ]
        );
        assert_eq!(
            lex("0x12345"),
            vec![(TokenKind::Literal(Literal::Int32), "0x12345")]
        );
        assert_eq!(
            lex("0b101101"),
            vec![(TokenKind::Literal(Literal::Int32), "0b101101")]
        );
        assert_eq!(
            lex("12345L"),
            vec![(TokenKind::Literal(Literal::Int64), "12345L")]
        );
        assert_eq!(
            lex("0x12345_L"),
            vec![(TokenKind::Literal(Literal::Int64), "0x12345_L")]
        );
        assert_eq!(
            lex("12345i64"),
            vec![(TokenKind::Literal(Literal::Int64), "12345i64")]
        );
        assert_eq!(
            lex("12345_i64"),
            vec![(TokenKind::Literal(Literal::Int64), "12345_i64")]
        );
    }

    #[test]
    fn float_literals() {
        assert_eq!(
            lex("0.0"),
            vec![(TokenKind::Literal(Literal::Float64), "0.0")]
        );
        assert_eq!(
            lex("1234.0"),
            vec![(TokenKind::Literal(Literal::Float64), "1234.0")]
        );
        assert_eq!(
            lex("1234.0_f64"),
            vec![(TokenKind::Literal(Literal::Float64), "1234.0_f64")]
        );
        assert_eq!(
            lex("1234.0f"),
            vec![(TokenKind::Literal(Literal::Float32), "1234.0f")]
        );
        assert_eq!(
            lex("1234.0_f32"),
            vec![(TokenKind::Literal(Literal::Float32), "1234.0_f32")]
        )
    }

    #[test]
    fn complex_literals() {
        assert_eq!(
            lex("123.0i"),
            vec![(TokenKind::Literal(Literal::Imaginary64), "123.0i")]
        );
        assert_eq!(
            lex("123.0fi"),
            vec![(TokenKind::Literal(Literal::Imaginary32), "123.0fi")]
        );
        assert_eq!(
            lex("123.0_i"),
            vec![(TokenKind::Literal(Literal::Imaginary64), "123.0_i")]
        );
        assert_eq!(
            lex("123.0_fi"),
            vec![(TokenKind::Literal(Literal::Imaginary32), "123.0_fi")]
        );
    }

    #[test]
    fn string_literal_with_escapes() {
        assert_eq!(
            lex(r#""Hello\n World\n 😀""#),
            vec![(
                TokenKind::Literal(Literal::String),
                r#""Hello\n World\n 😀""#
            )]
        );
    }

    #[test]
    fn unterminated_string_is_an_error() {
        assert_eq!(lex("\"abc"), vec![(TokenKind::Error, "\"abc")]);
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
        assert_eq!(lex("/* comment"), vec![(TokenKind::Error, "/* comment")]);
    }

    #[test]
    fn operators_use_maximal_munch() {
        assert_eq!(
            lex("a->b<-c>>>d"),
            vec![
                (TokenKind::Identifier, "a"),
                (TokenKind::ArrowRight, "->"),
                (TokenKind::Identifier, "b"),
                (TokenKind::ArrowLeft, "<-"),
                (TokenKind::Identifier, "c"),
                (TokenKind::ShiftRightShiftRight, ">>>"),
                (TokenKind::Identifier, "d"),
            ]
        );
    }

    #[test]
    fn compound_assignment_operators() {
        assert_eq!(
            lex("a+=b-=c*=d/=e%=f^=g&=h|=i<<=j>>=k>>>=l&&=m||=n"),
            vec![
                (TokenKind::Identifier, "a"),
                (TokenKind::PlusEqual, "+="),
                (TokenKind::Identifier, "b"),
                (TokenKind::MinusEqual, "-="),
                (TokenKind::Identifier, "c"),
                (TokenKind::StarEqual, "*="),
                (TokenKind::Identifier, "d"),
                (TokenKind::SlashEqual, "/="),
                (TokenKind::Identifier, "e"),
                (TokenKind::PercentEqual, "%="),
                (TokenKind::Identifier, "f"),
                (TokenKind::CaretEqual, "^="),
                (TokenKind::Identifier, "g"),
                (TokenKind::AmpersandEqual, "&="),
                (TokenKind::Identifier, "h"),
                (TokenKind::PipeEqual, "|="),
                (TokenKind::Identifier, "i"),
                (TokenKind::ShiftLeftEqual, "<<="),
                (TokenKind::Identifier, "j"),
                (TokenKind::ShiftRightEqual, ">>="),
                (TokenKind::Identifier, "k"),
                (TokenKind::ShiftRightShiftRightEqual, ">>>="),
                (TokenKind::Identifier, "l"),
                (TokenKind::AmpersandAmpersandEqual, "&&="),
                (TokenKind::Identifier, "m"),
                (TokenKind::PipePipeEqual, "||="),
                (TokenKind::Identifier, "n"),
            ]
        );
    }

    #[test]
    fn double_bracket_attribute_tokens() {
        assert_eq!(
            lex("[[a]]"),
            vec![
                (TokenKind::DoubleBracketLeft, "[["),
                (TokenKind::Identifier, "a"),
                (TokenKind::DoubleBracketRight, "]]"),
            ]
        );
    }

    #[test]
    fn ternary_and_range_slice_tokens() {
        assert_eq!(
            lex("b ? x[3:5] : y"),
            vec![
                (TokenKind::Identifier, "b"),
                (TokenKind::Trivia(Trivia::Whitespace), " "),
                (TokenKind::Question, "?"),
                (TokenKind::Trivia(Trivia::Whitespace), " "),
                (TokenKind::Identifier, "x"),
                (TokenKind::BracketLeft, "["),
                (TokenKind::Literal(Literal::Int32), "3"),
                (TokenKind::Colon, ":"),
                (TokenKind::Literal(Literal::Int32), "5"),
                (TokenKind::BracketRight, "]"),
                (TokenKind::Trivia(Trivia::Whitespace), " "),
                (TokenKind::Colon, ":"),
                (TokenKind::Trivia(Trivia::Whitespace), " "),
                (TokenKind::Identifier, "y"),
            ]
        );
    }

    #[test]
    fn namespace_path() {
        assert_eq!(
            lex("std::intrinsics::sin"),
            vec![
                (TokenKind::Identifier, "std"),
                (TokenKind::ColonColon, "::"),
                (TokenKind::Identifier, "intrinsics"),
                (TokenKind::ColonColon, "::"),
                (TokenKind::Identifier, "sin"),
            ]
        );
    }
}
