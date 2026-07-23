//! Diagnostic messages from the compiler and engine.

use {
    serde::Deserialize,
    serde_json::{Map as JsonMap, Value as JsonValue},
};

/// A diagnostic message from the compiler or engine.
#[derive(Debug, Deserialize)]
pub struct DiagnosticMessage {
    #[serde(default)]
    category: Option<Category>,
    severity: Severity,
    message: String,
    #[serde(rename = "fileName")]
    file_name: String,
    #[serde(rename = "sourceLine")]
    source_line: String,
    #[serde(rename = "columnNumber")]
    column_number: usize,
    #[serde(rename = "lineNumber")]
    line_number: usize,
    #[serde(rename = "annotatedLine")]
    annotated_line: String,
    #[serde(rename = "fullDescription")]
    full_description: String,
    #[serde(flatten)]
    _rest: JsonMap<String, JsonValue>,
}

/// A diagnostic category.
#[derive(Debug, Copy, Clone, PartialEq, Deserialize)]
pub enum Category {
    /// A diagnostic message from the compiler.
    #[serde(rename = "compile")]
    Compile,

    /// A diagnostic message from the runtime.
    #[serde(rename = "runtime")]
    Runtime,
}

/// The severity of a diagnostic message.
#[derive(Debug, Copy, Clone, PartialEq, Deserialize)]
pub enum Severity {
    /// An error.
    #[serde(rename = "error")]
    Error,

    /// A warning.
    #[serde(rename = "warning")]
    Warning,

    /// An informational message.
    #[serde(rename = "note")]
    Note,
}

/// A location in a source file.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Location {
    /// The line number.
    pub line: usize,

    /// The column number.
    pub column: usize,
}

impl DiagnosticMessage {
    /// Get the category of the diagnostic message.
    pub fn category(&self) -> Option<Category> {
        self.category
    }

    /// Get the severity of the diagnostic message.
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// Get the message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the name of the file that the diagnostic message is from (if available).
    pub fn file_name(&self) -> Option<&str> {
        (!self.file_name.is_empty()).then(|| self.file_name.as_ref())
    }

    /// Get the location of the diagnostic message.
    pub fn location(&self) -> Location {
        Location {
            line: self.line_number,
            column: self.column_number,
        }
    }

    /// Get the source line that the diagnostic message is from.
    pub fn source_line(&self) -> &str {
        &self.source_line
    }

    /// Get the annotated source line.
    pub fn annotated_line(&self) -> &str {
        &self.annotated_line
    }

    /// Get the full description of the diagnostic message.
    pub fn full_description(&self) -> &str {
        &self.full_description
    }
}
