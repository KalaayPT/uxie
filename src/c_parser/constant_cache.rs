use crate::c_parser::defines::CFunctionMacro;
use crate::c_parser::symbol_table::{ConstantFamily, SymbolTable, SymbolTag};
use crate::game::GameFamily;
use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use xxhash_rust::xxh3::xxh3_64;

pub const CONSTANT_CACHE_VERSION: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq, Eq)]
pub struct ConstantCache {
    pub version: u32,
    pub uxie_version: String,
    pub game_family: String,
    pub file_hashes: HashMap<String, u64>,
    pub snapshot: SymbolSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq, Eq)]
pub struct SymbolSnapshot {
    pub symbols: HashMap<String, i64>,
    pub value_to_names: HashMap<i64, Vec<String>>,
    pub pending: HashMap<String, String>,
    pub function_macros: HashMap<String, CFunctionMacro>,
    pub symbol_to_tags: HashMap<String, HashSet<SymbolTag>>,
    pub symbol_to_family: HashMap<String, ConstantFamily>,
    pub family_value_to_name: HashMap<(ConstantFamily, i64), String>,
}

impl ConstantCache {
    pub fn from_symbols(
        project_root: &Path,
        game_family: GameFamily,
        input_files: &[PathBuf],
        symbols: &SymbolTable,
    ) -> io::Result<Self> {
        let mut file_hashes = HashMap::new();
        for path in input_files {
            let relative = relative_cache_path(project_root, path)?;
            file_hashes.insert(relative, xxh3_64(&std::fs::read(path)?));
        }

        Ok(Self {
            version: CONSTANT_CACHE_VERSION,
            uxie_version: env!("CARGO_PKG_VERSION").to_string(),
            game_family: game_family_key(game_family).to_string(),
            file_hashes,
            snapshot: symbols.to_snapshot(),
        })
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let bytes = std::fs::read(path)?;
        bitcode::decode(&bytes).map_err(|source| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Failed to decode constant cache {}: {source}",
                    path.display()
                ),
            )
        })
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = bitcode::encode(self);
        std::fs::write(path, bytes)
    }

    pub fn is_current(
        &self,
        project_root: &Path,
        game_family: GameFamily,
        input_files: &[PathBuf],
    ) -> io::Result<bool> {
        if self.version != CONSTANT_CACHE_VERSION
            || self.uxie_version != env!("CARGO_PKG_VERSION")
            || self.game_family != game_family_key(game_family)
        {
            return Ok(false);
        }

        if input_files.len() != self.file_hashes.len() {
            return Ok(false);
        }

        for path in input_files {
            let relative = relative_cache_path(project_root, path)?;
            let Some(expected_hash) = self.file_hashes.get(&relative) else {
                return Ok(false);
            };
            if !path.is_file() || xxh3_64(&std::fs::read(path)?) != *expected_hash {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

fn relative_cache_path(project_root: &Path, path: &Path) -> io::Result<String> {
    let relative = path.strip_prefix(project_root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Cached symbol file '{}' is outside project root '{}'",
                path.display(),
                project_root.display()
            ),
        )
    })?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn game_family_key(game_family: GameFamily) -> &'static str {
    match game_family {
        GameFamily::DP => "dp",
        GameFamily::Platinum => "platinum",
        GameFamily::HGSS => "hgss",
    }
}

#[cfg(test)]
mod tests {
    use super::{CONSTANT_CACHE_VERSION, ConstantCache};
    use crate::c_parser::defines::CFunctionMacro;
    use crate::c_parser::{ConstantFamily, SourceManager, SymbolSnapshot, SymbolTable, SymbolTag};
    use crate::game::GameFamily;
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn symbol_snapshot_round_trips_symbol_table_state() {
        let dir = tempdir().unwrap();
        let header = dir.path().join("test.h");
        fs::write(
            &header,
            "#define SPECIES_BULBASAUR 1\n#define OTHER SPECIES_BULBASAUR\n#define SCALE(x) ((x) * 2)\n",
        )
        .unwrap();

        let mut symbols = SymbolTable::with_source_manager(SourceManager::new());
        symbols.load_header(&header).unwrap();
        symbols
            .load_header_str_with_tag("#define MAP_CONST 7\n", &header, SymbolTag::Map(12))
            .unwrap();
        symbols.resolve_all();

        let snapshot = symbols.to_snapshot();
        let restored = SymbolTable::from_snapshot(&snapshot);

        assert_eq!(restored.resolve_constant("SPECIES_BULBASAUR"), Some(1));
        assert_eq!(restored.resolve_constant("OTHER"), Some(1));
        assert!(
            restored
                .get_symbols_by_tag(&SymbolTag::Map(12))
                .contains(&"MAP_CONST".to_string())
        );
        assert_eq!(
            restored.constant_family("SPECIES_BULBASAUR"),
            Some(ConstantFamily::Species)
        );
        assert_eq!(
            restored.to_snapshot().function_macros.get("SCALE").cloned(),
            Some(CFunctionMacro {
                name: "SCALE".to_string(),
                params: vec!["x".to_string()],
                value: "((x) * 2)".to_string(),
            })
        );
    }

