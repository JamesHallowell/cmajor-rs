use {
    crate::{
        lexer::TokenStream,
        resolver::{Resolution, ScopeId, SymbolKind},
        utils,
    },
    std::collections::HashSet,
};

pub struct ResolutionView<'a> {
    resolution: Resolution,
    tokens: TokenStream,
    source: &'a str,
}

impl<'a> ResolutionView<'a> {
    pub fn new(resolution: Resolution, tokens: TokenStream, source: &'a str) -> Self {
        Self {
            resolution,
            tokens,
            source,
        }
    }

    fn entries(&self, scope: ScopeId) -> Vec<SymbolEntry> {
        let table = &self.resolution.symbols;

        let mut entries: Vec<SymbolEntry> = table
            .symbols_in(scope)
            .map(|symbol| {
                let position = self.tokens.position(symbol.name_token);
                let (line, column) = utils::line_col(self.source, position);

                SymbolEntry {
                    name: symbol.name.clone(),
                    kind: symbol.kind,
                    location: format!("{line}:{column}"),
                    members: symbol
                        .inner_scope
                        .map(|scope| self.entries(scope))
                        .unwrap_or_default(),
                }
            })
            .collect();

        let owned: HashSet<ScopeId> = table
            .symbols_in(scope)
            .filter_map(|s| s.inner_scope)
            .collect();
        for child in table.child_scopes(scope) {
            if !owned.contains(&child) {
                entries.extend(self.entries(child));
            }
        }

        entries
    }

    fn scopes(&self) -> Vec<ScopeEntry> {
        let table = &self.resolution.symbols;

        let mut scopes: Vec<(u32, ScopeEntry)> = table
            .scopes()
            .filter_map(|scope| {
                let start = table.scope(scope).start()?;
                let position = self.tokens.position(start);
                let (line, column) = utils::line_col(self.source, position);
                Some((
                    position,
                    ScopeEntry {
                        location: format!("{line}:{column}"),
                    },
                ))
            })
            .collect();
        scopes.sort_by_key(|(position, _)| *position);

        scopes.into_iter().map(|(_, entry)| entry).collect()
    }
}

impl serde::Serialize for ResolutionView<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(serde::Serialize)]
        struct View {
            symbols: Vec<SymbolEntry>,
            scopes: Vec<ScopeEntry>,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            diagnostics: Vec<String>,
        }

        View {
            symbols: self.entries(self.resolution.symbols.global_scope()),
            scopes: self.scopes(),
            diagnostics: self
                .resolution
                .diagnostics
                .iter()
                .map(ToString::to_string)
                .collect(),
        }
        .serialize(serializer)
    }
}

#[derive(serde::Serialize)]
struct SymbolEntry {
    name: String,
    kind: SymbolKind,
    location: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    members: Vec<SymbolEntry>,
}

#[derive(serde::Serialize)]
struct ScopeEntry {
    location: String,
}
