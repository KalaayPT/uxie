//! ROM header reading and parsing for Nintendo DS Pokemon Gen 4 games
//!
//! This module provides functionality to read and parse ROM headers from various sources:
//! - Binary ROM files (NDS ROM dumps)
//! - DSPRE project directories
//! - ds-rom-tool YAML configurations
//! - Decompilation project configurations
//!
//! The [`RomHeader`] struct automatically detects the source format and provides
//! a unified interface for accessing header data.

use crate::game::{Game, GameFamily, GameLanguage};
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// ROM header size in bytes (standard NDS header size)
pub const ROM_HEADER_SIZE: usize = 0x200;

/// Source format of the ROM header
///
/// Indicates where the header data was loaded from, which affects
/// which fields are available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RomHeaderSource {
    /// Loaded from a binary ROM file or header.bin (all fields available)
    #[default]
    NdsTool,
    /// Loaded from ds-rom-tool YAML format (limited fields)
    DsRomTool,
    /// Loaded from decompilation project config (limited fields)
    Decomp,
}

/// Nintendo DS ROM header
///
/// Contains metadata about a Pokemon Gen 4 ROM, including game identification,
/// ARM9/ARM7 binary locations, and file system table offsets.
///
/// # Examples
///
/// ```rust,no_run
/// use uxie::RomHeader;
///
/// // Auto-detect format from any source
/// let header = RomHeader::open("path/to/header.bin")?;
///
/// // Detect game version
/// if let Some(game) = header.detect_game() {
///     println!("Detected game: {:?}", game);
/// }
///
/// // Get region
/// if let Some(region) = header.region() {
///     println!("Region: {}", region);
/// }
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RomHeader {
    pub game_title: String,
    pub game_code: String,
    pub maker_code: String,
    pub unit_code: u8,
    pub rom_version: u8,
    pub secure_area_delay: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm9_rom_offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm9_entry_address: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm9_ram_address: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm9_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm7_rom_offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm7_entry_address: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm7_ram_address: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm7_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fnt_offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fnt_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fat_offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fat_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_crc: Option<u16>,

    #[serde(skip)]
    pub source: RomHeaderSource,
}

impl RomHeader {
    /// Open and parse a ROM header from any supported source
    ///
    /// Automatically detects the format based on file extension and content:
    /// - `.bin` files: Binary ROM header
    /// - `.yaml`/`.yml` files: ds-rom-tool configuration
    /// - Directories: Searches for `header.bin` or `config.yaml`
    /// - Other files: Attempts binary parsing
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use uxie::RomHeader;
    ///
    /// // From binary file
    /// let header = RomHeader::open("header.bin")?;
    ///
    /// // From DSPRE project directory
    /// let header = RomHeader::open("path/to/dspre-project/")?;
    ///
    /// // From ds-rom-tool project
    /// let header = RomHeader::open("path/to/project/config.yaml")?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();

