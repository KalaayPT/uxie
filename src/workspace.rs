use crate::c_parser::SymbolTable;
use crate::game::{Game, GameFamily};
use crate::provider::{Arm9Provider, DataProvider};
use crate::rom_header::RomHeader;
use crate::script_file::ScriptTable;
use crate::text_bank::TextBankTable;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    pub symbols: SymbolTable,
    pub scripts: ScriptTable,
    pub text_banks: TextBankTable,
    script_to_text_cache: HashMap<u16, u16>,
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
                if let Ok(archive) = crate::text_bank::TextArchive::from_binary(&mut file) {
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

        Ok(Self {
            project_path: path,
            project_type: ProjectType::Dspre,
            game,
            family,
            provider: Box::new(Arm9Provider::new(arm9_path, offset, count, family)),
            symbols: SymbolTable::new(),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            script_to_text_cache: HashMap::new(),
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

        let mut symbols = SymbolTable::new();

        let include_dir = root.join("include/constants");
        if include_dir.exists() {
            symbols.load_headers_from_dir(include_dir)?;
        }

        let generated_dir = root.join("generated");
        if generated_dir.exists() {
            symbols.load_headers_from_dir(generated_dir.clone())?;
        }

        let build_dir = root.join("build");
        if build_dir.exists() {
            symbols.load_headers_from_dir(build_dir)?;
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

        Ok(Self {
            project_path: root.clone(),
            project_type: ProjectType::Decomp,
            game,
            family,
            provider: Box::new(crate::provider::DecompProvider::new(root, symbols.clone())),
            symbols,
            scripts,
            text_banks,
            script_to_text_cache: HashMap::new(),
            location_names: None,
            internal_names: None,
        })
    }

    pub fn new(
        provider: Box<dyn DataProvider>,
        symbols: SymbolTable,
        scripts: ScriptTable,
        text_banks: TextBankTable,
    ) -> Self {
        Self {
            project_path: PathBuf::new(),
            project_type: ProjectType::Dspre,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider,
            symbols,
            scripts,
            text_banks,
            script_to_text_cache: HashMap::new(),
            location_names: None,
            internal_names: None,
        }
    }

    pub fn from_arm9(
        arm9_path: impl AsRef<Path>,
        offset: u64,
        count: usize,
        family: GameFamily,
        headers_dir: Option<impl AsRef<Path>>,
    ) -> std::io::Result<Self> {
        let mut symbols = SymbolTable::new();
        if let Some(dir) = headers_dir {
            symbols.load_headers_from_dir(dir)?;
        }

        Ok(Self {
            project_path: arm9_path
                .as_ref()
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf(),
            project_type: ProjectType::Dspre,
            game: Game::Platinum,
            family,
            provider: Box::new(Arm9Provider::new(arm9_path, offset, count, family)),
            symbols,
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            script_to_text_cache: HashMap::new(),
            location_names: None,
            internal_names: None,
        })
    }

    pub fn load_scripts_order(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        self.scripts.load_order_file(path)
    }

    pub fn load_text_banks_list(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        self.text_banks.load_list_file(path)
    }

    pub fn get_text_bank_name_for_script(
        &mut self,
        script_id: u16,
    ) -> std::io::Result<Option<String>> {
        if let Some(&text_id) = self.script_to_text_cache.get(&script_id) {
            return Ok(self.text_banks.get_name(text_id as usize).cloned());
        }

        if let Some(text_id) = self.provider.get_text_archive_for_script(script_id)? {
            self.script_to_text_cache.insert(script_id, text_id);
            return Ok(self.text_banks.get_name(text_id as usize).cloned());
        }
        Ok(None)
    }

    pub fn get_script_name(&self, script_id: u16) -> Option<String> {
        self.scripts.get_name(script_id as usize).cloned()
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
        let mut current_token = String::new();

        for c in script.chars() {
            if c.is_alphanumeric() || c == '_' {
                current_token.push(c);
            } else {
                if !current_token.is_empty() {
                    if let Some(val) = self.resolve_constant(&current_token) {
                        result.push_str(&val.to_string());
                    } else if let Ok(val) = current_token.parse::<i64>() {
                        let matches = self.resolve_names(val, &[]);
                        if !matches.is_empty() {
                            result.push_str(&matches[0]);
                        } else {
                            result.push_str(&current_token);
                        }
                    } else {
                        result.push_str(&current_token);
                    }
                    current_token.clear();
                }
                result.push(c);
            }
        }

        if !current_token.is_empty() {
            if let Some(val) = self.resolve_constant(&current_token) {
                result.push_str(&val.to_string());
            } else if let Ok(val) = current_token.parse::<i64>() {
                let matches = self.resolve_names(val, &[]);
                if !matches.is_empty() {
                    result.push_str(&matches[0]);
                } else {
                    result.push_str(&current_token);
                }
            } else {
                result.push_str(&current_token);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn integration_workspace_open_decomp() {
        let decomp_path = "/home/kalaay/dev/pokeplatinum";
        if !std::path::Path::new(decomp_path).exists() {
            return;
        }

        let workspace = Workspace::open_decomp(decomp_path).unwrap();

        assert_eq!(
            workspace.get_script_name(2),
            Some("scripts_jubilife_city".to_string())
        );
        assert_eq!(workspace.resolve_constant("VARS_START"), Some(16384));

        let mut workspace = workspace;
        if let Ok(Some(name)) = workspace.get_text_bank_name_for_script(2) {
            assert_eq!(name, "TEXT_BANK_JUBILIFE_CITY");
        }

        let script = "SetFlag FLAG_UNK_0x000A";
        let resolved = workspace.resolve_script_symbols(script);
        assert_eq!(resolved, "SetFlag 10");
    }
}
