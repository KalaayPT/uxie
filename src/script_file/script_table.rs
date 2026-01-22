//! Script name to file ID mapping table.
//!
//! Parses `scripts.order` files from decompilation projects to map
//! symbolic script names to their numeric file IDs.

use rustc_hash::FxHashMap;
use std::path::Path;

/// Mapping between script names and their file IDs.
///
/// In decompilation projects, scripts are referenced by symbolic names
/// (e.g., `scripts_jubilife_city`). The `scripts.order` file maps these
/// names to numeric file IDs based on line number (0-indexed).
///
/// # Example
///
/// ```
/// use uxie::script_file::ScriptTable;
///
/// let mut table = ScriptTable::new();
/// table.load_order_str("scripts_unk_0000\nscripts_jubilife_city\n").unwrap();
///
/// assert_eq!(table.get_id("scripts_jubilife_city"), Some(1));
/// assert_eq!(table.get_name(0), Some("scripts_unk_0000"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ScriptTable {
    pub(crate) names: Vec<String>,
    pub(crate) name_to_id: FxHashMap<String, usize>,
}

impl ScriptTable {
    /// Creates an empty script table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads script names from a `scripts.order` file.
    ///
    /// Each line in the file becomes a script name, with its line number
    /// (0-indexed) as the file ID. Empty lines and lines starting with `#`
    /// are skipped.
    pub fn load_order_file(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.load_order_str(&content)
    }

    /// Loads script names from a string containing `scripts.order` content.
    ///
    /// Each line becomes a script name, with its line number (0-indexed)
    /// as the file ID. Empty lines and lines starting with `#` are skipped.
    pub fn load_order_str(&mut self, content: &str) -> std::io::Result<()> {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let name = line.to_string();
            self.name_to_id.insert(name.clone(), self.names.len());
            self.names.push(name);
        }
        Ok(())
    }

    /// Returns the script name for a given file ID.
    ///
    /// Returns `None` if the ID is out of range.
    pub fn get_name(&self, id: usize) -> Option<&str> {
        self.names.get(id).map(|s| s.as_str())
    }

    /// Returns the file ID for a given script name.
    ///
    /// Returns `None` if the name is not found.
    pub fn get_id(&self, name: &str) -> Option<usize> {
        self.name_to_id.get(name).copied()
    }

    /// Returns all script names in order by file ID.
    pub fn get_all_names(&self) -> &Vec<String> {
        &self.names
    }
}
