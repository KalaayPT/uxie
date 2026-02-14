//! Shared environment-path helpers for integration/fixture tests.
//!
//! This module is intentionally small and stable so tests across `src/` and
//! `tests/` can use one implementation for env-var based fixture discovery.

use std::path::{Path, PathBuf};

#[doc(hidden)]
pub fn existing_path_from_env(var: &str, context: &str) -> Option<PathBuf> {
    let Some(raw) = std::env::var_os(var) else {
        eprintln!("Skipping {}: env var {} is not set", context, var);
        return None;
    };

    let path = PathBuf::from(raw);
    if !path.exists() {
        eprintln!(
            "Skipping {}: path from {} does not exist: {}",
            context,
            var,
            path.display()
        );
        return None;
    }

    Some(path)
}

#[doc(hidden)]
pub fn existing_file_under_project_env(
    project_var: &str,
    relative_candidates: &[&str],
    context: &str,
) -> Option<PathBuf> {
    let project_root = existing_path_from_env(project_var, context)?;
    for rel in relative_candidates {
        let candidate = project_root.join(Path::new(rel));
        if candidate.exists() {
            return Some(candidate);
        }
    }

    eprintln!(
        "Skipping {}: none of the expected files exist under {}: {} (checked: {})",
        context,
        project_var,
        project_root.display(),
        relative_candidates.join(", ")
    );
    None
}
