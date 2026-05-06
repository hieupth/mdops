//! Text search across blocks in a [`DocumentBlock`].
//!
//! The main entry point is [`search_blocks`], which walks the block tree
//! (optionally scoped to a subtree via a [`BlockSelector`]) and returns
//! [`SearchMatch`] entries for every block whose text content contains the
//! query string.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use crate::domain::block_node::BlockNode;
use crate::domain::document::DocumentBlock;
use crate::domain::selectors::BlockSelector;
use crate::ops::resolve::resolve_selector;
use crate::ops::traverse::walk;
use crate::primitives::BlockId;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single search hit: the block that matched and a snippet of the matching
/// text (the first line that contains the query).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchMatch {
    pub block_id: BlockId,
    pub matched_text: String,
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Search for `query` across blocks in `doc`.
///
/// - If `selector` is provided, only the subtree rooted at the matched block
///   is searched.
/// - Otherwise, the entire document is searched from the root.
/// - `case_sensitive` controls whether the match is case-sensitive (default
///   is case-insensitive).
///
/// Returns a list of [`SearchMatch`] in document order (pre-order DFS).
pub fn search_blocks(
    doc: &DocumentBlock,
    query: &str,
    selector: Option<&BlockSelector>,
    case_sensitive: bool,
) -> Vec<SearchMatch> {
    if query.is_empty() {
        return Vec::new();
    }

    // Determine the root of the search scope.
    let scope_root = match selector {
        Some(sel) => match resolve_selector(doc, sel) {
            Ok(id) => id,
            Err(_) => return Vec::new(),
        },
        None => doc.root_id(),
    };

    let block_ids = walk(scope_root, doc);
    let mut matches = Vec::new();

    for block_id in block_ids {
        if let Some(bn) = doc.get_block(block_id) {
            let text = extract_text(doc, bn);
            if contains_query(&text, query, case_sensitive) {
                let snippet = first_matching_line(&text, query, case_sensitive);
                matches.push(SearchMatch {
                    block_id,
                    matched_text: snippet,
                });
            }
        }
    }

    matches
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Extract the searchable text content from a block node.
///
/// Block-type-specific extraction:
/// - **ParagraphBlock**: `raw_text`
/// - **SectionBlock**: `title`
/// - **FenceBlock**: `raw_text`
/// - **TableBlock**: `header_row` + `body_rows` joined
/// - **HtmlBlock**: `raw_text`
/// - **ListItemBlock**: text from children (paragraph text)
/// - Other blocks: fall back to `doc.source_text_for_range(block.line_range)`
fn extract_text(doc: &DocumentBlock, bn: &BlockNode) -> String {
    match bn {
        BlockNode::Paragraph(p) => p.raw_text.clone(),
        BlockNode::Section(s) => s.title.clone(),
        BlockNode::Fence(f) => f.raw_text.clone(),
        BlockNode::Table(t) => {
            let mut parts: Vec<String> = Vec::new();
            parts.push(t.header_row.join(" | "));
            for row in &t.body_rows {
                parts.push(row.join(" | "));
            }
            parts.join("\n")
        }
        BlockNode::Html(h) => h.raw_text.clone(),
        BlockNode::ListItem(_li) => {
            // ListItem text comes from its children (typically a paragraph).
            let children = bn.block().children_ids.clone();
            let mut texts = Vec::new();
            for child_id in children {
                if let Some(child) = doc.get_block(child_id) {
                    texts.push(extract_text(doc, child));
                }
            }
            texts.join(" ")
        }
        BlockNode::CodeBlock(c) => c.raw_text.clone(),
        BlockNode::Unknown(u) => u.raw_text.clone(),
        BlockNode::LinkRefDef(lrd) => {
            format!("{} {}", lrd.label, lrd.destination)
        }
        _ => {
            // Fallback: source text from line range
            let range = bn.block().line_range;
            doc.source_text_for_range(range)
        }
    }
}

/// Check whether `haystack` contains `needle`, respecting the case sensitivity
/// setting.
fn contains_query(haystack: &str, needle: &str, case_sensitive: bool) -> bool {
    if case_sensitive {
        haystack.contains(needle)
    } else {
        haystack.to_lowercase().contains(&needle.to_lowercase())
    }
}

/// Return the first line of `text` that contains `query`.
/// If no individual line matches, return the whole text truncated to a
/// reasonable length.
fn first_matching_line(text: &str, query: &str, case_sensitive: bool) -> String {
    let query_lower = query.to_lowercase();
    for line in text.lines() {
        let matches = if case_sensitive {
            line.contains(query)
        } else {
            line.to_lowercase().contains(&query_lower)
        };
        if matches {
            return line.to_string();
        }
    }
    // Fallback: return the text itself (truncated if very long)
    if text.len() > 200 {
        format!("{}...", &text[..200])
    } else {
        text.to_string()
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

    // ---- Basic search -------------------------------------------------------

    #[test]
    fn search_in_paragraph() {
        let input = "Hello world.\n\nGoodbye world.\n";
        let doc = build_tree(input).unwrap();
        let results = search_blocks(&doc, "Hello", None, false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].matched_text, "Hello world.");
    }

    #[test]
    fn search_case_insensitive() {
        let input = "Hello World.\n";
        let doc = build_tree(input).unwrap();
        let results = search_blocks(&doc, "hello", None, false);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_case_sensitive() {
        let input = "Hello World.\n";
        let doc = build_tree(input).unwrap();
        let results = search_blocks(&doc, "hello", None, true);
        assert!(results.is_empty(), "Case-sensitive search should not match");
    }

    #[test]
    fn search_empty_query_returns_nothing() {
        let input = "Some text.\n";
        let doc = build_tree(input).unwrap();
        let results = search_blocks(&doc, "", None, false);
        assert!(results.is_empty());
    }

    // ---- Search in section titles -------------------------------------------

    #[test]
    fn search_in_section_title() {
        let input = "# Architecture\n\nBody text.\n\n# Risks\n\nRisk text.\n";
        let doc = build_tree(input).unwrap();
        let results = search_blocks(&doc, "Risks", None, false);
        assert!(!results.is_empty());
        // At least the section title should match
        let section_matches: Vec<&SearchMatch> = results
            .iter()
            .filter(|m| {
                matches!(
                    doc.get_block(m.block_id),
                    Some(BlockNode::Section(_))
                )
            })
            .collect();
        assert!(
            !section_matches.is_empty(),
            "Should find a section matching 'Risks'"
        );
    }

    // ---- Search in fenced code blocks ---------------------------------------

    #[test]
    fn search_in_fence() {
        let input = "# Code\n\n```rust\nfn main() {}\n```\n";
        let doc = build_tree(input).unwrap();
        let results = search_blocks(&doc, "main", None, false);
        assert!(!results.is_empty());
        let fence_matches: Vec<&SearchMatch> = results
            .iter()
            .filter(|m| matches!(doc.get_block(m.block_id), Some(BlockNode::Fence(_))))
            .collect();
        assert!(!fence_matches.is_empty(), "Should find a fence block matching 'main'");
    }

    // ---- Search in tables ---------------------------------------------------

    #[test]
    fn search_in_table() {
        let input = "# Data\n\n| Name | Role |\n|------|------|\n| Alice | Engineer |\n| Bob | Manager |\n";
        let doc = build_tree(input).unwrap();
        let results = search_blocks(&doc, "Alice", None, false);
        assert!(!results.is_empty());
        let table_matches: Vec<&SearchMatch> = results
            .iter()
            .filter(|m| matches!(doc.get_block(m.block_id), Some(BlockNode::Table(_))))
            .collect();
        assert!(!table_matches.is_empty(), "Should find a table block matching 'Alice'");
    }

    // ---- Search scoped to subtree -------------------------------------------

    #[test]
    fn search_with_selector_scope() {
        let input = "# Alpha\n\nalpha keyword here.\n\n# Beta\n\nbeta keyword there.\n";
        let doc = build_tree(input).unwrap();

        // Search only within the Beta section
        let sel = BlockSelector {
            title: Some("Beta".to_string()),
            ..Default::default()
        };
        let results = search_blocks(&doc, "keyword", Some(&sel), false);

        // Should find matches only in Beta subtree, not Alpha
        assert!(!results.is_empty());
        for m in &results {
            let bn = doc.get_block(m.block_id).unwrap();
            // The match should be within the Beta subtree
            let is_in_beta_subtree = is_descendant_of(&doc, m.block_id, "Beta");
            assert!(
                is_in_beta_subtree,
                "Match should be in Beta subtree, got {:?}",
                bn.block_type_name()
            );
        }
    }

    /// Helper: check if `block_id` is the section named `title` or a
    /// descendant of it.
    fn is_descendant_of(doc: &DocumentBlock, block_id: BlockId, title: &str) -> bool {
        // Find the section
        let section_id = doc.block_by_id.iter().find_map(|(id, bn)| match bn {
            BlockNode::Section(s) if s.title == title => Some(*id),
            _ => None,
        });

        let section_id = match section_id {
            Some(id) => id,
            None => return false,
        };

        if block_id == section_id {
            return true;
        }

        // Walk subtree of section
        let subtree = walk(section_id, doc);
        subtree.contains(&block_id)
    }

    #[test]
    fn search_with_nonexistent_selector_returns_empty() {
        let input = "# Title\n\ntext\n";
        let doc = build_tree(input).unwrap();
        let sel = BlockSelector::from_path("/nonexistent");
        let results = search_blocks(&doc, "text", Some(&sel), false);
        assert!(results.is_empty());
    }

    // ---- Search across multiple block types ---------------------------------

    #[test]
    fn search_multiple_hits() {
        let input = "# Rust Language\n\nRust is fast.\n\n```rust\n// Rust code\n```\n";
        let doc = build_tree(input).unwrap();
        let results = search_blocks(&doc, "rust", None, false);
        // Should find at least: section title + paragraph + fence
        assert!(
            results.len() >= 2,
            "Should find multiple blocks matching 'rust', got {}",
            results.len()
        );
    }

    #[test]
    fn search_no_match() {
        let input = "# Title\n\nSome text.\n";
        let doc = build_tree(input).unwrap();
        let results = search_blocks(&doc, "nonexistent", None, false);
        assert!(results.is_empty());
    }

    // ---- Search in HTML blocks ----------------------------------------------

    #[test]
    fn search_in_html_block() {
        let input = "<div class=\"note\">\nImportant content.\n</div>\n";
        let doc = build_tree(input).unwrap();
        let results = search_blocks(&doc, "Important", None, false);
        assert!(!results.is_empty());
        let html_matches: Vec<&SearchMatch> = results
            .iter()
            .filter(|m| matches!(doc.get_block(m.block_id), Some(BlockNode::Html(_))))
            .collect();
        assert!(!html_matches.is_empty(), "Should find an HTML block matching 'Important'");
    }

    // ---- Search in list items -----------------------------------------------

    #[test]
    fn search_in_list() {
        let input = "- buy milk\n- buy eggs\n- buy bread\n";
        let doc = build_tree(input).unwrap();
        let results = search_blocks(&doc, "eggs", None, false);
        assert!(!results.is_empty());
    }

    // ---- Search matched_text snippets ---------------------------------------

    #[test]
    fn search_snippet_is_first_matching_line() {
        let input = "line one.\nline two has keyword.\nline three.\n";
        let doc = build_tree(input).unwrap();
        let results = search_blocks(&doc, "keyword", None, false);
        assert_eq!(results.len(), 1);
        assert!(results[0].matched_text.contains("keyword"));
    }

    // ---- Full integration test ----------------------------------------------

    #[test]
    fn search_full_document() {
        let input = r#"# Architecture

Overview text about the system.

## Parser

Parses markdown into blocks.

```rust
fn parse() {}
```

| Module | Role |
|--------|------|
| parser | parsing |

# Risks

Some risk text about parser bugs.
"#;
        let doc = build_tree(input).unwrap();
        let results = search_blocks(&doc, "parser", None, false);
        // Should match: section title "Parser", paragraph "Parses markdown",
        // table cell "parser", and "risk text about parser bugs"
        assert!(
            results.len() >= 3,
            "Should find at least 3 matches for 'parser', got {}",
            results.len()
        );
    }
}
