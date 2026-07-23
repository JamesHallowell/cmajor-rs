use {
    crate::lexer::{tokenize, Token},
    std::ops::Range,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenId(pub u32);

impl TokenId {
    fn index(self) -> usize {
        self.0 as usize
    }
}

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

    pub fn token(&self, id: TokenId) -> Token {
        self.tokens[id.index()]
    }

    pub fn span(&self, id: TokenId) -> Range<u32> {
        let start = self.positions[id.index()];
        let len = self.tokens[id.index()].len;
        start..start + len
    }

    pub fn text<'src>(&self, source: &'src str, id: TokenId) -> &'src str {
        let span = self.span(id);
        &source[span.start as usize..span.end as usize]
    }
}
