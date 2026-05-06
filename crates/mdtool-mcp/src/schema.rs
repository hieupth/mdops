use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use mdtool_core::domain::selectors::BlockSelector;
use mdtool_core::ops::edit::EditOp;
use mdtool_core::processing::normalize::NormalizeOptions;
use mdtool_core::ascii::model::AsciiMode;

// ---------------------------------------------------------------------------
// Request types for MCP tools
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadOutlineRequest {
    pub file_path: String,
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,
    #[serde(default)]
    pub include_paths: bool,
}

fn default_max_depth() -> u8 {
    6
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadBlockRequest {
    pub file_path: String,
    #[serde(default)]
    pub selector: Option<BlockSelector>,
    /// View mode: "block", "tree", "text", "children", "by_type".
    #[serde(default = "default_view")]
    pub view: String,
    #[serde(default)]
    pub block_type: Option<String>,
    #[serde(default)]
    pub depth: Option<i32>,
    #[serde(default)]
    pub include_text: Option<bool>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
}

fn default_view() -> String {
    "block".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchRequest {
    pub file_path: String,
    pub query: String,
    #[serde(default)]
    pub selector: Option<BlockSelector>,
    #[serde(default)]
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditRequest {
    pub file_path: String,
    pub operations: Vec<EditOp>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidateRequest {
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NormalizeRequest {
    pub file_path: String,
    #[serde(default)]
    pub options: Option<NormalizeOptions>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FormatAsciiRequest {
    pub file_path: String,
    #[serde(default)]
    pub mode: AsciiMode,
    #[serde(default)]
    pub dry_run: bool,
}
