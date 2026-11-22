use crate::{
    lexer::{
        TokenStream,
        cursor::Cursor,
        token::{Keyword, LexError, Literal, Token, TokenKind, Trivia},
    },
    token,
};

const MAX_IDENTIFIER_LENGTH: usize = 256;

pub fn tokenize(input: &str) -> TokenStream {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token() {
        tokens.push(token);
    }
    tokens.push(Token {
        kind: token!(eof),
        len: 0,
    });
    TokenStream::new(tokens)
}

struct Lexer<'a> {
    input: &'a str,
    pos: u32,
    cursor: Cursor<'a>,
    pending_error: Option<LexError>,
    follows_minus: bool,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            cursor: Cursor::new(input),
            pending_error: None,
            follows_minus: false,
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        self.cursor.reset_bytes_taken();
        let start = self.pos;

        if let Some(error) = self.pending_error.take() {
            if error == LexError::MalformedEscapeSequence {
                self.take_rest_of_string();
            } else {
                self.cursor.take();
                self.cursor.take_while(is_ident_continue);
            }
            let len = self.cursor.bytes_taken();
            self.pos += len;
            self.follows_minus = false;
            return Some(Token {
                kind: TokenKind::Error(error),
                len,
            });
        }

        let c = self.cursor.take()?;

        let kind = match c {
            c if is_whitespace(c) => {
                self.cursor.take_while(is_whitespace);
                TokenKind::Trivia(Trivia::Whitespace)
            }
            '/' if self.cursor.peek() == Some('/') => self.line_comment(),
            '/' if self.cursor.peek() == Some('*') => self.block_comment(),
            c if is_ident_start(c) => self.ident(),
            '0' if self.cursor.peek().is_some_and(|c| c.is_ascii_digit())
                && !self.digits_continue_as_float() =>
            {
                self.cursor.take_while(|c| c.is_ascii_digit());
                TokenKind::Error(LexError::OctalLiteral)
            }
            c if c.is_ascii_digit() => self.number(),
            '.' if self.cursor.peek().is_some_and(|c| c.is_ascii_digit()) => self.number_from_dot(),
            '"' => self.string(),
            '+' => match self.cursor.peek() {
                Some('+') => {
                    self.cursor.take();
                    token!(++)
                }
                Some('=') => {
                    self.cursor.take();
                    token!(+=)
                }
                _ => token!(+),
            },
            '-' => match self.cursor.peek() {
                Some('-') => {
                    self.cursor.take();
                    token!(--)
                }
                Some('>') => {
                    self.cursor.take();
                    token!(->)
                }
                Some('=') => {
                    self.cursor.take();
                    token!(-=)
                }
                _ => token!(-),
            },
            '*' => match self.cursor.peek() {
                Some('*') => {
                    self.cursor.take();
                    token!(**)
                }
                Some('=') => {
                    self.cursor.take();
                    token!(*=)
                }
                _ => token!(*),
            },
            '/' => self.one_or_two('=', token!(/=), token!(/)),
            '%' => self.one_or_two('=', token!(%=), token!(%)),
            '~' => token!(~),
            '^' => self.one_or_two('=', token!(^=), token!(^)),
            '&' if self.cursor.peek_at(0) == Some('&') && self.cursor.peek_at(1) == Some('=') => {
                self.cursor.take();
                self.cursor.take();
                token!(&&=)
            }
            '&' => match self.cursor.peek() {
                Some('&') => {
                    self.cursor.take();
                    token!(&&)
                }
                Some('=') => {
                    self.cursor.take();
                    token!(&=)
                }
                _ => token!(&),
            },
            '|' if self.cursor.peek_at(0) == Some('|') && self.cursor.peek_at(1) == Some('=') => {
                self.cursor.take();
                self.cursor.take();
                token!(||=)
            }
            '|' => match self.cursor.peek() {
                Some('|') => {
                    self.cursor.take();
                    token!(||)
                }
                Some('=') => {
                    self.cursor.take();
                    token!(|=)
                }
                _ => token!(|),
            },
            '!' => self.one_or_two('=', token!(!=), token!(!)),
            '=' => self.one_or_two('=', token!(==), token!(=)),
            '<' => match self.cursor.peek() {
                Some('<') if self.cursor.peek_two() == (Some('<'), Some('=')) => {
                    self.cursor.take();
                    self.cursor.take();
                    token!(<<=)
                }
                Some('<') => {
                    self.cursor.take();
                    token!(<<)
                }
                Some('=') => {
                    self.cursor.take();
                    token!(<=)
                }
                Some('-') => {
                    self.cursor.take();
                    token!(<-)
                }
                _ => token!(<),
            },
            '>' if self.cursor.peek_at(0) == Some('>')
                && self.cursor.peek_at(1) == Some('>')
                && self.cursor.peek_at(2) == Some('=') =>
            {
                self.cursor.take();
                self.cursor.take();
                self.cursor.take();
                token!(>>>=)
            }
            '>' if self.cursor.peek_at(0) == Some('>') && self.cursor.peek_at(1) == Some('>') => {
                self.cursor.take();
                self.cursor.take();
                token!(>>>)
            }
            '>' if self.cursor.peek_at(0) == Some('>') && self.cursor.peek_at(1) == Some('=') => {
                self.cursor.take();
                self.cursor.take();
                token!(>>=)
            }
            '>' => match self.cursor.peek() {
                Some('>') => {
                    self.cursor.take();
                    token!(>>)
                }
                Some('=') => {
                    self.cursor.take();
                    token!(>=)
                }
                _ => token!(>),
            },
            ':' => self.one_or_two(':', token!(::), token!(:)),
            ',' => token!(,),
            ';' => token!(;),
            '.' => token!(.),
            '?' => token!(?),
            '(' => token!('('),
            ')' => token!(')'),
            '[' => token!('['),
            ']' => token!(']'),
            '{' => token!('{'),
            '}' => token!('}'),
            c => TokenKind::Error(LexError::IllegalCharacter(c)),
        };

        let len = self.cursor.bytes_taken();
        self.pos += len;

        let kind = if kind == TokenKind::Identifier {
            let text = &self.input[start as usize..(start + len) as usize];
            if text == "_" {
                TokenKind::Error(LexError::IllegalCharacter('_'))
            } else if text.starts_with('_') {
                TokenKind::Error(LexError::LeadingUnderscore)
            } else if text.len() > MAX_IDENTIFIER_LENGTH {
                TokenKind::Error(LexError::IdentifierTooLong)
            } else {
                Keyword::try_from(text)
                    .map(TokenKind::from)
                    .unwrap_or(TokenKind::Identifier)
            }
        } else {
            kind
        };

        self.follows_minus = kind == token!(-);

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
                None => return TokenKind::Error(LexError::UnterminatedComment),
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
                if self.cursor.bytes_taken() == 2 {
                    return TokenKind::Error(LexError::MalformedNumber);
                }
                Literal::Int32
            }
            Some('b' | 'B') => {
                self.cursor.take();
                self.cursor.take_while(is_binary_digit);
                if self.cursor.bytes_taken() == 2 {
                    return TokenKind::Error(LexError::MalformedNumber);
                }
                Literal::Int32
            }
            _ => {
                self.cursor.take_while(|c| c.is_ascii_digit());

                let mut is_float = self.cursor.peek() == Some('.');
                if is_float {
                    self.cursor.take();
                    self.cursor.take_while(|c| c.is_ascii_digit());
                }

                if self.take_exponent() {
                    is_float = true;
                }

                if is_float {
                    Literal::Float64
                } else {
                    Literal::Int32
                }
            }
        };

        self.finish_number(kind)
    }

    fn digits_continue_as_float(&self) -> bool {
        let mut n = 0;
        while self.cursor.peek_at(n).is_some_and(|c| c.is_ascii_digit()) {
            n += 1;
        }
        self.cursor.peek_at(n) == Some('.') || self.exponent_follows(n)
    }

    fn number_from_dot(&mut self) -> TokenKind {
        self.cursor.take_while(|c| c.is_ascii_digit());
        self.take_exponent();
        self.finish_number(Literal::Float64)
    }

    fn exponent_follows(&self, n: usize) -> bool {
        let digit_at = |i| {
            self.cursor
                .peek_at(i)
                .is_some_and(|c: char| c.is_ascii_digit())
        };
        match (self.cursor.peek_at(n), self.cursor.peek_at(n + 1)) {
            (Some('e' | 'E'), Some('+' | '-')) => digit_at(n + 2),
            (Some('e' | 'E'), _) => digit_at(n + 1),
            _ => false,
        }
    }

    fn take_exponent(&mut self) -> bool {
        if !self.exponent_follows(0) {
            return false;
        }

        self.cursor.take();
        if matches!(self.cursor.peek(), Some('+' | '-')) {
            self.cursor.take();
        }
        self.cursor.take_while(|c| c.is_ascii_digit());
        true
    }

    fn finish_number(&mut self, kind: Literal) -> TokenKind {
        let integer_value = (kind == Literal::Int32).then(|| self.integer_value());

        if let (Some('_'), Some('f' | 'i' | 'L')) = self.cursor.peek_two() {
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
            (Some('i'), Some('3'), Some('2')) if kind == Literal::Int32 => {
                self.cursor.take_n(3);
                Literal::Int32
            }
            (Some('i'), Some('6'), Some('4')) if kind == Literal::Int32 => {
                self.cursor.take_n(3);
                Literal::Int64
            }
            (Some('i'), _, _) if kind == Literal::Float64 => {
                self.cursor.take();
                Literal::Imaginary64
            }
            (Some('L'), _, _) if kind == Literal::Int32 => {
                self.cursor.take();
                Literal::Int64
            }
            _ => kind,
        };

        if let Some(c) = self.cursor.peek().filter(|&c| is_ident_continue(c)) {
            self.pending_error = Some(match c {
                'i' | 'I' if integer_value.is_some() => LexError::ImaginaryIntegerLiteral,
                _ => LexError::UnrecognisedLiteralSuffix,
            });
        }

        let int32_limit = i32::MAX as u64 + u64::from(self.follows_minus);
        match integer_value {
            Some(Err(_)) => TokenKind::Error(LexError::IntegerLiteralTooLarge),
            Some(Ok(value)) if kind == Literal::Int32 && value > int32_limit => {
                TokenKind::Error(LexError::IntegerLiteralNeedsSuffix)
            }
            _ => TokenKind::Literal(kind),
        }
    }

    fn string(&mut self) -> TokenKind {
        loop {
            match self.cursor.take() {
                None | Some('\n') => return TokenKind::Error(LexError::UnterminatedString),
                Some('"') => return TokenKind::Literal(Literal::String),
                Some('\\') if self.cursor.peek().is_some() => match self.cursor.take() {
                    Some('u') if !self.take_unicode_escape_digits() => {
                        self.pending_error = Some(LexError::MalformedEscapeSequence);
                        return TokenKind::Literal(Literal::String);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn integer_value(&self) -> Result<u64, std::num::ParseIntError> {
        let digits = &self.input[self.pos as usize..][..self.cursor.bytes_taken() as usize];
        match digits.get(..2) {
            Some("0x" | "0X") => u64::from_str_radix(&digits[2..], 16),
            Some("0b" | "0B") => u64::from_str_radix(&digits[2..], 2),
            _ => digits.parse(),
        }
    }

    fn take_rest_of_string(&mut self) {
        while let Some(c) = self.cursor.peek().filter(|&c| c != '\n') {
            self.cursor.take();
            match c {
                '"' => return,
                '\\' if self.cursor.peek() != Some('\n') => {
                    self.cursor.take();
                }
                _ => {}
            }
        }
    }

    fn take_unicode_escape_digits(&mut self) -> bool {
        (0..4).all(|_| {
            self.cursor.peek().is_some_and(|c| c.is_ascii_hexdigit())
                && self.cursor.take().is_some()
        })
    }
}

fn is_whitespace(c: char) -> bool {
    c.is_whitespace()
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}

fn is_binary_digit(c: char) -> bool {
    c == '0' || c == '1'
}

#[cfg(test)]
mod tests {
    use {super::*, crate::lexer::token::Keyword};

    fn lex<'a>(input: &'a str) -> Vec<(TokenKind, &'a str)> {
        let token_stream = tokenize(input);

        token_stream
            .into_iter()
            .map(|(id, token)| {
                let span = token_stream.span(id);
                (token.kind, &input[span.start as usize..span.end as usize])
            })
            .collect()
    }

    #[test]
    fn empty_input_produces_a_single_end_of_file_token() {
        let tokens = tokenize("");

        assert_eq!(tokens.len(), 1);

        let eof = tokens.end_of_file();
        let eof = tokens.get(eof);
        assert!(eof.kind.is_end_of_file());
        assert_eq!(eof.len, 0);
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
                (TokenKind::AngleBracketLeft, "<"),
                (TokenKind::Literal(Literal::Int32), "4"),
                (TokenKind::AngleBracketRight, ">"),
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
        );
        assert_eq!(
            lex(".8f"),
            vec![(TokenKind::Literal(Literal::Float32), ".8f")]
        );
        assert_eq!(
            lex(".5"),
            vec![(TokenKind::Literal(Literal::Float64), ".5")]
        );
        assert_eq!(
            lex("x.5"),
            vec![
                (TokenKind::Identifier, "x"),
                (TokenKind::Literal(Literal::Float64), ".5"),
            ]
        );
    }

    #[test]
    fn exponent_float_literals() {
        assert_eq!(
            lex("4.6566129e-10"),
            vec![(TokenKind::Literal(Literal::Float64), "4.6566129e-10")]
        );
        assert_eq!(
            lex("3.0518e-5f"),
            vec![(TokenKind::Literal(Literal::Float32), "3.0518e-5f")]
        );
        assert_eq!(
            lex("1e10"),
            vec![(TokenKind::Literal(Literal::Float64), "1e10")]
        );
        assert_eq!(
            lex("2E+3"),
            vec![(TokenKind::Literal(Literal::Float64), "2E+3")]
        );
        assert_eq!(
            lex("1e"),
            vec![
                (TokenKind::Literal(Literal::Int32), "1"),
                (TokenKind::Error(LexError::UnrecognisedLiteralSuffix), "e"),
            ]
        );
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
        assert_eq!(
            lex("\"abc"),
            vec![(TokenKind::Error(LexError::UnterminatedString), "\"abc")]
        );
    }

    #[test]
    fn malformed_unicode_escape_is_an_error_at_the_bad_character() {
        assert_eq!(
            lex(r#""\u12""#),
            vec![
                (TokenKind::Literal(Literal::String), r#""\u12"#),
                (TokenKind::Error(LexError::MalformedEscapeSequence), "\""),
            ]
        );
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
        assert_eq!(
            lex("/* comment"),
            vec![(
                TokenKind::Error(LexError::UnterminatedComment),
                "/* comment"
            )]
        );
    }

    #[test]
    fn non_ascii_outside_strings_and_comments_is_illegal() {
        assert_eq!(
            lex("Ω"),
            vec![(TokenKind::Error(LexError::IllegalCharacter('Ω')), "Ω")]
        );
    }

    #[test]
    fn identifiers_cannot_start_with_an_underscore() {
        assert_eq!(
            lex("_x"),
            vec![(TokenKind::Error(LexError::LeadingUnderscore), "_x")]
        );
    }

    #[test]
    fn leading_zero_makes_an_octal_literal_which_is_an_error() {
        assert_eq!(
            lex("0123"),
            vec![(TokenKind::Error(LexError::OctalLiteral), "0123")]
        );
    }

    #[test]
    fn unrecognised_literal_suffix_is_an_error_at_the_suffix() {
        assert_eq!(
            lex("2.0f31"),
            vec![
                (TokenKind::Literal(Literal::Float32), "2.0f"),
                (TokenKind::Error(LexError::UnrecognisedLiteralSuffix), "31"),
            ]
        );
    }

    #[test]
    fn imaginary_suffix_needs_a_float_literal() {
        assert_eq!(
            lex("3i"),
            vec![
                (TokenKind::Literal(Literal::Int32), "3"),
                (TokenKind::Error(LexError::ImaginaryIntegerLiteral), "i"),
            ]
        );
    }

    #[test]
    fn i32_suffix_makes_an_int32_literal() {
        assert_eq!(
            lex("101i32"),
            vec![(TokenKind::Literal(Literal::Int32), "101i32")]
        );
    }

    #[test]
    fn int32_literal_out_of_range_needs_a_suffix() {
        assert_eq!(
            lex("2147483648"),
            vec![(
                TokenKind::Error(LexError::IntegerLiteralNeedsSuffix),
                "2147483648"
            )]
        );
    }

    #[test]
    fn negated_int32_minimum_fits_without_a_suffix() {
        assert_eq!(
            lex("-2147483648"),
            vec![
                (TokenKind::Minus, "-"),
                (TokenKind::Literal(Literal::Int32), "2147483648"),
            ]
        );
    }

    #[test]
    fn integer_literal_beyond_64_bits_is_too_large() {
        assert_eq!(
            lex("0xffffffffffffffff1"),
            vec![(
                TokenKind::Error(LexError::IntegerLiteralTooLarge),
                "0xffffffffffffffff1"
            )]
        );
    }

    #[test]
    fn hex_prefix_without_digits_is_malformed() {
        assert_eq!(
            lex("0x"),
            vec![(TokenKind::Error(LexError::MalformedNumber), "0x")]
        );
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
                (TokenKind::BracketLeft, "["),
                (TokenKind::BracketLeft, "["),
                (TokenKind::Identifier, "a"),
                (TokenKind::BracketRight, "]"),
                (TokenKind::BracketRight, "]"),
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

    #[test]
    fn rest_of_a_string_after_a_malformed_escape_stays_in_the_error() {
        assert_eq!(
            lex(r#""\u12zz abc" x"#),
            vec![
                (TokenKind::Literal(Literal::String), r#""\u12"#),
                (
                    TokenKind::Error(LexError::MalformedEscapeSequence),
                    r#"zz abc""#
                ),
                (TokenKind::Trivia(Trivia::Whitespace), " "),
                (TokenKind::Identifier, "x"),
            ]
        );
    }

    #[test]
    fn malformed_unicode_escape_at_end_of_input_is_an_error() {
        assert_eq!(
            lex(r#""\u"#),
            vec![
                (TokenKind::Literal(Literal::String), r#""\u"#),
                (TokenKind::Error(LexError::MalformedEscapeSequence), ""),
            ]
        );
    }

    #[test]
    fn integer_suffix_on_a_float_is_unrecognised() {
        assert_eq!(
            lex("1.5i32"),
            vec![
                (TokenKind::Literal(Literal::Imaginary64), "1.5i"),
                (TokenKind::Error(LexError::UnrecognisedLiteralSuffix), "32"),
            ]
        );
    }

    #[test]
    fn leading_zero_on_a_float_is_not_octal() {
        assert_eq!(
            lex("01.5"),
            vec![(TokenKind::Literal(Literal::Float64), "01.5")]
        );
    }

    #[test]
    fn underscore_without_a_suffix_is_unrecognised() {
        assert_eq!(
            lex("1_"),
            vec![
                (TokenKind::Literal(Literal::Int32), "1"),
                (TokenKind::Error(LexError::UnrecognisedLiteralSuffix), "_"),
            ]
        );
    }

    #[test]
    fn lone_underscore_is_an_illegal_character() {
        assert_eq!(
            lex("_"),
            vec![(TokenKind::Error(LexError::IllegalCharacter('_')), "_")]
        );
    }
}
