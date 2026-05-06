use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use crate::domain::document::DocumentBlock;
use crate::domain::selectors::{BlockSelector, InsertPosition};
use crate::error::{Diagnostic, MdtoolError};
use crate::ops::diff::DiffSummary;
use crate::ops::write::{replace_block, insert_block_after, delete_block, ensure_section, move_block};
use crate::ops::semantic::{
    rename_section, change_heading_level, add_table_row, update_table_cell,
    remove_table_row, toggle_task, add_list_item, remove_list_item,
};
use crate::primitives::BlockId;
use crate::ops::resolve::resolve_selector;

/// Enumeration of all supported edit operations.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub enum EditOperation {
    Replace,
    Insert,
    Delete,
    Move,
    EnsureSection,
    RenameSection,
    ChangeHeadingLevel,
    AddTableRow,
    UpdateTableCell,
    RemoveTableRow,
    ToggleTask,
    AddListItem,
    RemoveListItem,
}

/// A single edit operation with its parameters.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct EditOp {
    pub operation: EditOperation,
    pub selector: Option<BlockSelector>,
    pub content: Option<String>,
    pub position: Option<InsertPosition>,
    pub target_parent_id: Option<BlockId>,
    pub index: Option<usize>,
    pub path: Option<String>,
    pub heading_level: Option<u8>,
    pub new_title: Option<String>,
    pub row: Option<Vec<String>>,
    pub row_index: Option<usize>,
    pub col: Option<usize>,
    pub value: Option<String>,
}

/// Apply a batch of edit operations to a document.
///
/// Pre-validates all selectors first. If any selector fails to resolve,
/// the entire batch is rejected. Operations are then applied sequentially,
/// each triggering its own reparse.
pub fn edit(
    doc: &DocumentBlock,
    operations: &[EditOp],
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    if operations.is_empty() {
        return Ok((
            doc.clone(),
            DiffSummary {
                changed: false,
                changed_line_ranges: vec![],
                added_lines: 0,
                removed_lines: 0,
            },
            vec![],
        ));
    }

    // Pre-validate: resolve all selectors that are present
    for (i, op) in operations.iter().enumerate() {
        if let Some(ref selector) = op.selector {
            // Some operations don't need selector validation (e.g., EnsureSection uses path)
            match op.operation {
                EditOperation::EnsureSection => {
                    // EnsureSection uses path field, selector is optional
                }
                _ => {
                    if let Err(e) = resolve_selector(doc, selector) {
                        return Err(MdtoolError::Transformation(format!(
                            "Operation {} selector resolution failed: {}",
                            i, e
                        )));
                    }
                }
            }
        }
    }

    // Apply operations sequentially
    let mut current_doc = doc.clone();
    let mut accumulated_diff = DiffSummary {
        changed: false,
        changed_line_ranges: vec![],
        added_lines: 0,
        removed_lines: 0,
    };
    let mut accumulated_diagnostics: Vec<Diagnostic> = vec![];

    for op in operations {
        let result = apply_single_op(&current_doc, op)?;
        current_doc = result.0;
        // Merge diff
        accumulated_diff.changed = accumulated_diff.changed || result.1.changed;
        accumulated_diff.changed_line_ranges.extend(result.1.changed_line_ranges);
        accumulated_diff.added_lines += result.1.added_lines;
        accumulated_diff.removed_lines += result.1.removed_lines;
        accumulated_diagnostics.extend(result.2);
    }

    Ok((current_doc, accumulated_diff, accumulated_diagnostics))
}

