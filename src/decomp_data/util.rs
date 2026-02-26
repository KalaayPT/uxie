use serde::de::DeserializeOwned;
use std::fs;
use std::io;
use std::path::Path;

/// Resolve a required symbolic constant using a caller-provided resolver.
///
/// This keeps decomp conversion code decoupled from any specific symbol source
/// (for example `Workspace::resolve_constant`) while still failing fast when a
/// required constant is missing.
pub(crate) fn resolve_required_constant<F>(
    resolver: &F,
    constant: &str,
    field: &str,
    domain: &str,
) -> io::Result<i64>
where
    F: Fn(&str) -> Option<i64>,
{
    resolver(constant).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to resolve required constant '{}' for {} field '{}'",
                constant, domain, field
            ),
        )
    })
}

/// Read and deserialize a JSON file into `T`, mapping serde failures to
/// `InvalidData`.
pub(crate) fn load_json_file<T>(path: impl AsRef<Path>) -> io::Result<T>
where
    T: DeserializeOwned,
{
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
