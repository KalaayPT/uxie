//! Data provider abstraction for accessing ROM data from different sources
//!
//! This module defines the [`DataProvider`] trait and concrete implementations
//! for different project types:
//! - [`Arm9Provider`]: Reads from ARM9 binary files (DSPRE projects)
//! - [`DecompProvider`]: Reads from decompilation source files

use crate::error::{Result, UxieError};
use crate::game::GameFamily;
use crate::map_header::{
    MAP_HEADER_SIZE, MapHeader, read_map_header_from_bytes, read_map_headers_from_arm9,
};
use std::fs::File;
use std::path::{Path, PathBuf};

/// Trait for accessing ROM data regardless of source format
///
/// Implementations provide unified access to map headers and related data
/// from different project types (DSPRE, decompilation, etc.).
pub trait DataProvider {
    /// Get a specific map header by ID
    fn get_map_header(&self, id: u16) -> Result<MapHeader>;
    /// Get the total number of map headers
    fn get_map_header_count(&self) -> Result<usize>;
    /// Get the text archive ID associated with a script file
    fn get_text_archive_for_script(&self, script_id: u16) -> Result<Option<u16>>;
    /// Find the map ID for a given script file ID
    ///
    /// Returns the first map ID that uses the specified script file ID,
    /// or None if no map uses it.
    fn find_map_by_script_file_id(&self, script_file_id: u16) -> Result<Option<u16>>;

    /// Find the map ID for a given level script file ID
    ///
    /// Returns the first map ID that uses the specified level script file ID,
    /// or None if no map uses it.
    fn find_map_by_level_script_file_id(&self, level_script_file_id: u16) -> Result<Option<u16>>;
}

/// Provider for reading map data from ARM9 binary files
///
/// Used primarily for DSPRE projects where map headers are stored
/// in a binary table within the ARM9 executable.
///
/// # Examples
///
/// ```rust,no_run
/// use uxie::{Arm9Provider, GameFamily, DataProvider};
///
/// let provider = Arm9Provider::new(
///     "path/to/arm9.bin",
///     0xE601C,  // Platinum table offset
///     559,      // Number of map headers
///     GameFamily::Platinum,
/// );
///
/// let header = provider.get_map_header(0)?;
/// println!("Map 0 has {} script file", header.script_file_id());
/// # Ok::<(), uxie::UxieError>(())
/// ```
pub struct Arm9Provider {
    arm9_path: PathBuf,
    header_table_offset: u64,
    header_count: usize,
    game_family: GameFamily,
}

impl Arm9Provider {
    /// Create a new ARM9 provider
    ///
    /// # Arguments
    ///
    /// * `arm9_path` - Path to the arm9.bin file
    /// * `header_table_offset` - Byte offset to the map header table in ARM9
    /// * `header_count` - Number of map headers in the table
    /// * `game_family` - Game family (DP, Platinum, or HGSS)
    pub fn new(
        arm9_path: impl AsRef<Path>,
        header_table_offset: u64,
        header_count: usize,
        game_family: GameFamily,
    ) -> Self {
        Self {
            arm9_path: arm9_path.as_ref().to_path_buf(),
            header_table_offset,
            header_count,
            game_family,
        }
    }

    /// Create a provider for Platinum (US version)
    ///
    /// Convenience constructor with hardcoded offsets for Platinum US.
    pub fn platinum_us(arm9_path: impl AsRef<Path>) -> Self {
        Self::new(arm9_path, 0xE601C, 559, GameFamily::Platinum)
    }

    fn read_all_headers(&self) -> Result<Vec<MapHeader>> {
        let mut file = File::open(&self.arm9_path)?;
        Ok(read_map_headers_from_arm9(
            &mut file,
            self.header_table_offset,
            self.header_count,
            self.game_family,
        )?)
    }
}

impl DataProvider for Arm9Provider {
    fn get_map_header(&self, id: u16) -> Result<MapHeader> {
        if id as usize >= self.header_count {
            return Err(UxieError::out_of_bounds(
                "Map header",
                id as u32,
                self.header_count.saturating_sub(1) as u32,
            ));
        }

        use std::io::{Read, Seek, SeekFrom};
        let mut file = File::open(&self.arm9_path)?;
        let offset = self.header_table_offset + (id as u64 * MAP_HEADER_SIZE as u64);
        file.seek(SeekFrom::Start(offset))?;

        let mut buf = [0u8; MAP_HEADER_SIZE];
        file.read_exact(&mut buf)?;
        Ok(read_map_header_from_bytes(&buf, self.game_family)?)
    }

