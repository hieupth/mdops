use crate::domain::block_node::BlockNode;
use crate::domain::document::DocumentBlock;
use crate::domain::selectors::{BlockSelector, InsertPosition};
use crate::domain::section::SectionBlock;
use crate::domain::container_blocks::{ListBlock, ListItemBlock, TableBlock};
use crate::error::{Diagnostic, MdtoolError};
use crate::ops::diff::{block_content_line_range, patch_and_reparse, DiffSummary};
use crate::ops::resolve::resolve_selector;
use crate::primitives::LineRange;

/// Resolve a selector that must match a `SectionBlock`.
fn resolve_section<'a>(doc: &'a DocumentBlock, selector: &BlockSelector) -> Result<&'a SectionBlock, MdtoolError> {
    let block_id = resolve_selector(doc, selector)?;
    let bn = doc.get_block(block_id).ok_or_else(|| MdtoolError::BlockNotFound {
        selector: format!("id={:?}", block_id),
    })?;
    match bn {
        BlockNode::Section(s) => Ok(s),
        _ => Err(MdtoolError::Transformation(format!(
            "Expected a section block, got {:?}",
            bn.block_type_name()
        ))),
    }
}

/// Resolve a selector that must match a `TableBlock`.
fn resolve_table<'a>(doc: &'a DocumentBlock, selector: &BlockSelector) -> Result<&'a TableBlock, MdtoolError> {
    let block_id = resolve_selector(doc, selector)?;
    let bn = doc.get_block(block_id).ok_or_else(|| MdtoolError::BlockNotFound {
        selector: format!("id={:?}", block_id),
    })?;
    match bn {
        BlockNode::Table(t) => Ok(t),
        _ => Err(MdtoolError::Transformation(format!(
            "Expected a table block, got {:?}",
            bn.block_type_name()
        ))),
    }
}

/// Resolve a selector that must match a `ListBlock`.
fn resolve_list<'a>(doc: &'a DocumentBlock, selector: &BlockSelector) -> Result<&'a ListBlock, MdtoolError> {
    let block_id = resolve_selector(doc, selector)?;
    let bn = doc.get_block(block_id).ok_or_else(|| MdtoolError::BlockNotFound {
        selector: format!("id={:?}", block_id),
    })?;
    match bn {
        BlockNode::List(l) => Ok(l),
        _ => Err(MdtoolError::Transformation(format!(
            "Expected a list block, got {:?}",
            bn.block_type_name()
        ))),
    }
}

/// Resolve a selector that must match a `ListItemBlock`.
fn resolve_list_item<'a>(doc: &'a DocumentBlock, selector: &BlockSelector) -> Result<&'a ListItemBlock, MdtoolError> {
    let block_id = resolve_selector(doc, selector)?;
    let bn = doc.get_block(block_id).ok_or_else(|| MdtoolError::BlockNotFound {
        selector: format!("id={:?}", block_id),
    })?;
    match bn {
        BlockNode::ListItem(li) => Ok(li),
        _ => Err(MdtoolError::Transformation(format!(
            "Expected a list item block, got {:?}",
            bn.block_type_name()
        ))),
    }
}

/// Rename a section's heading title while preserving the heading level and prefix style.
///
/// For example, "## Old Title" becomes "## New Title".
pub fn rename_section(
    doc: &DocumentBlock,
    selector: &BlockSelector,
    new_title: &str,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let section = resolve_section(doc, selector)?;
    let level = section.level;
    let hashes = "#".repeat(level as usize);
    let new_heading = format!("{} {}", hashes, new_title);

    // The heading is on the first line of the section's line range
    let heading_line = section.block.line_range.start;
    let heading_range = LineRange {
        start: heading_line,
        end: heading_line,
    };

    let (_, new_doc, diff) = patch_and_reparse(doc, heading_range, &new_heading)?;
    let diagnostics = new_doc.diagnostics.clone();
    Ok((new_doc, diff, diagnostics))
}

