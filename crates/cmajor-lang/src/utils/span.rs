use {
    crate::{
        lexer::{TokenId, TokenStream},
        utils::source::{Source, SourceLocation},
    },
    std::range::Range,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span<T> {
    pub start: T,
    pub end: T,
}

impl<T> From<Range<T>> for Span<T> {
    fn from(range: Range<T>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl<T> From<std::ops::Range<T>> for Span<T> {
    fn from(range: std::ops::Range<T>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl Span<TokenId> {
    pub fn to_position_span(self, token_stream: &TokenStream) -> Span<u32> {
        Span {
            start: token_stream.span(self.start).start,
            end: token_stream.span(self.end).end,
        }
    }

    pub fn to_source_location_span(
        self,
        token_stream: &TokenStream,
        source: &Source<'_>,
    ) -> Span<SourceLocation> {
        self.to_position_span(token_stream)
            .to_source_location_span(source)
    }
}

impl Span<u32> {
    pub fn to_source_location_span(self, source: &Source<'_>) -> Span<SourceLocation> {
        Span {
            start: source.location(self.start),
            end: source.location(self.end),
        }
    }

    pub fn to_range(&self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}

impl Span<SourceLocation> {
    pub fn to_position_span(self, source: &Source<'_>) -> Span<u32> {
        (source.index(self.start)..source.index(self.end)).into()
    }
}
