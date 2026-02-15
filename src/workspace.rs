//! High-level workspace API for managing ROM hacking projects
//!
//! The [`Workspace`] struct provides a unified interface for working with both
//! DSPRE projects and decompilation sources (pokeplatinum/pokeheartgold).

use crate::c_parser::{SourceManager, SymbolTable};
use crate::game::{Game, GameFamily};
use crate::provider::{Arm9Provider, DataProvider};
use crate::rom_header::RomHeader;
use crate::script_file::{
    GlobalScriptTable, MapScriptInfo, ScriptResolution, ScriptTable, is_common_script_id,
};
use crate::text_bank::{GameStrings, TextBankTable};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Dspre,
    Decomp,
}

pub struct Workspace {
    pub project_path: PathBuf,
    pub project_type: ProjectType,
    pub game: Game,
    pub family: GameFamily,
    pub provider: Box<dyn DataProvider>,
    pub symbols: Arc<SymbolTable>,
    pub scripts: ScriptTable,
    pub text_banks: TextBankTable,
    pub game_strings: GameStrings,
    pub global_script_table: GlobalScriptTable,
    pub source_manager: SourceManager,
    location_names: Option<Vec<String>>,
    internal_names: Option<Vec<String>>,
}

impl Workspace {
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();

        let mut ws =
            if path.join("include/constants").exists() || path.join("res/field/scripts").exists() {
                Self::open_decomp(path)?
            } else {
                Self::open_dspre(path)?
            };

