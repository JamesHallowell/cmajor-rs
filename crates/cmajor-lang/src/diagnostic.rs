use crate::{
    compile::Program,
    lexer::{Keyword, LexError, TokenKind},
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
    IllegalCharacter(char),
    UnterminatedComment,
    IdentifierTooLong,
    LeadingUnderscore,
    OctalLiteral,
    IntegerLiteralTooLarge,
    IntegerLiteralNeedsSuffix,
    MalformedNumber,
    UnrecognisedLiteralSuffix,
    ImaginaryIntegerLiteral,
    UnterminatedString,
    MalformedEscapeSequence,
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
    AssignmentInsideExpression,
    EndpointWriteInsideExpression,
    InPlaceOperatorInsideExpression {
        operator: TokenKind,
    },
    RightArrowInStatement,
    IdentifierMustBeUnqualified,
    ExpectedBlockAfterLabel,
    InvalidEndpointName {
        name: String,
    },
    ExpectedGenericWildcardName,
    ExpectedImportModule,
    SemicolonAfterBrace,
    TooManyConsts,
    DoubleIsNotAType,
    EmptyEnum,
    ExternalWithInitialiser,
    ExternalSpecialisationParam,
    DeclarationOutsideNamespace {
        keyword: Keyword,
    },
    NodesInNamespace,
    EndpointsInNamespace,
    EndpointDeclarationsNotFirst,
    ImportsNotAtStart,
    ExpectedFunctionOrVariable,
    LatencyOutsideProcessor,
    ExpectedProcessorName,
    ExpectedStatement,
    UnknownProcessorProperty,
    EventHandlerOutsideProcessor,
    GraphAlias,
    ExpectedTopLevelDecl,
    IllegalVectorElementType,
    ProcessorSpecialisationNotAllowed,
    NamespaceSpecialisationNotAllowed,
    NotYetImplemented(UnimplementedFeature),
    MultipleTypesOnEndpoint,
    AssignmentToProcessorProperty,
    ExpectedSingleVariableDeclaration,
    ExpectedInterpolationType,
    ExpectedProcessorOrEndpoint,
    TooManyParameters,
    LatencyAlreadyDeclared,
    InitialiserListTooLong,
    ExternalFunctionWithBody,
    ExternalInFunction,
    PropertiesOutsideProcessor,
    ConstNonMemberFunction,
    NestingTooDeep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnimplementedFeature {
    MultiDimensionalEndpointArrays,
    MultiDimensionalGraphNodes,
}

impl std::fmt::Display for UnimplementedFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            UnimplementedFeature::MultiDimensionalEndpointArrays => {
                "Multi-dimensional endpoint arrays"
            }
            UnimplementedFeature::MultiDimensionalGraphNodes => "Multi-dimensional graph nodes",
        })
    }
}

