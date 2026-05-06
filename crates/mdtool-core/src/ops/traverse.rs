use crate::domain::block::Block;
use crate::domain::block_node::BlockNode;
use crate::domain::document::DocumentBlock;
use crate::primitives::BlockId;

/// Retrieve the `Block` for a given ID, handling the root DocumentBlock
/// which is stored in `doc.block` rather than `doc.block_by_id`.
fn get_block(doc: &DocumentBlock, id: BlockId) -> Option<Block> {
    if id == doc.root_id() {
        Some(doc.block.clone())
    } else {
        doc.block_by_id.get(&id).map(|bn| bn.block().clone())
    }
}

/// Depth-first traversal of a block and all its descendants.
///
/// Returns the block itself followed by all descendants in pre-order
/// (parent before children, left to right).
pub fn walk(block_id: BlockId, doc: &DocumentBlock) -> Vec<BlockId> {
    let mut result = Vec::new();
    let mut stack = vec![block_id];

    while let Some(id) = stack.pop() {
        result.push(id);
        if let Some(block) = get_block(doc, id) {
            // Push children in reverse order so leftmost child is processed first
            for child in block.children_ids.iter().rev() {
                stack.push(*child);
            }
        }
    }

    result
}

/// Return the ordered list of child block IDs for the given block.
pub fn find_children(doc: &DocumentBlock, id: BlockId) -> Vec<BlockId> {
    get_block(doc, id)
            .map(|b| b.children_ids)
            .unwrap_or_default()
}

/// Walk up the parent chain from `id` to the root, returning all ancestors
/// (excluding `id` itself) in order from immediate parent to root.
pub fn find_ancestors(doc: &DocumentBlock, id: BlockId) -> Vec<BlockId> {
    let mut ancestors = Vec::new();
    let mut current = get_block(doc, id).and_then(|b| b.parent_id);

    while let Some(parent_id) = current {
        ancestors.push(parent_id);
        current = get_block(doc, parent_id).and_then(|b| b.parent_id);
    }

    ancestors
}

/// Return all block IDs whose `block_type_name()` matches the given type string.
pub fn find_by_type(doc: &DocumentBlock, block_type: &str) -> Vec<BlockId> {
    doc.block_by_id
        .iter()
        .filter(|(_, bn)| bn.block_type_name() == block_type)
        .map(|(id, _)| *id)
        .collect()
}

