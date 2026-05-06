use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use crate::primitives::LineRange;

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// A diagnostic message produced during parsing or mutation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Diagnostic {
    /// Machine-readable diagnostic code (e.g., "HEADING_LEVEL_JUMP").
    pub code: String,
    /// Severity level.
    pub severity: Severity,
    /// Human-readable message.
    pub message: String,
    /// 1-based line number (if applicable).
    pub line: Option<usize>,
    /// 1-based column number (if applicable).
    pub column: Option<usize>,
    /// Line range (if applicable).
    pub line_range: Option<LineRange>,
    /// Optional suggested fix.
    pub suggested_fix: Option<String>,
}

/// Error type for all mdtool operations.
#[derive(Debug, thiserror::Error)]
pub enum MdtoolError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("block not found for selector: {selector}")]
    BlockNotFound { selector: String },

    #[error("ambiguous block selector: {n} matches for {selector}")]
    AmbiguousBlock { selector: String, n: usize },

    #[error("transformation error: {0}")]
    Transformation(String),

    #[error("ASCII layout error: {0}")]
    AsciiLayout(String),

    #[error("policy violation: {0}")]
    Policy(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_parse() {
        let err = MdtoolError::Parse("bad input".to_string());
        assert_eq!(err.to_string(), "parse error: bad input");
    }

    #[test]
    fn error_display_block_not_found() {
        let err = MdtoolError::BlockNotFound {
            selector: "path=/Foo".to_string(),
        };
        assert!(err.to_string().contains("path=/Foo"));
    }

    #[test]
    fn error_display_ambiguous() {
        let err = MdtoolError::AmbiguousBlock {
            selector: "title=Foo".to_string(),
            n: 3,
        };
        assert!(err.to_string().contains("3 matches"));
    }

    #[test]
    fn diagnostic_serialization() {
        let diag = Diagnostic {
            code: "HEADING_LEVEL_JUMP".to_string(),
            severity: Severity::Warning,
            message: "Heading level jumped from 1 to 3".to_string(),
            line: Some(5),
            column: None,
            line_range: Some(LineRange { start: 5, end: 5 }),
            suggested_fix: None,
        };
        let json = serde_json::to_string(&diag).unwrap();
        assert!(json.contains("HEADING_LEVEL_JUMP"));
        let deserialized: Diagnostic = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, diag);
    }
}
