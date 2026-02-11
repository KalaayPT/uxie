//! Shared environment-path helpers for integration/fixture tests.
//!
//! This module is intentionally small and stable so tests across `src/` and
//! `tests/` can use one implementation for env-var based fixture discovery.

use std::path::PathBuf;

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
pub fn existing_path_from_env_with_fallback(
    primary_var: &str,
    fallback_var: &str,
    context: &str,
) -> Option<PathBuf> {
    let chosen = if std::env::var_os(primary_var).is_some() {
        primary_var
    } else if std::env::var_os(fallback_var).is_some() {
        fallback_var
    } else {
        eprintln!(
            "Skipping {}: neither {} nor {} is set",
            context, primary_var, fallback_var
        );
        return None;
    };

    existing_path_from_env(chosen, context)
}
