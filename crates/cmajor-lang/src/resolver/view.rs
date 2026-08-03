use crate::{
    lexer::TokenStream,
    resolver::{Resolution, ScopeId, SymbolKind},
    utils,
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

    fn location(&self, position: u32) -> String {
        let (line, column) = utils::line_col(self.source, position);
        format!("{line}:{column}")
    }

    fn scope_entry(&self, scope: ScopeId) -> ScopeEntry {
        let table = &self.resolution.symbols;

        let location = table
            .scope(scope)
            .start()
            .map(|start| self.location(self.tokens.position(start)));

        let symbols = table
            .symbols_in(scope)
            .map(|symbol| SymbolEntry {
                name: symbol.name.clone(),
                kind: symbol.kind,
                location: self.location(self.tokens.position(symbol.name_token)),
            })
            .collect();

        let mut children: Vec<(u32, ScopeEntry)> = table
            .child_scopes(scope)
            .map(|child| {
                let position = table
                    .scope(child)
                    .start()
                    .map(|start| self.tokens.position(start))
                    .unwrap_or(0);
                (position, self.scope_entry(child))
            })
            .collect();
        children.sort_by_key(|(position, _)| *position);

        ScopeEntry {
            location,
            symbols,
            scopes: children.into_iter().map(|(_, entry)| entry).collect(),
        }
    }
}

impl serde::Serialize for ResolutionView<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(serde::Serialize)]
        struct View {
            #[serde(flatten)]
            scope: ScopeEntry,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            diagnostics: Vec<String>,
        }

        View {
            scope: self.scope_entry(self.resolution.symbols.global_scope()),
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
struct ScopeEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    symbols: Vec<SymbolEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    scopes: Vec<ScopeEntry>,
}

#[derive(serde::Serialize)]
struct SymbolEntry {
    name: String,
    kind: SymbolKind,
    location: String,
}