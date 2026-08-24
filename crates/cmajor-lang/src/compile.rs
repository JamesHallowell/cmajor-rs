use crate::{
    Diagnostic,
    ast::Ast,
    lexer::{TokenStream, tokenize},
    parser,
    resolver::{self, Resolution, UnitId},
    utils::arena::Arena,
};

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub name: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct CompilationUnit {
    pub source: SourceFile,
    pub tokens: TokenStream,
    pub ast: Ast,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct Program {
    pub units: Arena<UnitId, CompilationUnit>,
    pub resolution: Resolution,
}

pub fn compile(units: impl IntoIterator<Item = SourceFile>) -> Program {
    let compiled: Arena<UnitId, CompilationUnit> = units
        .into_iter()
        .map(|unit| {
            let tokens = tokenize(&unit.source);
            let (ast, diagnostics) = parser::parse(&unit.source, &tokens);

            CompilationUnit {
                source: unit,
                tokens,
                ast,
                diagnostics,
            }
        })
        .collect();

    let resolver_units: Vec<resolver::Unit<'_>> = compiled
        .into_iter()
        .map(|(_, unit)| resolver::Unit {
            name: &unit.source.name,
            source: &unit.source.source,
            tokens: &unit.tokens,
            ast: &unit.ast,
        })
        .collect();

    Program {
        resolution: resolver::resolve_all(&resolver_units),
        units: compiled,
    }
}
