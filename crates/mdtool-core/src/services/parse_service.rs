use crate::builder::tree_builder::build_tree;
use crate::domain::document::DocumentBlock;
use crate::error::MdtoolError;

pub struct ParseService;

impl ParseService {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_text(&self, text: &str) -> Result<DocumentBlock, MdtoolError> {
        build_tree(text)
    }

    pub fn parse_file(&self, path: &str) -> Result<DocumentBlock, MdtoolError> {
        let text = std::fs::read_to_string(path)?;
        build_tree(&text)
    }
}
