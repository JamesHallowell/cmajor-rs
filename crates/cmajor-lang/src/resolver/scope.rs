use crate::{arena_key, resolver::SymbolId, resolver::unit::Anchor};

arena_key!(ScopeId);

#[derive(Debug, Clone)]
pub enum Scope {
    Global {
        symbols: Vec<SymbolId>,
    },
    Child {
        parent: ScopeId,
        symbols: Vec<SymbolId>,
        start: Anchor,
    },
}

impl Scope {
    pub fn global() -> Self {
        Self::Global { symbols: vec![] }
    }

    pub fn add_symbol(&mut self, symbol: SymbolId) {
        match self {
            Scope::Global { symbols } => symbols.push(symbol),
            Scope::Child { symbols, .. } => symbols.push(symbol),
        }
    }

    pub fn symbols(&self) -> impl Iterator<Item = &SymbolId> {
        match self {
            Scope::Global { symbols } => symbols.iter(),
            Scope::Child { symbols, .. } => symbols.iter(),
        }
    }

    pub fn parent(&self) -> Option<ScopeId> {
        match self {
            Self::Global { .. } => None,
            Self::Child { parent, .. } => Some(*parent),
        }
    }

    pub fn start(&self) -> Option<Anchor> {
        match self {
            Self::Global { .. } => None,
            Self::Child { start, .. } => Some(*start),
        }
    }
}
