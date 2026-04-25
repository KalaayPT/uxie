use crate::c_parser::defines::{CDefine, CFunctionMacro, parse_defines, parse_function_macros};
use crate::c_parser::enums::{CEnum, parse_enums};
use crate::c_parser::includes::{CInclude, parse_includes};
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub defines: Vec<CDefine>,
    pub enums: Vec<CEnum>,
    pub function_macros: Vec<CFunctionMacro>,
    pub includes: Vec<CInclude>,
}

#[derive(Debug, Default, Clone)]
pub struct SourceManager {
    files: Arc<DashMap<PathBuf, Arc<FileEntry>>>,
    canonical_cache: Arc<DashMap<PathBuf, PathBuf>>,
}

impl SourceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_parse(&self, path: impl AsRef<Path>) -> std::io::Result<Arc<FileEntry>> {
        let path = path.as_ref();
        let canonical = self.canonicalize_strict(path)?;

        if let Some(entry) = self.files.get(&canonical) {
            return Ok(Arc::clone(&entry));
        }

        let bytes = std::fs::read(path).map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!(
                    "Failed to read source file {}: {err}",
                    path.display()
                ),
            )
        })?;
        let content = String::from_utf8_lossy(&bytes);
        let entry = Arc::new(FileEntry {
            defines: parse_defines(&content),
            enums: parse_enums(&content),
            function_macros: parse_function_macros(&content),
            includes: parse_includes(&content),
        });

        self.files.insert(canonical, Arc::clone(&entry));
        Ok(entry)
    }

    pub fn canonicalize(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        self.canonicalize_strict(path)
            .unwrap_or_else(|_| path.to_path_buf())
    }

    pub fn canonicalize_strict(&self, path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
        let path = path.as_ref();

        if let Some(cached) = self.canonical_cache.get(path) {
            return Ok(cached.clone());
        }

        let canonical = path.canonicalize()?;
        self.canonical_cache
            .insert(path.to_path_buf(), canonical.clone());
        Ok(canonical)
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_parser::SymbolTable;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_recursive_loading_with_cache() {
        let dir = tempdir().unwrap();
        let sm = SourceManager::new();

        let header_path = dir.path().join("consts.h");
        let mut header = std::fs::File::create(&header_path).unwrap();
        writeln!(header, "#define GLOBAL_CONST 100").unwrap();

        let script_path = dir.path().join("script.s");
        let mut script = std::fs::File::create(&script_path).unwrap();
        writeln!(script, "#include \"consts.h\"").unwrap();
        writeln!(script, "#define LOCAL_CONST 1").unwrap();

        let mut table = SymbolTable::with_source_manager(sm.clone());
        table.load_recursive(&script_path, &[]).unwrap();

        assert_eq!(table.resolve_constant("GLOBAL_CONST"), Some(100));
        assert_eq!(table.resolve_constant("LOCAL_CONST"), Some(1));
        assert_eq!(sm.len(), 2);

        let mut table2 = SymbolTable::with_source_manager(sm.clone());
        table2.load_recursive(&script_path, &[]).unwrap();
        assert_eq!(sm.len(), 2);
    }
}