    #[test]
    fn constant_cache_round_trips_and_uses_relative_paths() {
        let dir = tempdir().unwrap();
        let project_root = dir.path();
        let include_dir = project_root.join("include/constants");
        fs::create_dir_all(&include_dir).unwrap();
        let header = include_dir.join("test.h");
        fs::write(&header, "#define TEST_CONST 42\n").unwrap();

        let mut symbols = SymbolTable::with_source_manager(SourceManager::new());
        symbols
            .load_header_str_with_tag("#define SPECIES_BULBASAUR 1\n", &header, SymbolTag::Global)
            .unwrap();
        let cache = ConstantCache::from_symbols(
            project_root,
            GameFamily::Platinum,
            std::slice::from_ref(&header),
            &symbols,
        )
        .unwrap();
        let cache_path = project_root.join("cache/constants.bin");
        cache.save(&cache_path).unwrap();
        let loaded = ConstantCache::load(&cache_path).unwrap();

        assert_eq!(loaded.version, CONSTANT_CACHE_VERSION);
        assert!(loaded.file_hashes.contains_key("include/constants/test.h"));
        assert!(
            loaded
                .is_current(
                    project_root,
                    GameFamily::Platinum,
                    std::slice::from_ref(&header)
                )
                .unwrap()
        );
        assert_eq!(
            SymbolTable::from_snapshot(&loaded.snapshot).resolve_constant("SPECIES_BULBASAUR"),
            Some(1)
        );
        assert_eq!(
            SymbolTable::from_snapshot(&loaded.snapshot).constant_family("SPECIES_BULBASAUR"),
            Some(ConstantFamily::Species)
        );
    }

    #[test]
    fn constant_cache_detects_stale_or_corrupt_files() {
        let dir = tempdir().unwrap();
        let project_root = dir.path();
        let include_dir = project_root.join("include/constants");
        fs::create_dir_all(&include_dir).unwrap();
        let header = include_dir.join("test.h");
        fs::write(&header, "#define TEST_CONST 42\n").unwrap();

        let mut symbols = SymbolTable::with_source_manager(SourceManager::new());
        symbols.load_header(&header).unwrap();
        let cache = ConstantCache::from_symbols(
            project_root,
            GameFamily::Platinum,
            std::slice::from_ref(&header),
            &symbols,
        )
        .unwrap();
        let cache_path = project_root.join("cache/constants.bin");
        cache.save(&cache_path).unwrap();

        fs::write(&header, "#define TEST_CONST 43\n").unwrap();
        assert!(
            !ConstantCache::load(&cache_path)
                .unwrap()
                .is_current(
                    project_root,
                    GameFamily::Platinum,
                    std::slice::from_ref(&header)
                )
                .unwrap()
        );

        fs::write(&cache_path, b"not-bitcode").unwrap();
        assert!(ConstantCache::load(&cache_path).is_err());
    }

    #[test]
    fn constant_cache_metadata_mismatches_are_stale() {
        let dir = tempdir().unwrap();
        let project_root = dir.path();
        let include_dir = project_root.join("include/constants");
        fs::create_dir_all(&include_dir).unwrap();
        let header = include_dir.join("test.h");
        fs::write(&header, "#define TEST_CONST 42\n").unwrap();

        let mut symbols = SymbolTable::with_source_manager(SourceManager::new());
        symbols.load_header(&header).unwrap();
        let cache = ConstantCache::from_symbols(
            project_root,
            GameFamily::Platinum,
            std::slice::from_ref(&header),
            &symbols,
        )
        .unwrap();

        let mut wrong_version = cache.clone();
        wrong_version.version += 1;
        assert!(
            !wrong_version
                .is_current(
                    project_root,
                    GameFamily::Platinum,
                    std::slice::from_ref(&header)
                )
                .unwrap()
        );

        let mut wrong_uxie_version = cache.clone();
        wrong_uxie_version.uxie_version.push_str("-mutated");
        assert!(
            !wrong_uxie_version
                .is_current(
                    project_root,
                    GameFamily::Platinum,
                    std::slice::from_ref(&header)
                )
                .unwrap()
        );

        assert!(
            !cache
                .is_current(
                    project_root,
                    GameFamily::HGSS,
                    std::slice::from_ref(&header)
                )
                .unwrap()
        );
    }

    #[test]
    fn symbol_snapshot_supports_manual_round_trip_shape() {
        let snapshot = SymbolSnapshot {
            symbols: HashMap::from([("VALUE".to_string(), 5)]),
            value_to_names: HashMap::from([(5, vec!["VALUE".to_string()])]),
            pending: HashMap::from([("VALUE".to_string(), "5".to_string())]),
            function_macros: HashMap::new(),
            symbol_to_tags: HashMap::from([(
                "VALUE".to_string(),
                HashSet::from([SymbolTag::Global]),
            )]),
            symbol_to_family: HashMap::from([("VALUE".to_string(), ConstantFamily::Item)]),
            family_value_to_name: HashMap::from([((ConstantFamily::Item, 5), "VALUE".to_string())]),
        };

        assert_eq!(
            SymbolTable::from_snapshot(&snapshot)
                .to_snapshot()
                .symbols
                .get("VALUE")
                .copied(),
            Some(5)
        );
        assert_eq!(
            SymbolTable::from_snapshot(&snapshot).constant_family("VALUE"),
            Some(ConstantFamily::Item)
        );
    }
}
