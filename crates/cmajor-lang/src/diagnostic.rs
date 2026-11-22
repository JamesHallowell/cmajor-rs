use crate::{
    compile::Program,
    lexer::TokenKind,
    resolver::UnitId,
    utils::{source::SourceLocation, span::Span},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub location: Span<SourceLocation>,
    pub error: CompileError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitLocation {
    pub unit: UnitId,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    FoundWhenExpecting {
        found: TokenKind,
        expected: TokenKind,
    },
    ExpectedType,
    ExpectedValue,
    ExpectedStreamType,
    ManyToManyConnection,
    ChainedConnectionWithMultipleEndpoints,
    EndpointNamedInChain,
    UnresolvedSymbol {
        name: String,
    },
    NameInUse {
        name: String,
        previous_declaration: Option<UnitLocation>,
    },
    NotAScope {
        name: String,
    },
    UnexpectedNode,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::FoundWhenExpecting { found, expected } => {
                write!(f, "Found {found} when expecting {expected}")
            }
            CompileError::ExpectedType => f.write_str("Expected a type"),
            CompileError::ExpectedValue => f.write_str("Expected a value"),
            CompileError::ExpectedStreamType => f.write_str("Expected a stream type specifier"),
            CompileError::ManyToManyConnection => {
                f.write_str("Many-to-many connections are not supported")
            }
            CompileError::ChainedConnectionWithMultipleEndpoints => f.write_str(
                "Cannot create a chained sequence of connections when multiple endpoints are specified",
            ),
            CompileError::EndpointNamedInChain => f.write_str(
                "A processor that is chained between two others cannot specify an endpoint name",
            ),
            CompileError::UnresolvedSymbol { name } => write!(f, "Cannot find symbol '{name}'"),
            CompileError::NameInUse { name, .. } => {
                write!(f, "The name '{name}' is already in use")
            }
            CompileError::NotAScope { name } => {
                write!(
                    f,
                    "'{name}' is not a namespace, processor, graph, struct, or enum"
                )
            }
            CompileError::UnexpectedNode => f.write_str("Unexpected node"),
        }
    }
}

impl Diagnostic {
    /// Displays this diagnostic as reported in `unit` of `program`, naming the
    /// file of any note that lies in a different unit.
    pub fn in_unit<'a>(&'a self, unit: UnitId, program: &'a Program) -> DiagnosticInUnit<'a> {
        DiagnosticInUnit {
            diagnostic: self,
            unit,
            program,
        }
    }

    fn previous_declaration(&self) -> Option<UnitLocation> {
        match self.error {
            CompileError::NameInUse {
                previous_declaration,
                ..
            } => previous_declaration,
            _ => None,
        }
    }

    fn write<'n>(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        note_file: impl FnOnce(UnitLocation) -> Option<&'n str>,
    ) -> std::fmt::Result {
        write!(f, "{}: error: {}", self.location.start, self.error)?;

        if let Some(previous) = self.previous_declaration() {
            f.write_str(" //// ")?;
            if let Some(file) = note_file(previous) {
                write!(f, "{file}:")?;
            }
            write!(f, "{}: note: See previous declaration", previous.location)?;
        }

        Ok(())
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write(f, |_| None)
    }
}

pub struct DiagnosticInUnit<'a> {
    diagnostic: &'a Diagnostic,
    unit: UnitId,
    program: &'a Program,
}

impl std::fmt::Display for DiagnosticInUnit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.diagnostic.write(f, |note| {
            (note.unit != self.unit)
                .then(|| self.program[note.unit].source.name().unwrap_or("<unknown>"))
        })
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            SourceFile,
            lexer::{Keyword, Literal},
            token,
        },
    };

    fn at(line: usize, column: usize) -> SourceLocation {
        SourceLocation {
            line: line.into(),
            column: column.into(),
        }
    }

    fn some_unit() -> UnitId {
        Program::new().add_source_file(SourceFile::new(""))
    }

    fn diagnostic_at(line: usize, column: usize, error: CompileError) -> Diagnostic {
        Diagnostic {
            location: (at(line, column)..at(line, column + 1)).into(),
            error,
        }
    }

    #[test]
    fn found_punctuation_is_quoted() {
        let error = CompileError::FoundWhenExpecting {
            found: token!(=),
            expected: token!(:),
        };
        assert_eq!(error.to_string(), r#"Found "=" when expecting ":""#);
    }

    #[test]
    fn found_keyword_is_quoted_and_identifier_is_not() {
        let error = CompileError::FoundWhenExpecting {
            found: TokenKind::Keyword(Keyword::Return),
            expected: TokenKind::Identifier,
        };
        assert_eq!(
            error.to_string(),
            r#"Found "return" when expecting identifier"#
        );
    }

    #[test]
    fn found_literal_is_described_by_its_type() {
        let error = CompileError::FoundWhenExpecting {
            found: TokenKind::Literal(Literal::Int32),
            expected: token!(,),
        };
        assert_eq!(
            error.to_string(),
            r#"Found int32 literal when expecting ",""#
        );
    }

    #[test]
    fn unresolved_symbol_names_the_full_path() {
        let error = CompileError::UnresolvedSymbol {
            name: "blah::cos".into(),
        };
        assert_eq!(error.to_string(), "Cannot find symbol 'blah::cos'");
    }

    #[test]
    fn diagnostic_is_prefixed_with_its_location_and_severity() {
        let diagnostic = diagnostic_at(2, 12, CompileError::ExpectedType);
        assert_eq!(diagnostic.to_string(), "2:12: error: Expected a type");
    }

    #[test]
    fn name_in_use_points_at_previous_declaration() {
        let diagnostic = diagnostic_at(
            3,
            7,
            CompileError::NameInUse {
                name: "A".into(),
                previous_declaration: Some(UnitLocation {
                    unit: some_unit(),
                    location: at(2, 8),
                }),
            },
        );
        assert_eq!(
            diagnostic.to_string(),
            "3:7: error: The name 'A' is already in use //// 2:8: note: See previous declaration"
        );
    }

    fn name_in_use_declared_in(unit: UnitId) -> Diagnostic {
        diagnostic_at(
            3,
            7,
            CompileError::NameInUse {
                name: "A".into(),
                previous_declaration: Some(UnitLocation {
                    unit,
                    location: at(2, 8),
                }),
            },
        )
    }

    #[test]
    fn note_in_another_unit_names_its_file() {
        let mut program = Program::new();
        let library = program.add_source_file(SourceFile::new("").with_name("lib.cmajor"));
        let main = program.add_source_file(SourceFile::new("").with_name("main.cmajor"));

        assert_eq!(
            name_in_use_declared_in(library)
                .in_unit(main, &program)
                .to_string(),
            "3:7: error: The name 'A' is already in use //// lib.cmajor:2:8: note: See previous declaration"
        );
    }

    #[test]
    fn note_in_the_same_unit_has_no_file_name() {
        let mut program = Program::new();
        let main = program.add_source_file(SourceFile::new("").with_name("main.cmajor"));

        assert_eq!(
            name_in_use_declared_in(main)
                .in_unit(main, &program)
                .to_string(),
            "3:7: error: The name 'A' is already in use //// 2:8: note: See previous declaration"
        );
    }

    #[test]
    fn name_in_use_without_previous_declaration_has_no_note() {
        let diagnostic = diagnostic_at(
            3,
            11,
            CompileError::NameInUse {
                name: "std".into(),
                previous_declaration: None,
            },
        );
        assert_eq!(
            diagnostic.to_string(),
            "3:11: error: The name 'std' is already in use"
        );
    }
}
