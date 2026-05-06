use crate::builder::tree_builder::build_tree;
use crate::domain::policy::FilesystemPolicy;
use crate::domain::response::ReadResponse;
use crate::error::MdtoolError;
use crate::processing::validate::validate;

pub struct ValidateService {
    policy: FilesystemPolicy,
}

impl ValidateService {
    pub fn new(policy: FilesystemPolicy) -> Self {
        Self { policy }
    }

    pub fn validate(&self, file_path: &str) -> Result<ReadResponse, MdtoolError> {
        crate::infrastructure::policy_guard::check_path(&self.policy, file_path)?;
        let text = std::fs::read_to_string(file_path)?;
        let doc = build_tree(&text)?;
        let diagnostics = validate(&doc);
        let data = serde_json::to_value(&diagnostics).unwrap_or(serde_json::Value::Null);
        Ok(ReadResponse {
            success: true,
            data,
            diagnostics: Some(diagnostics),
        })
    }
}
