use crate::domain::block_node::BlockNode;
use crate::domain::document::DocumentBlock;
use crate::domain::selectors::BlockSelector;
use crate::error::{Diagnostic, MdtoolError};
use crate::ops::diff::{block_content_line_range, last_descendant_line_end, patch_and_reparse, DiffSummary};
use crate::ops::resolve::resolve_selector;
use crate::primitives::{BlockId, LineRange};

/// Resolve a selector to a `&BlockNode`, returning an error if not found.
fn resolve_block<'a>(doc: &'a DocumentBlock, selector: &BlockSelector) -> Result<&'a BlockNode, MdtoolError> {
    let block_id = resolve_selector(doc, selector)?;
    doc.get_block(block_id).ok_or_else(|| MdtoolError::BlockNotFound {
        selector: format!("id={:?}", block_id),
    })
}

/// Compute the effective line range for editing a block.
///
/// For Section blocks, includes the entire subtree (heading + all descendant content).
/// For all other blocks, uses only the block's own line range.
fn effective_edit_range(doc: &DocumentBlock, block: &BlockNode) -> LineRange {
    match block {
        BlockNode::Section(_) => block_content_line_range(doc, block),
        _ => block.block().line_range,
    }
}

/// Replace the content of the block matched by `selector` with `new_text`.
///
/// Resolves the selector to find the target block, computes its effective line
/// range (including descendants), patches the source text, and reparses.
pub fn replace_block(
    doc: &DocumentBlock,
    selector: &BlockSelector,
    new_text: &str,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let block_node = resolve_block(doc, selector)?;
    let content_range = effective_edit_range(doc, block_node);
    let (_, new_doc, diff) = patch_and_reparse(doc, content_range, new_text)?;
    let diagnostics = new_doc.diagnostics.clone();
    Ok((new_doc, diff, diagnostics))
}

/// Insert `new_text` as a new block immediately before the block matched by `selector`.
pub fn insert_block_before(
    doc: &DocumentBlock,
    selector: &BlockSelector,
    new_text: &str,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let block_node = resolve_block(doc, selector)?;
    let insert_line = block_node.block().line_range.start;
    // Use an empty range to insert before the start line
    let insert_range = LineRange {
        start: insert_line,
        end: insert_line.saturating_sub(1),
    };
    // Prepend new_text with a newline separator to separate from the existing block
    let text_with_sep = format!("{}\n", new_text);
    let (_, new_doc, diff) = patch_and_reparse(doc, insert_range, &text_with_sep)?;
    let diagnostics = new_doc.diagnostics.clone();
    Ok((new_doc, diff, diagnostics))
}

/// Insert `new_text` as a new block immediately after the block matched by `selector`.
pub fn insert_block_after(
    doc: &DocumentBlock,
    selector: &BlockSelector,
    new_text: &str,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let block_node = resolve_block(doc, selector)?;
    let edit_range = effective_edit_range(doc, block_node);
    // Insert after the block's last line
    let insert_line = edit_range.end + 1;
    let doc_lines = doc.line_count();

    if insert_line > doc_lines {
        // Append at end of document
        let text_with_sep = format!("\n{}", new_text);
        let (_, new_doc, diff) = patch_and_reparse(
            doc,
            LineRange {
                start: doc_lines + 1,
                end: doc_lines,
            },
            &text_with_sep.trim_start_matches('\n'),
        )?;
        let diagnostics = new_doc.diagnostics.clone();
        Ok((new_doc, diff, diagnostics))
    } else {
        // Insert before the line after the block
        let text_with_sep = format!("{}\n", new_text);
        let insert_range = LineRange {
            start: insert_line,
            end: insert_line.saturating_sub(1),
        };
        let (_, new_doc, diff) = patch_and_reparse(doc, insert_range, &text_with_sep)?;
        let diagnostics = new_doc.diagnostics.clone();
        Ok((new_doc, diff, diagnostics))
    }
}

