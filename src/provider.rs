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
use std::sync::{Arc, Mutex, OnceLock};

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
    fn get_text_archive_for_script_file(&self, script_file_id: u16) -> Result<Option<u16>>;
    /// Find all map IDs for a given script file ID.
    fn find_maps_by_script_file_id(&self, script_file_id: u16) -> Result<Vec<u16>>;

    /// Find the first map ID for a given script file ID.
    ///
    /// Returns the first map ID that uses the specified script file ID,
    /// or None if no map uses it.
    fn find_map_by_script_file_id(&self, script_file_id: u16) -> Result<Option<u16>> {
        Ok(self
            .find_maps_by_script_file_id(script_file_id)?
            .into_iter()
            .next())
    }

    /// Find all map IDs for a given level script file ID.
    fn find_maps_by_level_script_file_id(&self, level_script_file_id: u16) -> Result<Vec<u16>>;

    /// Find the first map ID for a given level script file ID.
    ///
    /// Returns the first map ID that uses the specified level script file ID,
    /// or None if no map uses it.
    fn find_map_by_level_script_file_id(&self, level_script_file_id: u16) -> Result<Option<u16>> {
        Ok(self
            .find_maps_by_level_script_file_id(level_script_file_id)?
            .into_iter()
            .next())
    }
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
    headers_cache: OnceLock<Arc<Vec<MapHeader>>>,
    headers_init_lock: Mutex<()>,
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
            headers_cache: OnceLock::new(),
            headers_init_lock: Mutex::new(()),
        }
    }

    /// Create a provider for Platinum (US version)
    ///
    /// Convenience constructor with hardcoded offsets for Platinum US.
    pub fn platinum_us(arm9_path: impl AsRef<Path>) -> Self {
        Self::new(arm9_path, 0xE601C, 559, GameFamily::Platinum)
    }

    fn parse_all_headers(&self) -> Result<Vec<MapHeader>> {
        let mut file = File::open(&self.arm9_path)?;
        Ok(read_map_headers_from_arm9(
            &mut file,
            self.header_table_offset,
            self.header_count,
            self.game_family,
        )?)
    }

    fn load_all_headers(&self) -> Result<Arc<Vec<MapHeader>>> {
        if let Some(headers) = self.headers_cache.get() {
            return Ok(headers.clone());
        }

        let init_guard = self
            .headers_init_lock
            .lock()
            .map_err(|_| UxieError::invalid_format("Arm9 header init lock poisoned"))?;
        if let Some(headers) = self.headers_cache.get() {
            return Ok(headers.clone());
        }

        let parsed = Arc::new(self.parse_all_headers()?);
        let _ = self.headers_cache.set(parsed.clone());
        drop(init_guard);

        Ok(parsed)
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

    fn get_text_archive_for_script_file(&self, script_file_id: u16) -> Result<Option<u16>> {
        let headers = self.load_all_headers()?;
        Ok(find_text_archive_in_headers(&headers, script_file_id))
    }

    fn find_maps_by_script_file_id(&self, script_file_id: u16) -> Result<Vec<u16>> {
        let headers = self.load_all_headers()?;
        Ok(find_maps_by_script_file_in_headers(
            &headers,
            script_file_id,
        ))
    }

    fn find_maps_by_level_script_file_id(&self, level_script_file_id: u16) -> Result<Vec<u16>> {
        let headers = self.load_all_headers()?;
        Ok(find_maps_by_level_script_in_headers(
            &headers,
            level_script_file_id,
        ))
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
    headers_cache: OnceLock<Arc<Vec<MapHeader>>>,
    headers_init_lock: Mutex<()>,
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
            headers_cache: OnceLock::new(),
            headers_init_lock: Mutex::new(()),
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

    fn parse_all_headers(&self) -> Result<Vec<MapHeader>> {
        let path = self.map_headers_path();
        let content = std::fs::read_to_string(path)?;
        let parsed = crate::map_header::parse_map_headers_from_c(&content);

        let mut headers = Vec::new();
        for p in parsed {
            let header = match self.family {
                GameFamily::HGSS => {
                    MapHeader::HGSS(crate::map_header::parsed_to_hgss_header(&p, &self.symbols)?)
                }
                GameFamily::DP | GameFamily::Platinum => {
                    MapHeader::Pt(crate::map_header::parsed_to_pt_header(&p, &self.symbols)?)
                }
            };
            headers.push(header);
        }
        Ok(headers)
    }

    fn load_all_headers(&self) -> Result<Arc<Vec<MapHeader>>> {
        if let Some(headers) = self.headers_cache.get() {
            return Ok(headers.clone());
        }

        let init_guard = self
            .headers_init_lock
            .lock()
            .map_err(|_| UxieError::invalid_format("Decomp header init lock poisoned"))?;
        if let Some(headers) = self.headers_cache.get() {
            return Ok(headers.clone());
        }

        let parsed = Arc::new(self.parse_all_headers()?);
        let _ = self.headers_cache.set(parsed.clone());
        drop(init_guard);

        Ok(parsed)
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

    fn get_text_archive_for_script_file(&self, script_file_id: u16) -> Result<Option<u16>> {
        let headers = self.load_all_headers()?;
        Ok(find_text_archive_in_headers(&headers, script_file_id))
    }

    fn find_maps_by_script_file_id(&self, script_file_id: u16) -> Result<Vec<u16>> {
        let headers = self.load_all_headers()?;
        Ok(find_maps_by_script_file_in_headers(
            &headers,
            script_file_id,
        ))
    }

    fn find_maps_by_level_script_file_id(&self, level_script_file_id: u16) -> Result<Vec<u16>> {
        let headers = self.load_all_headers()?;
        Ok(find_maps_by_level_script_in_headers(
            &headers,
            level_script_file_id,
        ))
    }
}

fn find_text_archive_in_headers(headers: &[MapHeader], script_file_id: u16) -> Option<u16> {
    for header in headers {
        if header.script_file_id() == script_file_id {
            return Some(header.text_archive_id());
        }
    }
    None
}

fn find_maps_by_script_file_in_headers(headers: &[MapHeader], script_file_id: u16) -> Vec<u16> {
    headers
        .iter()
        .enumerate()
        .filter_map(|(map_id, header)| {
            (header.script_file_id() == script_file_id).then_some(map_id as u16)
        })
        .collect()
}

fn find_maps_by_level_script_in_headers(
    headers: &[MapHeader],
    level_script_file_id: u16,
) -> Vec<u16> {
    headers
        .iter()
        .enumerate()
        .filter_map(|(map_id, header)| {
            (header.level_script_id() == level_script_file_id).then_some(map_id as u16)
        })
        .collect()
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
    use proptest::prelude::*;
    use std::fs;
    use std::io::{Seek, Write};
    use tempfile::NamedTempFile;

    fn create_test_pt_header(script_id: u16, text_id: u16) -> MapHeader {
        MapHeader::Pt(crate::map_header::MapHeaderPt {
            script_file_id: script_id,
            text_archive_id: text_id,
            ..Default::default()
        })
    }

    fn create_test_pt_header_with_level(
        script_id: u16,
        text_id: u16,
        level_script_id: u16,
    ) -> MapHeader {
        MapHeader::Pt(crate::map_header::MapHeaderPt {
            script_file_id: script_id,
            text_archive_id: text_id,
            level_script_id,
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
    fn test_arm9_provider_get_text_archive_for_script_file() {
        let headers = vec![
            create_test_pt_header(10, 100),
            create_test_pt_header(20, 200),
            create_test_pt_header(30, 300),
        ];
        let file = create_test_arm9_file(&headers);

        let provider = Arm9Provider::new(file.path(), 0, 3, GameFamily::Platinum);

        assert_eq!(
            provider.get_text_archive_for_script_file(20).unwrap(),
            Some(200)
        );
        assert_eq!(
            provider.get_text_archive_for_script_file(999).unwrap(),
            None
        );
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
    fn test_arm9_provider_find_maps_by_script_and_level_file_id() {
        let headers = vec![
            create_test_pt_header_with_level(10, 100, 1),
            create_test_pt_header_with_level(20, 200, 2),
            create_test_pt_header_with_level(10, 300, 2),
            create_test_pt_header_with_level(40, 400, 3),
        ];
        let file = create_test_arm9_file(&headers);

        let provider = Arm9Provider::new(file.path(), 0, 4, GameFamily::Platinum);

        assert_eq!(
            provider.find_maps_by_script_file_id(10).unwrap(),
            vec![0, 2]
        );
        assert_eq!(
            provider.find_maps_by_script_file_id(999).unwrap(),
            Vec::<u16>::new()
        );
        assert_eq!(
            provider.find_maps_by_level_script_file_id(2).unwrap(),
            vec![1, 2]
        );
        assert_eq!(
            provider.find_maps_by_level_script_file_id(999).unwrap(),
            Vec::<u16>::new()
        );
    }

    #[test]
    fn test_arm9_provider_caches_headers_after_first_full_table_load() {
        let initial_headers = vec![
            create_test_pt_header(10, 100),
            create_test_pt_header(20, 200),
            create_test_pt_header(30, 300),
        ];
        let updated_headers = vec![
            create_test_pt_header(10, 111),
            create_test_pt_header(20, 222),
            create_test_pt_header(30, 333),
        ];
        let mut file = create_test_arm9_file(&initial_headers);

        let provider = Arm9Provider::new(file.path(), 0, 3, GameFamily::Platinum);

        assert_eq!(
            provider.get_text_archive_for_script_file(20).unwrap(),
            Some(200)
        );

        {
            let inner = file.as_file_mut();
            inner.set_len(0).unwrap();
            inner.rewind().unwrap();
            for header in &updated_headers {
                let bytes = write_map_header_to_bytes(header);
                inner.write_all(&bytes).unwrap();
            }
            inner.flush().unwrap();
        }

        assert_eq!(
            provider.get_text_archive_for_script_file(20).unwrap(),
            Some(200)
        );
        assert_eq!(provider.find_maps_by_script_file_id(10).unwrap(), vec![0]);
        assert_eq!(
            provider.find_maps_by_level_script_file_id(0).unwrap(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn test_default_find_map_by_level_script_file_id_uses_first_match() {
        struct LevelOnlyProvider;

        impl DataProvider for LevelOnlyProvider {
            fn get_map_header(&self, id: u16) -> Result<MapHeader> {
                Err(UxieError::not_found("MapHeader", id.to_string()))
            }

            fn get_map_header_count(&self) -> Result<usize> {
                Ok(0)
            }

            fn get_text_archive_for_script_file(
                &self,
                _script_file_id: u16,
            ) -> Result<Option<u16>> {
                Ok(None)
            }

            fn find_maps_by_script_file_id(&self, _script_file_id: u16) -> Result<Vec<u16>> {
                Ok(Vec::new())
            }

            fn find_maps_by_level_script_file_id(
                &self,
                level_script_file_id: u16,
            ) -> Result<Vec<u16>> {
                match level_script_file_id {
                    5 => Ok(vec![2, 7]),
                    _ => Ok(Vec::new()),
                }
            }
        }

        let provider = LevelOnlyProvider;
        assert_eq!(
            provider.find_map_by_level_script_file_id(5).unwrap(),
            Some(2)
        );
        assert_eq!(
            provider.find_map_by_level_script_file_id(999).unwrap(),
            None
        );
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

    #[test]
    fn test_decomp_provider_invalid_platinum_header_value_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let header_path = dir.path().join("include/data/map_headers.h");
        fs::create_dir_all(header_path.parent().unwrap()).unwrap();
        fs::write(
            &header_path,
            r"
        [MAP_HEADER_BAD] = {
            .scriptsArchiveID = not-a-valid-literal,
        },
        ",
        )
        .unwrap();

        let provider = DecompProvider::new(dir.path(), SymbolTable::new(), GameFamily::Platinum);
        let err = provider.get_map_header(0).unwrap_err();
        assert!(err.to_string().contains("MAP_HEADER_BAD"));
        assert!(err.to_string().contains("scriptsArchiveID"));
    }

    #[test]
    fn test_decomp_provider_invalid_hgss_header_value_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let header_path = dir.path().join("src/data/map_headers.h");
        fs::create_dir_all(header_path.parent().unwrap()).unwrap();
        fs::write(
            &header_path,
            r"
        [MAP_HEADER_BAD_HG] = {
            .bikeAllowed = MAYBE_ENABLED,
        },
        ",
        )
        .unwrap();

        let provider = DecompProvider::new(dir.path(), SymbolTable::new(), GameFamily::HGSS);
        let err = provider.get_map_header(0).unwrap_err();
        assert!(err.to_string().contains("MAP_HEADER_BAD_HG"));
        assert!(err.to_string().contains("bikeAllowed"));
    }

    #[test]
    fn test_decomp_provider_does_not_cache_parse_failures() {
        let dir = tempfile::tempdir().unwrap();
        let header_path = dir.path().join("include/data/map_headers.h");
        fs::create_dir_all(header_path.parent().unwrap()).unwrap();
        fs::write(
            &header_path,
            r"
        [MAP_HEADER_BAD] = {
            .scriptsArchiveID = not-a-valid-literal,
        },
        ",
        )
        .unwrap();

        let provider = DecompProvider::new(dir.path(), SymbolTable::new(), GameFamily::Platinum);
        assert!(provider.get_map_header(0).is_err());

        fs::write(
            &header_path,
            r"
        [MAP_HEADER_GOOD] = {
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

        let header = provider.get_map_header(0).unwrap();
        assert_eq!(header.script_file_id(), 4);
    }

    #[test]
    fn test_decomp_provider_caches_headers_after_first_load() {
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

        let first = provider.get_map_header(0).unwrap();
        assert_eq!(first.script_file_id(), 4);

        fs::write(
            &header_path,
            r"
        [MAP_HEADER_TEST] = {
            .areaDataArchiveID = 1,
            .unk_01 = 2,
            .mapMatrixID = 3,
            .scriptsArchiveID = 99,
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

        let second = provider.get_map_header(0).unwrap();
        assert_eq!(second.script_file_id(), 4);
    }

    #[test]
    fn test_decomp_provider_query_helpers_cover_multi_match_and_missing() {
        let dir = tempfile::tempdir().unwrap();
        let header_path = dir.path().join("include/data/map_headers.h");
        fs::create_dir_all(header_path.parent().unwrap()).unwrap();
        fs::write(
            &header_path,
            r"
        [MAP_HEADER_A] = {
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
        [MAP_HEADER_B] = {
            .areaDataArchiveID = 21,
            .unk_01 = 22,
            .mapMatrixID = 23,
            .scriptsArchiveID = 4,
            .initScriptsArchiveID = 5,
            .msgArchiveID = 60,
            .dayMusicID = 27,
            .nightMusicID = 28,
            .wildEncountersArchiveID = 29,
            .eventsArchiveID = 30,
            .mapLabelTextID = 31,
            .mapLabelWindowID = 32,
            .weather = 33,
            .cameraType = 34,
            .mapType = 35,
            .battleBG = 36,
            .isBikeAllowed = TRUE,
            .isRunningAllowed = FALSE,
            .isEscapeRopeAllowed = TRUE,
            .isFlyAllowed = FALSE,
        },
        ",
        )
        .unwrap();

        let provider = DecompProvider::new(dir.path(), SymbolTable::new(), GameFamily::Platinum);

        assert_eq!(provider.get_map_header_count().unwrap(), 2);
        assert_eq!(
            provider.get_text_archive_for_script_file(4).unwrap(),
            Some(6)
        );
        assert_eq!(
            provider.get_text_archive_for_script_file(999).unwrap(),
            None
        );

        assert_eq!(provider.find_maps_by_script_file_id(4).unwrap(), vec![0, 1]);
        assert_eq!(
            provider.find_maps_by_script_file_id(999).unwrap(),
            Vec::<u16>::new()
        );

        assert_eq!(
            provider.find_maps_by_level_script_file_id(5).unwrap(),
            vec![0, 1]
        );
        assert_eq!(
            provider.find_maps_by_level_script_file_id(999).unwrap(),
            Vec::<u16>::new()
        );

        assert_eq!(
            provider.find_map_by_level_script_file_id(5).unwrap(),
            Some(0)
        );
        assert_eq!(
            provider.find_map_by_level_script_file_id(999).unwrap(),
            None
        );
    }

    #[test]
    #[ignore = "requires local DSPRE+decomp fixtures via UXIE_TEST_PLATINUM_DSPRE_PATH and UXIE_TEST_PLATINUM_DECOMP_PATH"]
    fn integration_platinum_provider_alignment_real_fixtures() {
        assert_provider_alignment_real_fixtures(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            GameFamily::Platinum,
            0xE601C,
            559,
            &[0, 3, 67, 150],
            "provider alignment integration test (Platinum)",
        );
    }

    #[test]
    #[ignore = "requires local DSPRE+decomp fixtures via UXIE_TEST_HGSS_DSPRE_PATH and UXIE_TEST_HGSS_DECOMP_PATH"]
    fn integration_hgss_provider_alignment_real_fixtures() {
        assert_provider_alignment_real_fixtures(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            "UXIE_TEST_HGSS_DECOMP_PATH",
            GameFamily::HGSS,
            0xF6BE0,
            540,
            &[0, 5, 36, 200],
            "provider alignment integration test (HGSS)",
        );
    }

    fn assert_provider_alignment_real_fixtures(
        dspre_env_var: &str,
        decomp_env_var: &str,
        family: GameFamily,
        arm9_offset: u64,
        expected_map_count: usize,
        sample_ids: &[u16],
        context: &str,
    ) {
        assert!(
            !sample_ids.is_empty(),
            "provider alignment sample set must not be empty"
        );

        let Some(arm9_path) = crate::test_env::existing_file_under_project_env(
            dspre_env_var,
            &["arm9.bin", "unpacked/arm9.bin", "arm9/arm9.bin"],
            context,
        ) else {
            return;
        };

        let Some(decomp_path) = crate::test_env::existing_path_from_env(decomp_env_var, context)
        else {
            return;
        };

        let dspre_provider = Arm9Provider::new(&arm9_path, arm9_offset, expected_map_count, family);
        let decomp_provider = DecompProvider::new(&decomp_path, SymbolTable::new(), family);

        let dspre_count = dspre_provider.get_map_header_count().unwrap();
        let decomp_count = decomp_provider.get_map_header_count().unwrap();
        assert_eq!(dspre_count, expected_map_count);
        assert_eq!(decomp_count, dspre_count);

        let mut checked_ids = sample_ids.to_vec();
        checked_ids.push((dspre_count.saturating_sub(1)) as u16);

        for map_id in checked_ids {
            let dspre = dspre_provider.get_map_header(map_id).unwrap();
            let decomp = decomp_provider.get_map_header(map_id).unwrap();

            assert_eq!(
                decomp.script_file_id(),
                dspre.script_file_id(),
                "script_file_id mismatch for map {}",
                map_id
            );
            assert_eq!(
                decomp.level_script_id(),
                dspre.level_script_id(),
                "level_script_id mismatch for map {}",
                map_id
            );
            assert_eq!(
                decomp.text_archive_id(),
                dspre.text_archive_id(),
                "text_archive_id mismatch for map {}",
                map_id
            );
            assert_eq!(
                decomp.event_file_id(),
                dspre.event_file_id(),
                "event_file_id mismatch for map {}",
                map_id
            );
        }

        let script_file_id = dspre_provider
            .get_map_header(sample_ids[0])
            .unwrap()
            .script_file_id();
        assert_eq!(
            decomp_provider
                .find_map_by_script_file_id(script_file_id)
                .unwrap(),
            dspre_provider
                .find_map_by_script_file_id(script_file_id)
                .unwrap()
        );
    }

    fn pt_headers_strategy() -> impl Strategy<Value = Vec<MapHeader>> {
        prop::collection::vec((any::<u16>(), any::<u16>(), any::<u16>()), 0..64).prop_map(
            |triples| {
                triples
                    .into_iter()
                    .map(|(script_id, text_id, level_script_id)| {
                        create_test_pt_header_with_level(script_id, text_id, level_script_id)
                    })
                    .collect()
            },
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_find_headers_using_script_matches_manual(headers in pt_headers_strategy(), script_id in any::<u16>()) {
            let expected: Vec<usize> = headers
                .iter()
                .enumerate()
                .filter_map(|(idx, h)| (h.script_file_id() == script_id).then_some(idx))
                .collect();
            let actual = find_headers_using_script(&headers, script_id);
            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn prop_find_headers_using_text_matches_manual(headers in pt_headers_strategy(), text_id in any::<u16>()) {
            let expected: Vec<usize> = headers
                .iter()
                .enumerate()
                .filter_map(|(idx, h)| (h.text_archive_id() == text_id).then_some(idx))
                .collect();
            let actual = find_headers_using_text(&headers, text_id);
            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn prop_find_map_by_script_file_returns_first_match(headers in pt_headers_strategy(), script_id in any::<u16>()) {
            let matches = find_maps_by_script_file_in_headers(&headers, script_id);
            let expected = headers
                .iter()
                .enumerate()
                .find_map(|(idx, h)| (h.script_file_id() == script_id).then_some(idx as u16));
            let actual = matches.first().copied();
            prop_assert_eq!(matches.first().copied(), expected);
            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn prop_find_map_by_level_script_returns_first_match(headers in pt_headers_strategy(), level_script_id in any::<u16>()) {
            let matches = find_maps_by_level_script_in_headers(&headers, level_script_id);
            let expected = headers
                .iter()
                .enumerate()
                .find_map(|(idx, h)| (h.level_script_id() == level_script_id).then_some(idx as u16));
            let actual = matches.first().copied();
            prop_assert_eq!(matches.first().copied(), expected);
            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn prop_find_maps_by_script_file_matches_manual(headers in pt_headers_strategy(), script_id in any::<u16>()) {
            let expected: Vec<u16> = headers
                .iter()
                .enumerate()
                .filter_map(|(idx, h)| (h.script_file_id() == script_id).then_some(idx as u16))
                .collect();
            let actual = find_maps_by_script_file_in_headers(&headers, script_id);
            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn prop_find_maps_by_level_script_file_matches_manual(headers in pt_headers_strategy(), level_script_id in any::<u16>()) {
            let expected: Vec<u16> = headers
                .iter()
                .enumerate()
                .filter_map(|(idx, h)| (h.level_script_id() == level_script_id).then_some(idx as u16))
                .collect();
            let actual = find_maps_by_level_script_in_headers(&headers, level_script_id);
            prop_assert_eq!(actual, expected);
        }
    }
}
