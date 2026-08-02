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

    fn entries(&self, scope: ScopeId) -> Vec<SymbolEntry> {
        self.resolution
            .table
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
            .collect()
    }
}

impl serde::Serialize for ResolutionView<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(serde::Serialize)]
        struct View {
            symbols: Vec<SymbolEntry>,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            diagnostics: Vec<String>,
        }

        View {
            symbols: self.entries(self.resolution.table.global_scope()),
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
