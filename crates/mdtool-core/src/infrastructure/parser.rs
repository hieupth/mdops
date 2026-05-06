use std::collections::HashMap;

use comrak::nodes::{
    AstNode, ListType, NodeList, NodeTable, NodeValue,
    TableAlignment,
};
use comrak::{parse_document, Arena, Options};

use crate::domain::block::Block;
use crate::domain::block_node::BlockNode;
use crate::domain::container_blocks::{
    Alignment, BlockquoteBlock, ListBlock, ListItemBlock, TableBlock,
};
use crate::domain::document::DocumentBlock;
use crate::domain::leaf_blocks::{
    CodeBlock, FenceBlock, HtmlBlock, ParagraphBlock, ThematicBreakBlock, UnknownBlock,
};
use crate::domain::section::{HeadingVariant, SectionBlock};
use crate::error::{Diagnostic, MdtoolError};
use crate::infrastructure::normalizer::{compute_line_starts, preprocess};
use crate::primitives::{BlockId, ByteRange, LineRange};

/// Parse raw markdown text into a DocumentBlock with flat block list.
pub fn parse_markdown(source: &str) -> Result<DocumentBlock, MdtoolError> {
    let normalized = preprocess(source);
    let line_starts = compute_line_starts(&normalized);
    let arena = Arena::new();
    let options = make_gfm_options();
    let root = parse_document(&arena, &normalized, &options);

    let mut next_id: u32 = 1; // 0 is reserved for document root
    let mut blocks: Vec<BlockNode> = Vec::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    for child in root.children() {
        let block_id = BlockId(next_id);
        next_id += 1; // Reserve this ID for the block itself
        if let Some(block_nodes) = extract_block(child, block_id, &mut next_id, &mut diagnostics) {
            blocks.extend(block_nodes);
        }
    }

    let doc_line_count = if normalized.is_empty() {
        1
    } else {
        normalized.lines().count().max(1)
    };

    let root_id = BlockId(0);
    let root_block = Block {
        id: root_id,
        line_range: LineRange {
            start: 1,
            end: doc_line_count,
        },
        parent_id: None,
        children_ids: blocks.iter().map(|b| b.block().id).collect(),
        byte_range: if normalized.is_empty() {
            None
        } else {
            Some(ByteRange {
                start: 0,
                end: normalized.len(),
            })
        },
    };

    let mut block_by_id: HashMap<BlockId, BlockNode> = HashMap::new();
    for bn in &blocks {
        block_by_id.insert(bn.block().id, bn.clone());
    }

    let doc = DocumentBlock {
        block: root_block,
        source_text: normalized,
        normalized_text: source.to_string(),
        line_starts,
        block_by_id,
        diagnostics,
        metadata: HashMap::new(),
    };

    Ok(doc)
}

fn make_gfm_options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    // footnotes and front_matter are disabled by default
    options.parse.smart = false;
    options.render.hardbreaks = false;
    options.render.github_pre_lang = false;
    options.render.width = 0;
    options.render.unsafe_ = true;
    options
}

fn sourcepos_to_line_range(node: &AstNode) -> LineRange {
    let ast = node.data.borrow();
    LineRange {
        start: ast.sourcepos.start.line,
        end: ast.sourcepos.end.line,
    }
}

/// Extract blocks from a comrak AST node. Returns one or more BlockNodes.
fn extract_block<'a>(
    node: &'a AstNode<'a>,
    block_id: BlockId,
    next_id: &mut u32,
    _diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<BlockNode>> {
    let ast = node.data.borrow();
    let line_range = sourcepos_to_line_range(node);
    let byte_range = Some(ByteRange {
        start: ast.sourcepos.start.column.saturating_sub(1),
        end: ast.sourcepos.end.column,
    });

    let block = Block {
        id: block_id,
        line_range,
        parent_id: None,
        children_ids: Vec::new(),
        byte_range,
    };

    let result = match &ast.value {
        NodeValue::Heading(heading) => {
            let title = extract_text_content(node);
            let slug = compute_slug(&title);
            let variant = if heading.setext {
                HeadingVariant::Setext
            } else {
                HeadingVariant::Atx
            };
            vec![BlockNode::Section(SectionBlock {
                block,
                level: heading.level,
                title,
                slug,
                path: String::new(),
                ordinal: 0,
                variant,
            })]
        }
        NodeValue::Paragraph => {
            let raw_text = extract_text_content(node);
            vec![BlockNode::Paragraph(ParagraphBlock { block, raw_text })]
        }
        NodeValue::CodeBlock(code) => {
            if code.fenced {
                let info_string = if code.info.is_empty() {
                    None
                } else {
                    Some(code.info.clone())
                };
                let language = info_string
                    .as_ref()
                    .map(|s: &String| s.split_whitespace().next().unwrap_or("").to_string());
                vec![BlockNode::Fence(FenceBlock {
                    block,
                    language,
                    info_string,
                    raw_text: code.literal.clone(),
                })]
            } else {
                vec![BlockNode::CodeBlock(CodeBlock {
                    block,
                    raw_text: code.literal.clone(),
                })]
            }
        }
        NodeValue::List(list_node) => {
            extract_list(node, block, list_node, next_id)
        }
        NodeValue::BlockQuote => {
            vec![BlockNode::Blockquote(BlockquoteBlock { block })]
        }
        NodeValue::ThematicBreak => {
            vec![BlockNode::ThematicBreak(ThematicBreakBlock { block })]
        }
        NodeValue::HtmlBlock(html_node) => {
            let raw_text = html_node.literal.clone();
            vec![BlockNode::Html(HtmlBlock { block, raw_text })]
        }
        NodeValue::Table(table_info) => {
            let table_block = extract_table(node, block, table_info);
            vec![BlockNode::Table(table_block)]
        }
        _ => {
            let raw_text = extract_text_content(node);
            if raw_text.is_empty() && node.first_child().is_none() {
                return None;
            }
            vec![BlockNode::Unknown(UnknownBlock { block, raw_text })]
        }
    };

    drop(ast);
    Some(result)
}

