//! Read operations on the document block tree.
//!
//! Provides query functions to extract outlines, block text, nested tree
//! structures, children lists, and type-filtered block IDs from a parsed
//! [`DocumentBlock`].

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use crate::domain::block_node::BlockNode;
use crate::domain::document::DocumentBlock;
use crate::domain::selectors::BlockSelector;
use crate::error::MdtoolError;
use crate::ops::resolve::resolve_selector;
use crate::primitives::BlockId;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single entry in a document outline (heading hierarchy).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OutlineEntry {
    pub id: BlockId,
    /// Renamed to "lv" for compact serialization.
    #[serde(rename = "lv")]
    pub level: u8,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Nested tree representation returned by [`read_block_tree`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BlockTree {
    pub id: BlockId,
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cols: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<BlockTree>,
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Traverse all `SectionBlock` nodes up to `max_depth` and collect an outline
/// in document order (sorted by `line_range.start`).
///
/// `max_depth` is an inclusive upper bound on heading level (1-6). Pass `6`
/// (or higher) to include all levels.
///
/// `include_paths` controls whether canonical paths are included in the response.
/// Pass `false` for a compact outline (saves ~60% output size on documents with
/// long Unicode paths).
pub fn read_outline(doc: &DocumentBlock, max_depth: u8, include_paths: bool) -> Vec<OutlineEntry> {
    let effective_max = max_depth.min(6);

    let mut entries: Vec<OutlineEntry> = doc
        .block_by_id
        .iter()
        .filter_map(|(id, bn)| match bn {
            BlockNode::Section(s) if s.level <= effective_max => Some(OutlineEntry {
                id: *id,
                level: s.level,
                title: s.title.clone(),
                path: if include_paths { Some(s.path.clone()) } else { None },
            }),
            _ => None,
        })
        .collect();

    entries.sort_by_key(|e| {
        doc.block_by_id
            .get(&e.id)
            .map(|bn| bn.block().line_range.start)
            .unwrap_or(0)
    });

    entries
}

/// Resolve `selector` and return the source text covered by the matched block
/// (via [`DocumentBlock::source_text_for_range`]).
pub fn read_block_text(
    doc: &DocumentBlock,
    selector: &BlockSelector,
) -> Result<String, MdtoolError> {
    let block_id = resolve_selector(doc, selector)?;
    let range = if block_id == doc.root_id() {
        doc.block.line_range
    } else {
        doc.get_block(block_id)
            .ok_or_else(|| MdtoolError::BlockNotFound {
                selector: format!("id={:?}", block_id),
            })?
            .block()
            .line_range
    };
    Ok(doc.source_text_for_range(range))
}

/// Resolve `selector` and return the matched block together with its subtree
/// as a nested [`BlockTree`].
///
/// `depth` controls how many levels of children to include:
/// - `-1` means unlimited (entire subtree).
/// - `0` means the block itself only (no children).
/// - `n` means include up to `n` levels of children.
pub fn read_block_tree(
    doc: &DocumentBlock,
    selector: &BlockSelector,
    depth: i32,
) -> Result<BlockTree, MdtoolError> {
    let block_id = resolve_selector(doc, selector)?;
    Ok(build_block_tree_recursive(doc, block_id, depth))
}

/// Resolve `selector` and return the ordered children of the matched block.
pub fn read_block_children(
    doc: &DocumentBlock,
    selector: &BlockSelector,
) -> Result<Vec<BlockId>, MdtoolError> {
    let block_id = resolve_selector(doc, selector)?;
    if block_id == doc.root_id() {
        return Ok(doc.block.children_ids.clone());
    }
    let bn = doc
        .get_block(block_id)
        .ok_or_else(|| MdtoolError::BlockNotFound {
            selector: format!("id={:?}", block_id),
        })?;
    Ok(bn.block().children_ids.clone())
}

/// Return all block IDs whose `block_type_name()` equals `block_type`.
pub fn read_blocks_by_type(doc: &DocumentBlock, block_type: &str) -> Vec<BlockId> {
    let mut ids: Vec<BlockId> = doc
        .block_by_id
        .iter()
        .filter(|(_, bn)| bn.block_type_name() == block_type)
        .map(|(id, _)| *id)
        .collect();

    ids.sort_by_key(|id| {
        doc.block_by_id
            .get(id)
            .map(|bn| bn.block().line_range.start)
            .unwrap_or(0)
    });

    ids
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Recursively build a [`BlockTree`] starting at `block_id`, descending at
/// most `remaining_depth` levels (`-1` = unlimited).
fn build_block_tree_recursive(
    doc: &DocumentBlock,
    block_id: BlockId,
    remaining_depth: i32,
) -> BlockTree {
    // Special case: the root DocumentBlock is not in block_by_id.
    if block_id == doc.root_id() {
        let block = &doc.block;
        let children = if remaining_depth == 0 {
            Vec::new()
        } else {
            let next_depth = if remaining_depth > 0 {
                remaining_depth - 1
            } else {
                -1
            };
            block
                .children_ids
                .iter()
                .map(|child_id| build_block_tree_recursive(doc, *child_id, next_depth))
                .collect()
        };
        return BlockTree {
            id: block_id,
            block_type: "document".to_string(),
            title: None,
            level: None,
            rows: None,
            cols: None,
            items: None,
            children,
        };
    }

    let bn = doc.get_block(block_id);
    let bn = match bn {
        Some(bn) => bn,
        None => {
            return BlockTree {
                id: block_id,
                block_type: "unknown".to_string(),
                title: None,
                level: None,
                rows: None,
                cols: None,
                items: None,
                children: Vec::new(),
            }
        }
    };

    let block = bn.block();

    // Extract variant-specific metadata.
    let (title, level, rows, cols, items) = extract_block_metadata(bn);

    // Recurse into children if depth budget allows.
    let children = if remaining_depth == 0 {
        Vec::new()
    } else {
        let next_depth = if remaining_depth > 0 {
            remaining_depth - 1
        } else {
            // -1 means unlimited, keep passing -1.
            -1
        };
        block
            .children_ids
            .iter()
            .map(|child_id| build_block_tree_recursive(doc, *child_id, next_depth))
            .collect()
    };

    BlockTree {
        id: block_id,
        block_type: bn.block_type_name().to_string(),
        title,
        level,
        rows,
        cols,
        items,
        children,
    }
}

/// Pull type-specific fields out of a [`BlockNode`] for serialization into
/// [`BlockTree`].
fn extract_block_metadata(bn: &BlockNode) -> (Option<String>, Option<u8>, Option<usize>, Option<usize>, Option<usize>) {
    match bn {
        BlockNode::Section(s) => (
            Some(s.title.clone()),
            Some(s.level),
            None,
            None,
            None,
        ),
        BlockNode::Table(t) => {
            let row_count = 1 + t.body_rows.len(); // header + body
            let col_count = t.header_row.len();
            (None, None, Some(row_count), Some(col_count), None)
        }
        BlockNode::List(l) => {
            let item_count = l.block.children_ids.len();
            (None, None, None, None, Some(item_count))
        }
        _ => (None, None, None, None, None),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::tree_builder::build_tree;
    use crate::domain::selectors::BlockSelector;

    // ---- read_outline -------------------------------------------------------

    #[test]
    fn outline_basic() {
        let input = "# Alpha\n\na\n\n## Beta\n\nb\n\n## Gamma\n\ng\n";
        let doc = build_tree(input).unwrap();
        let outline = read_outline(&doc, 6, true);

        assert_eq!(outline.len(), 3);
        assert_eq!(outline[0].title, "Alpha");
        assert_eq!(outline[0].level, 1);
        assert_eq!(outline[1].title, "Beta");
        assert_eq!(outline[1].level, 2);
        assert_eq!(outline[2].title, "Gamma");
        assert_eq!(outline[2].level, 2);
    }

    #[test]
    fn outline_max_depth_filters_deep_headings() {
        let input = "# A\n\n## B\n\n### C\n\n#### D\n";
        let doc = build_tree(input).unwrap();
        let outline = read_outline(&doc, 2, true);

        assert!(outline.iter().all(|e| e.level <= 2));
        assert_eq!(outline.len(), 2); // A, B only
    }

    #[test]
    fn outline_document_order() {
        let input = "# First\n\n## Second\n\n# Third\n";
        let doc = build_tree(input).unwrap();
        let outline = read_outline(&doc, 6, true);

        assert_eq!(outline[0].title, "First");
        assert_eq!(outline[1].title, "Second");
        assert_eq!(outline[2].title, "Third");
    }

    #[test]
    fn outline_includes_paths() {
        let input = "# Architecture\n\n## Parser\n\n### Builder\n";
        let doc = build_tree(input).unwrap();
        let outline = read_outline(&doc, 6, true);

        assert!(outline.iter().any(|e| e.path.as_deref() == Some("/architecture")));
        assert!(outline.iter().any(|e| e.path.as_deref() == Some("/architecture/parser")));
        assert!(outline.iter().any(|e| e.path.as_deref() == Some("/architecture/parser/builder")));
    }

    #[test]
    fn outline_empty_document() {
        let doc = build_tree("").unwrap();
        let outline = read_outline(&doc, 6, true);
        assert!(outline.is_empty());
    }

    // ---- read_block_text ----------------------------------------------------

    #[test]
    fn read_text_by_path() {
        let input = "# Hello\n\nSome body text.\n";
        let doc = build_tree(input).unwrap();

        let sel = BlockSelector::from_path("/hello");
        let text = read_block_text(&doc, &sel).unwrap();
        assert!(text.contains("Hello"));
    }

    #[test]
    fn read_text_paragraph() {
        let input = "Standalone paragraph.\n";
        let doc = build_tree(input).unwrap();

        // Find the paragraph block ID
        let para_id = doc
            .block_by_id
            .iter()
            .find_map(|(id, bn)| match bn {
                BlockNode::Paragraph(_) => Some(*id),
                _ => None,
            })
            .unwrap();

        let sel = BlockSelector::from_id(para_id);
        let text = read_block_text(&doc, &sel).unwrap();
        assert_eq!(text, "Standalone paragraph.");
    }

    #[test]
    fn read_text_not_found() {
        let doc = build_tree("# Title\n").unwrap();
        let sel = BlockSelector::from_path("/nonexistent");
        let result = read_block_text(&doc, &sel);
        assert!(result.is_err());
    }

    // ---- read_block_tree ----------------------------------------------------

    #[test]
    fn block_tree_root_unlimited() {
        let input = "# A\n\ntext a\n\n## B\n\ntext b\n";
        let doc = build_tree(input).unwrap();

        let sel = BlockSelector::default(); // empty → root
        let tree = read_block_tree(&doc, &sel, -1).unwrap();

        assert_eq!(tree.block_type, "document");
        assert!(!tree.children.is_empty(), "Root should have children");
    }

    #[test]
    fn block_tree_depth_zero_no_children() {
        let input = "# A\n\n## B\n\ntext\n";
        let doc = build_tree(input).unwrap();

        // Find section A
        let a_id = doc
            .block_by_id
            .iter()
            .find_map(|(id, bn)| match bn {
                BlockNode::Section(s) if s.title == "A" => Some(*id),
                _ => None,
            })
            .unwrap();

        let sel = BlockSelector::from_id(a_id);
        let tree = read_block_tree(&doc, &sel, 0).unwrap();

        assert_eq!(tree.block_type, "section");
        assert_eq!(tree.title.as_deref(), Some("A"));
        assert!(tree.children.is_empty(), "depth=0 should suppress children");
    }

    #[test]
    fn block_tree_depth_limited() {
        let input = "# A\n\n## B\n\n### C\n\ntext\n";
        let doc = build_tree(input).unwrap();

        let sel = BlockSelector::default();
        // depth=1 → root + its direct children, no deeper nesting
        let tree = read_block_tree(&doc, &sel, 1).unwrap();

        // The root's children should exist but their children should be empty
        for child in &tree.children {
            assert!(
                child.children.is_empty(),
                "depth=1 means children of root's children should be empty"
            );
        }
    }

    #[test]
    fn block_tree_section_metadata() {
        let input = "# My Title\n\ntext\n";
        let doc = build_tree(input).unwrap();

        let section_id = doc
            .block_by_id
            .iter()
            .find_map(|(id, bn)| match bn {
                BlockNode::Section(s) if s.title == "My Title" => Some(*id),
                _ => None,
            })
            .unwrap();

        let sel = BlockSelector::from_id(section_id);
        let tree = read_block_tree(&doc, &sel, -1).unwrap();

        assert_eq!(tree.title.as_deref(), Some("My Title"));
        assert_eq!(tree.level, Some(1));
    }

    #[test]
    fn block_tree_table_metadata() {
        let input = "# Title\n\n| H1 | H2 |\n|----|-----|\n| a  | b  |\n| c  | d  |\n";
        let doc = build_tree(input).unwrap();

        let table_id = doc
            .block_by_id
            .iter()
            .find_map(|(id, bn)| match bn {
                BlockNode::Table(_) => Some(*id),
                _ => None,
            })
            .unwrap();

        let sel = BlockSelector::from_id(table_id);
        let tree = read_block_tree(&doc, &sel, -1).unwrap();

        assert_eq!(tree.block_type, "table");
        assert_eq!(tree.rows, Some(3)); // 1 header + 2 body
        assert_eq!(tree.cols, Some(2));
    }

    #[test]
    fn block_tree_list_metadata() {
        let input = "- alpha\n- beta\n- gamma\n";
        let doc = build_tree(input).unwrap();

        let list_id = doc
            .block_by_id
            .iter()
            .find_map(|(id, bn)| match bn {
                BlockNode::List(_) => Some(*id),
                _ => None,
            })
            .unwrap();

        let sel = BlockSelector::from_id(list_id);
        let tree = read_block_tree(&doc, &sel, -1).unwrap();

        assert_eq!(tree.block_type, "list");
        assert_eq!(tree.items, Some(3));
    }

    #[test]
    fn block_tree_serialization() {
        let input = "# Title\n\ntext\n";
        let doc = build_tree(input).unwrap();

        let sel = BlockSelector::default();
        let tree = read_block_tree(&doc, &sel, -1).unwrap();

        let json = serde_json::to_string(&tree).unwrap();
        assert!(json.contains("\"type\":\"document\""));
        // Fields with None values should be absent
        assert!(!json.contains("\"title\":null"));
        assert!(!json.contains("\"level\":null"));
    }

    // ---- read_block_children ------------------------------------------------

    #[test]
    fn children_of_root() {
        let input = "# A\n\ntext\n\n# B\n\ntext\n";
        let doc = build_tree(input).unwrap();

        let sel = BlockSelector::default();
        let kids = read_block_children(&doc, &sel).unwrap();
        assert!(kids.len() >= 2, "Root should have at least 2 section children");
    }

    #[test]
    fn children_of_section() {
        let input = "# A\n\n## B\n\n## C\n";
        let doc = build_tree(input).unwrap();

        let sel = BlockSelector {
            title: Some("A".to_string()),
            ..Default::default()
        };
        let kids = read_block_children(&doc, &sel).unwrap();
        assert!(kids.len() >= 2, "Section A should have subsections B and C as children");
    }

    #[test]
    fn children_of_leaf_is_empty() {
        let input = "Standalone paragraph.\n";
        let doc = build_tree(input).unwrap();

        let para_id = doc
            .block_by_id
            .iter()
            .find_map(|(id, bn)| match bn {
                BlockNode::Paragraph(_) => Some(*id),
                _ => None,
            })
            .unwrap();

        let sel = BlockSelector::from_id(para_id);
        let kids = read_block_children(&doc, &sel).unwrap();
        assert!(kids.is_empty());
    }

    // ---- read_blocks_by_type ------------------------------------------------

    #[test]
    fn by_type_sections() {
        let input = "# A\n\n## B\n\n### C\n";
        let doc = build_tree(input).unwrap();

        let ids = read_blocks_by_type(&doc, "section");
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn by_type_paragraphs() {
        let input = "Para one.\n\nPara two.\n\nPara three.\n";
        let doc = build_tree(input).unwrap();

        let ids = read_blocks_by_type(&doc, "paragraph");
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn by_type_returns_document_order() {
        let input = "# First\n\ntext\n\n## Second\n\ntext\n";
        let doc = build_tree(input).unwrap();

        let ids = read_blocks_by_type(&doc, "section");
        let titles: Vec<&str> = ids
            .iter()
            .map(|id| match doc.get_block(*id).unwrap() {
                BlockNode::Section(s) => s.title.as_str(),
                _ => "",
            })
            .collect();
        assert_eq!(titles, vec!["First", "Second"]);
    }

    #[test]
    fn by_type_no_match() {
        let doc = build_tree("# Title\n").unwrap();
        let ids = read_blocks_by_type(&doc, "fence");
        assert!(ids.is_empty());
    }

    #[test]
    fn by_type_mixed_blocks() {
        let input = "# Title\n\nParagraph.\n\n```\ncode\n```\n\n- item\n";
        let doc = build_tree(input).unwrap();

        assert_eq!(read_blocks_by_type(&doc, "section").len(), 1);
        assert_eq!(read_blocks_by_type(&doc, "paragraph").len(), 1);
        assert_eq!(read_blocks_by_type(&doc, "fence").len(), 1);
        assert_eq!(read_blocks_by_type(&doc, "list").len(), 1);
    }
}
