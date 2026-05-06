use crate::domain::block::Block;

/// Table cell alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
    None,
}

/// Block quote container.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockquoteBlock {
    pub block: Block,
}

/// List container (ordered or unordered).
#[derive(Debug, Clone, PartialEq)]
pub struct ListBlock {
    pub block: Block,
    /// true = ordered (1. 2. 3.)
    pub ordered: bool,
    /// The actual marker character used (-, *, + for unordered; . for ordered).
    pub marker: char,
    /// tight = no blank lines between items.
    pub tight: bool,
}

/// List item within a ListBlock.
#[derive(Debug, Clone, PartialEq)]
pub struct ListItemBlock {
    pub block: Block,
    /// None = normal, Some(true) = [x], Some(false) = [ ]
    pub checked: Option<bool>,
    /// Numeric value for ordered list items.
    pub order: Option<u32>,
}

/// Table block with header and body rows.
#[derive(Debug, Clone, PartialEq)]
pub struct TableBlock {
    pub block: Block,
    /// Column alignments.
    pub alignments: Vec<Alignment>,
    /// Header row cell values.
    pub header_row: Vec<String>,
    /// Body rows (each row is a vector of cell values).
    pub body_rows: Vec<Vec<String>>,
}
