use crate::{arena_key, lexer::TokenId};

arena_key!(UnitId);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    pub unit: UnitId,
    pub token: TokenId,
}
