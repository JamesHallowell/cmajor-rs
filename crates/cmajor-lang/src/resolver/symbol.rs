use crate::{arena_key, ast::NodeId, lexer::TokenId, utils::arena::Arena};

arena_key!(ScopeId);
arena_key!(SymbolId);

#[derive(Debug, Default)]
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub symbols: Vec<SymbolId>,
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
    global: ScopeId,
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
        let global = scopes.push(Scope {
            parent: None,
            symbols: vec![],
        });

        Self {
            global,
            scopes,
            symbols: Arena::default(),
        }
    }

    pub fn global_scope(&self) -> ScopeId {
        self.global
    }

    pub fn child_scope(&mut self, symbol: SymbolId, parent: ScopeId) -> ScopeId {
        let scope = self.scopes.push(Scope {
            parent: Some(parent),
            symbols: Vec::new(),
        });
        self.symbols[symbol].inner_scope = Some(scope);
        scope
    }

    pub fn parent_scope(&self, scope: ScopeId) -> Option<ScopeId> {
        self.scopes[scope].parent
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
        self.scopes[scope].symbols.push(id);
        id
    }

    pub fn lookup_local(&self, scope: ScopeId, name: &str) -> Option<SymbolId> {
        self.scopes[scope]
            .symbols
            .iter()
            .copied()
            .find(|&id| self.symbols[id].name == name)
    }

    pub fn lookup_visible(&self, scope: ScopeId, name: &str) -> Option<SymbolId> {
        let mut current_scope = scope;
        loop {
            match self.lookup_local(current_scope, name) {
                Some(id) => return Some(id),
                None => {
                    current_scope = self.parent_scope(current_scope)?;
                }
            }
        }
    }

    pub fn symbols_in(&self, scope: ScopeId) -> impl Iterator<Item = &Symbol> {
        self.scopes[scope]
            .symbols
            .iter()
            .map(|&id| &self.symbols[id])
    }

    pub fn symbol(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id]
    }

    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }
}