/// Apply a single edit operation to the document.
fn apply_single_op(
    doc: &DocumentBlock,
    op: &EditOp,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let default_selector = BlockSelector::default();
    let selector = op.selector.as_ref().unwrap_or(&default_selector);

    match op.operation {
        EditOperation::Replace => {
            let content = op.content.as_deref().unwrap_or("");
            replace_block(doc, selector, content)
        }
        EditOperation::Insert => {
            let content = op.content.as_deref().unwrap_or("");
            insert_block_after(doc, selector, content)
        }
        EditOperation::Delete => {
            delete_block(doc, selector)
        }
        EditOperation::Move => {
            let target_parent_id = op.target_parent_id.unwrap_or(doc.root_id());
            move_block(doc, selector, target_parent_id, op.index)
        }
        EditOperation::EnsureSection => {
            let path = op.path.as_deref().unwrap_or("");
            let level = op.heading_level.unwrap_or(2);
            ensure_section(doc, path, level)
        }
        EditOperation::RenameSection => {
            let new_title = op.new_title.as_deref().unwrap_or("");
            rename_section(doc, selector, new_title)
        }
        EditOperation::ChangeHeadingLevel => {
            let level = op.heading_level.unwrap_or(2);
            change_heading_level(doc, selector, level)
        }
        EditOperation::AddTableRow => {
            let row = op.row.as_deref().unwrap_or(&[]);
            let position = op.position.unwrap_or(InsertPosition::Append);
            add_table_row(doc, selector, row, position)
        }
        EditOperation::UpdateTableCell => {
            let row = op.row_index.unwrap_or(0);
            let col = op.col.unwrap_or(0);
            let value = op.value.as_deref().unwrap_or("");
            update_table_cell(doc, selector, row, col, value)
        }
        EditOperation::RemoveTableRow => {
            let row = op.row_index.unwrap_or(0);
            remove_table_row(doc, selector, row)
        }
        EditOperation::ToggleTask => {
            toggle_task(doc, selector)
        }
        EditOperation::AddListItem => {
            let text = op.value.as_deref().unwrap_or("");
            add_list_item(doc, selector, text, op.index)
        }
        EditOperation::RemoveListItem => {
            let index = op.index.unwrap_or(0);
            remove_list_item(doc, selector, index)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::tree_builder::build_tree;
    use crate::domain::block_node::BlockNode;

    /// Helper to find a section's BlockId by title.
    fn find_section_id(doc: &DocumentBlock, title: &str) -> Option<BlockId> {
        doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Section(s) if s.title == title => Some(s.block.id),
            _ => None,
        })
    }

    #[test]
    fn edit_single_rename() {
        let source = "# Old Title\n\nbody text\n";
        let doc = build_tree(source).unwrap();
        let selector = BlockSelector::from_id(find_section_id(&doc, "Old Title").unwrap());

        let ops = vec![EditOp {
            operation: EditOperation::RenameSection,
            selector: Some(selector),
            new_title: Some("New Title".to_string()),
            content: None,
            position: None,
            target_parent_id: None,
            index: None,
            path: None,
            heading_level: None,
            row: None,
            row_index: None,
            col: None,
            value: None,
        }];

        let (new_doc, diff, _) = edit(&doc, &ops).unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("# New Title"));
    }

    #[test]
    fn edit_batch_two_operations() {
        let source = "# Title\n\nparagraph one\n";
        let doc = build_tree(source).unwrap();
        let para_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Paragraph(_) => Some(bn.block().id),
            _ => None,
        }).unwrap();

        // First replace paragraph, then rename section
        let section_selector = BlockSelector::from_id(find_section_id(&doc, "Title").unwrap());

        let ops = vec![
            EditOp {
                operation: EditOperation::Replace,
                selector: Some(BlockSelector::from_id(para_id)),
                content: Some("replaced paragraph".to_string()),
                new_title: None,
                position: None,
                target_parent_id: None,
                index: None,
                path: None,
                heading_level: None,
                row: None,
                row_index: None,
                col: None,
                value: None,
            },
            EditOp {
                operation: EditOperation::RenameSection,
                selector: Some(section_selector),
                new_title: Some("Renamed".to_string()),
                content: None,
                position: None,
                target_parent_id: None,
                index: None,
                path: None,
                heading_level: None,
                row: None,
                row_index: None,
                col: None,
                value: None,
            },
        ];

        let (new_doc, diff, _) = edit(&doc, &ops).unwrap();
        assert!(diff.changed);
        // Note: The second operation runs on the reparsed doc after the first,
        // so we check the final state
        assert!(new_doc.source_text.contains("replaced paragraph"));
    }

    #[test]
    fn edit_batch_bad_selector_rejected() {
        let source = "# Title\n\nbody\n";
        let doc = build_tree(source).unwrap();

        let ops = vec![EditOp {
            operation: EditOperation::RenameSection,
            selector: Some(BlockSelector {
                path: Some("/nonexistent".to_string()),
                ..Default::default()
            }),
            new_title: Some("New".to_string()),
            content: None,
            position: None,
            target_parent_id: None,
            index: None,
            path: None,
            heading_level: None,
            row: None,
            row_index: None,
            col: None,
            value: None,
        }];

        let result = edit(&doc, &ops);
        assert!(result.is_err());
    }

    #[test]
    fn edit_empty_operations() {
        let source = "# Title\n\nbody\n";
        let doc = build_tree(source).unwrap();

        let (new_doc, diff, _) = edit(&doc, &[]).unwrap();
        assert!(!diff.changed);
        assert_eq!(new_doc.source_text, doc.source_text);
    }

    #[test]
    fn edit_ensure_section_operation() {
        let source = "# Existing\n\ncontent\n";
        let doc = build_tree(source).unwrap();

        let ops = vec![EditOp {
            operation: EditOperation::EnsureSection,
            selector: None,
            path: Some("/new-section".to_string()),
            heading_level: Some(2),
            content: None,
            new_title: None,
            position: None,
            target_parent_id: None,
            index: None,
            row: None,
            row_index: None,
            col: None,
            value: None,
        }];

        let (new_doc, diff, _) = edit(&doc, &ops).unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("## new-section"));
    }
}
