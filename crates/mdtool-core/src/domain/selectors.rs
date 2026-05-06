use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use crate::primitives::BlockId;

/// Unified block selector for addressing blocks in the document tree.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct BlockSelector {
    /// Direct block ID lookup.
    pub id: Option<BlockId>,
    /// Canonical section path (e.g., "/Architecture/Parser").
    pub path: Option<String>,
    /// Nth child of resolved parent (0-based).
    pub block_index: Option<usize>,
    /// Filter by type name (e.g., "fence", "table").
    pub block_type: Option<String>,
    /// Match SectionBlock title.
    pub title: Option<String>,
    /// Match SectionBlock level.
    pub level: Option<u8>,
    /// Resolve ambiguity by taking first match.
    #[serde(default)]
    pub allow_first_match: bool,
}

impl BlockSelector {
    pub fn from_id(id: BlockId) -> Self {
        Self {
            id: Some(id),
            ..Default::default()
        }
    }

    pub fn from_path(path: &str) -> Self {
        Self {
            path: Some(path.to_string()),
            ..Default::default()
        }
    }

    /// Returns true if all fields are None (empty selector resolves to root).
    pub fn is_empty(&self) -> bool {
        self.id.is_none()
            && self.path.is_none()
            && self.block_index.is_none()
            && self.block_type.is_none()
            && self.title.is_none()
            && self.level.is_none()
    }
}

/// Position for insert operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub enum InsertPosition {
    Before,
    After,
    Append,
    Index(usize),
}
