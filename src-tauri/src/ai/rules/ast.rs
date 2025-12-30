//! Abstract Syntax Tree types for the rule DSL.
//!
//! The rule DSL allows expressing file matching conditions in a declarative way.
//! Rules are parsed into an AST which can then be evaluated against files.

use serde::{Deserialize, Serialize};

/// Root expression type for the rule DSL.
/// Supports boolean operators (AND, OR, NOT), comparisons, and function calls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    /// Logical OR: left || right
    Or(Box<Expression>, Box<Expression>),
    /// Logical AND: left && right
    And(Box<Expression>, Box<Expression>),
    /// Logical NOT: !expr
    Not(Box<Expression>),
    /// Field comparison: file.field op value
    Comparison(Comparison),
    /// Function call: file.function(args) or file.field.function(args)
    FunctionCall(FunctionCall),
    /// Boolean literal: true or false
    Literal(bool),
}

/// Comparison between a file field and a value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comparison {
    /// The file field being compared
    pub field: Field,
    /// The comparison operator
    pub op: ComparisonOp,
    /// The value to compare against
    pub value: Value,
}

/// Supported comparison operators.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComparisonOp {
    /// Equal: ==
    Eq,
    /// Not equal: !=
    Ne,
    /// Greater than: >
    Gt,
    /// Less than: <
    Lt,
    /// Greater than or equal: >=
    Gte,
    /// Less than or equal: <=
    Lte,
    /// Value is in array: IN [...]
    In,
    /// Value matches regex pattern: MATCHES 'pattern'
    Matches,
}

impl ComparisonOp {
    /// Parse operator from string token
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "==" => Some(ComparisonOp::Eq),
            "!=" => Some(ComparisonOp::Ne),
            ">" => Some(ComparisonOp::Gt),
            "<" => Some(ComparisonOp::Lt),
            ">=" => Some(ComparisonOp::Gte),
            "<=" => Some(ComparisonOp::Lte),
            _ => None,
        }
    }
}

/// File fields that can be accessed in rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Field {
    /// File name without extension: file.name
    FileName,
    /// File extension (lowercase, no dot): file.ext
    FileExt,
    /// File size in bytes: file.size
    FileSize,
    /// Full file path: file.path
    FilePath,
    /// Last modified timestamp (ISO 8601 or unix ms): file.modifiedAt
    FileModifiedAt,
    /// Created timestamp (ISO 8601 or unix ms): file.createdAt
    FileCreatedAt,
    /// MIME type: file.mimeType
    FileMimeType,
    /// Whether file is hidden: file.isHidden
    FileIsHidden,
}

impl Field {
    /// Parse field from string identifier.
    /// Supports both camelCase and snake_case variants.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "name" | "filename" => Some(Field::FileName),
            "ext" | "extension" => Some(Field::FileExt),
            "size" | "filesize" => Some(Field::FileSize),
            "path" | "filepath" => Some(Field::FilePath),
            "modifiedat" | "modified_at" | "modified" | "mtime" => Some(Field::FileModifiedAt),
            "createdat" | "created_at" | "created" | "ctime" => Some(Field::FileCreatedAt),
            "mimetype" | "mime_type" | "mime" => Some(Field::FileMimeType),
            "ishidden" | "is_hidden" | "hidden" => Some(Field::FileIsHidden),
            _ => None,
        }
    }

    /// Get the canonical name for this field
    pub fn canonical_name(&self) -> &'static str {
        match self {
            Field::FileName => "name",
            Field::FileExt => "ext",
            Field::FileSize => "size",
            Field::FilePath => "path",
            Field::FileModifiedAt => "modifiedAt",
            Field::FileCreatedAt => "createdAt",
            Field::FileMimeType => "mimeType",
            Field::FileIsHidden => "isHidden",
        }
    }
}

/// Function call on a file or field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The receiver object, typically "file" or a field chain like "file.name"
    pub receiver: String,
    /// The function being called
    pub function: FunctionName,
    /// Arguments passed to the function
    pub args: Vec<Value>,
}

