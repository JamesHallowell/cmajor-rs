use {
    crate::{arena_key, ast::NodeId, lexer::TokenId, utils::arena::Arena},
    std::assert_matches,
};

arena_key!(ScopeId);
arena_key!(SymbolId);

#[derive(Debug)]
pub enum Scope {
    Global {
        symbols: Vec<SymbolId>,
    },
    Child {
        parent: ScopeId,
        symbols: Vec<SymbolId>,
        start: TokenId,
    },
}

impl Scope {
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

    pub fn start(&self) -> Option<TokenId> {
        match self {
            Self::Global { .. } => None,
            Self::Child { start, .. } => Some(*start),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Serialize))]
pub enum SymbolKind {
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
}

impl SymbolKind {
    pub fn allows_duplicates(self) -> bool {
        matches!(self, SymbolKind::Function)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub name_token: TokenId,
    pub node: NodeId,
    pub scope: ScopeId,
    pub inner_scope: Option<ScopeId>,
}

#[derive(Debug)]
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
        scopes.push(Scope::Global { symbols: vec![] });

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

    pub fn new_scope(&mut self, parent: ScopeId, start: TokenId) -> ScopeId {
        self.scopes.push(Scope::Child {
            parent,
            symbols: Vec::new(),
            start,
        })
    }

    pub fn child_scope(&mut self, symbol: SymbolId, parent: ScopeId, start: TokenId) -> ScopeId {
        let scope = self.new_scope(parent, start);
        self.symbols[symbol].inner_scope = Some(scope);
        scope
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

    pub fn declare(
        &mut self,
        scope: ScopeId,
        name: String,
        kind: SymbolKind,
        node: NodeId,
        name_token: TokenId,
    ) -> SymbolId {
        let id = self.symbols.push(Symbol {
            name,
            kind,
            name_token,
            node,
            scope,
            inner_scope: None,
        });
        self.scopes[scope].add_symbol(id);
        id
    }

    pub fn lookup_local(&self, scope: ScopeId, name: &str) -> Option<SymbolId> {
        self.scopes[scope]
            .symbols()
            .find(|&&id| self.symbols[id].name == name)
            .copied()
    }

    pub fn lookup_visible(&self, scope: ScopeId, name: &str) -> Option<SymbolId> {
        let mut current_scope = scope;
        loop {
            match self.lookup_local(current_scope, name) {
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