        if path.is_dir() {
            let config = path.join("config.yaml");
            if config.exists() {
                return Self::from_ds_rom_tool_project(path);
            }

            let header_bin = path.join("header.bin");
            if header_bin.exists() {
                return Self::from_binary(header_bin);
            }
        }

        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        match extension.to_lowercase().as_str() {
            "yaml" | "yml" => {
                let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if filename == "config.yaml" {
                    Self::from_ds_rom_tool_project(path.parent().unwrap_or(path))
                } else {
                    Self::from_ds_rom_yaml(path)
                }
            }
            "bin" => Self::from_binary(path),
            _ => {
                let mut file = File::open(path)?;
                let mut magic = [0u8; 4];
                file.read_exact(&mut magic)?;
                file.seek(SeekFrom::Start(0))?;

                if magic.starts_with(b"titl") || magic.starts_with(b"game") {
                    drop(file);
                    Self::from_ds_rom_yaml(path)
                } else {
                    Self::from_reader(&mut file, RomHeaderSource::NdsTool)
                }
            }
        }
    }

    /// Read header from a binary file
    ///
    /// Parses the standard 512-byte NDS ROM header format.
    pub fn from_binary(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut file = File::open(path)?;
        Self::from_reader(&mut file, RomHeaderSource::NdsTool)
    }

    pub fn from_file(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::from_binary(path)
    }

    pub fn from_ds_rom_yaml(path: impl AsRef<Path>) -> io::Result<Self> {
        let content = std::fs::read_to_string(path)?;

        #[derive(Deserialize)]
        struct DsRomYaml {
            title: String,
            gamecode: String,
            makercode: String,
            #[serde(default)]
            unitcode: u8,
            #[serde(default)]
            rom_version: u8,
            #[serde(default)]
            secure_area_delay: u16,
        }

        let yaml: DsRomYaml = serde_yaml::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        Ok(Self {
            game_title: yaml.title,
            game_code: yaml.gamecode,
            maker_code: yaml.makercode,
            unit_code: yaml.unitcode,
            rom_version: yaml.rom_version,
            secure_area_delay: yaml.secure_area_delay,
            arm9_rom_offset: None,
            arm9_entry_address: None,
            arm9_ram_address: None,
            arm9_size: None,
            arm7_rom_offset: None,
            arm7_entry_address: None,
            arm7_ram_address: None,
            arm7_size: None,
            fnt_offset: None,
            fnt_size: None,
            fat_offset: None,
            fat_size: None,
            header_crc: None,
            source: RomHeaderSource::DsRomTool,
        })
    }

    pub fn from_ds_rom_tool_project(project_dir: impl AsRef<Path>) -> io::Result<Self> {
        let project_dir = project_dir.as_ref();

        let config_path = if project_dir.is_file() {
            project_dir.to_path_buf()
        } else {
            project_dir.join("config.yaml")
        };

        let config_content = std::fs::read_to_string(&config_path)?;

        #[derive(Deserialize)]
        struct ConfigYaml {
            header: String,
            arm9_config: Option<String>,
        }

        let config: ConfigYaml = serde_yaml::from_str(&config_content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        let root = config_path.parent().unwrap_or(Path::new("."));
        let header_path = root.join(&config.header);

        let mut header = Self::from_ds_rom_yaml(&header_path)?;

        if let Some(arm9_config_rel) = config.arm9_config {
            let arm9_config_path = root.join(&arm9_config_rel);
            if arm9_config_path.exists() {
                let arm9_content = std::fs::read_to_string(&arm9_config_path)?;

                #[derive(Deserialize)]
                struct Arm9Yaml {
                    #[serde(default)]
                    base_address: u32,
                    #[serde(default)]
                    entry_function: u32,
                }

                let arm9 = serde_yaml::from_str::<Arm9Yaml>(&arm9_content).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Failed to parse arm9_config {}: {e}",
                            arm9_config_path.display()
                        ),
                    )
                })?;
                header.arm9_ram_address = Some(arm9.base_address);
                header.arm9_entry_address = Some(arm9.entry_function);
            }
        }

        Ok(header)
    }

    fn from_reader<R: Read + Seek>(reader: &mut R, source: RomHeaderSource) -> io::Result<Self> {
        reader.seek(SeekFrom::Start(0))?;

        let mut title_buf = [0u8; 12];
        reader.read_exact(&mut title_buf)?;
        let game_title = String::from_utf8_lossy(&title_buf)
            .trim_end_matches('\0')
            .to_string();

        let mut code_buf = [0u8; 4];
        reader.read_exact(&mut code_buf)?;
        let game_code = String::from_utf8_lossy(&code_buf).to_string();

        let mut maker_buf = [0u8; 2];
        reader.read_exact(&mut maker_buf)?;
        let maker_code = String::from_utf8_lossy(&maker_buf).to_string();

        let unit_code = reader.read_u8()?;
        let _encryption_seed = reader.read_u8()?;
        let _device_capacity = reader.read_u8()?;

        let mut reserved = [0u8; 7];
        reader.read_exact(&mut reserved)?;

        let rom_version = reader.read_u8()?;
        let _autostart = reader.read_u8()?;

        let mut padding = [0u8; 2];
        reader.read_exact(&mut padding)?;

        let arm9_rom_offset = reader.read_u32::<LittleEndian>()?;
        let arm9_entry_address = reader.read_u32::<LittleEndian>()?;
        let arm9_ram_address = reader.read_u32::<LittleEndian>()?;
        let arm9_size = reader.read_u32::<LittleEndian>()?;
        let arm7_rom_offset = reader.read_u32::<LittleEndian>()?;
        let arm7_entry_address = reader.read_u32::<LittleEndian>()?;
        let arm7_ram_address = reader.read_u32::<LittleEndian>()?;
        let arm7_size = reader.read_u32::<LittleEndian>()?;
        let fnt_offset = reader.read_u32::<LittleEndian>()?;
        let fnt_size = reader.read_u32::<LittleEndian>()?;
        let fat_offset = reader.read_u32::<LittleEndian>()?;
        let fat_size = reader.read_u32::<LittleEndian>()?;
        let _arm9_overlay_offset = reader.read_u32::<LittleEndian>()?;
        let _arm9_overlay_size = reader.read_u32::<LittleEndian>()?;
        let _arm7_overlay_offset = reader.read_u32::<LittleEndian>()?;
        let _arm7_overlay_size = reader.read_u32::<LittleEndian>()?;

        let mut port_settings = [0u8; 8];
        reader.read_exact(&mut port_settings)?;

        let _icon_title_offset = reader.read_u32::<LittleEndian>()?;
        let _secure_area_crc = reader.read_u16::<LittleEndian>()?;
        let secure_area_delay = reader.read_u16::<LittleEndian>()?;

        reader.seek(SeekFrom::Start(0x15E))?;
        let header_crc = reader.read_u16::<LittleEndian>()?;

        Ok(Self {
            game_title,
            game_code,
            maker_code,
            unit_code,
            rom_version,
            secure_area_delay,
            arm9_rom_offset: Some(arm9_rom_offset),
            arm9_entry_address: Some(arm9_entry_address),
            arm9_ram_address: Some(arm9_ram_address),
            arm9_size: Some(arm9_size),
            arm7_rom_offset: Some(arm7_rom_offset),
            arm7_entry_address: Some(arm7_entry_address),
            arm7_ram_address: Some(arm7_ram_address),
            arm7_size: Some(arm7_size),
            fnt_offset: Some(fnt_offset),
            fnt_size: Some(fnt_size),
            fat_offset: Some(fat_offset),
            fat_size: Some(fat_size),
            header_crc: Some(header_crc),
            source,
        })
    }

    /// Detect the specific game version from the game code
    ///
    /// Returns `None` if the game code doesn't match any known Gen 4 Pokemon game.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use uxie::RomHeader;
    /// let header = RomHeader::open("header.bin")?;
    /// match header.detect_game() {
    ///     Some(game) => println!("Game: {:?}", game),
    ///     None => println!("Unknown game"),
    /// }
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn detect_game(&self) -> Option<Game> {
        match self.game_code.as_str() {
            "ADAE" | "ADAJ" | "ADAP" | "ADAS" | "ADAK" | "ADAD" | "ADAF" | "ADAI" => {
                Some(Game::Diamond)
            }
            "APAE" | "APAJ" | "APAP" | "APAS" | "APAK" | "APAD" | "APAF" | "APAI" => {
                Some(Game::Pearl)
            }
            "CPUE" | "CPUJ" | "CPUP" | "CPUS" | "CPUK" | "CPUD" | "CPUF" | "CPUI" => {
                Some(Game::Platinum)
            }
            "IPKE" | "IPKJ" | "IPKP" | "IPKS" | "IPKK" | "IPKD" | "IPKF" | "IPKI" => {
                Some(Game::HeartGold)
            }
            "IPGE" | "IPGJ" | "IPGP" | "IPGS" | "IPGK" | "IPGD" | "IPGF" | "IPGI" => {
                Some(Game::SoulSilver)
            }
            _ => None,
        }
    }

    /// Detect the game family (DP, Platinum, or HGSS)
    ///
    /// This is useful when you need to know the general game family without
    /// caring about the specific version (e.g., Diamond vs Pearl).
    pub fn detect_game_family(&self) -> Option<GameFamily> {
        self.detect_game().map(|g| g.family())
    }

    /// Get the region from the game code
    ///
    /// Returns the full region name based on the 4th character of the game code:
    /// - 'E': USA
    /// - 'J': Japan
    /// - 'P': Europe
    /// - 'S': Spain
    /// - 'K': Korea
    /// - 'F': France
    /// - 'D': Germany
    /// - 'I': Italy
    pub fn region(&self) -> Option<&'static str> {
        self.game_code.chars().nth(3).and_then(|c| match c {
            'E' => Some("USA"),
            'J' => Some("Japan"),
            'P' => Some("Europe"),
            'S' => Some("Spain"),
            'K' => Some("Korea"),
            'F' => Some("France"),
            'D' => Some("Germany"),
            'I' => Some("Italy"),
            _ => None,
        })
    }

    pub fn detect_language(&self) -> GameLanguage {
        self.game_code
            .chars()
            .nth(3)
            .map(GameLanguage::from_region_code)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::tempdir;

    fn header_with_code(game_code: String) -> RomHeader {
        RomHeader {
            game_title: "TEST".into(),
            game_code,
            maker_code: "01".into(),
            unit_code: 0,
            rom_version: 0,
            secure_area_delay: 0,
            arm9_rom_offset: None,
            arm9_entry_address: None,
            arm9_ram_address: None,
            arm9_size: None,
            arm7_rom_offset: None,
            arm7_entry_address: None,
            arm7_ram_address: None,
            arm7_size: None,
            fnt_offset: None,
            fnt_size: None,
            fat_offset: None,
            fat_size: None,
            header_crc: None,
            source: RomHeaderSource::DsRomTool,
        }
    }

    fn known_code_mappings() -> Vec<(&'static str, Game)> {
        vec![
            ("ADAE", Game::Diamond),
            ("ADAJ", Game::Diamond),
            ("ADAP", Game::Diamond),
            ("ADAS", Game::Diamond),
            ("ADAK", Game::Diamond),
            ("APAE", Game::Pearl),
            ("APAJ", Game::Pearl),
            ("APAP", Game::Pearl),
            ("APAS", Game::Pearl),
            ("APAK", Game::Pearl),
            ("CPUE", Game::Platinum),
            ("CPUJ", Game::Platinum),
            ("CPUP", Game::Platinum),
            ("CPUS", Game::Platinum),
            ("CPUK", Game::Platinum),
            ("IPKE", Game::HeartGold),
            ("IPKJ", Game::HeartGold),
            ("IPKP", Game::HeartGold),
            ("IPKS", Game::HeartGold),
            ("IPKK", Game::HeartGold),
            ("IPGE", Game::SoulSilver),
            ("IPGJ", Game::SoulSilver),
            ("IPGP", Game::SoulSilver),
            ("IPGS", Game::SoulSilver),
            ("IPGK", Game::SoulSilver),
        ]
    }

    fn is_known_code(code: &str) -> bool {
        known_code_mappings().iter().any(|(c, _)| *c == code)
    }

    fn expected_region(ch: char) -> Option<&'static str> {
        match ch {
            'E' => Some("USA"),
            'J' => Some("Japan"),
            'P' => Some("Europe"),
            'S' => Some("Spain"),
            'K' => Some("Korea"),
            'F' => Some("France"),
            'D' => Some("Germany"),
            'I' => Some("Italy"),
            _ => None,
        }
    }

    #[test]
    fn test_detect_platinum_us() {
        let mut header = header_with_code("CPUE".into());
        header.game_title = "POKEMON PL".into();

        assert_eq!(header.detect_game(), Some(Game::Platinum));
        assert_eq!(header.detect_game_family(), Some(GameFamily::Platinum));
        assert_eq!(header.region(), Some("USA"));
    }

    #[test]
    fn test_from_ds_rom_yaml() {
        let yaml = r"
title: POKEMON PL
gamecode: CPUE
makercode: '01'
unitcode: 0
rom_version: 1
secure_area_delay: 3454
";
        let temp = std::env::temp_dir().join("test_header.yaml");
        std::fs::write(&temp, yaml).unwrap();

        let header = RomHeader::from_ds_rom_yaml(&temp).unwrap();
        assert_eq!(header.game_title, "POKEMON PL");
        assert_eq!(header.game_code, "CPUE");
        assert_eq!(header.rom_version, 1);
        assert_eq!(header.source, RomHeaderSource::DsRomTool);
        assert!(header.arm9_size.is_none());

        std::fs::remove_file(&temp).ok();
    }

    #[test]
    fn test_from_ds_rom_tool_project_with_arm9_config() {
        let dir = tempdir().unwrap();
        let header_path = dir.path().join("header.yaml");
        let config_path = dir.path().join("config.yaml");
        let arm9_config_path = dir.path().join("arm9.yaml");

        let header_yaml = r"
title: POKEMON HG
gamecode: IPKE
makercode: '01'
unitcode: 0
rom_version: 0
secure_area_delay: 0
";
        let config_yaml = r"
header: header.yaml
arm9_config: arm9.yaml
";
        let arm9_yaml = r"
base_address: 33554432
entry_function: 33587200
";

        std::fs::write(&header_path, header_yaml).unwrap();
        std::fs::write(&config_path, config_yaml).unwrap();
        std::fs::write(&arm9_config_path, arm9_yaml).unwrap();

        let header = RomHeader::from_ds_rom_tool_project(dir.path()).unwrap();
        assert_eq!(header.game_code, "IPKE");
        assert_eq!(header.arm9_ram_address, Some(33_554_432));
        assert_eq!(header.arm9_entry_address, Some(33_587_200));
    }

    #[test]
    fn test_from_ds_rom_tool_project_invalid_arm9_config_returns_invalid_data() {
        let dir = tempdir().unwrap();
        let header_path = dir.path().join("header.yaml");
        let config_path = dir.path().join("config.yaml");
        let arm9_config_path = dir.path().join("arm9.yaml");

        let header_yaml = r"
title: POKEMON HG
gamecode: IPKE
makercode: '01'
unitcode: 0
rom_version: 0
secure_area_delay: 0
";
        let config_yaml = r"
header: header.yaml
arm9_config: arm9.yaml
";
        let invalid_arm9_yaml = "base_address: [";

        std::fs::write(&header_path, header_yaml).unwrap();
        std::fs::write(&config_path, config_yaml).unwrap();
        std::fs::write(&arm9_config_path, invalid_arm9_yaml).unwrap();

        let err = RomHeader::from_ds_rom_tool_project(dir.path()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Failed to parse arm9_config"));
        assert!(err.to_string().contains("arm9.yaml"));
    }

    #[test]
    #[ignore = "requires a real Platinum DSPRE project path via UXIE_TEST_PLATINUM_DSPRE_PATH"]
    fn integration_open_platinum_dspre_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            "Platinum RomHeader integration test",
        ) else {
            return;
        };

        let header = RomHeader::open(root).unwrap();
        assert_eq!(header.detect_game(), Some(Game::Platinum));
        assert_eq!(header.detect_game_family(), Some(GameFamily::Platinum));
    }

    #[test]
    #[ignore = "requires a real HGSS DSPRE project path via UXIE_TEST_HGSS_DSPRE_PATH"]
    fn integration_open_hgss_dspre_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            "HGSS RomHeader integration test",
        ) else {
            return;
        };

        let header = RomHeader::open(root).unwrap();
        assert_eq!(header.detect_game_family(), Some(GameFamily::HGSS));
        assert!(matches!(
            header.detect_game(),
            Some(Game::HeartGold | Game::SoulSilver)
        ));
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_detect_game_known_codes((code, expected_game) in prop::sample::select(known_code_mappings())) {
            let header = header_with_code(code.to_string());
            prop_assert_eq!(header.detect_game(), Some(expected_game));
            prop_assert_eq!(header.detect_game_family(), Some(expected_game.family()));
        }

        #[test]
        fn prop_detect_game_unknown_4char_codes(
            a in b'A'..=b'Z',
            b in b'A'..=b'Z',
            c in b'A'..=b'Z',
            d in b'A'..=b'Z'
        ) {
            let code = String::from_utf8(vec![a, b, c, d]).unwrap();
            prop_assume!(!is_known_code(&code));
            let header = header_with_code(code);
            prop_assert_eq!(header.detect_game(), None);
            prop_assert_eq!(header.detect_game_family(), None);
        }

        #[test]
        fn prop_region_and_language_follow_fourth_char(ch in any::<char>()) {
            let mut code = String::from("AAA");
            code.push(ch);
            let header = header_with_code(code);
            prop_assert_eq!(header.region(), expected_region(ch));
            prop_assert_eq!(header.detect_language(), GameLanguage::from_region_code(ch));
        }

        #[test]
        fn prop_short_codes_are_non_game_and_default_language(code_bytes in prop::collection::vec(b'A'..=b'Z', 0..3)) {
            let code = String::from_utf8(code_bytes).unwrap();
            let header = header_with_code(code);
            prop_assert_eq!(header.detect_game(), None);
            prop_assert_eq!(header.detect_game_family(), None);
            prop_assert_eq!(header.region(), None);
            prop_assert_eq!(header.detect_language(), GameLanguage::English);
        }
    }
}
