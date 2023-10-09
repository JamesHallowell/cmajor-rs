use {
    serde::Deserialize,
    serde_json::{Map as JsonMap, Value as JsonValue},
};

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

#[derive(Debug, Copy, Clone, PartialEq, Deserialize)]
pub enum Category {
    #[serde(rename = "compile")]
    Compile,
    #[serde(rename = "runtime")]
    Runtime,
}

#[derive(Debug, Copy, Clone, PartialEq, Deserialize)]
pub enum Severity {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "note")]
    Note,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl DiagnosticMessage {
    pub fn category(&self) -> Option<Category> {
        self.category
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn file_name(&self) -> Option<&str> {
        (!self.file_name.is_empty()).then(|| self.file_name.as_ref())
    }

    pub fn location(&self) -> Location {
        Location {
            line: self.line_number,
            column: self.column_number,
        }
    }

    pub fn source_line(&self) -> &str {
        &self.source_line
    }

    pub fn annotated_line(&self) -> &str {
        &self.annotated_line
    }

    pub fn full_description(&self) -> &str {
        &self.full_description
    }
}
