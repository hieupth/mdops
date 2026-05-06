use crate::domain::block::Block;

/// Paragraph block.
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphBlock {
    pub block: Block,
    pub raw_text: String,
}

/// Fenced code block.
#[derive(Debug, Clone, PartialEq)]
pub struct FenceBlock {
    pub block: Block,
    /// Language identifier (first word of info string).
    pub language: Option<String>,
    /// Full info string after the opening fence marker.
    pub info_string: Option<String>,
    pub raw_text: String,
}

/// Indented code block.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeBlock {
    pub block: Block,
    pub raw_text: String,
}

/// Thematic break (horizontal rule: ---, ***, ___).
#[derive(Debug, Clone, PartialEq)]
pub struct ThematicBreakBlock {
    pub block: Block,
}

/// Raw HTML block.
#[derive(Debug, Clone, PartialEq)]
pub struct HtmlBlock {
    pub block: Block,
    pub raw_text: String,
}

/// Link reference definition.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkRefDefBlock {
    pub block: Block,
    pub label: String,
    pub destination: String,
    pub title: Option<String>,
}

/// Unknown/unrecognized block type.
#[derive(Debug, Clone, PartialEq)]
pub struct UnknownBlock {
    pub block: Block,
    pub raw_text: String,
}