/// Append `new_text` as the last child of the section matched by `selector`.
///
/// If the matched block is not a section, this is equivalent to `insert_block_after`.
pub fn append_child(
    doc: &DocumentBlock,
    selector: &BlockSelector,
    new_text: &str,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let block_id = resolve_selector(doc, selector)?;
    let block_node = doc.get_block(block_id).ok_or_else(|| MdtoolError::BlockNotFound {
        selector: format!("id={:?}", block_id),
    })?;

    match block_node {
        BlockNode::Section(section) => {
            // Find the last descendant's line end
            let last_line = last_descendant_line_end(doc, section.block.id);
            let insert_after_line = last_line;
            let doc_lines = doc.line_count();

            if insert_after_line >= doc_lines {
                // Append at end of document
                let text_with_sep = format!("\n{}", new_text);
                let (_, new_doc, diff) = patch_and_reparse(
                    doc,
                    LineRange {
                        start: doc_lines + 1,
                        end: doc_lines,
                    },
                    &text_with_sep.trim_start_matches('\n'),
                )?;
                let diagnostics = new_doc.diagnostics.clone();
                Ok((new_doc, diff, diagnostics))
            } else {
                let text_with_sep = format!("{}\n", new_text);
                let insert_line = insert_after_line + 1;
                let insert_range = LineRange {
                    start: insert_line,
                    end: insert_line.saturating_sub(1),
                };
                let (_, new_doc, diff) = patch_and_reparse(doc, insert_range, &text_with_sep)?;
                let diagnostics = new_doc.diagnostics.clone();
                Ok((new_doc, diff, diagnostics))
            }
        }
        _ => {
            // Non-section: fall back to insert_after
            insert_block_after(doc, selector, new_text)
        }
    }
}

/// Delete the block matched by `selector` from the document.
///
/// Removes the block and all its descendants from the source text, then reparses.
pub fn delete_block(
    doc: &DocumentBlock,
    selector: &BlockSelector,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let block_node = resolve_block(doc, selector)?;
    let content_range = effective_edit_range(doc, block_node);

    // Compute whether we need to clean up surrounding blank lines
    let lines: Vec<&str> = doc.source_text.lines().collect();
    let before = if content_range.start > 1 {
        content_range.start - 1
    } else {
        0
    };

    // Check if the line before is blank — if so, remove it too for cleanliness
    let (final_range, new_text) = if before > 0 && lines.get(before - 1).map(|l| l.trim().is_empty()).unwrap_or(false) {
        (LineRange { start: before, end: content_range.end }, "")
    } else {
        (content_range, "")
    };

    let (_, new_doc, diff) = patch_and_reparse(doc, final_range, new_text)?;
    let diagnostics = new_doc.diagnostics.clone();
    Ok((new_doc, diff, diagnostics))
}

/// Move the block matched by `selector` to a new position under `target_parent_id`.
///
/// The block is first deleted from its original location, then inserted at the
/// given `index` within the target parent's children (or appended if `index` is None).
pub fn move_block(
    doc: &DocumentBlock,
    selector: &BlockSelector,
    target_parent_id: BlockId,
    index: Option<usize>,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let block_node = resolve_block(doc, selector)?;

    // 1. Extract the block's source text
    let content_range = block_content_line_range(doc, block_node);
    let block_text = doc.source_text_for_range(content_range);

    // 2. Delete the block from its original location
    let (doc_after_delete, _, _) = delete_block(doc, selector)?;

    // 3. Find the insertion point in the target parent
    // Handle root block (BlockId(0)) specially since it's not in block_by_id
    let insert_line = if target_parent_id == doc_after_delete.root_id() {
        compute_insert_line_for_root(&doc_after_delete, index)
    } else {
        let target_parent = doc_after_delete.get_block(target_parent_id)
            .ok_or_else(|| MdtoolError::BlockNotFound {
                selector: format!("id={:?}", target_parent_id),
            })?;
        compute_insert_line_for_parent(&doc_after_delete, target_parent, index)
    };

    // 4. Insert the block text at the computed position
    let insert_range = LineRange {
        start: insert_line,
        end: insert_line.saturating_sub(1),
    };
    let text_with_sep = format!("{}\n", block_text);
    let (_, new_doc, diff) = patch_and_reparse(&doc_after_delete, insert_range, &text_with_sep)?;
    let diagnostics = new_doc.diagnostics.clone();

    Ok((new_doc, diff, diagnostics))
}

