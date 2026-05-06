use crate::ascii::detect::detect_ascii_blocks;
use crate::ascii::format::format_block;
use crate::builder::tree_builder::build_tree;
use crate::domain::block_node::BlockNode;
use crate::domain::document::DocumentBlock;
use crate::domain::policy::FilesystemPolicy;
use crate::domain::response::{ReadResponse, WriteResponse};
use crate::domain::selectors::{BlockSelector, InsertPosition};
use crate::error::MdtoolError;
use crate::ops::diff::DiffSummary;
use crate::ops::edit::{edit, EditOp};
use crate::ops::read::{read_block_children, read_block_text, read_block_tree, read_blocks_by_type, read_outline};
use crate::ops::resolve::resolve_selector;
use crate::ops::search::search_blocks;
use crate::ops::semantic::*;
use crate::ops::write;
use crate::primitives::BlockId;
use crate::processing::normalize::{normalize, NormalizeOptions};
use crate::processing::validate::validate;

pub struct BlockService {
    policy: FilesystemPolicy,
}

impl BlockService {
    pub fn new(policy: FilesystemPolicy) -> Self {
        Self { policy }
    }

    fn load_doc(&self, file_path: &str) -> Result<DocumentBlock, MdtoolError> {
        crate::infrastructure::policy_guard::check_path(&self.policy, file_path)?;
        let text = std::fs::read_to_string(file_path)?;
        build_tree(&text)
    }

    fn write_back(&self, file_path: &str, doc: &DocumentBlock) -> Result<(), MdtoolError> {
        crate::infrastructure::policy_guard::check_write_allowed(&self.policy)?;
        std::fs::write(file_path, &doc.source_text)?;
        Ok(())
    }

    // === Read operations ===

