use crate::domain::block_node::BlockNode;
use crate::domain::document::DocumentBlock;
use crate::domain::selectors::BlockSelector;
use crate::error::MdtoolError;
use crate::primitives::BlockId;

/// Resolve a `BlockSelector` to a single `BlockId`.
///
/// Resolution strategy (in priority order):
/// 1. **by id** — O(1) lookup via `doc.block_by_id`.
/// 2. **by path** — walk sections for a matching canonical path.
/// 3. **by query fields** (`title`, `level`, `block_type`) — filter all blocks.
///
/// Rules:
/// - Empty selector (all fields `None`) resolves to the `DocumentBlock` root.
/// - 0 matches returns `MdtoolError::BlockNotFound`.
/// - 1 match returns the `BlockId`.
/// - >1 matches returns `MdtoolError::AmbiguousBlock` unless `allow_first_match` is set.
pub fn resolve_selector(
    doc: &DocumentBlock,
    selector: &BlockSelector,
) -> Result<BlockId, MdtoolError> {
    // Empty selector -> document root
    if selector.is_empty() {
        return Ok(doc.root_id());
    }

    // 1. By id — O(1) lookup
    if let Some(id) = selector.id {
        if id == doc.root_id() {
            return Ok(id);
        }
        if doc.block_by_id.contains_key(&id) {
            return Ok(id);
        }
        return Err(MdtoolError::BlockNotFound {
            selector: format!("id={:?}", id),
        });
    }

    // 2. By path — walk sections for matching canonical path
    if let Some(ref path) = selector.path {
        let mut matches: Vec<BlockId> = doc
            .block_by_id
            .iter()
            .filter_map(|(id, bn)| match bn {
                BlockNode::Section(s) if s.path == *path => Some(*id),
                _ => None,
            })
            .collect();

        sort_matches_by_line(&mut matches, doc);

        return match matches.len() {
            0 => Err(MdtoolError::BlockNotFound {
                selector: format!("path={}", path),
            }),
            1 => Ok(matches[0]),
            _ if selector.allow_first_match => Ok(matches[0]),
            n => Err(MdtoolError::AmbiguousBlock {
                selector: format!("path={}", path),
                n,
            }),
        };
    }

    // 3. By query fields (title, level, block_type)
    let mut matches: Vec<BlockId> = doc
        .block_by_id
        .iter()
        .filter(|(_, bn)| matches_query(bn, selector))
        .map(|(id, _)| *id)
        .collect();

    sort_matches_by_line(&mut matches, doc);

    match matches.len() {
        0 => Err(MdtoolError::BlockNotFound {
            selector: selector_summary(selector),
        }),
        1 => Ok(matches[0]),
        _ if selector.allow_first_match => Ok(matches[0]),
        n => Err(MdtoolError::AmbiguousBlock {
            selector: selector_summary(selector),
            n,
        }),
    }
}

/// Sort matched block IDs by their start line number for deterministic ordering.
fn sort_matches_by_line(matches: &mut Vec<BlockId>, doc: &DocumentBlock) {
    matches.sort_by_key(|id| {
        doc.block_by_id
            .get(id)
            .map(|bn| bn.block().line_range.start)
            .unwrap_or(0)
    });
}

/// Check whether a `BlockNode` satisfies the query fields of a selector.
fn matches_query(bn: &BlockNode, selector: &BlockSelector) -> bool {
    if let Some(ref title) = selector.title {
        match bn {
            BlockNode::Section(s) if s.title == *title => {}
            _ => return false,
        }
    }

    if let Some(level) = selector.level {
        match bn {
            BlockNode::Section(s) if s.level == level => {}
            _ => return false,
        }
    }

    if let Some(ref block_type) = selector.block_type {
        if bn.block_type_name() != block_type.as_str() {
            return false;
        }
    }

    true
}