/// Ensure a section exists at the given `path` with the specified `heading_level`.
///
/// If the section already exists (matched by path), returns the document unchanged
/// with an empty diff. Otherwise, appends a new heading at the end of the document.
pub fn ensure_section(
    doc: &DocumentBlock,
    path: &str,
    heading_level: u8,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    // Check if section already exists at this path
    let path_selector = BlockSelector::from_path(path);
    if resolve_selector(doc, &path_selector).is_ok() {
        // Section already exists; return unchanged
        let diff = DiffSummary {
            changed: false,
            changed_line_ranges: vec![],
            added_lines: 0,
            removed_lines: 0,
        };
        return Ok((doc.clone(), diff, vec![]));
    }

    // Extract the heading title from the last path segment
    let title = path
        .split('/')
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or("Untitled");

    // Build heading prefix
    let hashes = "#".repeat(heading_level.max(1).min(6) as usize);
    let heading_text = format!("{} {}", hashes, title);

    // Append at end of document
    let doc_lines = doc.line_count();
    let new_text = format!("\n\n{}\n", heading_text);

    let insert_range = if doc_lines == 0 {
        LineRange { start: 1, end: 0 }
    } else {
        LineRange {
            start: doc_lines + 1,
            end: doc_lines,
        }
    };

    let (_, new_doc, diff) = patch_and_reparse(doc, insert_range, new_text.trim_start_matches('\n'))?;
    let diagnostics = new_doc.diagnostics.clone();

    Ok((new_doc, diff, diagnostics))
}

/// Compute the line number where a new child should be inserted under the root
/// document block at the given `index`.
fn compute_insert_line_for_root(doc: &DocumentBlock, index: Option<usize>) -> usize {
    let children = &doc.block.children_ids;
    if children.is_empty() {
        doc.block.line_range.end + 1
    } else if let Some(idx) = index {
        if idx == 0 {
            if let Some(&first_child_id) = children.first() {
                if let Some(first_child) = doc.get_block(first_child_id) {
                    return first_child.block().line_range.start;
                }
            }
            doc.block.line_range.end + 1
        } else if idx >= children.len() {
            // Append after last child
            let &last_child_id = children.last().unwrap();
            if let Some(last_child) = doc.get_block(last_child_id) {
                let last_end = block_content_line_range(doc, last_child).end;
                return last_end + 1;
            }
            doc.block.line_range.end + 1
        } else {
            // Insert before child at index
            if let Some(&child_id) = children.get(idx) {
                if let Some(child) = doc.get_block(child_id) {
                    return child.block().line_range.start;
                }
            }
            doc.block.line_range.end + 1
        }
    } else {
        // Append after last child
        let &last_child_id = children.last().unwrap();
        if let Some(last_child) = doc.get_block(last_child_id) {
            let last_end = block_content_line_range(doc, last_child).end;
            return last_end + 1;
        }
        doc.block.line_range.end + 1
    }
}

