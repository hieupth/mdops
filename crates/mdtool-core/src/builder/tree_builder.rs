use std::collections::HashMap;

use crate::domain::block_node::BlockNode;
use crate::domain::document::DocumentBlock;
use crate::error::{Diagnostic, MdtoolError, Severity};
use crate::infrastructure::parser::{compute_slug, parse_markdown};
use crate::primitives::{BlockId, LineRange};

/// Build a complete DocumentBlock with tree structure from markdown source.
/// Orchestrates: parse → flat blocks → heading tree → section ranges → paths/slugs → block_by_id.
pub fn build_tree(source: &str) -> Result<DocumentBlock, MdtoolError> {
    let mut doc = parse_markdown(source)?;
    let root_id = doc.block.id;
    let mut diagnostics = Vec::new();

    // Collect section IDs and their levels, sorted by line number (document order)
    let mut sections: Vec<(BlockId, u8, usize)> = doc
        .block_by_id
        .iter()
        .filter_map(|(id, bn)| match bn {
            BlockNode::Section(s) => Some((*id, s.level, s.block.line_range.start)),
            _ => None,
        })
        .collect();
    sections.sort_by_key(|(_, _, line)| *line);

    // Build heading tree using stack-based algorithm
    let mut stack: Vec<(u8, BlockId)> = Vec::new();
    let mut section_parent_map: HashMap<BlockId, BlockId> = HashMap::new();

    for (section_id, level, _) in &sections {
        // Pop stack until we find a section with level < current
        while let Some((stack_level, _)) = stack.last() {
            if *stack_level >= *level {
                stack.pop();
            } else {
                break;
            }
        }

        let parent_id = stack
            .last()
            .map(|(_, id)| *id)
            .unwrap_or(root_id);

        section_parent_map.insert(*section_id, parent_id);
        stack.push((*level, *section_id));
    }

    // Assign parents to sections
    for (section_id, parent_id) in &section_parent_map {
        if let Some(BlockNode::Section(section)) = doc.block_by_id.get_mut(section_id) {
            section.block.parent_id = Some(*parent_id);
        }
    }

    // Detect heading level jumps (separate pass to avoid borrow issues)
    for (section_id, parent_id) in &section_parent_map {
        if *parent_id != root_id {
            let (section_level, section_line_range) = match doc.block_by_id.get(section_id) {
                Some(BlockNode::Section(s)) => (s.level, s.block.line_range),
                _ => continue,
            };
            if let Some(BlockNode::Section(parent_section)) = doc.block_by_id.get(parent_id) {
                if section_level > parent_section.level + 1 {
                    diagnostics.push(Diagnostic {
                        code: "HEADING_LEVEL_JUMP".to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "Heading level jumped from {} to {}",
                            parent_section.level,
                            section_level
                        ),
                        line: Some(section_line_range.start),
                        column: None,
                        line_range: Some(section_line_range),
                        suggested_fix: None,
                    });
                }
            }
        }
    }

    // Assign non-section blocks to their nearest ancestor section
    // The order of root children determines the document flow
    let root_children = doc.block.children_ids.clone();
    let mut current_section_id: Option<BlockId> = None;

    for child_id in &root_children {
        if let Some(bn) = doc.block_by_id.get(child_id) {
            if matches!(bn, BlockNode::Section(_)) {
                // This is a section — update current_section_id
                current_section_id = Some(*child_id);
            } else {
                // Non-section block — assign to current section or root
                let parent = if let Some(section_id) = current_section_id {
                    section_id
                } else {
                    root_id
                };
                if let Some(block_node) = doc.block_by_id.get_mut(child_id) {
                    block_node.block_mut().parent_id = Some(parent);
                }
            }
        }
    }

    // Build children_ids for each block based on parent_id assignments
    let mut children_map: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for (id, bn) in &doc.block_by_id {
        if let Some(parent_id) = bn.block().parent_id {
            children_map.entry(parent_id).or_default().push(*id);
        }
    }

    // Sort children by line number to maintain document order
    for children in children_map.values_mut() {
        children.sort_by_key(|id| {
            doc.block_by_id
                .get(id)
                .map(|bn| bn.block().line_range.start)
                .unwrap_or(0)
        });
    }

    // Update children_ids for all blocks
    for (id, bn) in &mut doc.block_by_id {
        if let Some(children) = children_map.get(id) {
            bn.block_mut().children_ids = children.clone();
        }
    }
    // Also update root block's children_ids to only contain direct children
    if let Some(root_children) = children_map.get(&root_id) {
        doc.block.children_ids = root_children.clone();
    }

    // Compute section ranges bottom-up
    compute_section_ranges(&mut doc);

    // Assign paths and slugs to sections
    assign_paths(&mut doc);

    // Merge diagnostics from parsing
    doc.diagnostics.extend(diagnostics);

    Ok(doc)
}

