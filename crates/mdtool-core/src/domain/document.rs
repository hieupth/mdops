use std::collections::HashMap;

use crate::domain::block::Block;
use crate::domain::block_node::BlockNode;
use crate::error::Diagnostic;
use crate::primitives::{BlockId, LineRange};

/// Root block — represents the entire parsed document.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentBlock {
    pub block: Block,
    /// Original source text (after line-ending normalization).
    pub source_text: String,
    /// Normalized text (for patch operations).
    pub normalized_text: String,
    /// Byte offsets of the start of each line (0-based).
    pub line_starts: Vec<usize>,
    /// O(1) lookup of any block by ID.
    pub block_by_id: HashMap<BlockId, BlockNode>,
    /// Diagnostics collected during parsing and operations.
    pub diagnostics: Vec<Diagnostic>,
    /// Optional metadata (e.g., YAML front matter key-value pairs).
    pub metadata: HashMap<String, String>,
}

impl DocumentBlock {
    /// Create a new DocumentBlock for the given source text.
    pub fn new(source_text: String) -> Self {
        let line_starts = compute_line_starts(&source_text);
        let normalized_text = source_text.clone();
        let root_id = BlockId(0);
        let line_count = if source_text.is_empty() {
            0
        } else {
            source_text.lines().count().max(1)
        };
        let root_block = Block {
            id: root_id,
            line_range: LineRange { start: 1, end: line_count },
            parent_id: None,
            children_ids: vec![],
            byte_range: if source_text.is_empty() {
                None
            } else {
                Some(crate::primitives::ByteRange {
                    start: 0,
                    end: source_text.len(),
                })
            },
        };
        let doc = DocumentBlock {
            block: root_block,
            source_text,
            normalized_text,
            line_starts,
            block_by_id: HashMap::new(),
            diagnostics: vec![],
            metadata: HashMap::new(),
        };
        doc
    }

    /// Returns the document's own block ID.
    pub fn root_id(&self) -> BlockId {
        self.block.id
    }

    /// Total number of blocks in the tree (including root).
    pub fn total_blocks(&self) -> usize {
        self.block_by_id.len() + 1 // +1 for root DocumentBlock
    }

    /// O(1) lookup of a block by ID.
    pub fn get_block(&self, id: BlockId) -> Option<&BlockNode> {
        self.block_by_id.get(&id)
    }

    /// Extract source text for a given line range.
    pub fn source_text_for_range(&self, range: LineRange) -> String {
        if self.source_text.is_empty() || range.start > range.end {
            return String::new();
        }

        let lines: Vec<&str> = self.source_text.lines().collect();
        if range.start == 0 || (range.start as usize) > lines.len() {
            return String::new();
        }

        let start_idx = (range.start - 1) as usize;
        let end_idx = (range.end as usize).min(lines.len());
        lines[start_idx..end_idx].join("\n")
    }

    /// Get the total number of lines in the document.
    pub fn line_count(&self) -> usize {
        if self.source_text.is_empty() {
            0
        } else {
            self.source_text.lines().count()
        }
    }
}

/// Compute byte offsets for the start of each line.
pub fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            starts.push(i + 1);
        }
    }
    starts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_document() {
        let doc = DocumentBlock::new("hello\nworld".to_string());
        assert_eq!(doc.root_id(), BlockId(0));
        assert_eq!(doc.line_count(), 2);
    }

    #[test]
    fn empty_document() {
        let doc = DocumentBlock::new("".to_string());
        assert_eq!(doc.line_count(), 0);
    }

    #[test]
    fn source_text_for_range() {
        let doc = DocumentBlock::new("line1\nline2\nline3".to_string());
        let text = doc.source_text_for_range(LineRange { start: 2, end: 3 });
        assert_eq!(text, "line2\nline3");
    }

    #[test]
    fn source_text_for_single_line() {
        let doc = DocumentBlock::new("line1\nline2\nline3".to_string());
        let text = doc.source_text_for_range(LineRange { start: 1, end: 1 });
        assert_eq!(text, "line1");
    }

    #[test]
    fn compute_line_starts_basic() {
        let starts = compute_line_starts("hello\nworld\n");
        assert_eq!(starts, vec![0, 6, 12]);
    }

    #[test]
    fn compute_line_starts_empty() {
        let starts = compute_line_starts("");
        assert_eq!(starts, vec![0]);
    }
}