/// Change a section's heading level.
///
/// For example, "## Title" becomes "### Title" when new_level=3.
pub fn change_heading_level(
    doc: &DocumentBlock,
    selector: &BlockSelector,
    new_level: u8,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let section = resolve_section(doc, selector)?;
    if new_level < 1 || new_level > 6 {
        return Err(MdtoolError::Validation(format!(
            "Heading level must be between 1 and 6, got {}",
            new_level
        )));
    }

    let hashes = "#".repeat(new_level as usize);
    let new_heading = format!("{} {}", hashes, section.title);

    let heading_line = section.block.line_range.start;
    let heading_range = LineRange {
        start: heading_line,
        end: heading_line,
    };

    let (_, new_doc, diff) = patch_and_reparse(doc, heading_range, &new_heading)?;
    let diagnostics = new_doc.diagnostics.clone();
    Ok((new_doc, diff, diagnostics))
}

/// Add a row to a table at the given position.
///
/// The row values are pipe-delimited to form a proper table row string.
pub fn add_table_row(
    doc: &DocumentBlock,
    selector: &BlockSelector,
    row: &[String],
    position: InsertPosition,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let table = resolve_table(doc, selector)?;

    let row_text = format!("| {} |", row.join(" | "));

    let table_range = table.block.line_range;
    // Count: 1 header + 1 delimiter + N body rows = total lines in table
    let header_line_count = 2; // header + delimiter
    let total_table_lines = header_line_count + table.body_rows.len();

    let insert_line = match position {
        InsertPosition::Append => {
            // Insert after last table row
            table_range.start + total_table_lines
        }
        InsertPosition::After => {
            // Insert after last table row (same as Append for tables)
            table_range.start + total_table_lines
        }
        InsertPosition::Before => {
            // Insert before first body row (after delimiter)
            table_range.start + 2
        }
        InsertPosition::Index(idx) => {
            // Insert at a specific body row index (0-based)
            // Clamp to valid range
            let idx = idx.min(table.body_rows.len());
            table_range.start + header_line_count + idx
        }
    };

    let insert_range = LineRange {
        start: insert_line,
        end: insert_line.saturating_sub(1),
    };

    let text_with_sep = format!("{}\n", row_text);
    let (_, new_doc, diff) = patch_and_reparse(doc, insert_range, &text_with_sep)?;
    let diagnostics = new_doc.diagnostics.clone();
    Ok((new_doc, diff, diagnostics))
}

/// Update a single cell in a table.
///
/// `row` is 0-based body row index. `col` is 0-based column index.
pub fn update_table_cell(
    doc: &DocumentBlock,
    selector: &BlockSelector,
    row: usize,
    col: usize,
    value: &str,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let table = resolve_table(doc, selector)?;

    if row >= table.body_rows.len() {
        return Err(MdtoolError::Validation(format!(
            "Row index {} out of bounds (table has {} body rows)",
            row,
            table.body_rows.len()
        )));
    }

    let num_cols = table.header_row.len().max(table.body_rows.get(row).map(|r| r.len()).unwrap_or(0));
    if col >= num_cols {
        return Err(MdtoolError::Validation(format!(
            "Column index {} out of bounds (table has {} columns)",
            col, num_cols
        )));
    }

    // Build updated row
    let mut updated_row = table.body_rows[row].clone();
    // Extend row if needed
    while updated_row.len() <= col {
        updated_row.push(String::new());
    }
    updated_row[col] = value.to_string();

    let row_text = format!("| {} |", updated_row.join(" | "));

    // Target line: header(1) + delimiter(1) + row_index
    let target_line = table.block.line_range.start + 2 + row;
    let target_range = LineRange {
        start: target_line,
        end: target_line,
    };

    let (_, new_doc, diff) = patch_and_reparse(doc, target_range, &row_text)?;
    let diagnostics = new_doc.diagnostics.clone();
    Ok((new_doc, diff, diagnostics))
}

/// Remove a body row from a table.
///
/// `row` is 0-based body row index.
pub fn remove_table_row(
    doc: &DocumentBlock,
    selector: &BlockSelector,
    row: usize,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let table = resolve_table(doc, selector)?;

    if row >= table.body_rows.len() {
        return Err(MdtoolError::Validation(format!(
            "Row index {} out of bounds (table has {} body rows)",
            row,
            table.body_rows.len()
        )));
    }

    // Target line: header(1) + delimiter(1) + row_index
    let target_line = table.block.line_range.start + 2 + row;
    let target_range = LineRange {
        start: target_line,
        end: target_line,
    };

    // Replace the row with empty string to delete it
    let (_, new_doc, diff) = patch_and_reparse(doc, target_range, "")?;
    let diagnostics = new_doc.diagnostics.clone();
    Ok((new_doc, diff, diagnostics))
}

