use std::collections::HashMap;

use crate::domain::block_node::BlockNode;
use crate::domain::document::DocumentBlock;
use crate::domain::leaf_blocks::FenceBlock;
use crate::domain::section::SectionBlock;
use crate::error::{Diagnostic, Severity};

/// Validate a parsed document and return a list of diagnostics.
///
/// Checks:
/// 1. `DUPLICATE_SECTION_PATH`: two sections with same canonical path and same ordinal.
/// 2. `UNCLOSED_FENCE`: FenceBlock whose source text has no closing fence marker.
/// 3. `EMPTY_SECTION`: SectionBlock with no children_ids.
///
/// Note: `HEADING_LEVEL_JUMP` is already emitted during tree building and
/// collected from `doc.diagnostics`.
pub fn validate(doc: &DocumentBlock) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // Collect pre-existing diagnostics (e.g., HEADING_LEVEL_JUMP from tree building)
    diagnostics.extend(doc.diagnostics.clone());

    let mut section_path_ordinals: HashMap<(String, u32), usize> = HashMap::new();

    for (_id, node) in &doc.block_by_id {
        match node {
            BlockNode::Section(section) => {
                check_duplicate_section_path(section, &mut section_path_ordinals, &mut diagnostics);
                check_empty_section(section, &mut diagnostics);
            }
            BlockNode::Fence(fence) => {
                check_unclosed_fence(fence, &doc.source_text, &mut diagnostics);
            }
            _ => {}
        }
    }

    diagnostics
}

/// Check for duplicate section paths: two sections with the same canonical path and ordinal.
fn check_duplicate_section_path(
    section: &SectionBlock,
    seen: &mut HashMap<(String, u32), usize>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let key = (section.path.clone(), section.ordinal);
    let count = seen.entry(key.clone()).or_insert(0);
    *count += 1;

    if *count > 1 {
        diagnostics.push(Diagnostic {
            code: "DUPLICATE_SECTION_PATH".to_string(),
            severity: Severity::Warning,
            message: format!(
                "Duplicate section path '{}' with ordinal {} (occurrence {})",
                section.path, section.ordinal, count
            ),
            line: Some(section.block.line_range.start),
            column: None,
            line_range: Some(section.block.line_range),
            suggested_fix: Some(format!(
                "Rename or renumber section '{}' to disambiguate",
                section.title
            )),
        });
    }
}

/// Check for unclosed fenced code blocks.
///
/// A fence is considered unclosed if the source text for the fence block's
/// line range contains an opening fence marker (``` or ~~~) on the first line
/// but no matching closing fence marker on a subsequent line within the block.
fn check_unclosed_fence(
    fence: &FenceBlock,
    source_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let lines: Vec<&str> = source_text.lines().collect();
    let start = fence.block.line_range.start;
    let end = fence.block.line_range.end;

    if start == 0 || (start as usize) > lines.len() {
        return;
    }

    // Extract the lines belonging to this fence block
    let start_idx = (start - 1) as usize;
    let end_idx = (end as usize).min(lines.len());
    if start_idx >= end_idx {
        return;
    }

    let fence_lines = &lines[start_idx..end_idx];

    // First line should be the opening fence
    let first_line = fence_lines[0].trim_start();
    let is_backtick = first_line.starts_with("```");
    let is_tilde = first_line.starts_with("~~~");

    if !is_backtick && !is_tilde {
        return;
    }

    let fence_char = if is_backtick { '`' } else { '~' };
    let fence_len = first_line
        .chars()
        .take_while(|&c| c == fence_char)
        .count();

    if fence_len < 3 {
        return;
    }

    // Look for closing fence on lines after the first
    let has_closer = fence_lines[1..].iter().any(|line| {
        let trimmed = line.trim_start();
        if !trimmed.starts_with(fence_char) {
            return false;
        }
        let close_len = trimmed
            .chars()
            .take_while(|&c| c == fence_char)
            .count();
        if close_len < fence_len {
            return false;
        }
        // Closing fence should only have optional trailing whitespace after fence chars
        let rest = &trimmed[close_len..];
        rest.trim().is_empty()
    });

    if !has_closer {
        diagnostics.push(Diagnostic {
            code: "UNCLOSED_FENCE".to_string(),
            severity: Severity::Error,
            message: format!(
                "Unclosed fenced code block starting at line {}",
                start
            ),
            line: Some(start),
            column: None,
            line_range: Some(fence.block.line_range),
            suggested_fix: Some(format!(
                "Add closing fence '{}' at the end of the code block",
                std::iter::repeat(fence_char).take(fence_len).collect::<String>()
            )),
        });
    }
}

