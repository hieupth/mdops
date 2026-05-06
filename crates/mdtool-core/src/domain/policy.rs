use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Filesystem access policy for constraining tool/agent operations.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct FilesystemPolicy {
    /// Allowed root directories. Empty means deny all.
    pub allowed_roots: Vec<String>,
    /// If true, no write operations allowed.
    pub read_only: bool,
    /// If true, existing files can be overwritten.
    pub allow_overwrite: bool,
    /// If true, create backup files before writing.
    pub create_backups: bool,
}

impl Default for FilesystemPolicy {
    fn default() -> Self {
        Self {
            allowed_roots: vec![".".to_string()],
            read_only: false,
            allow_overwrite: true,
            create_backups: false,
        }
    }
}
