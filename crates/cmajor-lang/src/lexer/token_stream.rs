use {
    crate::{
        arena_key,
        lexer::Token,
        utils::arena::{Arena, Iter as ArenaIter, SecondaryArena},
    },
    std::ops::Range,
};

arena_key!(TokenId);

#[derive(Debug, Clone)]
pub struct TokenStream {
    tokens: Arena<TokenId, Token>,
    positions: SecondaryArena<TokenId, u32>,
}

impl TokenStream {
    pub fn new(tokens: Vec<Token>) -> Self {
        let tokens: Arena<TokenId, Token> = tokens.into_iter().collect();

        let positions = tokens
            .into_iter()
            .scan(0, |position, (id, token)| {
                let token_position = *position;
                *position += token.len;
                Some((id, token_position))
            })
            .collect::<SecondaryArena<TokenId, u32>>();

        Self { tokens, positions }
    }

    #[allow(clippy::len_without_is_empty, reason = "token stream is never empty")]
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn get(&self, id: TokenId) -> Token {
        self.tokens[id]
    }

    pub fn position(&self, id: TokenId) -> Option<u32> {
        self.positions.get(id).copied()
    }

    pub fn span(&self, id: TokenId) -> Option<Range<u32>> {
        let token = self.get(id);
        let start = self.position(id)?;
        Some(start..start + token.len)
    }

    pub fn text<'src>(&self, source: &'src str, id: TokenId) -> Option<&'src str> {
        let span = self.span(id)?;
        Some(&source[span.start as usize..span.end as usize])
    }

    pub fn end_of_file(&self) -> TokenId {
        self.tokens
            .last()
            .map(|(id, token)| {
                debug_assert!(token.kind.is_end_of_file());
                id
            })
            .expect("token stream should always have an end-of-file token")
    }
}

#[derive(Debug, Clone)]
pub struct TokenStreamIterator<'a> {
    tokens: &'a TokenStream,
    iter: ArenaIter<'a, TokenId, Token>,
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

    pub fn peek(&self) -> (TokenId, Token) {
        self.clone().next().unwrap_or_else(|| {
            let eof = self.stream().end_of_file();
            (eof, self.stream().get(eof))
        })
    }
}

impl<'a> Iterator for TokenStreamIterator<'a> {
    type Item = (TokenId, &'a Token);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .filter(|(_, token)| !token.kind.is_end_of_file())
    }
}

impl<'a> Iterator for NonTrivialTokenStreamIterator<'a> {
    type Item = (TokenId, Token);
    fn next(&mut self) -> Option<Self::Item> {
        for (id, &token) in &mut self.iter {
            if !token.kind.is_trivia() {
                return Some((id, token));
            }
        }
        None
    }
}

impl<'a> IntoIterator for &'a TokenStream {
    type Item = (TokenId, &'a Token);
    type IntoIter = TokenStreamIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            tokens: self,
            iter: self.tokens.into_iter(),
        }
    }
}

#[macro_export]
macro_rules! skip_to_matching {
    ($tokens:ident, ()) => {
        skip_to_matching!(@impl $tokens, $crate::token!('('), $crate::token!(')'))
    };
    ($tokens:ident, <>) => {
        skip_to_matching!(@impl $tokens, $crate::token!(<), $crate::token!(>))
    };
    ($tokens:ident, []) => {
        skip_to_matching!(@impl $tokens, $crate::token!('['), $crate::token!(']'))
    };
    (@impl $tokens:ident, $start:pat, $end:pat) => {
        {
            let mut skipped = $tokens.clone();
            skipped.next();
            let mut depth = 1;
            loop {
                match skipped.next().map(|(_, token)| token.kind) {
                    Some($start) => depth += 1,
                    Some($end) => {
                        depth -= 1;
                        if depth == 0 {
                            break Ok(skipped);
                        }
                    }
                    Some($crate::token!(; | '{' | '}')) | None => break Err($tokens),
                    _ => {}
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use {super::*, crate::lexer::tokenize, std::collections::HashMap};

    #[test]
    fn token_stream_assigns_each_token_an_id() {
        let token_stream = tokenize("let x = 42;");

        let tokens = token_stream.into_iter().collect::<HashMap<TokenId, _>>();

        assert_eq!(tokens.len(), 8);
    }

    #[test]
    fn token_stream_iterator_can_ignore_trivia() {
        let token_stream = tokenize("let x = 42;");

        let tokens = token_stream.into_iter().ignore_trivia().collect::<Vec<_>>();
        assert_eq!(tokens.len(), 5);
    }

    #[test]
    fn skip_to_matching_delimiter() {
        let tokens = tokenize("(([[()]))()");
        let tokens = tokens.into_iter();

        assert_eq!(tokens.clone().count(), 11);

        let result = skip_to_matching!(tokens, ()).expect("found a matching delimiter");
        assert_eq!(result.count(), 2);
    }

    #[test]
    fn skip_to_matching_delimiter_returns_unchanged_token_stream_if_no_matching_delim() {
        let tokens = tokenize("(([[()])");
        let tokens = tokens.into_iter();

        assert_eq!(tokens.clone().count(), 8);

        let result =
            skip_to_matching!(tokens, ()).expect_err("no matching delimiter expected to be found");
        assert_eq!(result.count(), 8);
    }
}
