use crate::primitives::{BlockId, ByteRange, LineRange};

/// Core data carried by every block node in the document tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Unique identifier assigned sequentially during parsing.
    pub id: BlockId,
    /// 1-based inclusive line range in source text.
    pub line_range: LineRange,
    /// ID of the parent block, None for root.
    pub parent_id: Option<BlockId>,
    /// Ordered list of child block IDs.
    pub children_ids: Vec<BlockId>,
    /// Half-open byte range in source text.
    pub byte_range: Option<ByteRange>,
}

impl Block {
    pub fn has_children(&self) -> bool {
        !self.children_ids.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_no_children() {
        let block = Block {
            id: BlockId(0),
            line_range: LineRange { start: 1, end: 1 },
            parent_id: None,
            children_ids: vec![],
            byte_range: None,
        };
        assert!(!block.has_children());
    }

    #[test]
    fn block_with_children() {
        let block = Block {
            id: BlockId(0),
            line_range: LineRange { start: 1, end: 5 },
            parent_id: None,
            children_ids: vec![BlockId(1), BlockId(2)],
            byte_range: None,
        };
        assert!(block.has_children());
    }
}
