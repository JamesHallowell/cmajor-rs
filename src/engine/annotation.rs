use serde_json::{Map as JsonMap, Value as JsonValue};

/// An annotation attached to a definition.
#[derive(Debug, Clone)]
pub struct Annotation(JsonMap<String, JsonValue>);

impl From<JsonMap<String, JsonValue>> for Annotation {
    fn from(annotation: JsonMap<String, JsonValue>) -> Self {
        Self(annotation)
    }
}

impl Annotation {
    /// Returns an iterator over the keys of the annotation.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    /// Returns the value of the annotation with the given key as a string.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(JsonValue::as_str)
    }

    /// Returns the value of the annotation with the given key as a floating-point number.
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.0.get(key).and_then(JsonValue::as_f64)
    }

    /// Returns the value of the annotation with the given key as an integer.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.0.get(key).and_then(JsonValue::as_i64)
    }

    /// Returns the value of the annotation with the given key as a boolean.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.0.get(key).and_then(JsonValue::as_bool)
    }
}
