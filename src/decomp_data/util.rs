use serde::de::DeserializeOwned;
use std::fs;
use std::io;
use std::path::Path;

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

pub(crate) fn load_json_file<T>(path: impl AsRef<Path>) -> io::Result<T>
where
    T: DeserializeOwned,
{
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
