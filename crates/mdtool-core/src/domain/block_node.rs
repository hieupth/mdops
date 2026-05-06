use crate::domain::block::Block;
use crate::domain::container_blocks::{BlockquoteBlock, ListBlock, ListItemBlock, TableBlock};
use crate::domain::document::DocumentBlock;
use crate::domain::leaf_blocks::{
    CodeBlock, FenceBlock, HtmlBlock, LinkRefDefBlock, ParagraphBlock, ThematicBreakBlock,
    UnknownBlock,
};
use crate::domain::section::SectionBlock;

/// Sum type over all concrete block variants. No inheritance — composition + enum.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockNode {
    Document(DocumentBlock),
    Section(SectionBlock),
    Blockquote(BlockquoteBlock),
    List(ListBlock),
    ListItem(ListItemBlock),
    Table(TableBlock),
    Paragraph(ParagraphBlock),
    Fence(FenceBlock),
    CodeBlock(CodeBlock),
    ThematicBreak(ThematicBreakBlock),
    Html(HtmlBlock),
    LinkRefDef(LinkRefDefBlock),
    Unknown(UnknownBlock),
}

impl BlockNode {
    pub fn block(&self) -> &Block {
        match self {
            BlockNode::Document(d) => &d.block,
            BlockNode::Section(s) => &s.block,
            BlockNode::Blockquote(b) => &b.block,
            BlockNode::List(l) => &l.block,
            BlockNode::ListItem(li) => &li.block,
            BlockNode::Table(t) => &t.block,
            BlockNode::Paragraph(p) => &p.block,
            BlockNode::Fence(f) => &f.block,
            BlockNode::CodeBlock(c) => &c.block,
            BlockNode::ThematicBreak(tb) => &tb.block,
            BlockNode::Html(h) => &h.block,
            BlockNode::LinkRefDef(lrd) => &lrd.block,
            BlockNode::Unknown(u) => &u.block,
        }
    }

    pub fn block_mut(&mut self) -> &mut Block {
        match self {
            BlockNode::Document(d) => &mut d.block,
            BlockNode::Section(s) => &mut s.block,
            BlockNode::Blockquote(b) => &mut b.block,
            BlockNode::List(l) => &mut l.block,
            BlockNode::ListItem(li) => &mut li.block,
            BlockNode::Table(t) => &mut t.block,
            BlockNode::Paragraph(p) => &mut p.block,
            BlockNode::Fence(f) => &mut f.block,
            BlockNode::CodeBlock(c) => &mut c.block,
            BlockNode::ThematicBreak(tb) => &mut tb.block,
            BlockNode::Html(h) => &mut h.block,
            BlockNode::LinkRefDef(lrd) => &mut lrd.block,
            BlockNode::Unknown(u) => &mut u.block,
        }
    }

    /// Returns the lowercase type name string for this block variant.
    pub fn block_type_name(&self) -> &'static str {
        match self {
            BlockNode::Document(_) => "document",
            BlockNode::Section(_) => "section",
            BlockNode::Blockquote(_) => "blockquote",
            BlockNode::List(_) => "list",
            BlockNode::ListItem(_) => "list_item",
            BlockNode::Table(_) => "table",
            BlockNode::Paragraph(_) => "paragraph",
            BlockNode::Fence(_) => "fence",
            BlockNode::CodeBlock(_) => "code_block",
            BlockNode::ThematicBreak(_) => "thematic_break",
            BlockNode::Html(_) => "html",
            BlockNode::LinkRefDef(_) => "link_ref_def",
            BlockNode::Unknown(_) => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn block_node_dispatch() {
        let node = BlockNode::Paragraph(ParagraphBlock {
            block: make_block(1),
            raw_text: "hello".to_string(),
        });
        assert_eq!(node.block().id, BlockId(1));
        assert_eq!(node.block_type_name(), "paragraph");
    }

    #[test]
    fn block_node_mutable() {
        let mut node = BlockNode::Paragraph(ParagraphBlock {
            block: make_block(1),
            raw_text: "hello".to_string(),
        });
        node.block_mut().parent_id = Some(BlockId(0));
        assert_eq!(node.block().parent_id, Some(BlockId(0)));
    }

    #[test]
    fn all_type_names() {
        let cases: Vec<(BlockNode, &'static str)> = vec![
            (BlockNode::Document(DocumentBlock::new("".to_string())), "document"),
            (BlockNode::Section(SectionBlock {
                block: make_block(0),
                level: 1,
                title: "Test".to_string(),
                slug: "test".to_string(),
                path: "/Test".to_string(),
                ordinal: 1,
                variant: crate::domain::section::HeadingVariant::Atx,
            }), "section"),
            (BlockNode::Blockquote(BlockquoteBlock { block: make_block(0) }), "blockquote"),
            (BlockNode::List(ListBlock {
                block: make_block(0),
                ordered: false,
                marker: '-',
                tight: true,
            }), "list"),
            (BlockNode::ListItem(ListItemBlock {
                block: make_block(0),
                checked: None,
                order: None,
            }), "list_item"),
            (BlockNode::Table(TableBlock {
                block: make_block(0),
                alignments: vec![],
                header_row: vec![],
                body_rows: vec![],
            }), "table"),
            (BlockNode::Paragraph(ParagraphBlock {
                block: make_block(0),
                raw_text: "".to_string(),
            }), "paragraph"),
            (BlockNode::Fence(FenceBlock {
                block: make_block(0),
                language: None,
                info_string: None,
                raw_text: "".to_string(),
            }), "fence"),
            (BlockNode::CodeBlock(CodeBlock {
                block: make_block(0),
                raw_text: "".to_string(),
            }), "code_block"),
            (BlockNode::ThematicBreak(ThematicBreakBlock { block: make_block(0) }), "thematic_break"),
            (BlockNode::Html(HtmlBlock {
                block: make_block(0),
                raw_text: "".to_string(),
            }), "html"),
            (BlockNode::LinkRefDef(LinkRefDefBlock {
                block: make_block(0),
                label: "".to_string(),
                destination: "".to_string(),
                title: None,
            }), "link_ref_def"),
            (BlockNode::Unknown(UnknownBlock {
                block: make_block(0),
                raw_text: "".to_string(),
            }), "unknown"),
        ];

        for (node, expected_name) in cases {
            assert_eq!(node.block_type_name(), expected_name);
        }
    }
}