    pub fn read_outline(&self, file_path: &str, max_depth: u8, include_paths: bool) -> Result<ReadResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let entries = read_outline(&doc, max_depth, include_paths);
        let data = serde_json::to_value(&entries).unwrap_or(serde_json::Value::Null);
        Ok(ReadResponse {
            success: true,
            data,
            diagnostics: if doc.diagnostics.is_empty() { None } else { Some(doc.diagnostics) },
        })
    }

    pub fn read_block(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        include_text: bool,
    ) -> Result<ReadResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let block_id = resolve_selector(&doc, selector)?;
        let bn = doc.get_block(block_id).ok_or_else(|| MdtoolError::BlockNotFound {
            selector: format!("{:?}", selector),
        })?;

        let data = if include_text {
            let text = doc.source_text_for_range(bn.block().line_range);
            serde_json::json!({
                "id": block_id,
                "type": bn.block_type_name(),
                "line_range": bn.block().line_range,
                "text": text,
            })
        } else {
            serde_json::json!({
                "id": block_id,
                "type": bn.block_type_name(),
                "line_range": bn.block().line_range,
            })
        };

        Ok(ReadResponse {
            success: true,
            data,
            diagnostics: None,
        })
    }

    pub fn read_block_text(
        &self,
        file_path: &str,
        selector: &BlockSelector,
    ) -> Result<ReadResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let text = crate::ops::read::read_block_text(&doc, selector)?;
        Ok(ReadResponse {
            success: true,
            data: serde_json::json!({ "text": text }),
            diagnostics: None,
        })
    }

    pub fn read_block_tree(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        depth: i32,
        include_text: bool,
    ) -> Result<ReadResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let tree = read_block_tree(&doc, selector, depth)?;
        let data = serde_json::to_value(&tree).unwrap_or(serde_json::Value::Null);
        Ok(ReadResponse {
            success: true,
            data,
            diagnostics: None,
        })
    }

    pub fn read_block_children(
        &self,
        file_path: &str,
        selector: &BlockSelector,
    ) -> Result<ReadResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let children = read_block_children(&doc, selector)?;
        let data = serde_json::to_value(&children).unwrap_or(serde_json::Value::Null);
        Ok(ReadResponse {
            success: true,
            data,
            diagnostics: None,
        })
    }

    pub fn read_blocks_by_type(
        &self,
        file_path: &str,
        block_type: &str,
    ) -> Result<ReadResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let blocks = read_blocks_by_type(&doc, block_type);
        let data = serde_json::to_value(&blocks).unwrap_or(serde_json::Value::Null);
        Ok(ReadResponse {
            success: true,
            data,
            diagnostics: None,
        })
    }

    pub fn search_blocks(
        &self,
        file_path: &str,
        query: &str,
        selector: Option<&BlockSelector>,
        case_sensitive: bool,
    ) -> Result<ReadResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let matches = search_blocks(&doc, query, selector, case_sensitive);
        let data = serde_json::to_value(&matches).unwrap_or(serde_json::Value::Null);
        Ok(ReadResponse {
            success: true,
            data,
            diagnostics: None,
        })
    }

    // === Write operations ===

    fn apply_write(
        &self,
        file_path: &str,
        result: (DocumentBlock, DiffSummary, Vec<crate::error::Diagnostic>),
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let (new_doc, diff, diags) = result;
        if !dry_run {
            self.write_back(file_path, &new_doc)?;
        }
        Ok(WriteResponse {
            success: true,
            changed: diff.changed,
            diagnostics: diags,
            diff_summary: Some(diff),
            content: None,
        })
    }

    pub fn replace_block(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        content: &str,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let result = write::replace_block(&doc, selector, content)?;
        self.apply_write(file_path, result, dry_run)
    }

    pub fn insert_block(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        content: &str,
        position: InsertPosition,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let result = match position {
            InsertPosition::Before => write::insert_block_before(&doc, selector, content)?,
            InsertPosition::After => write::insert_block_after(&doc, selector, content)?,
            InsertPosition::Append => write::append_child(&doc, selector, content)?,
            InsertPosition::Index(_) => write::insert_block_after(&doc, selector, content)?,
        };
        self.apply_write(file_path, result, dry_run)
    }

    pub fn delete_block(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let result = write::delete_block(&doc, selector)?;
        self.apply_write(file_path, result, dry_run)
    }

    pub fn move_block(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        target_parent_id: BlockId,
        index: Option<usize>,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let result = write::move_block(&doc, selector, target_parent_id, index)?;
        self.apply_write(file_path, result, dry_run)
    }

    pub fn ensure_section(
        &self,
        file_path: &str,
        path: &str,
        heading_level: u8,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let result = write::ensure_section(&doc, path, heading_level)?;
        self.apply_write(file_path, result, dry_run)
    }

    // === Semantic helpers ===

    pub fn rename_section(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        new_title: &str,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let result = crate::ops::semantic::rename_section(&doc, selector, new_title)?;
        self.apply_write(file_path, result, dry_run)
    }

    pub fn change_heading_level(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        new_level: u8,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let result = crate::ops::semantic::change_heading_level(&doc, selector, new_level)?;
        self.apply_write(file_path, result, dry_run)
    }

    pub fn add_table_row(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        row: &[String],
        position: InsertPosition,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let result = crate::ops::semantic::add_table_row(&doc, selector, row, position)?;
        self.apply_write(file_path, result, dry_run)
    }

    pub fn update_table_cell(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        row: usize,
        col: usize,
        value: &str,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let result = crate::ops::semantic::update_table_cell(&doc, selector, row, col, value)?;
        self.apply_write(file_path, result, dry_run)
    }

    pub fn remove_table_row(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        row: usize,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let result = crate::ops::semantic::remove_table_row(&doc, selector, row)?;
        self.apply_write(file_path, result, dry_run)
    }

    pub fn toggle_task(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let result = crate::ops::semantic::toggle_task(&doc, selector)?;
        self.apply_write(file_path, result, dry_run)
    }

    pub fn add_list_item(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        text: &str,
        index: Option<usize>,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let result = crate::ops::semantic::add_list_item(&doc, selector, text, index)?;
        self.apply_write(file_path, result, dry_run)
    }

    pub fn remove_list_item(
        &self,
        file_path: &str,
        selector: &BlockSelector,
        index: usize,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let result = crate::ops::semantic::remove_list_item(&doc, selector, index)?;
        self.apply_write(file_path, result, dry_run)
    }

    // === Batch edit ===

    pub fn edit(
        &self,
        file_path: &str,
        operations: &[EditOp],
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let (new_doc, diff, diags) = crate::ops::edit::edit(&doc, operations)?;
        if !dry_run {
            self.write_back(file_path, &new_doc)?;
        }
        Ok(WriteResponse {
            success: true,
            changed: diff.changed,
            diagnostics: diags,
            diff_summary: Some(diff),
            content: None,
        })
    }

    // === Processing ===

    pub fn normalize(
        &self,
        file_path: &str,
        options: Option<NormalizeOptions>,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let opts = options.unwrap_or_default();
        let normalized = crate::processing::normalize::normalize(&doc.source_text, &opts);
        let changed = normalized != doc.source_text;

        if !dry_run && changed {
            crate::infrastructure::policy_guard::check_write_allowed(&self.policy)?;
            std::fs::write(file_path, &normalized)?;
        }

        let diff_summary = if changed {
            Some(crate::ops::diff::compute_diff(&doc.source_text, &normalized))
        } else {
            None
        };

        Ok(WriteResponse {
            success: true,
            changed,
            diagnostics: vec![],
            diff_summary,
            content: None,
        })
    }

    pub fn validate(&self, file_path: &str) -> Result<ReadResponse, MdtoolError> {
        let doc = self.load_doc(file_path)?;
        let diagnostics = validate(&doc);
        let data = serde_json::to_value(&diagnostics).unwrap_or(serde_json::Value::Null);
        Ok(ReadResponse {
            success: true,
            data,
            diagnostics: Some(diagnostics),
        })
    }
}