fn extract_list<'a>(
    node: &'a AstNode<'a>,
    mut block: Block,
    list_node: &NodeList,
    next_id: &mut u32,
) -> Vec<BlockNode> {
    let ordered = list_node.list_type == ListType::Ordered;
    let marker = if ordered { '.' } else { '-' };
    let tight = list_node.tight;

    let mut item_ids = Vec::new();
    let mut all_nodes = Vec::new();

    for item_node in node.children() {
        let item_id = BlockId(*next_id);
        *next_id += 1;
        let item_ast = item_node.data.borrow();
        let item_line_range = LineRange {
            start: item_ast.sourcepos.start.line,
            end: item_ast.sourcepos.end.line,
        };
        let item_block = Block {
            id: item_id,
            line_range: item_line_range,
            parent_id: None,
            children_ids: Vec::new(),
            byte_range: None,
        };

        // Check for task item
        let checked = item_node.children().find_map(|c| {
            let c_ast = c.data.borrow();
            if let NodeValue::TaskItem(sym) = &c_ast.value {
                Some(sym.is_some())
            } else {
                None
            }
        });

        let order = if ordered {
            Some((item_ids.len() as u32) + 1)
        } else {
            None
        };

        item_ids.push(item_id);
        all_nodes.push(BlockNode::ListItem(ListItemBlock {
            block: item_block,
            checked,
            order,
        }));
    }

    block.children_ids = item_ids;
    all_nodes.insert(0, BlockNode::List(ListBlock {
        block,
        ordered,
        marker,
        tight,
    }));

    all_nodes
}

fn extract_table<'a>(node: &'a AstNode<'a>, block: Block, table_info: &NodeTable) -> TableBlock {
    let alignments: Vec<Alignment> = table_info
        .alignments
        .iter()
        .map(|a| match a {
            TableAlignment::None => Alignment::None,
            TableAlignment::Left => Alignment::Left,
            TableAlignment::Center => Alignment::Center,
            TableAlignment::Right => Alignment::Right,
        })
        .collect();

    let mut header_row: Vec<String> = Vec::new();
    let mut body_rows: Vec<Vec<String>> = Vec::new();
    let mut found_header = false;

    for child in node.children() {
        let child_ast = child.data.borrow();
        match &child_ast.value {
            NodeValue::TableRow(is_header) => {
                if *is_header {
                    header_row = extract_table_row_cells(child);
                    found_header = true;
                } else {
                    body_rows.push(extract_table_row_cells(child));
                }
            }
            _ => {}
        }
        drop(child_ast);
    }

    // Fallback: if no header row found, first child with cells becomes header
    if !found_header {
        if let Some(first_child) = node.first_child() {
            header_row = extract_table_row_cells(first_child);
            if !body_rows.is_empty() {
                body_rows.remove(0);
            }
        }
    }

    TableBlock {
        block,
        alignments,
        header_row,
        body_rows,
    }
}

fn extract_table_row_cells<'a>(row_node: &'a AstNode<'a>) -> Vec<String> {
    let mut cells = Vec::new();
    for cell_node in row_node.children() {
        let cell_ast = cell_node.data.borrow();
        if matches!(cell_ast.value, NodeValue::TableCell) {
            drop(cell_ast);
            cells.push(extract_text_content(cell_node));
        }
    }
    cells
}

/// Extract all text content from a node and its descendants.
fn extract_text_content<'a>(node: &'a AstNode<'a>) -> String {
    let mut result = String::new();
    collect_text(node, &mut result);
    result.trim().to_string()
}

fn collect_text<'a>(node: &'a AstNode<'a>, result: &mut String) {
    let ast = node.data.borrow();
    match &ast.value {
        NodeValue::Text(text) => {
            result.push_str(text);
        }
        NodeValue::Code(code) => {
            result.push_str(&code.literal);
        }
        NodeValue::SoftBreak => {
            result.push('\n');
        }
        NodeValue::LineBreak => {
            result.push('\n');
        }
        NodeValue::HtmlInline(html) => {
            result.push_str(html);
        }
        _ => {
            drop(ast);
            for child in node.children() {
                collect_text(child, result);
            }
        }
    }
}

