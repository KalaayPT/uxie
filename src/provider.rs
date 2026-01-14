//! Data provider abstraction for accessing ROM data from different sources
//!
//! This module defines the [`DataProvider`] trait and concrete implementations
//! for different project types:
//! - [`Arm9Provider`]: Reads from ARM9 binary files (DSPRE projects)
//! - [`DecompProvider`]: Reads from decompilation source files

use crate::game::GameFamily;
use crate::map_header::{
    MAP_HEADER_SIZE, MapHeader, read_map_header_from_bytes, read_map_headers_from_arm9,
};
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

/// Trait for accessing ROM data regardless of source format
///
/// Implementations provide unified access to map headers and related data
/// from different project types (DSPRE, decompilation, etc.).
pub trait DataProvider {
    /// Get a specific map header by ID
    fn get_map_header(&self, id: u16) -> io::Result<MapHeader>;
    /// Get the total number of map headers
    fn get_map_header_count(&self) -> io::Result<usize>;
    /// Get the text archive ID associated with a script file
    fn get_text_archive_for_script(&self, script_id: u16) -> io::Result<Option<u16>>;
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
/// # Ok::<(), std::io::Error>(())
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

    fn read_all_headers(&self) -> io::Result<Vec<MapHeader>> {
        let mut file = File::open(&self.arm9_path)?;
        read_map_headers_from_arm9(
            &mut file,
            self.header_table_offset,
            self.header_count,
            self.game_family,
        )
    }
}

impl DataProvider for Arm9Provider {
    fn get_map_header(&self, id: u16) -> io::Result<MapHeader> {
        if id as usize >= self.header_count {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Header ID {} out of range (max {})",
                    id,
                    self.header_count - 1
                ),
            ));
        }

        use std::io::{Read, Seek, SeekFrom};
        let mut file = File::open(&self.arm9_path)?;
        let offset = self.header_table_offset + (id as u64 * MAP_HEADER_SIZE as u64);
        file.seek(SeekFrom::Start(offset))?;

        let mut buf = [0u8; MAP_HEADER_SIZE];
        file.read_exact(&mut buf)?;
        read_map_header_from_bytes(&buf, self.game_family)
    }

    fn get_map_header_count(&self) -> io::Result<usize> {
        Ok(self.header_count)
    }

    fn get_text_archive_for_script(&self, script_id: u16) -> io::Result<Option<u16>> {
        let headers = self.read_all_headers()?;
        for header in headers {
            if header.script_file_id() == script_id {
                return Ok(Some(header.text_archive_id()));
            }
        }
        Ok(None)
    }
}

/// Provider for reading map data from decompilation source files
///
/// Parses C header files and JSON data from pokeplatinum/pokeheartgold
/// decompilation projects.
pub struct DecompProvider {
    pub root: PathBuf,
    pub symbols: crate::c_parser::SymbolTable,
}

impl DecompProvider {
    /// Create a new decompilation provider
    ///
    /// # Arguments
    ///
    /// * `root` - Root directory of the decompilation project
    /// * `symbols` - Pre-loaded symbol table with project constants
    pub fn new(root: impl AsRef<Path>, symbols: crate::c_parser::SymbolTable) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            symbols,
        }
    }

    fn load_all_headers(&self) -> io::Result<Vec<MapHeader>> {
        let path = self.root.join("include/data/map_headers.h");
        let content = std::fs::read_to_string(path)?;
        let parsed = crate::map_header::parse_map_headers_from_c(&content);

        let mut headers = Vec::new();
        for p in parsed {
            headers.push(MapHeader::Pt(crate::map_header::parsed_to_pt_header(
                &p,
                &self.symbols,
            )));
        }
        Ok(headers)
    }
}

impl DataProvider for DecompProvider {
    fn get_map_header(&self, id: u16) -> io::Result<MapHeader> {
        let headers = self.load_all_headers()?;
        headers
            .get(id as usize)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Header ID out of range"))
    }

    fn get_map_header_count(&self) -> io::Result<usize> {
        let headers = self.load_all_headers()?;
        Ok(headers.len())
    }

    fn get_text_archive_for_script(&self, script_id: u16) -> io::Result<Option<u16>> {
        let headers = self.load_all_headers()?;
        for header in headers {
            if header.script_file_id() == script_id {
                return Ok(Some(header.text_archive_id()));
            }
        }
        Ok(None)
    }
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
