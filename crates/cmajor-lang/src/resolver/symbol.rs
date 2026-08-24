use {
    crate::{
        arena_key,
        ast::NodeId,
        lexer::TokenId,
        resolver::{
            Scope, ScopeId,
            unit::{Anchor, UnitId},
        },
        utils::arena::Arena,
    },
    std::assert_matches,
};

arena_key!(SymbolId);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolOrigin {
    Source {
        unit: UnitId,
        name: TokenId,
        node: NodeId,
    },
    Builtin {
        name: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Serialize))]
pub enum SymbolKind {
    Primitive,
    Namespace,
    Processor,
    Graph,
    Struct,
    Enum,
    EnumValue,
    Function,
    Variable,
    Alias,
    Node,
    Endpoint,
    Generic,
}

impl SymbolKind {
    pub fn allows_duplicates(self) -> bool {
        matches!(self, SymbolKind::Function)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub origin: SymbolOrigin,
    pub kind: SymbolKind,
    pub scope: ScopeId,
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    scopes: Arena<ScopeId, Scope>,
    symbols: Arena<SymbolId, Symbol>,
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut scopes = Arena::default();
        scopes.push(Scope::global());

        Self {
            scopes,
            symbols: Arena::default(),
        }
    }

    pub fn global_scope(&self) -> ScopeId {
        let (id, scope) = self.scopes.first();
        assert_matches!(scope, Scope::Global { .. });
        id
    }

    pub fn scope(&self, scope: ScopeId) -> &Scope {
        &self.scopes[scope]
    }

    pub fn new_scope(&mut self, parent: ScopeId, start: Anchor) -> ScopeId {
        self.scopes.push(Scope::Child {
            parent,
            symbols: Vec::new(),
            start,
        })
    }

    pub fn child_scopes(&self, scope: ScopeId) -> impl Iterator<Item = ScopeId> + '_ {
        (&self.scopes)
            .into_iter()
            .filter(move |(_, s)| s.parent() == Some(scope))
            .map(|(id, _)| id)
    }

    pub fn scopes(&self) -> impl Iterator<Item = ScopeId> + '_ {
        (&self.scopes).into_iter().map(|(id, _)| id)
    }

    pub fn declare(&mut self, origin: SymbolOrigin, kind: SymbolKind, scope: ScopeId) -> SymbolId {
        let id = self.symbols.push(Symbol {
            origin,
            kind,
            scope,
        });
        self.scopes[scope].add_symbol(id);
        id
    }

    pub fn find_local<F>(&self, scope: ScopeId, mut predicate: F) -> Option<SymbolId>
    where
        F: FnMut(&Symbol) -> bool,
    {
        self.scopes[scope]
            .symbols()
            .find(|&&id| predicate(&self.symbols[id]))
            .copied()
    }

    pub fn find_visible<F>(&self, scope: ScopeId, mut predicate: F) -> Option<SymbolId>
    where
        F: FnMut(&Symbol) -> bool,
    {
        let mut current_scope = scope;
        loop {
            match self.find_local(current_scope, &mut predicate) {
                Some(id) => return Some(id),
                None => {
                    current_scope = self.scope(current_scope).parent()?;
                }
            }
        }
    }

    pub fn symbols_in(&self, scope: ScopeId) -> impl Iterator<Item = &Symbol> {
        self.scopes[scope].symbols().map(|&id| &self.symbols[id])
    }

    pub fn symbol(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id]
    }

    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }
}