/// Build a human-readable summary of the selector for error messages.
fn selector_summary(selector: &BlockSelector) -> String {
    let mut parts = Vec::new();
    if let Some(ref title) = selector.title {
        parts.push(format!("title={}", title));
    }
    if let Some(level) = selector.level {
        parts.push(format!("level={}", level));
    }
    if let Some(ref bt) = selector.block_type {
        parts.push(format!("block_type={}", bt));
    }
    if let Some(idx) = selector.block_index {
        parts.push(format!("block_index={}", idx));
    }
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::tree_builder::build_tree;
    use crate::domain::selectors::BlockSelector;

    #[test]
    fn empty_selector_resolves_to_root() {
        let doc = build_tree("# Title\n\nbody\n").unwrap();
        let selector = BlockSelector::default();
        let result = resolve_selector(&doc, &selector).unwrap();
        assert_eq!(result, doc.root_id());
    }

    #[test]
    fn resolve_by_id() {
        let doc = build_tree("# Title\n\nbody\n").unwrap();
        let selector = BlockSelector::from_id(BlockId(1));
        let result = resolve_selector(&doc, &selector).unwrap();
        assert_eq!(result, BlockId(1));
    }

    #[test]
    fn resolve_by_id_not_found() {
        let doc = build_tree("# Title\n\nbody\n").unwrap();
        let selector = BlockSelector::from_id(BlockId(999));
        let err = resolve_selector(&doc, &selector).unwrap_err();
        assert!(matches!(err, MdtoolError::BlockNotFound { .. }));
    }

    #[test]
    fn resolve_by_path() {
        let doc = build_tree("# Architecture\n\n## Parser\n\nbody\n").unwrap();
        let selector = BlockSelector::from_path("/architecture/parser");
        let result = resolve_selector(&doc, &selector).unwrap();
        // Verify the result is a section titled "Parser"
        let bn = doc.get_block(result).unwrap();
        match bn {
            BlockNode::Section(s) => assert_eq!(s.title, "Parser"),
            _ => panic!("Expected a Section block"),
        }
    }

    #[test]
    fn resolve_by_path_not_found() {
        let doc = build_tree("# Title\n\nbody\n").unwrap();
        let selector = BlockSelector::from_path("/nonexistent");
        let err = resolve_selector(&doc, &selector).unwrap_err();
        assert!(matches!(err, MdtoolError::BlockNotFound { .. }));
    }

    #[test]
    fn resolve_by_title() {
        let doc = build_tree("# Architecture\n\n## Parser\n\nbody\n").unwrap();
        let selector = BlockSelector {
            title: Some("Parser".to_string()),
            ..Default::default()
        };
        let result = resolve_selector(&doc, &selector).unwrap();
        let bn = doc.get_block(result).unwrap();
        match bn {
            BlockNode::Section(s) => assert_eq!(s.title, "Parser"),
            _ => panic!("Expected a Section block"),
        }
    }

    #[test]
    fn resolve_by_level() {
        let doc = build_tree("# A\n\n## B\n\n### C\n").unwrap();
        let selector = BlockSelector {
            level: Some(2),
            ..Default::default()
        };
        let result = resolve_selector(&doc, &selector).unwrap();
        let bn = doc.get_block(result).unwrap();
        match bn {
            BlockNode::Section(s) => assert_eq!(s.level, 2),
            _ => panic!("Expected a Section block"),
        }
    }

    #[test]
    fn resolve_by_block_type() {
        let doc = build_tree("# Title\n\nparagraph text\n\n```\ncode\n```\n").unwrap();
        let selector = BlockSelector {
            block_type: Some("fence".to_string()),
            ..Default::default()
        };
        let result = resolve_selector(&doc, &selector).unwrap();
        let bn = doc.get_block(result).unwrap();
        assert_eq!(bn.block_type_name(), "fence");
    }

    #[test]
    fn resolve_no_match_returns_block_not_found() {
        let doc = build_tree("# Title\n\nbody\n").unwrap();
        let selector = BlockSelector {
            title: Some("Nonexistent".to_string()),
            ..Default::default()
        };
        let err = resolve_selector(&doc, &selector).unwrap_err();
        assert!(matches!(err, MdtoolError::BlockNotFound { .. }));
    }

    #[test]
    fn resolve_ambiguous_without_allow_first() {
        // Two level-1 headings
        let doc = build_tree("# A\n\na\n\n# B\n\nb\n").unwrap();
        let selector = BlockSelector {
            level: Some(1),
            ..Default::default()
        };
        let err = resolve_selector(&doc, &selector).unwrap_err();
        assert!(matches!(err, MdtoolError::AmbiguousBlock { .. }));
    }

    #[test]
    fn resolve_ambiguous_with_allow_first() {
        let doc = build_tree("# A\n\na\n\n# B\n\nb\n").unwrap();
        let selector = BlockSelector {
            level: Some(1),
            allow_first_match: true,
            ..Default::default()
        };
        let result = resolve_selector(&doc, &selector).unwrap();
        // Should return the first level-1 heading (A)
        let bn = doc.get_block(result).unwrap();
        match bn {
            BlockNode::Section(s) => assert_eq!(s.title, "A"),
            _ => panic!("Expected a Section block"),
        }
    }

    #[test]
    fn resolve_combined_title_and_level() {
        let doc = build_tree("# A\n\n## B\n\n### B\n").unwrap();
        let selector = BlockSelector {
            title: Some("B".to_string()),
            level: Some(3),
            ..Default::default()
        };
        let result = resolve_selector(&doc, &selector).unwrap();
        let bn = doc.get_block(result).unwrap();
        match bn {
            BlockNode::Section(s) => {
                assert_eq!(s.title, "B");
                assert_eq!(s.level, 3);
            }
            _ => panic!("Expected a Section block"),
        }
    }
}