/// Compute section line ranges bottom-up.
/// A section's range spans from its heading line through all descendant content.
fn compute_section_ranges(doc: &mut DocumentBlock) {
    let root_id = doc.block.id;

    // Get all section IDs ordered by their start line
    let mut sections: Vec<(BlockId, usize)> = doc
        .block_by_id
        .iter()
        .filter_map(|(id, bn)| match bn {
            BlockNode::Section(s) => Some((*id, s.block.line_range.start)),
            _ => None,
        })
        .collect();
    sections.sort_by_key(|(_, start)| *start);

    // For each section, compute its range as heading line to the line before
    // the next section at same or shallower level, or end of document
    let doc_end_line = doc.block.line_range.end;

    for (i, (section_id, _)) in sections.iter().enumerate() {
        let section_level = match doc.block_by_id.get(section_id) {
            Some(BlockNode::Section(s)) => s.level,
            _ => continue,
        };

        let start_line = match doc.block_by_id.get(section_id) {
            Some(BlockNode::Section(s)) => s.block.line_range.start,
            _ => continue,
        };

        // Find the end line: the line before the next section at same or shallower level
        let mut end_line = doc_end_line;
        for (other_id, _) in sections.iter().skip(i + 1) {
            let other_level = match doc.block_by_id.get(other_id) {
                Some(BlockNode::Section(s)) => s.level,
                _ => continue,
            };
            if other_level <= section_level {
                end_line = match doc.block_by_id.get(other_id) {
                    Some(BlockNode::Section(s)) => s.block.line_range.start.saturating_sub(1),
                    _ => end_line,
                };
                break;
            }
        }

        if let Some(BlockNode::Section(s)) = doc.block_by_id.get_mut(section_id) {
            s.block.line_range = LineRange {
                start: start_line,
                end: end_line,
            };
        }
    }

    // Update root's range to encompass everything
    doc.block.line_range.end = doc_end_line;
}