/// Compute the line number where a new child should be inserted under `parent`
/// at the given `index`.
fn compute_insert_line_for_parent(
    doc: &DocumentBlock,
    parent: &BlockNode,
    index: Option<usize>,
) -> usize {
    let children = &parent.block().children_ids;
    if children.is_empty() {
        // No children: insert right after the parent's own line
        parent.block().line_range.end + 1
    } else if let Some(idx) = index {
        if idx == 0 {
            // Insert before the first child
            if let Some(first_child_id) = children.first() {
                if let Some(first_child) = doc.get_block(*first_child_id) {
                    return first_child.block().line_range.start;
                }
            }
            parent.block().line_range.end + 1
        } else if idx >= children.len() {
            // Insert after the last child
            let last_child_id = children.last().unwrap();
            if let Some(last_child) = doc.get_block(*last_child_id) {
                let last_end = block_content_line_range(doc, last_child).end;
                return last_end + 1;
            }
            parent.block().line_range.end + 1
        } else {
            // Insert before the child at index
            if let Some(child_id) = children.get(idx) {
                if let Some(child) = doc.get_block(*child_id) {
                    return child.block().line_range.start;
                }
            }
            parent.block().line_range.end + 1
        }
    } else {
        // Append after last child
        let last_child_id = children.last().unwrap();
        if let Some(last_child) = doc.get_block(*last_child_id) {
            let last_end = block_content_line_range(doc, last_child).end;
            return last_end + 1;
        }
        parent.block().line_range.end + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::tree_builder::build_tree;

    /// Helper to build a document from markdown source.
    fn doc_from(source: &str) -> DocumentBlock {
        build_tree(source).unwrap()
    }

    /// Helper to find a section's BlockId by title.
    fn find_section_id(doc: &DocumentBlock, title: &str) -> Option<BlockId> {
        doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Section(s) if s.title == title => Some(s.block.id),
            _ => None,
        })
    }

    /// Helper to find a paragraph's BlockId by text content.
    fn find_paragraph_id(doc: &DocumentBlock, contains: &str) -> Option<BlockId> {
        doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Paragraph(p) if p.raw_text.contains(contains) => Some(p.block.id),
            _ => None,
        })
    }

    // ---- replace_block tests ----

    #[test]
    fn replace_block_paragraph() {
        let source = "# Title\n\nold paragraph\n";
        let doc = doc_from(source);
        let selector = BlockSelector::from_id(find_paragraph_id(&doc, "old").unwrap());
        let (new_doc, diff, _diags) = replace_block(&doc, &selector, "new paragraph").unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("new paragraph"));
        assert!(!new_doc.source_text.contains("old paragraph"));
    }

    #[test]
    fn replace_block_no_change() {
        let source = "# Title\n\nsame content\n";
        let doc = doc_from(source);
        let selector = BlockSelector::from_id(find_paragraph_id(&doc, "same").unwrap());
        let (_, diff, _) = replace_block(&doc, &selector, "same content").unwrap();
        assert!(!diff.changed);
    }

    // ---- insert_block_before tests ----

    #[test]
    fn insert_block_before_paragraph() {
        let source = "# Title\n\nexisting paragraph\n";
        let doc = doc_from(source);
        let selector = BlockSelector::from_id(find_paragraph_id(&doc, "existing").unwrap());
        let (new_doc, diff, _diags) = insert_block_before(&doc, &selector, "inserted before").unwrap();
        assert!(diff.changed);
        let lines: Vec<&str> = new_doc.source_text.lines().collect();
        let inserted_idx = lines.iter().position(|l| l.contains("inserted before")).unwrap();
        let existing_idx = lines.iter().position(|l| l.contains("existing paragraph")).unwrap();
        assert!(inserted_idx < existing_idx, "Inserted line should come before existing line");
    }

    // ---- insert_block_after tests ----

    #[test]
    fn insert_block_after_paragraph() {
        let source = "# Title\n\nfirst paragraph\n";
        let doc = doc_from(source);
        let selector = BlockSelector::from_id(find_paragraph_id(&doc, "first").unwrap());
        let (new_doc, diff, _diags) = insert_block_after(&doc, &selector, "second paragraph").unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("second paragraph"));
        let lines: Vec<&str> = new_doc.source_text.lines().collect();
        let first_idx = lines.iter().position(|l| l.contains("first paragraph")).unwrap();
        let second_idx = lines.iter().position(|l| l.contains("second paragraph")).unwrap();
        assert!(first_idx < second_idx, "First paragraph should come before second");
    }

    // ---- append_child tests ----

    #[test]
    fn append_child_to_section() {
        let source = "# Title\n\nexisting content\n";
        let doc = doc_from(source);
        let selector = BlockSelector::from_id(find_section_id(&doc, "Title").unwrap());
        let (new_doc, diff, _diags) = append_child(&doc, &selector, "appended child").unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("appended child"));
    }

    #[test]
    fn append_child_to_section_with_subsection() {
        let source = "# Title\n\n## Sub\n\nsub content\n";
        let doc = doc_from(source);
        let selector = BlockSelector::from_id(find_section_id(&doc, "Title").unwrap());
        let (new_doc, diff, _diags) = append_child(&doc, &selector, "appended after sub").unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("appended after sub"));
    }

    // ---- delete_block tests ----

    #[test]
    fn delete_block_paragraph() {
        let source = "# Title\n\ntarget paragraph\n\nother content\n";
        let doc = doc_from(source);
        let selector = BlockSelector::from_id(find_paragraph_id(&doc, "target").unwrap());
        let (new_doc, diff, _diags) = delete_block(&doc, &selector).unwrap();
        assert!(diff.changed);
        assert!(!new_doc.source_text.contains("target paragraph"));
        assert!(new_doc.source_text.contains("other content"));
    }

    #[test]
    fn delete_block_section() {
        let source = "# Keep\n\nkeep content\n\n# Delete\n\ndelete content\n";
        let doc = doc_from(source);
        let selector = BlockSelector::from_id(find_section_id(&doc, "Delete").unwrap());
        let (new_doc, diff, _diags) = delete_block(&doc, &selector).unwrap();
        assert!(diff.changed);
        assert!(!new_doc.source_text.contains("# Delete"));
        assert!(!new_doc.source_text.contains("delete content"));
        assert!(new_doc.source_text.contains("keep content"));
    }

    // ---- ensure_section tests ----

    #[test]
    fn ensure_section_creates_new() {
        let source = "# Existing\n\ncontent\n";
        let doc = doc_from(source);
        let (new_doc, diff, _diags) = ensure_section(&doc, "/new-section", 2).unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("## new-section"));
    }

    #[test]
    fn ensure_section_already_exists() {
        let source = "# Existing\n\ncontent\n";
        let doc = doc_from(source);
        let (_, diff, _) = ensure_section(&doc, "/existing", 1).unwrap();
        assert!(!diff.changed);
    }

    // ---- move_block tests ----

    #[test]
    fn move_block_to_end() {
        let source = "# A\n\na content\n\n# B\n\nb content\n";
        let doc = doc_from(source);
        let a_id = find_section_id(&doc, "A").unwrap();
        let selector = BlockSelector::from_id(a_id);
        // Move section A after section B (as child of root)
        let root_id = doc.root_id();
        let (new_doc, diff, _diags) = move_block(&doc, &selector, root_id, None).unwrap();
        assert!(diff.changed);
        // Both sections should still exist
        assert!(new_doc.source_text.contains("a content"));
        assert!(new_doc.source_text.contains("b content"));
    }

    // ---- block not found tests ----

    #[test]
    fn replace_block_not_found() {
        let source = "# Title\n\ncontent\n";
        let doc = doc_from(source);
        let selector = BlockSelector {
            id: Some(BlockId(9999)),
            ..Default::default()
        };
        let result = replace_block(&doc, &selector, "new");
        assert!(result.is_err());
    }

    #[test]
    fn delete_block_not_found() {
        let source = "# Title\n\ncontent\n";
        let doc = doc_from(source);
        let selector = BlockSelector {
            path: Some("/nonexistent".to_string()),
            ..Default::default()
        };
        let result = delete_block(&doc, &selector);
        assert!(result.is_err());
    }
}
