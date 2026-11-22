use crate::{
    Diagnostic,
    ast::Ast,
    lexer::{TokenId, TokenStream, tokenize},
    parser::parse,
    resolver::UnitId,
    utils::{
        arena::Arena,
        source::{SourceFile, SourceLocation},
        span::Span,
    },
};

#[derive(Debug, Clone)]
pub struct ParsedUnit {
    pub source: SourceFile,
    pub tokens: TokenStream,
    pub ast: Ast,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Default, Clone)]
pub struct Program {
    units: Arena<UnitId, ParsedUnit>,
}

impl ParsedUnit {
    pub fn text(&self, token: TokenId) -> &str {
        let range = self.tokens.span(token);
        &self.source[range]
    }

    pub fn location(&self, token: TokenId) -> Span<SourceLocation> {
        self.tokens
            .span(token)
            .to_source_location_span(&self.source)
    }
}

impl FromIterator<SourceFile> for Program {
    fn from_iter<T: IntoIterator<Item = SourceFile>>(source_files: T) -> Self {
        Program {
            units: source_files
                .into_iter()
                .map(|unit| {
                    let tokens = tokenize(unit.contents());
                    let (ast, diagnostics) = parse(unit.contents(), &tokens);

                    ParsedUnit {
                        source: unit,
                        tokens,
                        ast,
                        diagnostics,
                    }
                })
                .collect(),
        }
    }
}

impl Program {
    pub fn new() -> Self {
        Program::default()
    }

    pub fn add_source_file(&mut self, source_file: SourceFile) -> UnitId {
        let tokens = tokenize(source_file.contents());
        let (ast, diagnostics) = parse(source_file.contents(), &tokens);

        self.units.push(ParsedUnit {
            source: source_file,
            tokens,
            ast,
            diagnostics,
        })
    }

    pub fn with_source_file(mut self, source_file: SourceFile) -> Self {
        self.add_source_file(source_file);
        self
    }

    pub fn add_parsed_unit(&mut self, unit: ParsedUnit) -> UnitId {
        self.units.push(unit)
    }

    pub fn units(&self) -> impl Iterator<Item = UnitId> {
        self.units.into_iter().map(|(unit_id, _)| unit_id)
    }

    pub fn error_tokens(&self) -> impl Iterator<Item = (UnitId, TokenId)> {
        self.units
            .into_iter()
            .flat_map(|(id, unit)| unit.ast.error_tokens().map(move |token| (id, token)))
    }

    pub fn diagnostics(&self) -> impl Iterator<Item = (UnitId, &Diagnostic)> {
        self.units.into_iter().flat_map(|(id, unit)| {
            unit.diagnostics
                .iter()
                .map(move |diagnostic| (id, diagnostic))
        })
    }
}

impl IntoIterator for Program {
    type Item = ParsedUnit;
    type IntoIter = std::vec::IntoIter<ParsedUnit>;

    fn into_iter(self) -> Self::IntoIter {
        self.units.into_values()
    }
}

impl std::ops::Index<UnitId> for Program {
    type Output = ParsedUnit;

    fn index(&self, unit_id: UnitId) -> &Self::Output {
        &self.units[unit_id]
    }
}
