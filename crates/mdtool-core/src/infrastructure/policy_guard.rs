use std::path::PathBuf;

use crate::domain::policy::FilesystemPolicy;
use crate::error::MdtoolError;

/// Check if a file path is allowed by the filesystem policy.
pub fn check_path(policy: &FilesystemPolicy, path: &str) -> Result<PathBuf, MdtoolError> {
    let abs = std::path::Path::new(path);
    let canonical = if abs.is_absolute() {
        abs.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };

    // Check allowed roots
    if !policy.allowed_roots.is_empty() {
        let allowed = policy
            .allowed_roots
            .iter()
            .any(|root| {
                let root_path = std::path::Path::new(root);
                let abs_root = if root_path.is_absolute() {
                    root_path.to_path_buf()
                } else {
                    std::env::current_dir().unwrap_or_default().join(root_path)
                };
                canonical.starts_with(&abs_root)
            });
        if !allowed {
            return Err(MdtoolError::Policy(format!(
                "Path '{}' is outside allowed roots",
                path
            )));
        }
    } else {
        return Err(MdtoolError::Policy(
            "No allowed roots configured".to_string(),
        ));
    }

    Ok(canonical)
}

/// Check if a write operation is allowed.
pub fn check_write_allowed(policy: &FilesystemPolicy) -> Result<(), MdtoolError> {
    if policy.read_only {
        return Err(MdtoolError::Policy(
            "Write operations not allowed (read-only policy)".to_string(),
        ));
    }
    Ok(())
}