/// Check for empty sections (no children).
fn check_empty_section(section: &SectionBlock, diagnostics: &mut Vec<Diagnostic>) {
    if section.block.children_ids.is_empty() {
        diagnostics.push(Diagnostic {
            code: "EMPTY_SECTION".to_string(),
            severity: Severity::Warning,
            message: format!(
                "Section '{}' at line {} has no content",
                section.title, section.block.line_range.start
            ),
            line: Some(section.block.line_range.start),
            column: None,
            line_range: Some(section.block.line_range),
            suggested_fix: Some(format!(
                "Add content under heading '{}' or remove the empty section",
                section.title
            )),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::block::Block;
    use crate::domain::section::{HeadingVariant, SectionBlock};
    use crate::domain::leaf_blocks::FenceBlock;
    use crate::primitives::{BlockId, LineRange};

    fn make_block(id: u32, start: usize, end: usize) -> Block {
        Block {
            id: BlockId(id),
            line_range: LineRange { start, end },
            parent_id: None,
            children_ids: vec![],
            byte_range: None,
        }
    }

    #[test]
    fn validate_empty_document() {
        let doc = DocumentBlock::new(String::new());
        let diags = validate(&doc);
        assert!(diags.is_empty());
    }

    #[test]
    fn validate_duplicate_section_path() {
        let mut doc = DocumentBlock::new("# A\n## B\n# A".to_string());

        // Two sections with the same path and ordinal
        let section1 = SectionBlock {
            block: make_block(1, 1, 1),
            level: 1,
            title: "A".to_string(),
            slug: "a".to_string(),
            path: "/A".to_string(),
            ordinal: 1,
            variant: HeadingVariant::Atx,
        };

        let section2 = SectionBlock {
            block: make_block(2, 3, 3),
            level: 1,
            title: "A".to_string(),
            slug: "a".to_string(),
            path: "/A".to_string(),
            ordinal: 1,
            variant: HeadingVariant::Atx,
        };

        doc.block_by_id.insert(BlockId(1), BlockNode::Section(section1));
        doc.block_by_id.insert(BlockId(2), BlockNode::Section(section2));

        let diags = validate(&doc);
        let dup_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.code == "DUPLICATE_SECTION_PATH")
            .collect();
        assert_eq!(dup_diags.len(), 1);
        assert_eq!(dup_diags[0].severity, Severity::Warning);
    }

    #[test]
    fn validate_no_duplicate_different_ordinals() {
        let mut doc = DocumentBlock::new("# A\n## A".to_string());

        let section1 = SectionBlock {
            block: make_block(1, 1, 1),
            level: 1,
            title: "A".to_string(),
            slug: "a".to_string(),
            path: "/A".to_string(),
            ordinal: 1,
            variant: HeadingVariant::Atx,
        };

        let section2 = SectionBlock {
            block: make_block(2, 2, 2),
            level: 2,
            title: "A".to_string(),
            slug: "a".to_string(),
            path: "/A/A".to_string(),
            ordinal: 1,
            variant: HeadingVariant::Atx,
        };

        doc.block_by_id.insert(BlockId(1), BlockNode::Section(section1));
        doc.block_by_id.insert(BlockId(2), BlockNode::Section(section2));

        let diags = validate(&doc);
        let dup_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.code == "DUPLICATE_SECTION_PATH")
            .collect();
        assert!(dup_diags.is_empty());
    }

    #[test]
    fn validate_unclosed_fence() {
        let source = "```\nlet x = 1;\nlet y = 2;";
        let mut doc = DocumentBlock::new(source.to_string());

        let fence = FenceBlock {
            block: Block {
                id: BlockId(1),
                line_range: LineRange { start: 1, end: 3 },
                parent_id: None,
                children_ids: vec![],
                byte_range: None,
            },
            language: None,
            info_string: None,
            raw_text: "```\nlet x = 1;\nlet y = 2;".to_string(),
        };

        doc.block_by_id.insert(BlockId(1), BlockNode::Fence(fence));

        let diags = validate(&doc);
        let unclosed: Vec<_> = diags
            .iter()
            .filter(|d| d.code == "UNCLOSED_FENCE")
            .collect();
        assert_eq!(unclosed.len(), 1);
        assert_eq!(unclosed[0].severity, Severity::Error);
        assert_eq!(unclosed[0].line, Some(1));
    }

    #[test]
    fn validate_closed_fence_no_diagnostic() {
        let source = "```\nlet x = 1;\n```";
        let mut doc = DocumentBlock::new(source.to_string());

        let fence = FenceBlock {
            block: Block {
                id: BlockId(1),
                line_range: LineRange { start: 1, end: 3 },
                parent_id: None,
                children_ids: vec![],
                byte_range: None,
            },
            language: None,
            info_string: None,
            raw_text: "```\nlet x = 1;\n```".to_string(),
        };

        doc.block_by_id.insert(BlockId(1), BlockNode::Fence(fence));

        let diags = validate(&doc);
        let unclosed: Vec<_> = diags
            .iter()
            .filter(|d| d.code == "UNCLOSED_FENCE")
            .collect();
        assert!(unclosed.is_empty());
    }

    #[test]
    fn validate_unclosed_tilde_fence() {
        let source = "~~~\ncode here";
        let mut doc = DocumentBlock::new(source.to_string());

        let fence = FenceBlock {
            block: Block {
                id: BlockId(1),
                line_range: LineRange { start: 1, end: 2 },
                parent_id: None,
                children_ids: vec![],
                byte_range: None,
            },
            language: None,
            info_string: None,
            raw_text: "~~~\ncode here".to_string(),
        };

        doc.block_by_id.insert(BlockId(1), BlockNode::Fence(fence));

        let diags = validate(&doc);
        let unclosed: Vec<_> = diags
            .iter()
            .filter(|d| d.code == "UNCLOSED_FENCE")
            .collect();
        assert_eq!(unclosed.len(), 1);
    }

    #[test]
    fn validate_empty_section() {
        let mut doc = DocumentBlock::new("# Empty".to_string());

        let section = SectionBlock {
            block: make_block(1, 1, 1),
            level: 1,
            title: "Empty".to_string(),
            slug: "empty".to_string(),
            path: "/Empty".to_string(),
            ordinal: 1,
            variant: HeadingVariant::Atx,
        };

        doc.block_by_id.insert(BlockId(1), BlockNode::Section(section));

        let diags = validate(&doc);
        let empty: Vec<_> = diags
            .iter()
            .filter(|d| d.code == "EMPTY_SECTION")
            .collect();
        assert_eq!(empty.len(), 1);
        assert_eq!(empty[0].severity, Severity::Warning);
        assert!(empty[0].message.contains("Empty"));
    }

    #[test]
    fn validate_non_empty_section_no_diagnostic() {
        let mut doc = DocumentBlock::new("# Not Empty\ncontent".to_string());

        let section = SectionBlock {
            block: Block {
                id: BlockId(1),
                line_range: LineRange { start: 1, end: 2 },
                parent_id: None,
                children_ids: vec![BlockId(2)],
                byte_range: None,
            },
            level: 1,
            title: "Not Empty".to_string(),
            slug: "not-empty".to_string(),
            path: "/Not Empty".to_string(),
            ordinal: 1,
            variant: HeadingVariant::Atx,
        };

        doc.block_by_id.insert(BlockId(1), BlockNode::Section(section));

        let diags = validate(&doc);
        let empty: Vec<_> = diags
            .iter()
            .filter(|d| d.code == "EMPTY_SECTION")
            .collect();
        assert!(empty.is_empty());
    }

    #[test]
    fn validate_collects_existing_diagnostics() {
        let mut doc = DocumentBlock::new("# A\n### Jump".to_string());

        let existing_diag = Diagnostic {
            code: "HEADING_LEVEL_JUMP".to_string(),
            severity: Severity::Warning,
            message: "Heading level jumped from 1 to 3".to_string(),
            line: Some(2),
            column: None,
            line_range: Some(LineRange { start: 2, end: 2 }),
            suggested_fix: None,
        };
        doc.diagnostics.push(existing_diag);

        let diags = validate(&doc);
        let jumps: Vec<_> = diags
            .iter()
            .filter(|d| d.code == "HEADING_LEVEL_JUMP")
            .collect();
        assert_eq!(jumps.len(), 1);
    }
}
