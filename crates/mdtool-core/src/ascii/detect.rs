use crate::domain::block_node::BlockNode;
use crate::domain::document::DocumentBlock;
use super::model::AsciiBlock;

/// Info strings that identify ASCII art fence blocks.
const ASCII_INFO_STRINGS: &[&str] = &["ascii", "box", "diagram"];

/// Returns true if the info string identifies an ASCII art block.
fn is_ascii_info_string(info_string: &str) -> bool {
    let lower = info_string.to_lowercase();
    ASCII_INFO_STRINGS.iter().any(|&s| lower == s)
}

/// Detect ASCII art blocks: FenceBlock with info_string in {"ascii", "box", "diagram"}.
///
/// Iterates `doc.block_by_id`, finds FenceBlock nodes where `info_string` matches.
/// For each, splits raw_text into lines, computes minimum indentation, returns AsciiBlock.
pub fn detect_ascii_blocks(doc: &DocumentBlock) -> Vec<AsciiBlock> {
    let mut results = Vec::new();

    for (block_id, node) in &doc.block_by_id {
        if let BlockNode::Fence(fence) = node {
            if let Some(ref info) = fence.info_string {
                if is_ascii_info_string(info) {
                    let lines: Vec<String> = fence.raw_text.lines().map(|l| l.to_string()).collect();
                    let indent = compute_min_indent(&lines);
                    results.push(AsciiBlock {
                        block_id: *block_id,
                        info_string: info.clone(),
                        indent,
                        lines,
                    });
                }
            }
        }
    }

    results
}

/// Compute minimum indentation across all non-empty lines.
/// Returns 0 if all lines are empty or there are no lines.
fn compute_min_indent(lines: &[String]) -> usize {
    let mut min_indent = usize::MAX;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let leading = line.chars().take_while(|&c| c == ' ' || c == '\t').count();
        if leading < min_indent {
            min_indent = leading;
        }
    }
    if min_indent == usize::MAX { 0 } else { min_indent }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::leaf_blocks::FenceBlock;
    use crate::domain::block::Block;
    use crate::primitives::{BlockId, LineRange};

    fn make_block(id: u32) -> Block {
        Block {
            id: BlockId(id),
            line_range: LineRange { start: 1, end: 1 },
            parent_id: None,
            children_ids: vec![],
            byte_range: None,
        }
    }

    fn make_doc_with_fences(fences: Vec<(u32, Option<&str>, &str)>) -> DocumentBlock {
        let mut doc = DocumentBlock::new(String::new());
        for (id, info_string, raw_text) in fences {
            let node = BlockNode::Fence(FenceBlock {
                block: make_block(id),
                language: info_string.map(|s| s.split_whitespace().next().unwrap_or(s).to_string()),
                info_string: info_string.map(|s| s.to_string()),
                raw_text: raw_text.to_string(),
            });
            doc.block_by_id.insert(BlockId(id), node);
        }
        doc
    }

    #[test]
    fn detect_finds_ascii_fences() {
        let doc = make_doc_with_fences(vec![
            (1, Some("ascii"), "hello"),
            (2, Some("box"), "+---+\n| a |\n+---+"),
            (3, Some("diagram"), "a -> b"),
        ]);
        let blocks = detect_ascii_blocks(&doc);
        assert_eq!(blocks.len(), 3);
        let ids: Vec<BlockId> = blocks.iter().map(|b| b.block_id).collect();
        assert!(ids.contains(&BlockId(1)));
        assert!(ids.contains(&BlockId(2)));
        assert!(ids.contains(&BlockId(3)));
    }

    #[test]
    fn detect_skips_other_fences() {
        let doc = make_doc_with_fences(vec![
            (1, Some("rust"), "fn main() {}"),
            (2, Some("python"), "print('hi')"),
            (3, Some("ascii"), "hello"),
        ]);
        let blocks = detect_ascii_blocks(&doc);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_id, BlockId(3));
    }

    #[test]
    fn detect_skips_no_info_string() {
        let doc = make_doc_with_fences(vec![
            (1, None, "some code"),
        ]);
        let blocks = detect_ascii_blocks(&doc);
        assert!(blocks.is_empty());
    }

    #[test]
    fn detect_case_insensitive() {
        let doc = make_doc_with_fences(vec![
            (1, Some("ASCII"), "hello"),
            (2, Some("Box"), "+---+"),
            (3, Some("Diagram"), "a->b"),
        ]);
        let blocks = detect_ascii_blocks(&doc);
        assert_eq!(blocks.len(), 3);
    }

    #[test]
    fn detect_computes_indent() {
        let doc = make_doc_with_fences(vec![
            (1, Some("ascii"), "  line1\n    line2\n  line3"),
        ]);
        let blocks = detect_ascii_blocks(&doc);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].indent, 2);
    }

    #[test]
    fn detect_min_indent_all_empty() {
        let doc = make_doc_with_fences(vec![
            (1, Some("ascii"), "\n\n"),
        ]);
        let blocks = detect_ascii_blocks(&doc);
        assert_eq!(blocks[0].indent, 0);
    }
}