/// Assign canonical paths and slugs to all sections.
fn assign_paths(doc: &mut DocumentBlock) {
    let root_id = doc.block.id;

    // Collect sections in document order
    let mut sections: Vec<BlockId> = doc
        .block_by_id
        .iter()
        .filter_map(|(id, bn)| match bn {
            BlockNode::Section(_) => Some(*id),
            _ => None,
        })
        .collect();

    // Sort by line range start for document order
    sections.sort_by_key(|id| {
        doc.block_by_id
            .get(id)
            .map(|bn| bn.block().line_range.start)
            .unwrap_or(0)
    });

    // Track ordinal counts per path for disambiguation
    let mut path_counts: HashMap<String, u32> = HashMap::new();

    for section_id in &sections {
        // Build path by walking up ancestor chain
        let mut ancestor_path_parts: Vec<(String, u8)> = Vec::new();
        let mut current_id = Some(*section_id);

        while let Some(id) = current_id {
            if id == root_id {
                break;
            }
            if let Some(BlockNode::Section(s)) = doc.block_by_id.get(&id) {
                let slug = compute_slug(&s.title);
                ancestor_path_parts.push((slug, s.level));
                current_id = s.block.parent_id;
            } else {
                break;
            }
        }

        ancestor_path_parts.reverse();

        // Build canonical path
        let path = if ancestor_path_parts.is_empty() {
            "/".to_string()
        } else {
            let parts: Vec<&str> = ancestor_path_parts.iter().map(|(s, _)| s.as_str()).collect();
            format!("/{}", parts.join("/"))
        };

        // Compute ordinal
        let ordinal = path_counts.entry(path.clone()).or_insert(0);
        *ordinal += 1;

        // Update section
        if let Some(BlockNode::Section(s)) = doc.block_by_id.get_mut(section_id) {
            s.path = path;
            s.ordinal = *ordinal;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_empty_tree() {
        let doc = build_tree("").unwrap();
        assert_eq!(doc.block.id, BlockId(0));
        assert!(doc.block.children_ids.is_empty());
    }

    #[test]
    fn build_heading_tree() {
        // # A → ## B → ### C → ## D → # E
        let input = "# A\n\na\n\n## B\n\nb\n\n### C\n\nc\n\n## D\n\nd\n\n# E\n\ne\n";
        let doc = build_tree(input).unwrap();

        // Find sections
        let sections: Vec<(&str, u8, BlockId)> = doc
            .block_by_id
            .iter()
            .filter_map(|(id, bn)| match bn {
                BlockNode::Section(s) => Some((&*s.title, s.level, *id)),
                _ => None,
            })
            .collect();

        assert_eq!(sections.len(), 5);

        // # A should be child of root
        let a_id = sections.iter().find(|(t, _, _)| *t == "A").map(|(_, _, id)| *id).unwrap();
        let a_section = match doc.block_by_id.get(&a_id).unwrap() {
            BlockNode::Section(s) => s,
            _ => panic!(),
        };
        assert_eq!(a_section.block.parent_id, Some(BlockId(0)));

        // ## B should be child of # A
        let b_id = sections.iter().find(|(t, _, _)| *t == "B").map(|(_, _, id)| *id).unwrap();
        let b_section = match doc.block_by_id.get(&b_id).unwrap() {
            BlockNode::Section(s) => s,
            _ => panic!(),
        };
        assert_eq!(b_section.block.parent_id, Some(a_id));

        // ### C should be child of ## B
        let c_id = sections.iter().find(|(t, _, _)| *t == "C").map(|(_, _, id)| *id).unwrap();
        let c_section = match doc.block_by_id.get(&c_id).unwrap() {
            BlockNode::Section(s) => s,
            _ => panic!(),
        };
        assert_eq!(c_section.block.parent_id, Some(b_id));

        // # E should be child of root
        let e_id = sections.iter().find(|(t, _, _)| *t == "E").map(|(_, _, id)| *id).unwrap();
        let e_section = match doc.block_by_id.get(&e_id).unwrap() {
            BlockNode::Section(s) => s,
            _ => panic!(),
        };
        assert_eq!(e_section.block.parent_id, Some(BlockId(0)));
    }

    #[test]
    fn heading_level_jump_diagnostic() {
        let input = "# A\n\na\n\n### C\n\nc\n";
        let doc = build_tree(input).unwrap();
        assert!(
            doc.diagnostics.iter().any(|d| d.code == "HEADING_LEVEL_JUMP"),
            "Should emit HEADING_LEVEL_JUMP diagnostic"
        );
    }

    #[test]
    fn section_paths() {
        let input = "# Architecture\n\n## Parser\n\n### Builder\n\n# Risks\n";
        let doc = build_tree(input).unwrap();

        let sections: Vec<(String, String)> = doc
            .block_by_id
            .iter()
            .filter_map(|(_, bn)| match bn {
                BlockNode::Section(s) => Some((s.title.clone(), s.path.clone())),
                _ => None,
            })
            .collect();

        assert!(sections.iter().any(|(t, p)| *t == "Architecture" && p == "/architecture"));
        assert!(sections.iter().any(|(t, p)| *t == "Parser" && p == "/architecture/parser"));
        assert!(sections.iter().any(|(t, p)| *t == "Builder" && p == "/architecture/parser/builder"));
        assert!(sections.iter().any(|(t, p)| *t == "Risks" && p == "/risks"));
    }

    #[test]
    fn content_before_first_heading() {
        let input = "Intro text.\n\n# Title\n\nBody.\n";
        let doc = build_tree(input).unwrap();

        // The paragraph should be a direct child of root
        let root_children = &doc.block.children_ids;
        assert!(!root_children.is_empty());

        // First child should be a paragraph
        let first = doc.block_by_id.get(&root_children[0]).unwrap();
        assert!(matches!(first, BlockNode::Paragraph(_)));
    }

    #[test]
    fn empty_section() {
        let input = "# A\n\n# B\n\nbody.\n";
        let doc = build_tree(input).unwrap();

        let section_a = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Section(s) if s.title == "A" => Some(s.clone()),
            _ => None,
        }).unwrap();

        // Section A should have no children (except possibly its own sub-content)
        assert!(section_a.block.children_ids.is_empty() || section_a.block.children_ids.iter().all(|id| {
            matches!(doc.block_by_id.get(id), Some(BlockNode::Section(_)))
        }));
    }

    #[test]
    fn duplicate_heading_ordinals() {
        let input = "# Title\n\na\n\n# Title\n\nb\n";
        let doc = build_tree(input).unwrap();

        let mut ordinals: Vec<u32> = doc
            .block_by_id
            .iter()
            .filter_map(|(_, bn)| match bn {
                BlockNode::Section(s) if s.title == "Title" => Some(s.ordinal),
                _ => None,
            })
            .collect();
        ordinals.sort();

        assert_eq!(ordinals, vec![1, 2]);
    }

    #[test]
    fn full_example_tree() {
        let input = r#"# Architecture

Overview text.

## Parser

Body text.

| Module | Role |
|--------|------|
| builder | constructs tree |

- item A
- item B

# Risks

Some risk text.
"#;
        let doc = build_tree(input).unwrap();

        // Architecture should be child of root
        let arch = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Section(s) if s.title == "Architecture" => Some(s.clone()),
            _ => None,
        }).unwrap();
        assert_eq!(arch.block.parent_id, Some(BlockId(0)));

        // Parser should be child of Architecture
        let parser = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Section(s) if s.title == "Parser" => Some(s.clone()),
            _ => None,
        }).unwrap();
        assert_eq!(parser.block.parent_id, Some(arch.block.id));

        // Risks should be child of root
        let risks = doc.block_by_id.values().find_map(|bn| match bn {
            BlockNode::Section(s) if s.title == "Risks" => Some(s.clone()),
            _ => None,
        }).unwrap();
        assert_eq!(risks.block.parent_id, Some(BlockId(0)));
    }
}
