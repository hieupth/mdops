use crate::ascii::detect::detect_ascii_blocks;
use crate::ascii::format::format_block;
use crate::ascii::model::AsciiMode;
use crate::builder::tree_builder::build_tree;
use crate::domain::policy::FilesystemPolicy;
use crate::domain::response::WriteResponse;
use crate::error::MdtoolError;

pub struct AsciiService {
    policy: FilesystemPolicy,
}

impl AsciiService {
    pub fn new(policy: FilesystemPolicy) -> Self {
        Self { policy }
    }

    pub fn format_ascii(
        &self,
        file_path: &str,
        mode: AsciiMode,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        crate::infrastructure::policy_guard::check_path(&self.policy, file_path)?;
        let text = std::fs::read_to_string(file_path)?;
        let doc = build_tree(&text)?;

        let ascii_blocks = detect_ascii_blocks(&doc);
        let mut all_diagnostics = Vec::new();
        let mut changed = false;
        let mut current_text = text.clone();

        for ablock in &ascii_blocks {
            let edit = format_block(ablock, mode);
            all_diagnostics.extend(edit.diagnostics);
            if edit.changed {
                changed = true;
                // Replace the block's lines in the source text
                let lines: Vec<&str> = current_text.lines().collect();
                let bn = doc.get_block(ablock.block_id);
                if let Some(bnode) = bn {
                    let range = bnode.block().line_range;
                    // Replace lines in range with formatted lines
                    let mut new_lines: Vec<String> = Vec::new();
                    for (i, line) in lines.iter().enumerate() {
                        let line_num = i + 1;
                        if line_num >= range.start && line_num <= range.end {
                            // Skip original lines in range
                        } else {
                            new_lines.push(line.to_string());
                        }
                    }
                    // Insert formatted lines at the right position
                    let mut insert_idx = (range.start - 1).min(new_lines.len());
                    for formatted_line in &edit.after {
                        new_lines.insert(insert_idx, formatted_line.clone());
                        insert_idx += 1;
                    }
                    current_text = new_lines.join("\n") + "\n";
                }
            }
        }

        if !dry_run && changed {
            crate::infrastructure::policy_guard::check_write_allowed(&self.policy)?;
            std::fs::write(file_path, &current_text)?;
        }

        let diff_summary = if changed {
            Some(crate::ops::diff::compute_diff(&text, &current_text))
        } else {
            None
        };

        Ok(WriteResponse {
            success: true,
            changed,
            diagnostics: all_diagnostics,
            diff_summary,
            content: None,
        })
    }
}
