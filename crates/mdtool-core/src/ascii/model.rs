use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use crate::primitives::BlockId;
use crate::error::Diagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub enum AsciiMode { FormatOnly, RepairSafe, RepairAggressive }

impl Default for AsciiMode {
    fn default() -> Self { AsciiMode::FormatOnly }
}

#[derive(Debug, Clone)]
pub struct AsciiBlock {
    pub block_id: BlockId,
    pub info_string: String,
    pub indent: usize,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AsciiEdit {
    pub block_id: BlockId,
    pub changed: bool,
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}
