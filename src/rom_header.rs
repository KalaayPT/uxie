use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::fs::File;
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use crate::game::{Game, GameFamily};

pub const ROM_HEADER_SIZE: usize = 0x200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RomHeaderSource {
    #[default]
    NdsTool,
    DsRom,
    Decomp,
}

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
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        
        if path.is_dir() {
            let config = path.join("config.yaml");
            if config.exists() {
                return Self::from_ds_rom_project(path);
            }
        }
        
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        match extension.to_lowercase().as_str() {
            "yaml" | "yml" => {
                let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if filename == "config.yaml" {
                    Self::from_ds_rom_project(path.parent().unwrap_or(path))
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
        
        let yaml: DsRomYaml = serde_yaml::from_str(&content).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, e.to_string())
        })?;
        
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
            source: RomHeaderSource::DsRom,
        })
    }

    pub fn from_ds_rom_project(project_dir: impl AsRef<Path>) -> io::Result<Self> {
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
        
        let config: ConfigYaml = serde_yaml::from_str(&config_content).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, e.to_string())
        })?;
        
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
                
                if let Ok(arm9) = serde_yaml::from_str::<Arm9Yaml>(&arm9_content) {
                    header.arm9_ram_address = Some(arm9.base_address);
                    header.arm9_entry_address = Some(arm9.entry_function);
                }
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
        
        let mut _reserved = [0u8; 7];
        reader.read_exact(&mut _reserved)?;
        
        let rom_version = reader.read_u8()?;
        let _autostart = reader.read_u8()?;
        
        let mut _padding = [0u8; 2];
        reader.read_exact(&mut _padding)?;
        
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
        
        let mut _port_settings = [0u8; 8];
        reader.read_exact(&mut _port_settings)?;
        
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

    pub fn detect_game(&self) -> Option<Game> {
        match self.game_code.as_str() {
            "ADAE" | "ADAJ" | "ADAP" | "ADAS" | "ADAK" => Some(Game::Diamond),
            "APAE" | "APAJ" | "APAP" | "APAS" | "APAK" => Some(Game::Pearl),
            "CPUE" | "CPUJ" | "CPUP" | "CPUS" | "CPUK" => Some(Game::Platinum),
            "IPKE" | "IPKJ" | "IPKP" | "IPKS" | "IPKK" => Some(Game::HeartGold),
            "IPGE" | "IPGJ" | "IPGP" | "IPGS" | "IPGK" => Some(Game::SoulSilver),
            _ => None,
        }
    }

    pub fn detect_game_family(&self) -> Option<GameFamily> {
        self.detect_game().map(|g| g.family())
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_platinum_us() {
        let header = RomHeader {
            game_title: "POKEMON PL".into(),
            game_code: "CPUE".into(),
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
            source: RomHeaderSource::DsRom,
        };

        assert_eq!(header.detect_game(), Some(Game::Platinum));
        assert_eq!(header.detect_game_family(), Some(GameFamily::Platinum));
        assert_eq!(header.region(), Some("USA"));
    }

    #[test]
    fn test_from_ds_rom_yaml() {
        let yaml = r#"
title: POKEMON PL
gamecode: CPUE
makercode: '01'
unitcode: 0
rom_version: 1
secure_area_delay: 3454
"#;
        let temp = std::env::temp_dir().join("test_header.yaml");
        std::fs::write(&temp, yaml).unwrap();
        
        let header = RomHeader::from_ds_rom_yaml(&temp).unwrap();
        assert_eq!(header.game_title, "POKEMON PL");
        assert_eq!(header.game_code, "CPUE");
        assert_eq!(header.rom_version, 1);
        assert_eq!(header.source, RomHeaderSource::DsRom);
        assert!(header.arm9_size.is_none());
        
        std::fs::remove_file(&temp).ok();
    }
}
