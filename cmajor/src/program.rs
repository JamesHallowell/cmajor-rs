use {
    crate::{diagnostic::DiagnosticMessage, ffi::ProgramPtr, json},
    serde::Deserialize,
};

/// A Cmajor program.
#[derive(Debug)]
pub struct Program {
    pub(crate) inner: ProgramPtr,
}

/// An error that can occur when parsing a Cmajor program.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// An error occurred while parsing a program.
    #[error("Error parsing program: {0:?}")]
    ParserError(Box<DiagnosticMessage>),

    /// An error occurred whilst parsing the error from the library.
    #[error(transparent)]
    FailedToParseError(#[from] json::Error),
}

impl Program {
    pub(crate) fn parse(&mut self, program: impl AsRef<str>) -> Result<(), ParseError> {
        let file_name: Option<&str> = None;

        match self.inner.parse(file_name, program) {
            Ok(()) => Ok(()),
            Err(error) => {
                let parser_error = json::from_str(error.to_str())?;
                Err(ParseError::ParserError(Box::new(parser_error)))
            }
        }
    }

    /// Returns the current abstract syntax tree.
    pub fn get_syntax_tree(&self) -> Result<ast::Node, json::Error> {
        let syntax_tree = self.inner.get_syntax_tree();
        json::from_str(syntax_tree.to_str())
    }
}

/// The Cmajor Abstract Syntax Tree (AST).
pub mod ast {
    use super::*;

    /// A node in the AST.
    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(tag = "OBJECT")]
    pub enum Node {
        /// A namespace.
        Namespace(Namespace),

        /// A function.
        Function(Function),

        /// A type declaration.
        Type(Type),

        /// An endpoint declaration.
        EndpointDeclaration(Endpoint),

        /// An identifier.
        Identifier(Identifier),

        /// A processor declaration.
        Processor(Processor),

        /// An enum declaration.
        EnumType(Enum),

        /// A struct declaration.
        StructType(Struct),

        /// A primitive type declaration.
        PrimitiveType(Primitive),
    }

    /// A namespace.
    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Namespace {
        /// The name of the namespace.
        pub name: String,

        /// The submodules of the namespace.
        pub sub_modules: Vec<Node>,
    }

    /// A function declaration.
    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Function {
        /// The name of the function.
        pub name: String,
    }

    /// A type.
    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Type {
        /// The name of the type.
        #[serde(rename = "type")]
        pub r#type: String,
    }

    /// A struct type.
    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Struct {
        /// The name of the struct.
        pub name: String,

        /// The names of the members in the struct.
        pub member_names: Vec<String>,

        /// The types of the members in the struct.
        pub member_types: Vec<Node>,
    }

    /// A processor.
    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Processor {
        /// The name of the processor.
        pub name: String,

        /// The enums declared by the processor.
        pub enums: Vec<Enum>,

        /// The structs declared by the processor.
        pub structures: Vec<Struct>,

        /// The processors endpoints.
        pub endpoints: Vec<Endpoint>,

        /// The functions defined on the processor.
        pub functions: Vec<Function>,
    }

    /// An endpoint.
    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Endpoint {
        name: String,
    }

    /// An identifier.
    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Identifier {
        name: String,
    }

    impl AsRef<str> for Identifier {
        fn as_ref(&self) -> &str {
            self.name.as_str()
        }
    }

    /// An enum.
    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Enum {
        /// The name of the enum.
        pub name: String,

        /// The items in the enum.
        pub items: Vec<Identifier>,
    }

    /// A primitive type.
    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Primitive {
        r#type: String,
    }
}