/// Compute a URL-safe slug from a title.
pub fn compute_slug(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let doc = parse_markdown("").unwrap();
        assert_eq!(doc.block.id, BlockId(0));
        assert!(doc.block.children_ids.is_empty());
    }

    #[test]
    fn parse_single_paragraph() {
        let doc = parse_markdown("Hello world").unwrap();
        assert_eq!(doc.block.children_ids.len(), 1);
        let bn = doc.block_by_id.get(&doc.block.children_ids[0]).unwrap();
        match bn {
            BlockNode::Paragraph(p) => assert_eq!(p.raw_text, "Hello world"),
            _ => panic!("Expected Paragraph, got {:?}", bn.block_type_name()),
        }
    }

    #[test]
    fn parse_heading() {
        let doc = parse_markdown("# Title\n\nBody text.").unwrap();
        let found_section = doc.block_by_id.values().any(|bn| {
            matches!(bn, BlockNode::Section(s) if s.title == "Title" && s.level == 1)
        });
        assert!(found_section, "Should find a Section with title 'Title'");
    }

    #[test]
    fn parse_fenced_code() {
        let input = "```rust\nfn main() {}\n```\n";
        let doc = parse_markdown(input).unwrap();
        let found_fence = doc.block_by_id.values().any(|bn| {
            matches!(bn, BlockNode::Fence(f) if f.language.as_deref() == Some("rust"))
        });
        assert!(found_fence, "Should find a FenceBlock with language 'rust'");
    }

    #[test]
    fn parse_thematic_break() {
        let doc = parse_markdown("---\n").unwrap();
        let found = doc
            .block_by_id
            .values()
            .any(|bn| matches!(bn, BlockNode::ThematicBreak(_)));
        assert!(found, "Should find a ThematicBreak");
    }

    #[test]
    fn parse_list() {
        let input = "- item A\n- item B\n";
        let doc = parse_markdown(input).unwrap();
        let found = doc.block_by_id.values().any(|bn| {
            matches!(bn, BlockNode::List(l) if !l.ordered && l.block.children_ids.len() >= 2)
        });
        assert!(found, "Should find an unordered List with items");
    }

    #[test]
    fn parse_table() {
        let input = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let doc = parse_markdown(input).unwrap();
        let found = doc.block_by_id.values().any(|bn| {
            matches!(bn, BlockNode::Table(t) if t.header_row.len() == 2 && !t.body_rows.is_empty())
        });
        assert!(found, "Should find a Table with header and body");
    }

    #[test]
    fn parse_blockquote() {
        let input = "> quoted text\n";
        let doc = parse_markdown(input).unwrap();
        let found = doc
            .block_by_id
            .values()
            .any(|bn| matches!(bn, BlockNode::Blockquote(_)));
        assert!(found, "Should find a Blockquote");
    }

    #[test]
    fn parse_html_block() {
        let input = "<div>\nsome html\n</div>\n";
        let doc = parse_markdown(input).unwrap();
        let found = doc
            .block_by_id
            .values()
            .any(|bn| matches!(bn, BlockNode::Html(_)));
        assert!(found, "Should find an HtmlBlock");
    }

    #[test]
    fn parse_setext_heading() {
        let input = "Title\n===\n\nBody.\n";
        let doc = parse_markdown(input).unwrap();
        let found = doc.block_by_id.values().any(|bn| {
            matches!(bn, BlockNode::Section(s) if s.variant == HeadingVariant::Setext)
        });
        assert!(found, "Should find a setext heading");
    }

    #[test]
    fn parse_task_list() {
        let input = "- [ ] unchecked\n- [x] checked\n";
        let doc = parse_markdown(input).unwrap();
        let items: Vec<&BlockNode> = doc
            .block_by_id
            .values()
            .filter(|bn| matches!(bn, BlockNode::ListItem(_)))
            .collect();
        assert!(items.len() >= 2, "Should find list items");
    }

    #[test]
    fn compute_slug_basic() {
        assert_eq!(compute_slug("Hello World"), "hello-world");
    }

    #[test]
    fn compute_slug_special_chars() {
        assert_eq!(compute_slug("A & B! C"), "a-b-c");
    }

    #[test]
    fn parse_full_example() {
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
        let doc = parse_markdown(input).unwrap();
        // Should have sections
        let sections: Vec<&SectionBlock> = doc
            .block_by_id
            .values()
            .filter_map(|bn| match bn {
                BlockNode::Section(s) => Some(s),
                _ => None,
            })
            .collect();
        assert!(sections.len() >= 3, "Should have at least 3 sections, got {}", sections.len());

        // Should have a table
        let has_table = doc.block_by_id.values().any(|bn| matches!(bn, BlockNode::Table(_)));
        assert!(has_table, "Should have a table");

        // Should have a list
        let has_list = doc.block_by_id.values().any(|bn| matches!(bn, BlockNode::List(_)));
        assert!(has_list, "Should have a list");
    }
}