/// Toggle a task list item's checkbox between checked and unchecked.
///
/// Detects `[x]` or `[ ]` in the source text and toggles between them.
/// Works with any list item that contains a checkbox pattern.
pub fn toggle_task(
    doc: &DocumentBlock,
    selector: &BlockSelector,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let list_item = resolve_list_item(doc, selector)?;
    let line_range = list_item.block.line_range;
    let source_line = doc.source_text_for_range(line_range);

    // Detect checkbox state from source text (more robust than parser checked field)
    let has_checked = source_line.contains("[x]") || source_line.contains("[X]");
    let has_unchecked = source_line.contains("[ ]");

    if !has_checked && !has_unchecked {
        return Err(MdtoolError::Transformation(
            "Target list item does not contain a checkbox pattern".to_string(),
        ));
    }

    let new_source_line = if has_checked {
        // Currently checked [x]/[X] -> uncheck [ ]
        source_line.replace("[x]", "[ ]").replace("[X]", "[ ]")
    } else {
        // Currently unchecked [ ] -> check [x]
        // Only replace the first occurrence to handle multiple checkboxes
        if let Some(pos) = source_line.find("[ ]") {
            let mut result = source_line.clone();
            result.replace_range(pos..pos + 3, "[x]");
            result
        } else {
            source_line
        }
    };

    let (_, new_doc, diff) = patch_and_reparse(doc, line_range, &new_source_line)?;
    let diagnostics = new_doc.diagnostics.clone();
    Ok((new_doc, diff, diagnostics))
}

/// Add a list item to a list at the given position.
///
/// If `index` is `None`, the item is appended at the end.
pub fn add_list_item(
    doc: &DocumentBlock,
    selector: &BlockSelector,
    text: &str,
    index: Option<usize>,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let list = resolve_list(doc, selector)?;

    // Determine the marker to use
    let item_line = if list.ordered {
        // For ordered lists, use "1." as a simple default marker
        format!("1. {}", text)
    } else {
        format!("{} {}", list.marker, text)
    };

    // Find the insertion line based on index
    let list_range = list.block.line_range;
    let children = &list.block.children_ids;
    let num_children = children.len();

    let insert_line = if let Some(idx) = index {
        if idx == 0 {
            // Insert before the first item
            list_range.start
        } else if idx >= num_children {
            // Append after last item
            // Find the end line of the last child
            let last_child_end = children.last().and_then(|&cid| {
                doc.get_block(cid).map(|bn| bn.block().line_range.end)
            }).unwrap_or(list_range.end);
            last_child_end + 1
        } else {
            // Insert before child at index
            children.get(idx).and_then(|&cid| {
                doc.get_block(cid).map(|bn| bn.block().line_range.start)
            }).unwrap_or(list_range.end + 1)
        }
    } else {
        // Append after last child
        if num_children == 0 {
            list_range.end + 1
        } else {
            let last_child_end = children.last().and_then(|&cid| {
                doc.get_block(cid).map(|bn| bn.block().line_range.end)
            }).unwrap_or(list_range.end);
            last_child_end + 1
        }
    };

    let insert_range = LineRange {
        start: insert_line,
        end: insert_line.saturating_sub(1),
    };

    let text_with_sep = format!("{}\n", item_line);
    let (_, new_doc, diff) = patch_and_reparse(doc, insert_range, &text_with_sep)?;
    let diagnostics = new_doc.diagnostics.clone();
    Ok((new_doc, diff, diagnostics))
}