        ws.load_names()?;
        Ok(ws)
    }

    fn load_names(&mut self) -> std::io::Result<()> {
        let internal_names = match self.project_type {
            ProjectType::Dspre => {
                let mapname_bin = self
                    .project_path
                    .join("data/fielddata/maptable/mapname.bin");
                if mapname_bin.exists() {
                    let data = std::fs::read(mapname_bin)?;
                    Some(
                        data.chunks_exact(16)
                            .map(|chunk| {
                                String::from_utf8_lossy(chunk)
                                    .trim_end_matches('\0')
                                    .to_string()
                            })
                            .collect(),
                    )
                } else {
                    None
                }
            }
            ProjectType::Decomp => {
                let maps_txt = self.project_path.join("generated/maps.txt");
                if maps_txt.exists() {
                    let content = std::fs::read_to_string(maps_txt)?;
                    Some(
                        content
                            .lines()
                            .map(str::trim)
                            .filter(|line| !line.is_empty() && !line.starts_with('#'))
                            .map(|line| {
                                if let Some(pos) = line.find('=') {
                                    line[..pos].trim().to_string()
                                } else {
                                    line.to_string()
                                }
                            })
                            .collect(),
                    )
                } else {
                    None
                }
            }
        };

        self.internal_names = internal_names;

        let location_text_id = match self.family {
            GameFamily::DP => 382,
            GameFamily::Platinum => 433,
            GameFamily::HGSS => 279,
        };

        let location_names = match self.project_type {
            ProjectType::Dspre => {
                let archive_path = self
                    .project_path
                    .join(format!("unpacked/textArchives/{:04}", location_text_id));
                if archive_path.exists() {
                    let mut file = std::fs::File::open(&archive_path)?;
                    let charmap = chatot::get_default_charmap();
                    let archive =
                        chatot::decode_archive(charmap, &mut file, false).map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!(
                                    "Failed to decode location names archive {}: {e}",
                                    archive_path.display()
                                ),
                            )
                        })?;
                    Some(archive.messages)
                } else {
                    None
                }
            }
            ProjectType::Decomp => {
                let location_json = self.project_path.join("res/text/location_names.json");
                if location_json.exists() {
                    let content = std::fs::read_to_string(&location_json)?;
                    let json =
                        serde_json::from_str::<serde_json::Value>(&content).map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!(
                                    "Failed to parse location names JSON {}: {e}",
                                    location_json.display()
                                ),
                            )
                        })?;

                    let messages =
                        json.get("messages")
                            .and_then(|m| m.as_array())
                            .ok_or_else(|| {
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!(
                                        "Location names JSON {} is missing a 'messages' array",
                                        location_json.display()
                                    ),
                                )
                            })?;

                    Some(
                        messages
                            .iter()
                            .map(|m| {
                                m.get("en_US")
                                    .or_else(|| m.get("ja_JP"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown")
                                    .to_string()
                            })
                            .collect(),
                    )
                } else {
                    None
                }
            }
        };

        self.location_names = location_names;
        Ok(())
    }

    pub fn get_map_internal_name(&self, id: u16) -> Option<String> {
        self.internal_names.as_ref()?.get(id as usize).cloned()
    }

    pub fn get_map_location_name(&self, location_id: u8) -> Option<String> {
        self.location_names
            .as_ref()?
            .get(location_id as usize)
            .cloned()
    }

    pub fn get_script_file_for_map(&self, map_id: u16) -> Option<String> {
        let header = self.provider.get_map_header(map_id).ok()?;
        self.scripts
            .get_name(header.script_file_id() as usize)
            .map(str::to_string)
    }

    pub fn get_symbols_for_map(&self, map_id: u16) -> Vec<String> {
        use crate::c_parser::SymbolTag;
        self.symbols.get_symbols_by_tag(&SymbolTag::Map(map_id))
    }

    pub fn resolve_script_id_to_name(&self, script_id: u16) -> Option<String> {
        if is_common_script_id(script_id) {
            self.symbols.resolve_name(script_id as i64, "CommonScript_")
        } else {
            self.symbols.resolve_name(script_id as i64, "")
        }
    }

    pub fn resolve_script(
        &self,
        script_id: u16,
        map_id: Option<u16>,
    ) -> crate::error::Result<Option<ScriptResolution>> {
        crate::script_file::resolve_script_id(
            script_id,
            map_id,
            &self.global_script_table,
            self.provider.as_ref(),
        )
    }

    pub fn resolve_level_script(
        &self,
        map_id: u16,
    ) -> crate::error::Result<Option<ScriptResolution>> {
        crate::script_file::resolve_level_script(
            map_id,
            &self.global_script_table,
            self.provider.as_ref(),
        )
    }

    pub fn get_map_script_info(&self, map_id: u16) -> crate::error::Result<MapScriptInfo> {
        crate::script_file::get_script_file_info_for_map(map_id, self.provider.as_ref())
    }

    fn open_dspre(path: PathBuf) -> std::io::Result<Self> {
        let header = RomHeader::open(&path)?;
        let game = header.detect_game().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Could not detect game from ROM header",
            )
        })?;
        let family = game.family();

        let arm9_path = if path.join("arm9.bin").exists() {
            path.join("arm9.bin")
        } else {
            path.join("unpacked/arm9.bin")
        };

        if !arm9_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("arm9.bin not found in DSPRE project at {}", path.display()),
            ));
        }

        let (offset, count) = match (family, header.game_code.chars().nth(3)) {
            (GameFamily::DP, _) => (0xE4B24, 559),
            (GameFamily::Platinum, _) => (0xE601C, 559),
            (GameFamily::HGSS, _) => (0xF6BE0, 540),
        };

        let game_strings = GameStrings::load_from_dspre(&path, family, header.detect_language())?;

        let global_script_table = match family {
            GameFamily::HGSS => GlobalScriptTable::from_hgss_binary_file(&arm9_path)?,
            GameFamily::Platinum | GameFamily::DP => GlobalScriptTable::platinum_hardcoded(),
        };

        let sm = SourceManager::new();
        Ok(Self {
            project_path: path,
            project_type: ProjectType::Dspre,
            game,
            family,
            provider: Box::new(Arm9Provider::new(arm9_path, offset, count, family)),
            symbols: Arc::new(SymbolTable::with_source_manager(sm.clone())),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings,
            global_script_table,
            source_manager: sm,
            location_names: None,
            internal_names: None,
        })
    }

    pub fn open_decomp(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();

        let (game, family) = if root.to_string_lossy().contains("pokeheartgold")
            || root.to_string_lossy().contains("pokesoulsilver")
        {
            (Game::HeartGold, GameFamily::HGSS)
        } else {
            (Game::Platinum, GameFamily::Platinum)
        };

        let sm = SourceManager::new();
        let mut symbols = SymbolTable::with_source_manager(sm.clone());

        // Broad recursive parse of the entire project
        Self::load_project_symbols_broad(&root, &mut symbols)?;

        let mut scripts = ScriptTable::new();
        let scripts_order = root.join("res/field/scripts/scripts.order");
        if scripts_order.exists() {
            scripts.load_order_file(scripts_order)?;
        } else if family == GameFamily::HGSS {
            let hgss_script_dirs = [
                root.join("files/fielddata/script/scr_seq"),
                root.join("files/fielddata/script"),
            ];
            for script_dir in hgss_script_dirs {
                if script_dir.exists() {
                    scripts.load_hgss_script_dir(script_dir)?;
                }
            }
        }

        let mut text_banks = TextBankTable::new();
        let text_banks_list = root.join("generated/text_banks.txt");
        if text_banks_list.exists() {
            text_banks.load_list_file(text_banks_list)?;
        }

        symbols.resolve_all();
        let symbols = Arc::new(symbols);

        let global_script_table = match family {
            GameFamily::HGSS => {
                let fieldmap_path = root.join("src/fieldmap.c");
                if fieldmap_path.exists() {
                    GlobalScriptTable::from_hgss_decomp_file(&fieldmap_path, &symbols)?
                } else {
                    GlobalScriptTable::new()
                }
            }
            GameFamily::Platinum | GameFamily::DP => {
                let script_manager_path = root.join("src/script_manager.c");
                if script_manager_path.exists() {
                    GlobalScriptTable::from_platinum_decomp_file(&script_manager_path, &symbols)?
                } else {
                    GlobalScriptTable::platinum_hardcoded()
                }
            }
        };

        Ok(Self {
            project_path: root.clone(),
            project_type: ProjectType::Decomp,
            game,
            family,
            provider: Box::new(crate::provider::DecompProvider::new(
                root,
                (*symbols).clone(),
                family,
            )),
            symbols,
            scripts,
            text_banks,
            game_strings: GameStrings::new(),
            global_script_table,
            source_manager: sm,
            location_names: None,
            internal_names: None,
        })
    }

    fn load_project_symbols_broad(root: &Path, symbols: &mut SymbolTable) -> std::io::Result<()> {
        // 1. Load all constants from include/constants
        let include_constants = root.join("include/constants");
        if include_constants.exists() {
            symbols.load_headers_from_dir(&include_constants)?;
        }

        // 2. Load generated constants (prefer build/generated/*.py, fallback to generated/*.txt)
        let build_generated = root.join("build/generated");
        let generated = root.join("generated");

        if build_generated.exists() {
            symbols.load_headers_from_dir(&build_generated)?;
        } else if generated.exists() {
            symbols.load_headers_from_dir(&generated)?;
        }

        // Special handling for map events/scripts to apply tags
        let field_events = root.join("res/field/events");
        if field_events.exists() {
            symbols.load_headers_from_dir(&field_events)?;
        }

        let text_dir = root.join("res/text");
        if text_dir.exists() {
            symbols.load_headers_from_dir(&text_dir)?;
        }

        // 6. Load extra generated headers from build/ if they exist
        let build_text_bank = root.join("build/res/text/bank");
        if build_text_bank.exists() {
            symbols.load_headers_from_dir(&build_text_bank)?;
        }

        // NOTE: We intentionally do NOT load build/res/field/events headers here.
        // Those contain per-map LOCALID_* definitions that conflict across maps.
        // Each script should load its own events header via #include resolution
        // in collect_constants_for_file() or collect_constants_for_source().

        Ok(())
    }

    pub fn collect_constants_for_file(
        &self,
        path: impl AsRef<Path>,
    ) -> std::io::Result<SymbolTable> {
        let mut include_dirs = Vec::new();
        if self.project_type == ProjectType::Decomp {
            include_dirs.push(self.project_path.clone());
            include_dirs.push(self.project_path.join("include"));
            include_dirs.push(self.project_path.join("res/field/scripts"));
        }

        let mut table = SymbolTable::with_parent(self.symbols.clone());
        table.load_recursive(path, &include_dirs)?;
        Ok(table)
    }

    pub fn collect_constants_for_source(
        &self,
        source: &str,
        current_file_dir: impl AsRef<Path>,
    ) -> std::io::Result<SymbolTable> {
        let mut include_dirs = Vec::new();
        if self.project_type == ProjectType::Decomp {
            include_dirs.push(self.project_path.clone());
            include_dirs.push(self.project_path.join("include"));
            include_dirs.push(self.project_path.join("res/field/scripts"));
        }

        let mut table = SymbolTable::with_parent(self.symbols.clone());
        table.load_recursive_str(source, current_file_dir, &include_dirs)?;
        Ok(table)
    }

    pub fn resolve_constant(&self, name: &str) -> Option<i64> {
        self.symbols.resolve_constant(name)
    }

    pub fn resolve_name(&self, value: i64, prefix: &str) -> Option<String> {
        self.symbols.resolve_name(value, prefix)
    }

    pub fn resolve_names(&self, value: i64, prefixes: &[&str]) -> Vec<String> {
        self.symbols.resolve_names(value, prefixes)
    }

    pub fn resolve_script_symbols(&self, script: &str) -> String {
        let mut result = String::with_capacity(script.len());
        let mut start_idx = 0;

        for (i, c) in script.char_indices() {
            let is_token_char = c.is_alphanumeric() || c == '_';

            if !is_token_char {
                if i > start_idx {
                    let token = &script[start_idx..i];
                    self.append_resolved_token(&mut result, token);
                }
                result.push(c);
                start_idx = i + c.len_utf8();
            }
        }

        if start_idx < script.len() {
            let token = &script[start_idx..];
            self.append_resolved_token(&mut result, token);
        }

        result
    }

    fn append_resolved_token(&self, result: &mut String, token: &str) {
        if let Some(val) = self.resolve_constant(token) {
            result.push_str(&val.to_string());
        } else if let Ok(val) = token.parse::<i64>() {
            if let Some(name) = self.resolve_name(val, "") {
                result.push_str(&name);
            } else {
                result.push_str(token);
            }
        } else {
            result.push_str(token);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_project_type_equality() {
        assert_eq!(ProjectType::Dspre, ProjectType::Dspre);
        assert_eq!(ProjectType::Decomp, ProjectType::Decomp);
        assert_ne!(ProjectType::Dspre, ProjectType::Decomp);
    }

    #[test]
    fn test_resolve_script_symbols_with_constants() {
        let sm = SourceManager::new();
        let mut symbols = SymbolTable::with_source_manager(sm.clone());
        symbols.insert_define("FLAG_START".to_string(), 100);
        symbols.insert_define("VAR_TEMP".to_string(), 16384);

        let ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(MockProvider),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
        };

        let result = ws.resolve_script_symbols("SetFlag FLAG_START");
        assert_eq!(result, "SetFlag 100");

        let result = ws.resolve_script_symbols("GetVar VAR_TEMP + FLAG_START");
        assert_eq!(result, "GetVar 16384 + 100");
    }

    #[test]
    fn test_resolve_script_symbols_numeric_to_name() {
        let sm = SourceManager::new();
        let mut symbols = SymbolTable::with_source_manager(sm.clone());
        symbols.insert_define("FLAG_TEST".to_string(), 42);

        let ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(MockProvider),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
        };

        let result = ws.resolve_script_symbols("SetFlag 42");
        assert_eq!(result, "SetFlag FLAG_TEST");
    }

    #[test]
    fn test_resolve_script_symbols_passthrough() {
        let sm = SourceManager::new();
        let symbols = SymbolTable::with_source_manager(sm.clone());

        let ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(MockProvider),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
        };

        let result = ws.resolve_script_symbols("UnknownCmd UNKNOWN_FLAG");
        assert_eq!(result, "UnknownCmd UNKNOWN_FLAG");
    }

    #[test]
    fn test_get_map_internal_name() {
        let sm = SourceManager::new();
        let symbols = SymbolTable::with_source_manager(sm.clone());

        let mut ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(MockProvider),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: Some(vec!["D01R0101".to_string(), "D02R0102".to_string()]),
        };

        assert_eq!(ws.get_map_internal_name(0), Some("D01R0101".to_string()));
        assert_eq!(ws.get_map_internal_name(1), Some("D02R0102".to_string()));
        assert_eq!(ws.get_map_internal_name(99), None);

        ws.internal_names = None;
        assert_eq!(ws.get_map_internal_name(0), None);
    }

    #[test]
    fn test_get_map_location_name() {
        let sm = SourceManager::new();
        let symbols = SymbolTable::with_source_manager(sm.clone());

        let mut ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(MockProvider),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: Some(vec![
                "Twinleaf Town".to_string(),
                "Sandgem Town".to_string(),
            ]),
            internal_names: None,
        };

        assert_eq!(
            ws.get_map_location_name(0),
            Some("Twinleaf Town".to_string())
        );
        assert_eq!(
            ws.get_map_location_name(1),
            Some("Sandgem Town".to_string())
        );
        assert_eq!(ws.get_map_location_name(99), None);

        ws.location_names = None;
        assert_eq!(ws.get_map_location_name(0), None);
    }

    #[test]
    fn test_resolve_constant() {
        let sm = SourceManager::new();
        let mut symbols = SymbolTable::with_source_manager(sm.clone());
        symbols.insert_define("TEST_CONST".to_string(), 12345);

        let ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(MockProvider),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
        };

        assert_eq!(ws.resolve_constant("TEST_CONST"), Some(12345));
        assert_eq!(ws.resolve_constant("NONEXISTENT"), None);
    }

    #[test]
    fn test_get_script_file_for_map_hgss_decomp_missing_name_returns_none() {
        let sm = SourceManager::new();
        let symbols = SymbolTable::with_source_manager(sm.clone());

        let ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            game: Game::HeartGold,
            family: GameFamily::HGSS,
            provider: Box::new(HgssScriptProvider { script_file_id: 81 }),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
        };

        assert_eq!(ws.get_script_file_for_map(0), None);
    }

    #[test]
    fn test_open_decomp_detection() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("include/constants")).unwrap();
        let mut const_file = fs::File::create(root.join("include/constants/test.h")).unwrap();
        writeln!(const_file, "#define TEST_VALUE 42").unwrap();

        let ws = Workspace::open(root).unwrap();

        assert_eq!(ws.project_type, ProjectType::Decomp);
        assert_eq!(ws.game, Game::Platinum);
        assert_eq!(ws.family, GameFamily::Platinum);
        assert_eq!(ws.resolve_constant("TEST_VALUE"), Some(42));
    }

    #[test]
    fn test_open_decomp_with_scripts_order() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("res/field/scripts")).unwrap();

        let mut order_file =
            fs::File::create(root.join("res/field/scripts/scripts.order")).unwrap();
        writeln!(order_file, "script_main").unwrap();
        writeln!(order_file, "script_event").unwrap();

        let ws = Workspace::open(root).unwrap();

        assert_eq!(ws.scripts.get_name(0), Some("script_main"));
        assert_eq!(ws.scripts.get_name(1), Some("script_event"));
    }

    #[test]
    fn test_open_decomp_hgss_loads_id_named_script_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("pokeheartgold");

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("files/fielddata/script/scr_seq")).unwrap();
        fs::write(
            root.join("files/fielddata/script/scr_seq/scr_seq_0003_D01R0101.s"),
            "",
        )
        .unwrap();
        fs::write(
            root.join("files/fielddata/script/scr_seq/scr_seq_0081_D32R0102.s"),
            "",
        )
        .unwrap();

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.family, GameFamily::HGSS);
        assert_eq!(ws.scripts.get_name(3), Some("scr_seq_0003_D01R0101"));
        assert_eq!(ws.scripts.get_name(81), Some("scr_seq_0081_D32R0102"));
        assert_eq!(ws.scripts.get_name(4), None);
    }

    #[test]
    fn test_open_decomp_missing_script_manager_uses_hardcoded_global_script_table() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("include/constants")).unwrap();

        let ws = Workspace::open(root).unwrap();
        let hardcoded = GlobalScriptTable::platinum_hardcoded();

        assert_eq!(ws.global_script_table.len(), hardcoded.len());
        assert_eq!(ws.global_script_table.lookup(2000), hardcoded.lookup(2000));
    }

    #[test]
    fn test_open_decomp_invalid_script_manager_returns_invalid_data() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();

        let mut script_manager = fs::File::create(root.join("src/script_manager.c")).unwrap();
        writeln!(script_manager, "not a SCRIPT_RANGE_TABLE file").unwrap();

        match Workspace::open(root) {
            Ok(_) => panic!("Expected open to fail for invalid script_manager.c"),
            Err(err) => {
                assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
                assert!(err.to_string().contains("SCRIPT_RANGE_TABLE"));
            }
        }
    }

    struct MockProvider;

    struct HgssScriptProvider {
        script_file_id: u16,
    }

    impl DataProvider for MockProvider {
        fn get_map_header(&self, _id: u16) -> crate::error::Result<crate::map_header::MapHeader> {
            Err(crate::error::UxieError::not_found("Map header", "mock"))
        }

        fn get_map_header_count(&self) -> crate::error::Result<usize> {
            Ok(0)
        }

        fn get_text_archive_for_script_file(
            &self,
            _script_file_id: u16,
        ) -> crate::error::Result<Option<u16>> {
            Ok(None)
        }

        fn find_map_by_script_file_id(
            &self,
            _script_file_id: u16,
        ) -> crate::error::Result<Option<u16>> {
            Ok(None)
        }

        fn find_maps_by_script_file_id(
            &self,
            _script_file_id: u16,
        ) -> crate::error::Result<Vec<u16>> {
            Ok(Vec::new())
        }

        fn find_map_by_level_script_file_id(
            &self,
            _level_script_file_id: u16,
        ) -> crate::error::Result<Option<u16>> {
            Ok(None)
        }

        fn find_maps_by_level_script_file_id(
            &self,
            _level_script_file_id: u16,
        ) -> crate::error::Result<Vec<u16>> {
            Ok(Vec::new())
        }
    }

    impl DataProvider for HgssScriptProvider {
        fn get_map_header(&self, id: u16) -> crate::error::Result<crate::map_header::MapHeader> {
            if id != 0 {
                return Err(crate::error::UxieError::not_found(
                    "Map header",
                    id.to_string(),
                ));
            }

            let header = crate::map_header::MapHeaderHGSS {
                script_file_id: self.script_file_id,
                ..crate::map_header::MapHeaderHGSS::default()
            };

            Ok(crate::map_header::MapHeader::HGSS(header))
        }

        fn get_map_header_count(&self) -> crate::error::Result<usize> {
            Ok(1)
        }

        fn get_text_archive_for_script_file(
            &self,
            _script_file_id: u16,
        ) -> crate::error::Result<Option<u16>> {
            Ok(None)
        }

        fn find_maps_by_script_file_id(
            &self,
            script_file_id: u16,
        ) -> crate::error::Result<Vec<u16>> {
            if script_file_id == self.script_file_id {
                Ok(vec![0])
            } else {
                Ok(Vec::new())
            }
        }

        fn find_maps_by_level_script_file_id(
            &self,
            _level_script_file_id: u16,
        ) -> crate::error::Result<Vec<u16>> {
            Ok(Vec::new())
        }
    }
}
