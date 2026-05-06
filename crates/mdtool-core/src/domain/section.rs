use crate::domain::block::Block;

/// Heading variant: ATX-style (# prefix) or Setext-style (underline with === or ---).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadingVariant {
    Atx,
    Setext,
}

/// A heading-delimited container block. Always has children.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionBlock {
    pub block: Block,
    /// Heading level 1–6.
    pub level: u8,
    /// Heading title text (without # prefix).
    pub title: String,
    /// URL-safe title slug.
    pub slug: String,
    /// Canonical path e.g., "/Architecture/Parser".
    pub path: String,
    /// Disambiguates repeated sibling headings.
    pub ordinal: u32,
    /// ATX or Setext heading style.
    pub variant: HeadingVariant,
}
