#[cfg(test)]
mod tests {
    use crate::c_parser::{SourceManager, SymbolTable};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn create_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "{}", content).unwrap();
        path
    }

    #[test]
    fn test_diamond_dependency_caching() {
        let dir = tempdir().unwrap();
        let sm = SourceManager::new();

        create_file(dir.path(), "d.h", "#define VAL_D 400");
        create_file(dir.path(), "b.h", "#include \"d.h\"\n#define VAL_B 200");
        create_file(dir.path(), "c.h", "#include \"d.h\"\n#define VAL_C 300");
        let a_path = create_file(
            dir.path(),
            "a.h",
            "#include \"b.h\"\n#include \"c.h\"\n#define VAL_A 100",
        );

        let mut table = SymbolTable::with_source_manager(sm.clone());
        table.load_recursive(&a_path, &[]).unwrap();

        assert_eq!(table.resolve_constant("VAL_A"), Some(100));
        assert_eq!(table.resolve_constant("VAL_B"), Some(200));
        assert_eq!(table.resolve_constant("VAL_C"), Some(300));
        assert_eq!(table.resolve_constant("VAL_D"), Some(400));

        assert_eq!(sm.len(), 4);
    }

    #[test]
    fn test_circular_dependency() {
        let dir = tempdir().unwrap();
        let sm = SourceManager::new();

        let a_path = create_file(dir.path(), "a.h", "#include \"b.h\"\n#define VAL_A 1");
        create_file(dir.path(), "b.h", "#include \"a.h\"\n#define VAL_B 2");

        let mut table = SymbolTable::with_source_manager(sm.clone());
        table.load_recursive(&a_path, &[]).unwrap();

        assert_eq!(table.resolve_constant("VAL_A"), Some(1));
        assert_eq!(table.resolve_constant("VAL_B"), Some(2));
    }

    #[test]
    fn test_include_path_priority() {
        let dir = tempdir().unwrap();
        let global_dir = dir.path().join("global");
        let local_dir = dir.path().join("local");
        let sm = SourceManager::new();

        create_file(&global_dir, "config.h", "#define CONF 1");
        create_file(&local_dir, "config.h", "#define CONF 2");

        let main_path = create_file(&local_dir, "main.h", "#include \"config.h\"");

        let mut table = SymbolTable::with_source_manager(sm.clone());
        table.load_recursive(&main_path, &[global_dir]).unwrap();

        assert_eq!(table.resolve_constant("CONF"), Some(2));
    }

    #[test]
    fn test_cross_file_expression_resolution() {
        let dir = tempdir().unwrap();
        let sm = SourceManager::new();

        create_file(dir.path(), "consts.h", "#define BASE 10");
        let main_path = create_file(
            dir.path(),
            "main.h",
            "#include \"consts.h\"\n#define DERIVED (BASE + 5)",
        );

        let mut table = SymbolTable::with_source_manager(sm.clone());
        table.load_recursive(&main_path, &[]).unwrap();

        assert_eq!(table.resolve_constant("DERIVED"), Some(15));
    }

    #[test]
    fn test_load_header_missing_file_returns_error() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("missing.h");
        let mut table = SymbolTable::new();

        let err = table.load_header(&missing).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_load_headers_from_dir_propagates_read_errors() {
        let dir = tempdir().unwrap();
        let invalid = dir.path().join("broken.h");
        std::fs::write(&invalid, [0xFF, 0xFE, 0x00, 0x01]).unwrap();

        let mut table = SymbolTable::new();
        let err = table.load_headers_from_dir(dir.path()).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }
}