/// Resolve a canonical path string to the matching `SectionBlock`.
///
/// Returns `None` if no section has the given path.
pub fn find_by_path(doc: &DocumentBlock, path: &str) -> Option<BlockId> {
    doc.block_by_id
        .iter()
        .find_map(|(id, bn)| match bn {
            BlockNode::Section(s) if s.path == path => Some(*id),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::tree_builder::build_tree;

    /// Helper: build a simple document tree for testing.
    /// ```
    /// # Alpha
    ///
    /// alpha text
    ///
    /// ## Beta
    ///
    /// beta text
    ///
    /// ### Gamma
    ///
    /// gamma text
    ///
    /// ## Delta
    ///
    /// delta text
    /// ```
    fn simple_doc() -> DocumentBlock {
        let input = "# Alpha\n\nalpha text\n\n## Beta\n\nbeta text\n\n### Gamma\n\ngamma text\n\n## Delta\n\ndelta text\n";
        build_tree(input).unwrap()
    }

    #[test]
    fn walk_root_returns_all_blocks() {
        let doc = simple_doc();
        let all = walk(doc.root_id(), &doc);
        // Root should be the first element
        assert_eq!(all[0], doc.root_id());
        // Should contain root + 4 sections + 4 paragraphs = 9 blocks minimum
        assert!(all.len() >= 9, "walk from root should return all blocks, got {}", all.len());
    }

    #[test]
    fn walk_section_returns_subtree() {
        let doc = simple_doc();
        let alpha_id = doc
            .block_by_id
            .iter()
            .find_map(|(id, bn)| match bn {
                BlockNode::Section(s) if s.title == "Alpha" => Some(*id),
                _ => None,
            })
            .unwrap();

        let subtree = walk(alpha_id, &doc);
        assert!(subtree.len() > 1, "Alpha subtree should contain descendants");
        assert_eq!(subtree[0], alpha_id);
        // Alpha should contain Beta, Gamma, Delta as descendants
        let titles_in_subtree: Vec<&str> = subtree
            .iter()
            .filter_map(|id| match doc.block_by_id.get(id) {
                Some(BlockNode::Section(s)) => Some(s.title.as_str()),
                _ => None,
            })
            .collect();
        assert!(titles_in_subtree.contains(&"Beta"));
        assert!(titles_in_subtree.contains(&"Gamma"));
        assert!(titles_in_subtree.contains(&"Delta"));
    }

    #[test]
    fn walk_leaf_returns_single() {
        let doc = simple_doc();
        let para_id = doc
            .block_by_id
            .iter()
            .find_map(|(id, bn)| match bn {
                BlockNode::Paragraph(_) => Some(*id),
                _ => None,
            })
            .unwrap();

        let result = walk(para_id, &doc);
        assert_eq!(result, vec![para_id]);
    }

    #[test]
    fn find_children_of_root() {
        let doc = simple_doc();
        let children = find_children(&doc, doc.root_id());
        assert!(!children.is_empty(), "Root should have children");
        // First child should be the Alpha section
        let first = doc.block_by_id.get(&children[0]).unwrap();
        match first {
            BlockNode::Section(s) => assert_eq!(s.title, "Alpha"),
            _ => panic!("First child of root should be the Alpha section"),
        }
    }

    #[test]
    fn find_children_of_section() {
        let doc = simple_doc();
        let alpha_id = doc
            .block_by_id
            .iter()
            .find_map(|(id, bn)| match bn {
                BlockNode::Section(s) if s.title == "Alpha" => Some(*id),
                _ => None,
            })
            .unwrap();

        let children = find_children(&doc, alpha_id);
        assert!(!children.is_empty(), "Alpha should have children (Beta, Delta, paragraph)");
    }

    #[test]
    fn find_children_of_leaf_is_empty() {
        let doc = simple_doc();
        let para_id = doc
            .block_by_id
            .iter()
            .find_map(|(id, bn)| match bn {
                BlockNode::Paragraph(_) => Some(*id),
                _ => None,
            })
            .unwrap();

        let children = find_children(&doc, para_id);
        assert!(children.is_empty());
    }

    #[test]
    fn find_children_of_nonexistent_is_empty() {
        let doc = simple_doc();
        let children = find_children(&doc, BlockId(9999));
        assert!(children.is_empty());
    }

    #[test]
    fn find_ancestors_of_deep_section() {
        let doc = simple_doc();
        // Gamma is deepest: Gamma -> Beta -> Alpha -> Root
        let gamma_id = doc
            .block_by_id
            .iter()
            .find_map(|(id, bn)| match bn {
                BlockNode::Section(s) if s.title == "Gamma" => Some(*id),
                _ => None,
            })
            .unwrap();

        let ancestors = find_ancestors(&doc, gamma_id);
        assert!(ancestors.len() >= 2, "Gamma should have at least 2 ancestors");
        // Immediate parent should be Beta
        let first_ancestor = doc.block_by_id.get(&ancestors[0]).unwrap();
        match first_ancestor {
            BlockNode::Section(s) => assert_eq!(s.title, "Beta"),
            _ => panic!("First ancestor of Gamma should be Beta section"),
        }
        // Last ancestor should be root
        assert_eq!(*ancestors.last().unwrap(), doc.root_id());
    }

    #[test]
    fn find_ancestors_of_root_is_empty() {
        let doc = simple_doc();
        let ancestors = find_ancestors(&doc, doc.root_id());
        assert!(ancestors.is_empty());
    }

    #[test]
    fn find_ancestors_of_top_level_section() {
        let doc = simple_doc();
        let alpha_id = doc
            .block_by_id
            .iter()
            .find_map(|(id, bn)| match bn {
                BlockNode::Section(s) if s.title == "Alpha" => Some(*id),
                _ => None,
            })
            .unwrap();

        let ancestors = find_ancestors(&doc, alpha_id);
        assert_eq!(ancestors.len(), 1);
        assert_eq!(ancestors[0], doc.root_id());
    }

    #[test]
    fn find_by_type_returns_sections() {
        let doc = simple_doc();
        let sections = find_by_type(&doc, "section");
        assert_eq!(sections.len(), 4, "Should find Alpha, Beta, Gamma, Delta sections");
    }

    #[test]
    fn find_by_type_returns_paragraphs() {
        let doc = simple_doc();
        let paragraphs = find_by_type(&doc, "paragraph");
        assert_eq!(paragraphs.len(), 4, "Should find 4 paragraphs (one per section)");
    }

    #[test]
    fn find_by_type_unknown_returns_empty() {
        let doc = simple_doc();
        let unknowns = find_by_type(&doc, "fence");
        assert!(unknowns.is_empty());
    }

    #[test]
    fn find_by_path_exact_match() {
        let doc = simple_doc();
        let result = find_by_path(&doc, "/alpha/beta");
        assert!(result.is_some());
        let bn = doc.block_by_id.get(&result.unwrap()).unwrap();
        match bn {
            BlockNode::Section(s) => assert_eq!(s.title, "Beta"),
            _ => panic!("Expected a Section block"),
        }
    }

    #[test]
    fn find_by_path_nested() {
        let doc = simple_doc();
        let result = find_by_path(&doc, "/alpha/beta/gamma");
        assert!(result.is_some());
        let bn = doc.block_by_id.get(&result.unwrap()).unwrap();
        match bn {
            BlockNode::Section(s) => assert_eq!(s.title, "Gamma"),
            _ => panic!("Expected a Section block"),
        }
    }

    #[test]
    fn find_by_path_top_level() {
        let doc = simple_doc();
        let result = find_by_path(&doc, "/alpha");
        assert!(result.is_some());
        let bn = doc.block_by_id.get(&result.unwrap()).unwrap();
        match bn {
            BlockNode::Section(s) => assert_eq!(s.title, "Alpha"),
            _ => panic!("Expected a Section block"),
        }
    }

    #[test]
    fn find_by_path_no_match() {
        let doc = simple_doc();
        let result = find_by_path(&doc, "/nonexistent");
        assert!(result.is_none());
    }
}