/// Supported function names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionName {
    /// String contains: file.name.contains('text')
    Contains,
    /// String starts with: file.name.startsWith('prefix')
    StartsWith,
    /// String ends with: file.name.endsWith('suffix')
    EndsWith,
    /// Regex match: file.name.matches('pattern')
    Matches,
    /// Semantic similarity score (0.0-1.0): file.vector_similarity('query')
    VectorSimilarity,
}

impl FunctionName {
    /// Parse function name from string identifier.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "contains" => Some(FunctionName::Contains),
            "startswith" | "starts_with" => Some(FunctionName::StartsWith),
            "endswith" | "ends_with" => Some(FunctionName::EndsWith),
            "matches" => Some(FunctionName::Matches),
            "vector_similarity" | "vectorsimilarity" | "similarity" => {
                Some(FunctionName::VectorSimilarity)
            }
            _ => None,
        }
    }

    /// Get the canonical name for this function
    pub fn canonical_name(&self) -> &'static str {
        match self {
            FunctionName::Contains => "contains",
            FunctionName::StartsWith => "startsWith",
            FunctionName::EndsWith => "endsWith",
            FunctionName::Matches => "matches",
            FunctionName::VectorSimilarity => "vector_similarity",
        }
    }
}

/// Values that can appear in rule expressions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// String value: 'text' or "text"
    String(String),
    /// Numeric value: 42, 3.14
    Number(f64),
    /// Boolean value: true or false
    Boolean(bool),
    /// Array of values: ['a', 'b', 'c']
    Array(Vec<Value>),
    /// Size in bytes with unit: 10KB, 5MB, 1GB
    SizeBytes(u64),
    /// Null/None value
    Null,
}

impl Value {
    /// Convert value to string representation
    pub fn as_string(&self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Boolean(b) => Some(b.to_string()),
            Value::SizeBytes(b) => Some(b.to_string()),
            Value::Array(_) | Value::Null => None,
        }
    }

    /// Convert value to number
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            Value::SizeBytes(b) => Some(*b as f64),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Convert value to boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            Value::String(s) => match s.to_lowercase().as_str() {
                "true" | "yes" | "1" => Some(true),
                "false" | "no" | "0" => Some(false),
                _ => None,
            },
            Value::Number(n) => Some(*n != 0.0),
            _ => None,
        }
    }

    /// Convert value to array
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_parsing() {
        assert_eq!(Field::from_str("name"), Some(Field::FileName));
        assert_eq!(Field::from_str("ext"), Some(Field::FileExt));
        assert_eq!(Field::from_str("size"), Some(Field::FileSize));
        assert_eq!(Field::from_str("modifiedAt"), Some(Field::FileModifiedAt));
        assert_eq!(Field::from_str("modified_at"), Some(Field::FileModifiedAt));
        assert_eq!(Field::from_str("isHidden"), Some(Field::FileIsHidden));
        assert_eq!(Field::from_str("unknown"), None);
    }

    #[test]
    fn test_function_parsing() {
        assert_eq!(
            FunctionName::from_str("contains"),
            Some(FunctionName::Contains)
        );
        assert_eq!(
            FunctionName::from_str("startsWith"),
            Some(FunctionName::StartsWith)
        );
        assert_eq!(
            FunctionName::from_str("vector_similarity"),
            Some(FunctionName::VectorSimilarity)
        );
        assert_eq!(FunctionName::from_str("unknown"), None);
    }

    #[test]
    fn test_operator_parsing() {
        assert_eq!(ComparisonOp::from_str("=="), Some(ComparisonOp::Eq));
        assert_eq!(ComparisonOp::from_str("!="), Some(ComparisonOp::Ne));
        assert_eq!(ComparisonOp::from_str(">"), Some(ComparisonOp::Gt));
        assert_eq!(ComparisonOp::from_str(">="), Some(ComparisonOp::Gte));
        assert_eq!(ComparisonOp::from_str("invalid"), None);
    }

    #[test]
    fn test_value_conversions() {
        let s = Value::String("hello".to_string());
        assert_eq!(s.as_string(), Some("hello".to_string()));

        let n = Value::Number(42.0);
        assert_eq!(n.as_number(), Some(42.0));

        let b = Value::Boolean(true);
        assert_eq!(b.as_bool(), Some(true));

        let size = Value::SizeBytes(1024);
        assert_eq!(size.as_number(), Some(1024.0));
    }
}
