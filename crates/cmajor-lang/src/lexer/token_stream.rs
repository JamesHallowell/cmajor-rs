use {
    crate::lexer::{tokenize, Token, TokenKind},
    std::ops::Range,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenId(pub u32);

#[derive(Debug, Clone)]
pub struct TokenStream {
    tokens: Vec<Token>,
    positions: Vec<u32>,
}

impl TokenStream {
    pub fn tokenize(source: &str) -> Self {
        let mut tokens = Vec::new();
        let mut positions = Vec::new();

        let mut pos = 0u32;
        for token in tokenize(source) {
            if !token.kind.is_trivia() {
                tokens.push(token);
                positions.push(pos);
            }
            pos += token.len;
        }

        Self { tokens, positions }
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn token(&self, id: TokenId) -> Option<Token> {
        self.tokens.get(id.0 as usize).copied()
    }

    pub fn span(&self, id: TokenId) -> Option<Range<u32>> {
        let start = *self.positions.get(id.0 as usize)?;
        Some(start..start + self.tokens[id.0 as usize].len)
    }

    pub fn text<'src>(&self, source: &'src str, id: TokenId) -> Option<&'src str> {
        let span = self.span(id)?;
        Some(&source[span.start as usize..span.end as usize])
    }
}

pub struct TokenStreamIterator<'a> {
    stream: &'a TokenStream,
    pos: TokenId,
}

impl<'a> TokenStreamIterator<'a> {
    pub fn peek(&self) -> Option<Token> {
        self.stream.token(self.pos)
    }

    pub fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|t| t.kind)
    }

    pub fn peek_nth(&self, n: usize) -> Option<Token> {
        self.stream.token(TokenId(self.pos.0 + n as u32))
    }

    pub fn current(&self) -> TokenId {
        self.pos
    }

    pub fn span(&self, id: TokenId) -> Option<Range<u32>> {
        self.stream.span(id)
    }
}

impl<'a> Iterator for TokenStreamIterator<'a> {
    type Item = (TokenId, Token);
    fn next(&mut self) -> Option<Self::Item> {
        let token = self.stream.token(self.pos)?;
        let id = self.pos;
        self.pos.0 += 1;
        Some((id, token))
    }
}

impl<'a> IntoIterator for &'a TokenStream {
    type Item = (TokenId, Token);
    type IntoIter = TokenStreamIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            stream: self,
            pos: TokenId(0),
        }
    }
}
