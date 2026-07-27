use {
    crate::lexer::{Token, TokenKind},
    std::ops::Range,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenId(u32);

#[derive(Debug, Clone)]
pub struct TokenStream {
    tokens: Vec<Token>,
    positions: Vec<u32>,
}

impl TokenStream {
    pub fn new(tokens: Vec<Token>) -> Self {
        let positions = tokens
            .iter()
            .scan(0, |position, token| {
                let token_position = *position;
                *position += token.len;
                Some(token_position)
            })
            .collect::<Vec<_>>();

        Self { tokens, positions }
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn get(&self, id: TokenId) -> Option<Token> {
        self.tokens.get(id.0 as usize).copied()
    }

    pub fn position(&self, id: TokenId) -> Option<u32> {
        self.positions.get(id.0 as usize).copied()
    }

    pub fn span(&self, id: TokenId) -> Option<Range<u32>> {
        let token = self.get(id)?;
        let start = self.position(id)?;
        Some(start..start + token.len)
    }

    pub fn text<'src>(&self, source: &'src str, id: TokenId) -> Option<&'src str> {
        let span = self.span(id)?;
        Some(&source[span.start as usize..span.end as usize])
    }
}

#[derive(Debug, Clone)]
pub struct TokenStreamIterator<'a> {
    tokens: &'a TokenStream,
    current: TokenId,
}

#[derive(Debug, Clone)]
pub struct NonTrivialTokenStreamIterator<'a> {
    iter: TokenStreamIterator<'a>,
}

impl<'a> TokenStreamIterator<'a> {
    pub fn stream(&self) -> &TokenStream {
        self.tokens
    }

    pub fn ignore_trivia(self) -> NonTrivialTokenStreamIterator<'a> {
        NonTrivialTokenStreamIterator { iter: self }
    }
}

impl<'a> NonTrivialTokenStreamIterator<'a> {
    pub fn stream(&self) -> &TokenStream {
        self.iter.stream()
    }

    pub fn peek(&self) -> Option<Token> {
        self.clone().next().map(|(_, token)| token)
    }

    pub fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|t| t.kind)
    }

    pub fn peek_nth(&self, n: usize) -> Option<Token> {
        self.clone().nth(n).map(|(_, token)| token)
    }

    pub fn current(&self) -> TokenId {
        self.iter.current
    }
}

impl<'a> Iterator for TokenStreamIterator<'a> {
    type Item = (TokenId, Token);
    fn next(&mut self) -> Option<Self::Item> {
        let token = self.tokens.get(self.current)?;
        let id = self.current;
        self.current.0 += 1;
        Some((id, token))
    }
}

impl<'a> Iterator for NonTrivialTokenStreamIterator<'a> {
    type Item = (TokenId, Token);
    fn next(&mut self) -> Option<Self::Item> {
        for (id, token) in &mut self.iter {
            if !token.kind.is_trivia() {
                return Some((id, token));
            }
        }
        None
    }
}

impl<'a> IntoIterator for &'a TokenStream {
    type Item = (TokenId, Token);
    type IntoIter = TokenStreamIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            tokens: self,
            current: TokenId(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::lexer::{tokenize, Keyword, Literal, Trivia},
    };

    #[test]
    fn token_stream_assigns_each_token_an_id() {
        let token_stream = tokenize("let x = 42;");

        assert_eq!(
            token_stream.into_iter().collect::<Vec<_>>(),
            vec![
                (
                    TokenId(0),
                    Token {
                        kind: TokenKind::Keyword(Keyword::Let),
                        len: 3
                    }
                ),
                (
                    TokenId(1),
                    Token {
                        kind: TokenKind::Trivia(Trivia::Whitespace),
                        len: 1
                    }
                ),
                (
                    TokenId(2),
                    Token {
                        kind: TokenKind::Identifier,
                        len: 1
                    }
                ),
                (
                    TokenId(3),
                    Token {
                        kind: TokenKind::Trivia(Trivia::Whitespace),
                        len: 1
                    }
                ),
                (
                    TokenId(4),
                    Token {
                        kind: TokenKind::Equal,
                        len: 1
                    }
                ),
                (
                    TokenId(5),
                    Token {
                        kind: TokenKind::Trivia(Trivia::Whitespace),
                        len: 1
                    }
                ),
                (
                    TokenId(6),
                    Token {
                        kind: TokenKind::Literal(Literal::Int32),
                        len: 2
                    }
                ),
                (
                    TokenId(7),
                    Token {
                        kind: TokenKind::Semicolon,
                        len: 1
                    }
                )
            ]
        );
    }

    #[test]
    fn token_stream_iterator_can_ignore_trivia() {
        let token_stream = tokenize("let x = 42;");

        assert_eq!(
            token_stream.into_iter().ignore_trivia().collect::<Vec<_>>(),
            vec![
                (
                    TokenId(0),
                    Token {
                        kind: TokenKind::Keyword(Keyword::Let),
                        len: 3
                    }
                ),
                (
                    TokenId(2),
                    Token {
                        kind: TokenKind::Identifier,
                        len: 1
                    }
                ),
                (
                    TokenId(4),
                    Token {
                        kind: TokenKind::Equal,
                        len: 1
                    }
                ),
                (
                    TokenId(6),
                    Token {
                        kind: TokenKind::Literal(Literal::Int32),
                        len: 2
                    }
                ),
                (
                    TokenId(7),
                    Token {
                        kind: TokenKind::Semicolon,
                        len: 1
                    }
                )
            ]
        );
    }
}
