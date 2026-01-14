//! DSPRE and ds-rom-tool project structures
//!
//! This module provides support for:
//! - DSPRE project directories (legacy ROM hacking tool format)
//! - ds-rom-tool projects (modern YAML-based ROM build system)

use crate::game::{Game, GameFamily};
use crate::rom_header::RomHeader;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::io;
use std::path::{Path, PathBuf};

pub use crate::event_file::BinaryEventFile;

fn from_yaml_file<T: DeserializeOwned>(path: impl AsRef<Path>) -> io::Result<T> {
    let content = std::fs::read_to_string(path)?;
    serde_yaml::from_str(&content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

#[derive(Debug, Clone)]
pub struct DspreProject {
    pub root: PathBuf,
}

impl DspreProject {
    pub fn open(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        if !root.join("unpacked").exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Not a DSPRE project (missing 'unpacked' directory)",
            ));
        }
        Ok(Self { root })
    }

    pub fn event_files_dir(&self) -> PathBuf {
        self.root.join("unpacked").join("eventFiles")
    }

    pub fn load_event_file(&self, id: u32) -> io::Result<BinaryEventFile> {
        let path = self.event_files_dir().join(format!("{:04}", id));
        let mut file = std::fs::File::open(path)?;
        BinaryEventFile::from_binary(&mut file)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsRomArm9Config {
    pub base_address: u32,
    pub entry_function: u32,
    #[serde(default)]
    pub build_info: u32,
    #[serde(default)]
    pub autoload_callback: u32,
    #[serde(default)]
    pub overlay_signatures: u32,
    #[serde(default)]
    pub encrypted: bool,
    #[serde(default)]
    pub compressed: bool,
    #[serde(default)]
    pub bss_start: u32,
    #[serde(default)]
    pub bss_end: u32,
    #[serde(default)]
    pub sdk_version: u32,
}

impl DsRomArm9Config {
    pub fn from_yaml(path: impl AsRef<Path>) -> io::Result<Self> {
        from_yaml_file(path)
    }

    pub fn sdk_version_string(&self) -> String {
        let major = (self.sdk_version >> 24) & 0xFF;
        let minor = (self.sdk_version >> 16) & 0xFF;
        let patch = self.sdk_version & 0xFFFF;
        format!("{}.{}.{}", major, minor, patch)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsRomArm7Config {
    pub base_address: u32,
    pub entry_function: u32,
    #[serde(default)]
    pub encrypted: bool,
    #[serde(default)]
    pub compressed: bool,
    #[serde(default)]
    pub bss_start: u32,
    #[serde(default)]
    pub bss_end: u32,
}

impl DsRomArm7Config {
    pub fn from_yaml(path: impl AsRef<Path>) -> io::Result<Self> {
        from_yaml_file(path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsRomTcmConfig {
    pub base_address: u32,
    #[serde(default)]
    pub compressed: bool,
}

impl DsRomTcmConfig {
    pub fn from_yaml(path: impl AsRef<Path>) -> io::Result<Self> {
        from_yaml_file(path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TcmRef {
    bin: String,
    config: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct AlignmentConfig {
    arm9: u32,
    arm9_overlay_table: u32,
    arm9_overlay: u32,
    arm7: u32,
    arm7_overlay_table: u32,
    arm7_overlay: u32,
    file_name_table: u32,
    file_allocation_table: u32,
    banner: u32,
    file_image_block: u32,
    file: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawProjectConfig {
    header: String,
    header_logo: String,
    arm9_bin: String,
    arm9_config: String,
    arm7_bin: String,
    arm7_config: String,
    itcm: Option<TcmRef>,
    dtcm: Option<TcmRef>,
    arm9_overlays: Option<String>,
    arm7_overlays: Option<String>,
    banner: String,
    files_dir: String,
    path_order: Option<String>,
    #[serde(default)]
    file_image_padding_value: u8,
    #[serde(default)]
    section_padding_value: u8,
    alignment: Option<AlignmentConfig>,
}

#[derive(Debug, Clone)]
pub struct DsRomToolProject {
    root: PathBuf,
    pub header: RomHeader,
    pub arm9_config: DsRomArm9Config,
    pub arm7_config: DsRomArm7Config,
    pub itcm_config: Option<DsRomTcmConfig>,
    pub dtcm_config: Option<DsRomTcmConfig>,
    arm9_bin_path: PathBuf,
    arm7_bin_path: PathBuf,
    files_dir: PathBuf,
}

impl DsRomToolProject {
    pub fn open(config_path: impl AsRef<Path>) -> io::Result<Self> {
        let config_path = config_path.as_ref();
        let root = config_path.parent().unwrap_or(Path::new(".")).to_path_buf();

        let content = std::fs::read_to_string(config_path)?;
        let raw: RawProjectConfig = serde_yaml::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        let mut header = RomHeader::from_ds_rom_yaml(root.join(&raw.header))?;
        let arm9_config = DsRomArm9Config::from_yaml(root.join(&raw.arm9_config))?;
        let arm7_config = DsRomArm7Config::from_yaml(root.join(&raw.arm7_config))?;

        header.arm9_ram_address = Some(arm9_config.base_address);
        header.arm9_entry_address = Some(arm9_config.entry_function);
        header.arm7_ram_address = Some(arm7_config.base_address);
        header.arm7_entry_address = Some(arm7_config.entry_function);

        let itcm_config = if let Some(ref tcm) = raw.itcm {
            Some(DsRomTcmConfig::from_yaml(root.join(&tcm.config))?)
        } else {
            None
        };

        let dtcm_config = if let Some(ref tcm) = raw.dtcm {
            Some(DsRomTcmConfig::from_yaml(root.join(&tcm.config))?)
        } else {
            None
        };

        Ok(Self {
            root: root.clone(),
            header,
            arm9_config,
            arm7_config,
            itcm_config,
            dtcm_config,
            arm9_bin_path: root.join(&raw.arm9_bin),
            arm7_bin_path: root.join(&raw.arm7_bin),
            files_dir: root.join(&raw.files_dir),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn arm9_bin_path(&self) -> &Path {
        &self.arm9_bin_path
    }

    pub fn arm7_bin_path(&self) -> &Path {
        &self.arm7_bin_path
    }

    pub fn files_dir(&self) -> &Path {
        &self.files_dir
    }

    pub fn game(&self) -> Option<Game> {
        self.header.detect_game()
    }

    pub fn game_family(&self) -> Option<GameFamily> {
        self.header.detect_game_family()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arm9_config() {
        let yaml = r#"
base_address: 33554432
entry_function: 33556480
sdk_version: 67269937
"#;
        let config: DsRomArm9Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.base_address, 0x02000000);
        assert_eq!(config.sdk_version_string(), "4.2.30001");
    }
}
