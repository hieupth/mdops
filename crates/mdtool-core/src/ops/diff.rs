use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use crate::builder::tree_builder::build_tree;
use crate::domain::block_node::BlockNode;
use crate::domain::document::DocumentBlock;
use crate::error::MdtoolError;
use crate::primitives::LineRange;

/// Summary of differences between two texts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DiffSummary {
    /// Whether any changes were detected.
    pub changed: bool,
    /// 1-based inclusive line ranges that changed.
    pub changed_line_ranges: Vec<LineRange>,
    /// Total number of lines added.
    pub added_lines: usize,
    /// Total number of lines removed.
    pub removed_lines: usize,
}

/// Replace lines `[start, end]` (1-based inclusive) in `text` with `new_text`.
///
/// Lines outside the specified range are preserved. If `line_range` is empty
/// (`start > end`), the new text is inserted before `start`.
pub fn patch_text(text: &str, line_range: LineRange, new_text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();

    let start_idx = line_range.start.saturating_sub(1);
    let end_idx = if line_range.start > line_range.end {
        // Empty range: insert before start
        start_idx
    } else {
        line_range.end
    };

    // Clamp indices to valid range
    let start_idx = start_idx.min(lines.len());
    let end_idx = end_idx.min(lines.len());

    // Replace lines[start_idx..end_idx] with new_lines
    // We rebuild the line vector to avoid borrow issues
    let mut result: Vec<String> = Vec::with_capacity(start_idx + new_lines.len() + lines.len().saturating_sub(end_idx));
    for line in &lines[..start_idx] {
        result.push((*line).to_string());
    }
    for line in &new_lines {
        result.push((*line).to_string());
    }
    for line in &lines[end_idx..] {
        result.push((*line).to_string());
    }

    // If the original text ended with a trailing newline, preserve it
    let trailing_newline = text.ends_with('\n');
    let joined = result.join("\n");
    if trailing_newline && !joined.is_empty() {
        format!("{}\n", joined)
    } else {
        joined
    }
}

/// Compare two texts line by line and compute a diff summary.
///
/// Tracks which line ranges changed, and counts of added/removed lines.
pub fn compute_diff(old_text: &str, new_text: &str) -> DiffSummary {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();

    let max_len = old_lines.len().max(new_lines.len());

    let mut changed_line_ranges: Vec<LineRange> = Vec::new();
    let mut added_lines: usize = 0;
    let mut removed_lines: usize = 0;
    let mut current_range_start: Option<usize> = None;

    for i in 0..max_len {
        let old_line = old_lines.get(i);
        let new_line = new_lines.get(i);

        let lines_differ = match (old_line, new_line) {
            (Some(a), Some(b)) => a != b,
            (Some(_), None) => true,  // line removed
            (None, Some(_)) => true,  // line added
            (None, None) => false,     // impossible
        };

        if lines_differ {
            match (old_line, new_line) {
                (None, Some(_)) => added_lines += 1,
                (Some(_), None) => removed_lines += 1,
                (Some(_), Some(_)) => {
                    // Line changed: count as both removal and addition
                    added_lines += 1;
                    removed_lines += 1;
                }
                (None, None) => {}
            }

            if current_range_start.is_none() {
                current_range_start = Some(i);
            }
        } else if current_range_start.is_some() {
            // Close the current range (1-based inclusive)
            let start = current_range_start.unwrap();
            changed_line_ranges.push(LineRange {
                start: start + 1,
                end: i,
            });
            current_range_start = None;
        }
    }

    // Close any open range at the end
    if let Some(start) = current_range_start {
        changed_line_ranges.push(LineRange {
            start: start + 1,
            end: max_len,
        });
    }

    let changed = !changed_line_ranges.is_empty();

    DiffSummary {
        changed,
        changed_line_ranges,
        added_lines,
        removed_lines,
    }
}

