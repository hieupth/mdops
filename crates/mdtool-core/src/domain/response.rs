use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use crate::error::Diagnostic;
use crate::ops::diff::DiffSummary;

/// Compact response for read operations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadResponse {
    pub success: bool,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<Diagnostic>>,
}

/// Full response for write operations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteResponse {
    pub success: bool,
    pub changed: bool,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_summary: Option<DiffSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Error response — returned when success=false.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ErrorResponse {
    pub error_code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_action: Option<String>,
}
