use crate::c_parser::{SourceManager, SymbolTable};
use crate::game::{Game, GameFamily};
use crate::provider::{Arm9Provider, DataProvider};
use crate::rom_header::RomHeader;
use crate::script_file::ScriptTable;
use crate::text_bank::TextBankTable;
use rustc_hash::FxHashMap;
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
    pub source_manager: SourceManager,
    script_to_text_cache: FxHashMap<u16, u16>,
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
        if self.project_type == ProjectType::Dspre {
            let mapname_bin = self
                .project_path
                .join("data/fielddata/maptable/mapname.bin");
            if mapname_bin.exists() {
                let data = std::fs::read(mapname_bin)?;
                let mut names = Vec::new();
                for chunk in data.chunks_exact(16) {
                    let name = String::from_utf8_lossy(chunk)
                        .trim_end_matches('\0')
                        .to_string();
                    names.push(name);
                }
                self.internal_names = Some(names);
            }
        } else {
            let maps_txt = self.project_path.join("generated/maps.txt");
            if maps_txt.exists() {
                let content = std::fs::read_to_string(maps_txt)?;
                let mut names = Vec::new();
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        let name = if let Some(pos) = line.find('=') {
                            line[..pos].trim()
                        } else {
                            line
                        };
                        names.push(name.to_string());
                    }
                }
                self.internal_names = Some(names);
            }
        }

        let location_text_id = match self.family {
            GameFamily::DP => 382,
            GameFamily::Platinum => 433,
            GameFamily::HGSS => 279,
        };

        if self.project_type == ProjectType::Dspre {
            let archive_path = self
                .project_path
                .join(format!("unpacked/textArchives/{:04}", location_text_id));
            if archive_path.exists() {
                let mut file = std::fs::File::open(archive_path)?;
                let charmap = chatot::get_default_charmap();
                if let Ok(archive) = chatot::decode_archive(charmap, &mut file, false) {
                    self.location_names = Some(archive.messages);
                }
            }
        } else {
            let location_json = self.project_path.join("res/text/location_names.json");
            if location_json.exists() {
                let content = std::fs::read_to_string(location_json)?;
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(messages) = json.get("messages").and_then(|m| m.as_array()) {
                        let names: Vec<String> = messages
                            .iter()
                            .map(|m| {
                                m.get("en_US")
                                    .or_else(|| m.get("ja_JP"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown")
                                    .to_string()
                            })
                            .collect();
                        self.location_names = Some(names);
                    }
                }
            }
        }

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
            source_manager: sm,
            script_to_text_cache: FxHashMap::default(),
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

        let include_dir = root.join("include/constants");
        if include_dir.exists() {
            symbols.load_headers_from_dir(include_dir)?;
        }

        let generated_dir = root.join("generated");
        if generated_dir.exists() {
            symbols.load_headers_from_dir(generated_dir)?;
        }

        let build_generated_dir = root.join("build/generated");
        if build_generated_dir.exists() {
            symbols.load_headers_from_dir(build_generated_dir)?;
        }

        let text_dir = root.join("res/text");
        if text_dir.exists() {
            symbols.load_headers_from_dir(text_dir)?;
        }

        let mut scripts = ScriptTable::new();

        let scripts_order = root.join("res/field/scripts/scripts.order");
        if scripts_order.exists() {
            scripts.load_order_file(scripts_order)?;
        }

        let mut text_banks = TextBankTable::new();
        let text_banks_list = root.join("generated/text_banks.txt");
        if text_banks_list.exists() {
            text_banks.load_list_file(text_banks_list)?;
        }

        let symbols = Arc::new(symbols);

        Ok(Self {
            project_path: root.clone(),
            project_type: ProjectType::Decomp,
            game,
            family,
            provider: Box::new(crate::provider::DecompProvider::new(root, (*symbols).clone())),
            symbols,
            scripts,
            text_banks,
            source_manager: sm,
            script_to_text_cache: FxHashMap::default(),
            location_names: None,
            internal_names: None,
        })
    }

    pub fn collect_constants_for_file(&self, path: impl AsRef<Path>) -> std::io::Result<SymbolTable> {
        let mut include_dirs = Vec::new();
        if self.project_type == ProjectType::Decomp {
            include_dirs.push(self.project_path.join("include"));
            include_dirs.push(self.project_path.join("res/field/scripts"));
        }

        let mut table = SymbolTable::with_parent(Arc::clone(&self.symbols));
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
            include_dirs.push(self.project_path.join("include"));
            include_dirs.push(self.project_path.join("res/field/scripts"));
        }

        let mut table = SymbolTable::with_parent(Arc::clone(&self.symbols));
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