/// Patch a document's source text, reparse, and compute the diff.
///
/// This is the core patch-reparse-validate cycle:
/// 1. Apply `patch_text` to replace lines in the document source
/// 2. Rebuild the tree with `build_tree`
/// 3. Compute a `DiffSummary` between old and new source text
/// 4. Return the patched source, new document, and diff summary
pub fn patch_and_reparse(
    doc: &DocumentBlock,
    line_range: LineRange,
    new_text: &str,
) -> Result<(String, DocumentBlock, DiffSummary), MdtoolError> {
    // 1. Patch the source text
    let patched_text = patch_text(&doc.source_text, line_range, new_text);

    // 2. Rebuild the tree on the patched text
    let new_doc = build_tree(&patched_text)?;

    // 3. Compute diff between old and new source
    let diff = compute_diff(&doc.source_text, &patched_text);

    // 4. Return tuple
    Ok((patched_text, new_doc, diff))
}

/// Find the deepest descendant block's line range end for a given block.
///
/// Used to determine where to append content within a section.
pub fn last_descendant_line_end(doc: &DocumentBlock, block_id: crate::primitives::BlockId) -> usize {
    fn find_max_end(doc: &DocumentBlock, id: crate::primitives::BlockId) -> usize {
        let block = match doc.get_block(id) {
            Some(b) => b,
            None => return 0,
        };
        let own_end = block.block().line_range.end;
        let children_end = block
            .block()
            .children_ids
            .iter()
            .filter_map(|&cid| {
                doc.get_block(cid)?;
                // Recurse into all children to find the maximum line end
                Some(find_max_end(doc, cid))
            })
            .max()
            .unwrap_or(0);
        children_end.max(own_end)
    }
    find_max_end(doc, block_id)
}

/// Collect all descendant block IDs (depth-first) for a given block.
pub fn collect_descendants(doc: &DocumentBlock, block_id: crate::primitives::BlockId) -> Vec<crate::primitives::BlockId> {
    fn collect(doc: &DocumentBlock, id: crate::primitives::BlockId, result: &mut Vec<crate::primitives::BlockId>) {
        if let Some(block) = doc.get_block(id) {
            for &child_id in &block.block().children_ids {
                result.push(child_id);
                collect(doc, child_id, result);
            }
        }
    }
    let mut result = Vec::new();
    collect(doc, block_id, &mut result);
    result
}