    fn get_map_header_count(&self) -> Result<usize> {
        Ok(self.header_count)
    }

    fn get_text_archive_for_script(&self, script_id: u16) -> Result<Option<u16>> {
        let headers = self.read_all_headers()?;
        find_text_archive_in_headers(&headers, script_id)
    }

    fn find_map_by_script_file_id(&self, script_file_id: u16) -> Result<Option<u16>> {
        let headers = self.read_all_headers()?;
        find_map_by_script_file_in_headers(&headers, script_file_id)
    }

    fn find_map_by_level_script_file_id(&self, level_script_file_id: u16) -> Result<Option<u16>> {
        let headers = self.read_all_headers()?;
        find_map_by_level_script_in_headers(&headers, level_script_file_id)
    }
}

/// Provider for reading map data from decompilation source files
///
/// Parses C header files and JSON data from pokeplatinum/pokeheartgold
/// decompilation projects.
pub struct DecompProvider {
    pub root: PathBuf,
    pub symbols: crate::c_parser::SymbolTable,
    pub family: GameFamily,
}

impl DecompProvider {
    /// Create a new decompilation provider
    ///
    /// # Arguments
    ///
    /// * `root` - Root directory of the decompilation project
    /// * `symbols` - Pre-loaded symbol table with project constants
    pub fn new(
        root: impl AsRef<Path>,
        symbols: crate::c_parser::SymbolTable,
        family: GameFamily,
    ) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            symbols,
            family,
        }
    }

    fn map_headers_path(&self) -> PathBuf {
        let primary = match self.family {
            GameFamily::HGSS => self.root.join("src/data/map_headers.h"),
            GameFamily::DP | GameFamily::Platinum => self.root.join("include/data/map_headers.h"),
        };

        if primary.exists() {
            return primary;
        }

        let fallback = match self.family {
            GameFamily::HGSS => self.root.join("include/data/map_headers.h"),
            GameFamily::DP | GameFamily::Platinum => self.root.join("src/data/map_headers.h"),
        };

        if fallback.exists() { fallback } else { primary }
    }

    fn load_all_headers(&self) -> Result<Vec<MapHeader>> {
        let path = self.map_headers_path();
        let content = std::fs::read_to_string(path)?;
        let parsed = crate::map_header::parse_map_headers_from_c(&content);

        let mut headers = Vec::new();
        for p in parsed {
            let header = match self.family {
                GameFamily::HGSS => {
                    MapHeader::HGSS(crate::map_header::parsed_to_hgss_header(&p, &self.symbols))
                }
                GameFamily::DP | GameFamily::Platinum => {
                    MapHeader::Pt(crate::map_header::parsed_to_pt_header(&p, &self.symbols))
                }
            };
            headers.push(header);
        }
        Ok(headers)
    }
}

impl DataProvider for DecompProvider {
    fn get_map_header(&self, id: u16) -> Result<MapHeader> {
        let headers = self.load_all_headers()?;
        headers.get(id as usize).cloned().ok_or_else(|| {
            UxieError::out_of_bounds(
                "Map header",
                id as u32,
                headers.len().saturating_sub(1) as u32,
            )
        })
    }

    fn get_map_header_count(&self) -> Result<usize> {
        let headers = self.load_all_headers()?;
        Ok(headers.len())
    }

    fn get_text_archive_for_script(&self, script_id: u16) -> Result<Option<u16>> {
        let headers = self.load_all_headers()?;
        find_text_archive_in_headers(&headers, script_id)
    }

    fn find_map_by_script_file_id(&self, script_file_id: u16) -> Result<Option<u16>> {
        let headers = self.load_all_headers()?;
        find_map_by_script_file_in_headers(&headers, script_file_id)
    }

    fn find_map_by_level_script_file_id(&self, level_script_file_id: u16) -> Result<Option<u16>> {
        let headers = self.load_all_headers()?;
        find_map_by_level_script_in_headers(&headers, level_script_file_id)
    }
}

fn find_text_archive_in_headers(headers: &[MapHeader], script_id: u16) -> Result<Option<u16>> {
    for header in headers {
        if header.script_file_id() == script_id {
            return Ok(Some(header.text_archive_id()));
        }
    }
    Ok(None)
}

fn find_map_by_script_file_in_headers(
    headers: &[MapHeader],
    script_file_id: u16,
) -> Result<Option<u16>> {
    for (map_id, header) in headers.iter().enumerate() {
        if header.script_file_id() == script_file_id {
            return Ok(Some(map_id as u16));
        }
    }
    Ok(None)
}

