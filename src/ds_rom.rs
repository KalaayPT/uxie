//! DSPRE and ds-rom-tool project structures
//!
//! This module provides support for:
//! - DSPRE project directories (legacy ROM hacking tool format)
//! - ds-rom-tool projects (modern YAML-based ROM build system)

use crate::game::{Game, GameFamily};
use crate::rom_header::RomHeader;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
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

    pub fn unpacked_dir(&self) -> PathBuf {
        self.root.join("unpacked")
    }

    pub fn scripts_dir(&self) -> PathBuf {
        self.unpacked_dir().join("scripts")
    }

    pub fn text_archives_dir(&self) -> PathBuf {
        self.unpacked_dir().join("textArchives")
    }

    pub fn event_files_dir(&self) -> PathBuf {
        self.unpacked_dir().join("eventFiles")
    }

    pub fn dynamic_headers_dir(&self) -> PathBuf {
        self.unpacked_dir().join("dynamicHeaders")
    }

    pub fn maps_dir(&self) -> PathBuf {
        self.unpacked_dir().join("maps")
    }

    pub fn matrices_dir(&self) -> PathBuf {
        self.unpacked_dir().join("matrices")
    }

    pub fn learnsets_dir(&self) -> PathBuf {
        self.unpacked_dir().join("learnsets")
    }

    pub fn personal_poke_data_dir(&self) -> PathBuf {
        self.unpacked_dir().join("personalPokeData")
    }

    pub fn trainer_properties_dir(&self) -> PathBuf {
        self.unpacked_dir().join("trainerProperties")
    }

    pub fn trainer_party_dir(&self) -> PathBuf {
        self.unpacked_dir().join("trainerParty")
    }

    pub fn area_data_dir(&self) -> PathBuf {
        self.unpacked_dir().join("areaData")
    }

    pub fn script_file_path(&self, id: u16) -> PathBuf {
        self.scripts_dir().join(format!("{:04}", id))
    }

    pub fn text_archive_path(&self, id: u16) -> PathBuf {
        self.text_archives_dir().join(format!("{:04}", id))
    }

    pub fn event_file_path(&self, id: u16) -> PathBuf {
        self.event_files_dir().join(format!("{:04}", id))
    }

    pub fn dynamic_header_path(&self, id: u16) -> PathBuf {
        self.dynamic_headers_dir().join(format!("{:04}", id))
    }

    pub fn has_unpacked_scripts(&self) -> bool {
        self.scripts_dir().exists()
    }

    pub fn has_unpacked_text_archives(&self) -> bool {
        self.text_archives_dir().exists()
    }

    pub fn has_unpacked_events(&self) -> bool {
        self.event_files_dir().exists()
    }

    pub fn load_event_file(&self, id: u32) -> io::Result<BinaryEventFile> {
        let path = self.event_files_dir().join(format!("{:04}", id));
        let mut file = std::fs::File::open(path)?;
        BinaryEventFile::from_binary(&mut file)
    }

    pub fn load_script_file(&self, id: u16) -> io::Result<Vec<u8>> {
        std::fs::read(self.script_file_path(id))
    }

    pub fn load_text_archive(&self, id: u16) -> io::Result<Vec<u8>> {
        std::fs::read(self.text_archive_path(id))
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
    use proptest::prelude::*;

    fn test_header_with_code(game_code: String) -> RomHeader {
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
            source: crate::rom_header::RomHeaderSource::DsRomTool,
        }
    }

    #[test]
    fn test_parse_arm9_config() {
        let yaml = r"
base_address: 33554432
entry_function: 33556480
sdk_version: 67269937
";
        let config: DsRomArm9Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.base_address, 0x0200_0000);
        assert_eq!(config.sdk_version_string(), "4.2.30001");
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_sdk_version_string_matches_bit_layout(sdk_version in any::<u32>()) {
            let config = DsRomArm9Config {
                base_address: 0,
                entry_function: 0,
                build_info: 0,
                autoload_callback: 0,
                overlay_signatures: 0,
                encrypted: false,
                compressed: false,
                bss_start: 0,
                bss_end: 0,
                sdk_version,
            };

            let expected = format!(
                "{}.{}.{}",
                (sdk_version >> 24) & 0xFF,
                (sdk_version >> 16) & 0xFF,
                sdk_version & 0xFFFF
            );
            prop_assert_eq!(config.sdk_version_string(), expected);
        }

        #[test]
        fn prop_ds_rom_tool_project_game_passthrough(code in prop::sample::select(vec![
            "ADAE", "APAE", "CPUE", "IPKE", "IPGE", "ZZZZ"
        ])) {
            let header = test_header_with_code(code.to_string());
            let expected_game = header.detect_game();
            let expected_family = header.detect_game_family();

            let project = DsRomToolProject {
                root: std::env::temp_dir(),
                header,
                arm9_config: DsRomArm9Config {
                    base_address: 0,
                    entry_function: 0,
                    build_info: 0,
                    autoload_callback: 0,
                    overlay_signatures: 0,
                    encrypted: false,
                    compressed: false,
                    bss_start: 0,
                    bss_end: 0,
                    sdk_version: 0,
                },
                arm7_config: DsRomArm7Config {
                    base_address: 0,
                    entry_function: 0,
                    encrypted: false,
                    compressed: false,
                    bss_start: 0,
                    bss_end: 0,
                },
                itcm_config: None,
                dtcm_config: None,
                arm9_bin_path: std::env::temp_dir().join("arm9.bin"),
                arm7_bin_path: std::env::temp_dir().join("arm7.bin"),
                files_dir: std::env::temp_dir().join("files"),
            };

            prop_assert_eq!(project.game(), expected_game);
            prop_assert_eq!(project.game_family(), expected_family);
        }

        #[test]
        fn prop_dspre_id_paths_format_padded_names(id in any::<u16>()) {
            let project = DspreProject {
                root: std::env::temp_dir(),
            };
            let expected = format!("{:04}", id);

            let script = project.script_file_path(id);
            let text = project.text_archive_path(id);
            let event = project.event_file_path(id);
            let dynamic = project.dynamic_header_path(id);

            prop_assert_eq!(script.file_name().and_then(|n| n.to_str()), Some(expected.as_str()));
            prop_assert_eq!(text.file_name().and_then(|n| n.to_str()), Some(expected.as_str()));
            prop_assert_eq!(event.file_name().and_then(|n| n.to_str()), Some(expected.as_str()));
            prop_assert_eq!(dynamic.file_name().and_then(|n| n.to_str()), Some(expected.as_str()));
        }
    }
}
