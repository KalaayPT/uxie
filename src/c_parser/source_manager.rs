use crate::c_parser::defines::{CDefine, CFunctionMacro, parse_defines, parse_function_macros};
use crate::c_parser::enums::{CEnum, parse_enums};
use crate::c_parser::includes::{CInclude, parse_includes};
use dashmap::DashMap;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

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
    metang_types: Arc<DashMap<PathBuf, Arc<HashMap<String, bool>>>>,
}

static RE_METANG_MASK_TYPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"'(?P<name>[A-Za-z0-9_]+)'\s*:\s*\{\s*'type'\s*:\s*'(?P<kind>enum|mask)'").unwrap()
});

/// Parse a metang `meson.build` into `list stem -> is_mask`.
///
/// A missing file yields an empty map: directories that are not metang output
/// simply declare no list types.
pub fn parse_metang_types(meson_path: &Path) -> std::io::Result<HashMap<String, bool>> {
    if !meson_path.is_file() {
        return Ok(HashMap::new());
    }
    let meson = std::fs::read_to_string(meson_path).map_err(|err| {
        std::io::Error::new(
            err.kind(),
            format!(
                "Failed to read metang metadata {} as UTF-8: {err}",
                meson_path.display()
            ),
        )
    })?;

    let mut types = HashMap::new();
    for caps in RE_METANG_MASK_TYPE.captures_iter(&meson) {
        let (Some(name), Some(kind)) = (caps.name("name"), caps.name("kind")) else {
            continue;
        };
        // First entry wins, matching a first-match scan of the file.
        types
            .entry(name.as_str().to_owned())
            .or_insert(kind.as_str() == "mask");
    }
    Ok(types)
}

impl SourceManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Metang list types declared by `meson_path`, parsed once per manager.
    ///
    /// Decomp projects consult the same `generated/meson.build` for every list
    /// file they load, so the parsed map is memoized alongside parsed sources.
    pub fn metang_types(&self, meson_path: &Path) -> std::io::Result<Arc<HashMap<String, bool>>> {
        if let Some(types) = self.metang_types.get(meson_path) {
            return Ok(Arc::clone(&types));
        }

        let types = Arc::new(parse_metang_types(meson_path)?);
        self.metang_types
            .insert(meson_path.to_path_buf(), Arc::clone(&types));
        Ok(types)
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
                format!("Failed to read source file {}: {err}", path.display()),
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