/// Get the line range of the entire block, including all descendants.
pub fn block_content_line_range(doc: &DocumentBlock, block: &BlockNode) -> LineRange {
    let own_range = block.block().line_range;
    let end = last_descendant_line_end(doc, block.block().id);
    LineRange {
        start: own_range.start,
        end: end.max(own_range.end),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::LineRange;

    #[test]
    fn patch_text_replace_single_line() {
        let text = "line1\nline2\nline3";
        let result = patch_text(text, LineRange { start: 2, end: 2 }, "replaced");
        assert_eq!(result, "line1\nreplaced\nline3");
    }

    #[test]
    fn patch_text_replace_range() {
        let text = "line1\nline2\nline3\nline4";
        let result = patch_text(text, LineRange { start: 2, end: 3 }, "new2\nnew3");
        assert_eq!(result, "line1\nnew2\nnew3\nline4");
    }

    #[test]
    fn patch_text_replace_with_empty() {
        let text = "line1\nline2\nline3";
        let result = patch_text(text, LineRange { start: 2, end: 2 }, "");
        assert_eq!(result, "line1\nline3");
    }

    #[test]
    fn patch_text_replace_with_more_lines() {
        let text = "line1\nline2\nline3";
        let result = patch_text(text, LineRange { start: 2, end: 2 }, "new2a\nnew2b");
        assert_eq!(result, "line1\nnew2a\nnew2b\nline3");
    }

    #[test]
    fn patch_text_replace_all() {
        let text = "line1\nline2\nline3";
        let result = patch_text(text, LineRange { start: 1, end: 3 }, "all-new");
        assert_eq!(result, "all-new");
    }

    #[test]
    fn patch_text_preserves_trailing_newline() {
        let text = "line1\nline2\n";
        let result = patch_text(text, LineRange { start: 1, end: 1 }, "new1");
        assert_eq!(result, "new1\nline2\n");
    }

    #[test]
    fn patch_text_insert_before_line() {
        let text = "line1\nline2\nline3";
        // Empty range (start > end) means insert before start
        let result = patch_text(text, LineRange { start: 2, end: 1 }, "inserted");
        assert_eq!(result, "line1\ninserted\nline2\nline3");
    }

    #[test]
    fn compute_diff_identical() {
        let text = "line1\nline2\nline3";
        let diff = compute_diff(text, text);
        assert!(!diff.changed);
        assert!(diff.changed_line_ranges.is_empty());
        assert_eq!(diff.added_lines, 0);
        assert_eq!(diff.removed_lines, 0);
    }

    #[test]
    fn compute_diff_single_change() {
        let old = "line1\nline2\nline3";
        let new = "line1\nchanged\nline3";
        let diff = compute_diff(old, new);
        assert!(diff.changed);
        assert_eq!(diff.changed_line_ranges.len(), 1);
        assert_eq!(diff.changed_line_ranges[0], LineRange { start: 2, end: 2 });
    }

    #[test]
    fn compute_diff_additions() {
        let old = "line1\nline2";
        let new = "line1\nline2\nline3";
        let diff = compute_diff(old, new);
        assert!(diff.changed);
        assert_eq!(diff.added_lines, 1);
        assert_eq!(diff.changed_line_ranges.len(), 1);
        assert_eq!(diff.changed_line_ranges[0], LineRange { start: 3, end: 3 });
    }

    #[test]
    fn compute_diff_removals() {
        let old = "line1\nline2\nline3\nline4";
        let new = "line1\nline4";
        let diff = compute_diff(old, new);
        assert!(diff.changed);
        // Line 2: "line2" vs "line4" (changed -> 1 add + 1 remove)
        // Line 3: "line3" vs nothing (removed)
        // Line 4: "line4" vs nothing (removed)
        assert_eq!(diff.removed_lines, 3);
    }

    #[test]
    fn compute_diff_both_empty() {
        let diff = compute_diff("", "");
        assert!(!diff.changed);
        assert_eq!(diff.added_lines, 0);
        assert_eq!(diff.removed_lines, 0);
    }

    #[test]
    fn compute_diff_multiple_ranges() {
        let old = "a\nb\nc\nd\ne";
        let new = "a\nB\nc\nD\ne";
        let diff = compute_diff(old, new);
        assert!(diff.changed);
        assert_eq!(diff.changed_line_ranges.len(), 2);
        assert_eq!(diff.changed_line_ranges[0], LineRange { start: 2, end: 2 });
        assert_eq!(diff.changed_line_ranges[1], LineRange { start: 4, end: 4 });
    }

    #[test]
    fn patch_and_reparse_basic() {
        let source = "# Title\n\nhello\n";
        let doc = build_tree(source).unwrap();
        let (patched, new_doc, diff) = patch_and_reparse(
            &doc,
            LineRange { start: 3, end: 3 },
            "world",
        ).unwrap();

        assert!(patched.contains("world"));
        assert!(diff.changed);
        assert_eq!(diff.changed_line_ranges.len(), 1);
        assert!(new_doc.source_text.contains("world"));
    }

    #[test]
    fn patch_and_reparse_no_change() {
        let source = "# Title\n\nbody\n";
        let doc = build_tree(source).unwrap();
        let (_, _, diff) = patch_and_reparse(
            &doc,
            LineRange { start: 3, end: 3 },
            "body",
        ).unwrap();

        assert!(!diff.changed);
    }

    #[test]
    fn last_descendant_line_end_section() {
        let source = "# Title\n\nbody\n\n## Sub\n\nsubbody\n";
        let doc = build_tree(source).unwrap();

        // Find the "Title" section
        let title_section = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Section(s) if s.title == "Title" => Some(s.block.id),
            _ => None,
        }).unwrap();

        let end = last_descendant_line_end(&doc, title_section);
        assert!(end >= 6, "Section should extend to include sub-section content, got end={}", end);
    }

    #[test]
    fn block_content_line_range_paragraph() {
        let source = "# Title\n\nbody text\n";
        let doc = build_tree(source).unwrap();

        let paragraph = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Paragraph(p) if p.raw_text.contains("body") => Some(bn.clone()),
            _ => None,
        }).unwrap();

        let range = block_content_line_range(&doc, &paragraph);
        assert_eq!(range.start, range.end); // Leaf block: start == end
    }
}