fn find_map_by_level_script_in_headers(
    headers: &[MapHeader],
    level_script_file_id: u16,
) -> Result<Option<u16>> {
    for (map_id, header) in headers.iter().enumerate() {
        if header.level_script_id() == level_script_file_id {
            return Ok(Some(map_id as u16));
        }
    }
    Ok(None)
}

/// Find all map headers that use a specific script file
///
/// Returns the indices of map headers that reference the given script ID.
pub fn find_headers_using_script(headers: &[MapHeader], script_id: u16) -> Vec<usize> {
    headers
        .iter()
        .enumerate()
        .filter(|(_, h)| h.script_file_id() == script_id)
        .map(|(i, _)| i)
        .collect()
}

/// Find all map headers that use a specific text archive
///
/// Returns the indices of map headers that reference the given text ID.
pub fn find_headers_using_text(headers: &[MapHeader], text_id: u16) -> Vec<usize> {
    headers
        .iter()
        .enumerate()
        .filter(|(_, h)| h.text_archive_id() == text_id)
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_parser::SymbolTable;
    use crate::map_header::write_map_header_to_bytes;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_pt_header(script_id: u16, text_id: u16) -> MapHeader {
        MapHeader::Pt(crate::map_header::MapHeaderPt {
            script_file_id: script_id,
            text_archive_id: text_id,
            ..Default::default()
        })
    }

    fn create_test_arm9_file(headers: &[MapHeader]) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        for header in headers {
            let bytes = write_map_header_to_bytes(header);
            file.write_all(&bytes).unwrap();
        }
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_arm9_provider_get_header() {
        let headers = vec![
            create_test_pt_header(100, 200),
            create_test_pt_header(101, 201),
            create_test_pt_header(102, 202),
        ];
        let file = create_test_arm9_file(&headers);

        let provider = Arm9Provider::new(file.path(), 0, 3, GameFamily::Platinum);

        let h0 = provider.get_map_header(0).unwrap();
        assert_eq!(h0.script_file_id(), 100);
        assert_eq!(h0.text_archive_id(), 200);

        let h1 = provider.get_map_header(1).unwrap();
        assert_eq!(h1.script_file_id(), 101);

        let h2 = provider.get_map_header(2).unwrap();
        assert_eq!(h2.script_file_id(), 102);
    }

    #[test]
    fn test_arm9_provider_header_count() {
        let headers = vec![create_test_pt_header(1, 1), create_test_pt_header(2, 2)];
        let file = create_test_arm9_file(&headers);

        let provider = Arm9Provider::new(file.path(), 0, 2, GameFamily::Platinum);

        assert_eq!(provider.get_map_header_count().unwrap(), 2);
    }

    #[test]
    fn test_arm9_provider_out_of_bounds() {
        let headers = vec![create_test_pt_header(1, 1)];
        let file = create_test_arm9_file(&headers);

        let provider = Arm9Provider::new(file.path(), 0, 1, GameFamily::Platinum);

        let result = provider.get_map_header(5);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn test_arm9_provider_get_text_archive_for_script() {
        let headers = vec![
            create_test_pt_header(10, 100),
            create_test_pt_header(20, 200),
            create_test_pt_header(30, 300),
        ];
        let file = create_test_arm9_file(&headers);

        let provider = Arm9Provider::new(file.path(), 0, 3, GameFamily::Platinum);

        assert_eq!(provider.get_text_archive_for_script(20).unwrap(), Some(200));
        assert_eq!(provider.get_text_archive_for_script(999).unwrap(), None);
    }

    #[test]
    fn test_arm9_provider_file_not_found() {
        let provider = Arm9Provider::new("/nonexistent/arm9.bin", 0, 10, GameFamily::Platinum);

        let result = provider.get_map_header(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_headers_using_script() {
        let headers = vec![
            create_test_pt_header(10, 100),
            create_test_pt_header(20, 200),
            create_test_pt_header(10, 300),
            create_test_pt_header(30, 400),
        ];

        let indices = find_headers_using_script(&headers, 10);
        assert_eq!(indices, vec![0, 2]);

        let indices = find_headers_using_script(&headers, 20);
        assert_eq!(indices, vec![1]);

        let indices = find_headers_using_script(&headers, 999);
        assert!(indices.is_empty());
    }

    #[test]
    fn test_find_headers_using_text() {
        let headers = vec![
            create_test_pt_header(10, 100),
            create_test_pt_header(20, 200),
            create_test_pt_header(30, 100),
        ];

        let indices = find_headers_using_text(&headers, 100);
        assert_eq!(indices, vec![0, 2]);

        let indices = find_headers_using_text(&headers, 200);
        assert_eq!(indices, vec![1]);

        let indices = find_headers_using_text(&headers, 999);
        assert!(indices.is_empty());
    }

    #[test]
    fn test_platinum_us_constructor() {
        let provider = Arm9Provider::platinum_us("/some/path/arm9.bin");
        assert_eq!(provider.header_count, 559);
        assert_eq!(provider.header_table_offset, 0xE601C);
    }

    #[test]
    fn test_arm9_provider_find_map_by_script_file_id() {
        let headers = vec![
            create_test_pt_header(10, 100),
            create_test_pt_header(20, 200),
            create_test_pt_header(30, 300),
        ];
        let file = create_test_arm9_file(&headers);

        let provider = Arm9Provider::new(file.path(), 0, 3, GameFamily::Platinum);

        assert_eq!(provider.find_map_by_script_file_id(10).unwrap(), Some(0));
        assert_eq!(provider.find_map_by_script_file_id(20).unwrap(), Some(1));
        assert_eq!(provider.find_map_by_script_file_id(30).unwrap(), Some(2));
        assert_eq!(provider.find_map_by_script_file_id(999).unwrap(), None);
    }

    #[test]
    fn test_decomp_provider_loads_pt_headers() {
        let dir = tempfile::tempdir().unwrap();
        let header_path = dir.path().join("include/data/map_headers.h");
        fs::create_dir_all(header_path.parent().unwrap()).unwrap();
        fs::write(
            &header_path,
            r"
        [MAP_HEADER_TEST] = {
            .areaDataArchiveID = 1,
            .unk_01 = 2,
            .mapMatrixID = 3,
            .scriptsArchiveID = 4,
            .initScriptsArchiveID = 5,
            .msgArchiveID = 6,
            .dayMusicID = 7,
            .nightMusicID = 8,
            .wildEncountersArchiveID = 9,
            .eventsArchiveID = 10,
            .mapLabelTextID = 11,
            .mapLabelWindowID = 12,
            .weather = 13,
            .cameraType = 14,
            .mapType = 15,
            .battleBG = 16,
            .isBikeAllowed = TRUE,
            .isRunningAllowed = FALSE,
            .isEscapeRopeAllowed = TRUE,
            .isFlyAllowed = FALSE,
        },
        ",
        )
        .unwrap();

        let provider = DecompProvider::new(dir.path(), SymbolTable::new(), GameFamily::Platinum);
        let header = provider.get_map_header(0).unwrap();
        match header {
            MapHeader::Pt(h) => {
                assert_eq!(h.script_file_id, 4);
                assert_eq!(h.level_script_id, 5);
                assert_eq!(h.text_archive_id, 6);
                assert_eq!(h.flags, 0b0101);
            }
            _ => panic!("expected Platinum map header variant"),
        }
    }

    #[test]
    fn test_decomp_provider_loads_hgss_headers() {
        let dir = tempfile::tempdir().unwrap();
        let header_path = dir.path().join("src/data/map_headers.h");
        fs::create_dir_all(header_path.parent().unwrap()).unwrap();
        fs::write(
            &header_path,
            r"
        [MAP_EVERYWHERE] = {
            .wildEncounterBank = 1,
            .areaDataBank = 2,
            .moveModelBank = 3,
            .worldMapX = 4,
            .worldMapY = 5,
            .matrixId = 6,
            .scriptsBank = 7,
            .scriptHeaderBank = 8,
            .msgBank = 9,
            .dayMusicId = 10,
            .nightMusicId = 11,
            .eventsBank = 12,
            .mapsec = 13,
            .areaIcon = 14,
            .momCallIntroParam = 15,
            .isKanto = TRUE,
            .weather = 16,
            .mapType = 17,
            .cameraType = 18,
            .followMode = 2,
            .battleBg = 19,
            .bikeAllowed = TRUE,
            .runningAllowed_Unused = FALSE,
            .escapeRopeAllowed = TRUE,
            .flyAllowed = FALSE,
            .outgoingCalls = TRUE,
            .incomingCalls = FALSE,
            .radioSignal = TRUE,
        },
        ",
        )
        .unwrap();

        let provider = DecompProvider::new(dir.path(), SymbolTable::new(), GameFamily::HGSS);
        let header = provider.get_map_header(0).unwrap();
        match header {
            MapHeader::HGSS(h) => {
                assert_eq!(h.script_file_id, 7);
                assert_eq!(h.level_script_id, 8);
                assert_eq!(h.text_archive_id, 9);
                assert!(h.kanto_flag);
                assert_eq!(h.flags, 0x55);
            }
            _ => panic!("expected HGSS map header variant"),
        }
    }
}