/// Remove a list item at the given index.
///
/// `index` is 0-based position within the list's children.
pub fn remove_list_item(
    doc: &DocumentBlock,
    selector: &BlockSelector,
    index: usize,
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>), MdtoolError> {
    let list = resolve_list(doc, selector)?;
    let children = &list.block.children_ids;

    if index >= children.len() {
        return Err(MdtoolError::Validation(format!(
            "List item index {} out of bounds (list has {} items)",
            index,
            children.len()
        )));
    }

    let child_id = children[index];
    let child_node = doc.get_block(child_id).ok_or_else(|| MdtoolError::BlockNotFound {
        selector: format!("id={:?}", child_id),
    })?;

    let content_range = block_content_line_range(doc, child_node);

    let (_, new_doc, diff) = patch_and_reparse(doc, content_range, "")?;
    let diagnostics = new_doc.diagnostics.clone();
    Ok((new_doc, diff, diagnostics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::tree_builder::build_tree;
    use crate::domain::block_node::BlockNode;
    use crate::primitives::BlockId;

    /// Helper to find a section's BlockId by title.
    fn find_section_id(doc: &DocumentBlock, title: &str) -> Option<BlockId> {
        doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Section(s) if s.title == title => Some(s.block.id),
            _ => None,
        })
    }

    // ---- rename_section tests ----

    #[test]
    fn rename_section_basic() {
        let source = "# Old Title\n\nbody text\n";
        let doc = build_tree(source).unwrap();
        let selector = BlockSelector::from_id(find_section_id(&doc, "Old Title").unwrap());
        let (new_doc, diff, _) = rename_section(&doc, &selector, "New Title").unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("# New Title"));
        assert!(!new_doc.source_text.contains("# Old Title"));
    }

    #[test]
    fn rename_section_level2() {
        let source = "# Main\n\n## Sub\n\nbody\n";
        let doc = build_tree(source).unwrap();
        let selector = BlockSelector::from_id(find_section_id(&doc, "Sub").unwrap());
        let (new_doc, diff, _) = rename_section(&doc, &selector, "Renamed Sub").unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("## Renamed Sub"));
    }

    #[test]
    fn rename_section_not_a_section() {
        let source = "# Title\n\nparagraph\n";
        let doc = build_tree(source).unwrap();
        // Find the paragraph block
        let para_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Paragraph(_) => Some(bn.block().id),
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(para_id);
        let result = rename_section(&doc, &selector, "New Title");
        assert!(result.is_err());
    }

    // ---- change_heading_level tests ----

    #[test]
    fn change_heading_level_up() {
        let source = "# Title\n\nbody\n";
        let doc = build_tree(source).unwrap();
        let selector = BlockSelector::from_id(find_section_id(&doc, "Title").unwrap());
        let (new_doc, diff, _) = change_heading_level(&doc, &selector, 3).unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("### Title"));
    }

    #[test]
    fn change_heading_level_invalid() {
        let source = "# Title\n\nbody\n";
        let doc = build_tree(source).unwrap();
        let selector = BlockSelector::from_id(find_section_id(&doc, "Title").unwrap());
        let result = change_heading_level(&doc, &selector, 0);
        assert!(result.is_err());
    }

    #[test]
    fn change_heading_level_7() {
        let source = "# Title\n\nbody\n";
        let doc = build_tree(source).unwrap();
        let selector = BlockSelector::from_id(find_section_id(&doc, "Title").unwrap());
        let result = change_heading_level(&doc, &selector, 7);
        assert!(result.is_err());
    }

    // ---- add_table_row tests ----

    #[test]
    fn add_table_row_append() {
        let source = "# Title\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";
        let doc = build_tree(source).unwrap();
        let table_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Table(_) => Some(bn.block().id),
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(table_id);
        let row = vec!["3".to_string(), "4".to_string()];
        let (new_doc, diff, _) = add_table_row(&doc, &selector, &row, InsertPosition::Append).unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("| 3 | 4 |"));
    }

    #[test]
    fn add_table_row_at_index() {
        let source = "# Title\n\n| A | B |\n|---|---|\n| 1 | 2 |\n| 5 | 6 |\n";
        let doc = build_tree(source).unwrap();
        let table_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Table(_) => Some(bn.block().id),
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(table_id);
        let row = vec!["3".to_string(), "4".to_string()];
        let (new_doc, diff, _) = add_table_row(&doc, &selector, &row, InsertPosition::Index(1)).unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("| 3 | 4 |"));
    }

    // ---- update_table_cell tests ----

    #[test]
    fn update_table_cell_basic() {
        let source = "# Title\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";
        let doc = build_tree(source).unwrap();
        let table_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Table(_) => Some(bn.block().id),
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(table_id);
        let (new_doc, diff, _) = update_table_cell(&doc, &selector, 0, 1, "updated").unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("updated"));
    }

    #[test]
    fn update_table_cell_out_of_bounds() {
        let source = "# Title\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";
        let doc = build_tree(source).unwrap();
        let table_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Table(_) => Some(bn.block().id),
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(table_id);
        let result = update_table_cell(&doc, &selector, 5, 0, "x");
        assert!(result.is_err());
    }

    // ---- remove_table_row tests ----

    #[test]
    fn remove_table_row_basic() {
        let source = "# Title\n\n| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n";
        let doc = build_tree(source).unwrap();
        let table_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Table(_) => Some(bn.block().id),
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(table_id);
        let (new_doc, diff, _) = remove_table_row(&doc, &selector, 0).unwrap();
        assert!(diff.changed);
        assert!(!new_doc.source_text.contains("| 1 | 2 |"));
        assert!(new_doc.source_text.contains("| 3 | 4 |"));
    }

    #[test]
    fn remove_table_row_out_of_bounds() {
        let source = "# Title\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";
        let doc = build_tree(source).unwrap();
        let table_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Table(_) => Some(bn.block().id),
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(table_id);
        let result = remove_table_row(&doc, &selector, 5);
        assert!(result.is_err());
    }

    // ---- toggle_task tests ----

    #[test]
    fn toggle_task_unchecked_to_checked() {
        let source = "- [ ] buy milk\n- [ ] walk dog\n";
        let doc = build_tree(source).unwrap();
        // Find any ListItem whose source text contains "[ ]"
        let li_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::ListItem(li) => {
                let src = doc.source_text_for_range(li.block.line_range);
                if src.contains("[ ]") {
                    Some(li.block.id)
                } else {
                    None
                }
            }
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(li_id);
        let (new_doc, diff, _) = toggle_task(&doc, &selector).unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("[x]"));
    }

    #[test]
    fn toggle_task_checked_to_unchecked() {
        let source = "- [x] done task\n- [ ] pending task\n";
        let doc = build_tree(source).unwrap();
        // Find the ListItem whose source text contains "[x]"
        let li_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::ListItem(li) => {
                let src = doc.source_text_for_range(li.block.line_range);
                if src.contains("[x]") {
                    Some(li.block.id)
                } else {
                    None
                }
            }
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(li_id);
        let (new_doc, diff, _) = toggle_task(&doc, &selector).unwrap();
        assert!(diff.changed);
        // After toggling [x] -> [ ], the first item should now be unchecked
        assert!(new_doc.source_text.contains("[ ] done task"));
    }

    #[test]
    fn toggle_task_not_a_task() {
        let source = "# Title\n\n- regular item\n";
        let doc = build_tree(source).unwrap();
        let li_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::ListItem(_) => Some(bn.block().id),
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(li_id);
        let result = toggle_task(&doc, &selector);
        assert!(result.is_err());
    }

    // ---- add_list_item tests ----

    #[test]
    fn add_list_item_append() {
        let source = "# Title\n\n- alpha\n- beta\n";
        let doc = build_tree(source).unwrap();
        let list_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::List(_) => Some(bn.block().id),
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(list_id);
        let (new_doc, diff, _) = add_list_item(&doc, &selector, "gamma", None).unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("gamma"));
    }

    #[test]
    fn add_list_item_at_index() {
        let source = "# Title\n\n- alpha\n- gamma\n";
        let doc = build_tree(source).unwrap();
        let list_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::List(_) => Some(bn.block().id),
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(list_id);
        let (new_doc, diff, _) = add_list_item(&doc, &selector, "beta", Some(1)).unwrap();
        assert!(diff.changed);
        assert!(new_doc.source_text.contains("beta"));
    }

    // ---- remove_list_item tests ----

    #[test]
    fn remove_list_item_basic() {
        let source = "# Title\n\n- alpha\n- beta\n- gamma\n";
        let doc = build_tree(source).unwrap();
        let list_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::List(_) => Some(bn.block().id),
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(list_id);
        let (new_doc, diff, _) = remove_list_item(&doc, &selector, 1).unwrap();
        assert!(diff.changed);
        assert!(!new_doc.source_text.contains("beta"));
        assert!(new_doc.source_text.contains("alpha"));
        assert!(new_doc.source_text.contains("gamma"));
    }

    #[test]
    fn remove_list_item_out_of_bounds() {
        let source = "# Title\n\n- alpha\n- beta\n";
        let doc = build_tree(source).unwrap();
        let list_id = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::List(_) => Some(bn.block().id),
            _ => None,
        }).unwrap();
        let selector = BlockSelector::from_id(list_id);
        let result = remove_list_item(&doc, &selector, 5);
        assert!(result.is_err());
    }
}
