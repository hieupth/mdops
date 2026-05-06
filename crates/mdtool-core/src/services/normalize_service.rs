use crate::domain::policy::FilesystemPolicy;
use crate::domain::response::WriteResponse;
use crate::error::MdtoolError;
use crate::processing::normalize::{normalize, NormalizeOptions};

pub struct NormalizeService {
    policy: FilesystemPolicy,
}

impl NormalizeService {
    pub fn new(policy: FilesystemPolicy) -> Self {
        Self { policy }
    }

    pub fn normalize(
        &self,
        file_path: &str,
        options: Option<NormalizeOptions>,
        dry_run: bool,
    ) -> Result<WriteResponse, MdtoolError> {
        crate::infrastructure::policy_guard::check_path(&self.policy, file_path)?;
        let text = std::fs::read_to_string(file_path)?;
        let opts = options.unwrap_or_default();
        let normalized = normalize(&text, &opts);
        let changed = normalized != text;

        if !dry_run && changed {
            crate::infrastructure::policy_guard::check_write_allowed(&self.policy)?;
            std::fs::write(file_path, &normalized)?;
        }

        let diff_summary = if changed {
            Some(crate::ops::diff::compute_diff(&text, &normalized))
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
}