impl From<LexError> for CompileError {
    fn from(error: LexError) -> Self {
        match error {
            LexError::IllegalCharacter(c) => CompileError::IllegalCharacter(c),
            LexError::UnterminatedComment => CompileError::UnterminatedComment,
            LexError::IdentifierTooLong => CompileError::IdentifierTooLong,
            LexError::LeadingUnderscore => CompileError::LeadingUnderscore,
            LexError::OctalLiteral => CompileError::OctalLiteral,
            LexError::IntegerLiteralTooLarge => CompileError::IntegerLiteralTooLarge,
            LexError::IntegerLiteralNeedsSuffix => CompileError::IntegerLiteralNeedsSuffix,
            LexError::MalformedNumber => CompileError::MalformedNumber,
            LexError::UnrecognisedLiteralSuffix => CompileError::UnrecognisedLiteralSuffix,
            LexError::ImaginaryIntegerLiteral => CompileError::ImaginaryIntegerLiteral,
            LexError::UnterminatedString => CompileError::UnterminatedString,
            LexError::MalformedEscapeSequence => CompileError::MalformedEscapeSequence,
        }
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::IllegalCharacter(c) => write!(f, "Unexpected character '{c}' in source"),
            CompileError::UnterminatedComment => f.write_str("Unterminated '/*' comment"),
            CompileError::IdentifierTooLong => f.write_str("Identifier too long"),
            CompileError::LeadingUnderscore => {
                f.write_str("Identifiers beginning with an underscore are reserved for system use")
            }
            CompileError::OctalLiteral => f.write_str("Octal literals are not supported"),
            CompileError::IntegerLiteralTooLarge => {
                f.write_str("Integer literal is too large to be represented")
            }
            CompileError::IntegerLiteralNeedsSuffix => f.write_str(
                "This value is too large to fit into an int32, did you mean to add an 'i64' suffix?",
            ),
            CompileError::MalformedNumber => f.write_str("Syntax error in numeric constant"),
            CompileError::UnrecognisedLiteralSuffix => f.write_str("Unrecognised suffix on literal"),
            CompileError::ImaginaryIntegerLiteral => f.write_str(
                "The 'i' suffix for complex numbers requires a floating-point literal",
            ),
            CompileError::UnterminatedString => {
                f.write_str("Unexpected end-of-input in string constant")
            }
            CompileError::MalformedEscapeSequence => {
                f.write_str("Syntax error in unicode escape sequence")
            }
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
            CompileError::AssignmentInsideExpression => {
                f.write_str("Assignment is not allowed inside an expression")
            }
            CompileError::EndpointWriteInsideExpression => {
                f.write_str("Writing to an endpoint is not allowed inside an expression")
            }
            CompileError::InPlaceOperatorInsideExpression { operator } => write!(
                f,
                "The in-place operator '{operator}' must be used as a statement, not an expression"
            ),
            CompileError::RightArrowInStatement => f.write_str(
                "The '->' operator can only be used in connection declarations - did you mean to use the '<-' operator instead?",
            ),
            CompileError::IdentifierMustBeUnqualified => {
                f.write_str("This identifier cannot have a namespace qualifier")
            }
            CompileError::ExpectedBlockAfterLabel => f.write_str(
                "Expected a label name to be followed by a loop statement or braced block",
            ),
            CompileError::InvalidEndpointName { name } => {
                write!(f, "The name '{name}' is not a valid endpoint name")
            }
            CompileError::ExpectedGenericWildcardName => {
                f.write_str("Expected a generic function wildcard name")
            }
            CompileError::ExpectedImportModule => f.write_str("Expected a module identifier"),
            CompileError::SemicolonAfterBrace => {
                f.write_str("A brace-enclosed declaration should not be followed by a semicolon")
            }
            CompileError::TooManyConsts => {
                f.write_str("The 'const' keyword cannot be applied to a type that is already const")
            }
            CompileError::DoubleIsNotAType => {
                f.write_str("'double' is not a type name in Cmaj - did you mean `float64`?")
            }
            CompileError::EmptyEnum => {
                f.write_str("An enum declaration must contain at least one item")
            }
            CompileError::ExternalWithInitialiser => {
                f.write_str("External variables cannot be given an initialiser value")
            }
            CompileError::ExternalSpecialisationParam => {
                f.write_str("The 'external' flag can only be applied to state variables")
            }
            CompileError::DeclarationOutsideNamespace { keyword } => write!(
                f,
                "A {} can only be defined inside a namespace",
                keyword.as_str()
            ),
            CompileError::NodesInNamespace => {
                f.write_str("Nodes cannot be declared within namespaces")
            }
            CompileError::EndpointsInNamespace => {
                f.write_str("A namespace cannot contain endpoint declarations")
            }
            CompileError::EndpointDeclarationsNotFirst => f.write_str(
                "Endpoint declarations must all appear at the start of the processor",
            ),
            CompileError::ImportsNotAtStart => f.write_str(
                "Import statements can only be declared at the start of a namespace",
            ),
            CompileError::ExpectedFunctionOrVariable => {
                f.write_str("Expected a function or variable declaration")
            }
            CompileError::LatencyOutsideProcessor => {
                f.write_str("The processor.latency value can only be declared in a processor")
            }
            CompileError::ExpectedProcessorName => f.write_str("Expected a processor name"),
            CompileError::ExpectedStatement => f.write_str("Expected a statement"),
            CompileError::UnknownProcessorProperty => f.write_str("Unknown processor property"),
            CompileError::EventHandlerOutsideProcessor => {
                f.write_str("Event handlers can only be declared inside a processor or graph")
            }
            CompileError::GraphAlias => f.write_str(
                "If you're trying to declare an alias for a graph, use the 'processor' keyword",
            ),
            CompileError::ExpectedTopLevelDecl => {
                f.write_str("Expected a graph, processor or namespace declaration")
            }
            CompileError::IllegalVectorElementType => {
                f.write_str("Cannot create a vector with elements that are not primitive types")
            }
            CompileError::ProcessorSpecialisationNotAllowed => {
                f.write_str("Processor specialisations may only be used in graphs")
            }
            CompileError::NamespaceSpecialisationNotAllowed => {
                f.write_str("Namespace specialisations may only be used in namespaces")
            }
            CompileError::NotYetImplemented(feature) => {
                write!(f, "Language feature not yet implemented: {feature}!")
            }
            CompileError::MultipleTypesOnEndpoint => {
                f.write_str("Multiple data types not supported by this endpoint type")
            }
            CompileError::AssignmentToProcessorProperty => {
                f.write_str("Processor properties are constants, and cannot be modified")
            }
            CompileError::ExpectedSingleVariableDeclaration => {
                f.write_str("Expected a single variable declaraion")
            }
            CompileError::ExpectedInterpolationType => f.write_str("Expected an interpolation type"),
            CompileError::ExpectedProcessorOrEndpoint => {
                f.write_str("Expected a processor name or endpoint")
            }
            CompileError::TooManyParameters => f.write_str("Too many function parameters"),
            CompileError::LatencyAlreadyDeclared => {
                f.write_str("The processor.latency value must not be set more than once")
            }
            CompileError::InitialiserListTooLong => {
                f.write_str("Initialiser list exceeds max length limit")
            }
            CompileError::ExternalFunctionWithBody => {
                f.write_str("An external function declaration cannot have a body")
            }
            CompileError::ExternalInFunction => {
                f.write_str("External constants cannot be declared inside a function")
            }
            CompileError::PropertiesOutsideProcessor => f.write_str(
                "Processor properties are only valid inside a processor declaration",
            ),
            CompileError::ConstNonMemberFunction => {
                f.write_str("Only struct member functions can be declared `const`")
            }
            CompileError::NestingTooDeep => f.write_str("Code nesting is too deep"),
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
