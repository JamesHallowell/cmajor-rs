use crate::{
    Program,
    resolver::{Resolution, ScopeId, SymbolKind, symbol::SymbolOrigin},
};

pub struct ResolutionView {
    program: Program,
    resolution: Resolution,
}

impl ResolutionView {
    pub fn new(program: Program, resolution: Resolution) -> Self {
        Self {
            program,
            resolution,
        }
    }

    fn scope_entry(&self, scope: ScopeId) -> ScopeEntry {
        let table = &self.resolution.symbols;

        let location = table.scope(scope).start().map(|start| {
            self.program[start.unit]
                .location(start.token)
                .start
                .to_string()
        });

        let symbols = table
            .symbols_in(scope)
            .filter(|symbol| matches!(symbol.origin, SymbolOrigin::Source { .. }))
            .map(|symbol| {
                let (name, location) = match symbol.origin {
                    SymbolOrigin::Source { unit, name, .. } => {
                        let unit = &self.program[unit];

                        let span = unit.tokens.span(name);
                        (
                            unit.source[span].to_string(),
                            Some(unit.source.location(span.start).to_string()),
                        )
                    }
                    SymbolOrigin::Builtin { name } => (name.to_string(), None),
                };

                SymbolEntry {
                    name,
                    kind: symbol.kind,
                    location,
                }
            })
            .collect();

        let mut children: Vec<(u32, ScopeEntry)> = table
            .child_scopes(scope)
            .map(|child| {
                let position = table
                    .scope(child)
                    .start()
                    .map(|start| self.program[start.unit].tokens.span(start.token).start)
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

impl serde::Serialize for ResolutionView {
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
                .map(|(_, diagnostic)| diagnostic.to_string())
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
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>,
}
