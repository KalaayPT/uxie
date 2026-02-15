//! Script ID resolution for Gen 4 Pokémon games.
//!
//! This module resolves script IDs to their containing files and associated
//! metadata (text archives, event files, etc.). It handles both local scripts
//! (ID 0-1999) that belong to specific maps and common/global scripts (ID 2000+)
//! that are shared across the game.

use crate::error::Result;
use crate::provider::DataProvider;
use crate::script_file::GlobalScriptTable;

/// Script IDs at or above this threshold are common/global scripts.
///
/// Local scripts use IDs 0-1999, while common scripts use 2000+.
pub const COMMON_SCRIPT_THRESHOLD: u16 = 2000;

/// Basic information about a script file.
///
/// Contains the file ID and associated text archive, without map-specific data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptFileInfo {
    /// Index into the script NARC (`scr_seq.narc`).
    pub script_file_id: u16,
    /// Index into the message NARC (`msg.narc`).
    pub text_archive_id: u16,
}

/// Complete script information for a map.
///
/// Contains all file references from a map header related to scripts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapScriptInfo {
    /// The map ID this information belongs to.
    pub map_id: u16,
    /// Index into the script NARC for regular scripts.
    pub script_file_id: u16,
    /// Index into the message NARC for text strings.
    pub text_archive_id: u16,
    /// Index into the event NARC for NPC/trigger definitions.
    pub event_file_id: u16,
    /// Index into the script NARC for level/init scripts.
    pub level_script_id: u16,
}

/// Result of resolving a script ID to its containing file and metadata.
///
/// Scripts are either common (global) scripts shared across the game,
/// or map-specific scripts that belong to a particular location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptResolution {
    /// A common/global script (ID >= 2000).
    CommonScript {
        /// The original script ID that was resolved.
        script_id: u16,
        /// Index into the script NARC containing this script.
        script_file_id: u16,
        /// Index into the message NARC for text strings.
        text_archive_id: u16,
    },
    /// A map-specific script (ID 0-1999).
    MapScript {
        /// The original script ID that was resolved.
        script_id: u16,
        /// The map this script belongs to.
        map_id: u16,
        /// Index into the script NARC containing this script.
        script_file_id: u16,
        /// Index into the message NARC for text strings.
        text_archive_id: u16,
        /// Index into the event NARC for NPC/trigger definitions.
        event_file_id: u16,
    },
}

impl ScriptResolution {
    /// Returns the script file ID regardless of resolution type.
    pub fn script_file_id(&self) -> u16 {
        match self {
            ScriptResolution::CommonScript { script_file_id, .. } => *script_file_id,
            ScriptResolution::MapScript { script_file_id, .. } => *script_file_id,
        }
    }

    /// Returns the text archive ID regardless of resolution type.
    pub fn text_archive_id(&self) -> u16 {
        match self {
            ScriptResolution::CommonScript {
                text_archive_id, ..
            } => *text_archive_id,
            ScriptResolution::MapScript {
                text_archive_id, ..
            } => *text_archive_id,
        }
    }

    /// Returns `true` if this is a common/global script.
    pub fn is_common_script(&self) -> bool {
        matches!(self, ScriptResolution::CommonScript { .. })
    }

    /// Returns `true` if this is a map-specific script.
    pub fn is_map_script(&self) -> bool {
        matches!(self, ScriptResolution::MapScript { .. })
    }

    /// Returns the event file ID if this is a map script.
    pub fn event_file_id(&self) -> Option<u16> {
        match self {
            ScriptResolution::CommonScript { .. } => None,
            ScriptResolution::MapScript { event_file_id, .. } => Some(*event_file_id),
        }
    }

    /// Returns the map ID if this is a map script.
    pub fn map_id(&self) -> Option<u16> {
        match self {
            ScriptResolution::CommonScript { .. } => None,
            ScriptResolution::MapScript { map_id, .. } => Some(*map_id),
        }
    }
}

/// Returns `true` if the script ID is a common/global script (>= 2000).
pub fn is_common_script_id(script_id: u16) -> bool {
    script_id >= COMMON_SCRIPT_THRESHOLD
}

/// Resolves a script ID to its file and metadata.
///
/// Use this when you know the current map ID. For common scripts (ID >= 2000),
/// the map_id is ignored and resolution uses the global table.
///
/// Returns `None` if:
/// - The script is a common script not found in the global table
/// - The script is local but no map_id was provided
pub fn resolve_script_id(
    script_id: u16,
    map_id: Option<u16>,
    global_table: &GlobalScriptTable,
    provider: &dyn DataProvider,
) -> Result<Option<ScriptResolution>> {
    if is_common_script_id(script_id) {
        Ok(resolve_common_script(script_id, global_table))
    } else {
        resolve_map_script(script_id, map_id, provider)
    }
}

/// Resolves a script ID when you're in a regular script file.
///
/// Use this when processing a script file and you encounter a script call.
/// The function finds the map that uses this script file to resolve local scripts.
///
/// Returns `None` if:
/// - The script is a common script not found in the global table
/// - The script is local but no map uses this script file
pub fn resolve_script_id_by_file(
    script_id: u16,
    script_file_id: u16,
    global_table: &GlobalScriptTable,
    provider: &dyn DataProvider,
) -> Result<Option<ScriptResolution>> {
    if is_common_script_id(script_id) {
        Ok(resolve_common_script(script_id, global_table))
    } else {
        let map_id = first_map_for_script_file(script_file_id, provider)?;
        resolve_map_script(script_id, map_id, provider)
    }
}

/// Resolves a script ID when you're in a level/init script file.
///
/// Level scripts can call local scripts that live in the map's *regular*
/// script file (not the level script file). This function handles that
/// indirection by finding the map and returning its regular script file.
///
/// Returns `None` if:
/// - The script is a common script not found in the global table
/// - No map uses this level script file
pub fn resolve_script_id_by_level_script_file(
    script_id: u16,
    level_script_file_id: u16,
    global_table: &GlobalScriptTable,
    provider: &dyn DataProvider,
) -> Result<Option<ScriptResolution>> {
    if is_common_script_id(script_id) {
        Ok(resolve_common_script(script_id, global_table))
    } else {
        let map_id = first_map_for_level_script_file(level_script_file_id, provider)?;
        resolve_map_script(script_id, map_id, provider)
    }
}

fn resolve_common_script(
    script_id: u16,
    global_table: &GlobalScriptTable,
) -> Option<ScriptResolution> {
    global_table
        .lookup(script_id)
        .map(|e| ScriptResolution::CommonScript {
            script_id,
            script_file_id: e.script_file_id,
            text_archive_id: e.text_archive_id,
        })
}

fn first_map_for_script_file(
    script_file_id: u16,
    provider: &dyn DataProvider,
) -> Result<Option<u16>> {
    Ok(provider
        .find_maps_by_script_file_id(script_file_id)?
        .into_iter()
        .next())
}

fn first_map_for_level_script_file(
    level_script_file_id: u16,
    provider: &dyn DataProvider,
) -> Result<Option<u16>> {
    Ok(provider
        .find_maps_by_level_script_file_id(level_script_file_id)?
        .into_iter()
        .next())
}

fn resolve_map_script(
    script_id: u16,
    map_id: Option<u16>,
    provider: &dyn DataProvider,
) -> Result<Option<ScriptResolution>> {
    let Some(map_id) = map_id else {
        return Ok(None);
    };

    let header = provider.get_map_header(map_id)?;
    Ok(Some(ScriptResolution::MapScript {
        script_id,
        map_id,
        script_file_id: header.script_file_id(),
        text_archive_id: header.text_archive_id(),
        event_file_id: header.event_file_id(),
    }))
}

/// Resolves a map's level/init script to its file and metadata.
///
/// Level scripts run automatically on map transitions. This returns
/// information about where the level script is located.
pub fn resolve_level_script(
    map_id: u16,
    global_table: &GlobalScriptTable,
    provider: &dyn DataProvider,
) -> Result<Option<ScriptResolution>> {
    let header = provider.get_map_header(map_id)?;
    let level_script_id = header.level_script_id();

    if is_common_script_id(level_script_id) {
        Ok(resolve_common_script(level_script_id, global_table))
    } else {
        Ok(Some(ScriptResolution::MapScript {
            script_id: level_script_id,
            map_id,
            script_file_id: header.script_file_id(),
            text_archive_id: header.text_archive_id(),
            event_file_id: header.event_file_id(),
        }))
    }
}

/// Resolves level script information given only a level script file ID.
///
/// Finds the map that uses this level script file and returns its resolution.
/// Returns `None` if no map uses this level script file.
pub fn resolve_level_script_by_file(
    level_script_file_id: u16,
    global_table: &GlobalScriptTable,
    provider: &dyn DataProvider,
) -> Result<Option<ScriptResolution>> {
    let map_id = first_map_for_level_script_file(level_script_file_id, provider)?;
    let map_id = match map_id {
        Some(id) => id,
        None => return Ok(None),
    };

    resolve_level_script(map_id, global_table, provider)
}

/// Gets complete script file information for a map.
///
/// Returns all script-related file IDs from the map header.
pub fn get_script_file_info_for_map(
    map_id: u16,
    provider: &dyn DataProvider,
) -> Result<MapScriptInfo> {
    let header = provider.get_map_header(map_id)?;
    Ok(MapScriptInfo {
        map_id,
        script_file_id: header.script_file_id(),
        text_archive_id: header.text_archive_id(),
        event_file_id: header.event_file_id(),
        level_script_id: header.level_script_id(),
    })
}

/// Gets file information for a common/global script ID.
///
/// Returns `None` if the script ID is local (< 2000) or not found in the table.
pub fn get_common_script_info(
    script_id: u16,
    global_table: &GlobalScriptTable,
) -> Option<ScriptFileInfo> {
    if !is_common_script_id(script_id) {
        return None;
    }

    global_table.lookup(script_id).map(|e| ScriptFileInfo {
        script_file_id: e.script_file_id,
        text_archive_id: e.text_archive_id,
    })
}

/// Finds all maps that use a given script file.
///
/// Multiple maps can share the same script file. This returns all map IDs
/// that reference the given script file ID in their headers.
pub fn find_maps_for_script_file(
    script_file_id: u16,
    provider: &dyn DataProvider,
) -> Result<Vec<u16>> {
    provider.find_maps_by_script_file_id(script_file_id)
}

/// Finds all maps that use a given level script file.
///
/// Multiple maps can share the same level script file. This returns all
/// map IDs that reference the given level script file ID in their headers.
pub fn find_maps_for_level_script_file(
    level_script_file_id: u16,
    provider: &dyn DataProvider,
) -> Result<Vec<u16>> {
    provider.find_maps_by_level_script_file_id(level_script_file_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_header::{MapHeader, MapHeaderPt};
    use crate::script_file::GlobalScriptEntry;
    use proptest::prelude::*;

    struct MockProvider {
        headers: Vec<MapHeader>,
    }

    impl DataProvider for MockProvider {
        fn get_map_header(&self, id: u16) -> Result<MapHeader> {
            self.headers
                .get(id as usize)
                .cloned()
                .ok_or_else(|| crate::error::UxieError::not_found("MapHeader", id.to_string()))
        }

        fn get_map_header_count(&self) -> Result<usize> {
            Ok(self.headers.len())
        }

        fn get_text_archive_for_script_file(&self, script_file_id: u16) -> Result<Option<u16>> {
            for header in &self.headers {
                if header.script_file_id() == script_file_id {
                    return Ok(Some(header.text_archive_id()));
                }
            }
            Ok(None)
        }

        fn find_map_by_script_file_id(&self, script_file_id: u16) -> Result<Option<u16>> {
            for (map_id, header) in self.headers.iter().enumerate() {
                if header.script_file_id() == script_file_id {
                    return Ok(Some(map_id as u16));
                }
            }
            Ok(None)
        }

        fn find_maps_by_script_file_id(&self, script_file_id: u16) -> Result<Vec<u16>> {
            Ok(self
                .headers
                .iter()
                .enumerate()
                .filter_map(|(map_id, header)| {
                    (header.script_file_id() == script_file_id).then_some(map_id as u16)
                })
                .collect())
        }

        fn find_map_by_level_script_file_id(
            &self,
            level_script_file_id: u16,
        ) -> Result<Option<u16>> {
            for (map_id, header) in self.headers.iter().enumerate() {
                if header.level_script_id() == level_script_file_id {
                    return Ok(Some(map_id as u16));
                }
            }
            Ok(None)
        }

        fn find_maps_by_level_script_file_id(&self, level_script_file_id: u16) -> Result<Vec<u16>> {
            Ok(self
                .headers
                .iter()
                .enumerate()
                .filter_map(|(map_id, header)| {
                    (header.level_script_id() == level_script_file_id).then_some(map_id as u16)
                })
                .collect())
        }
    }

    fn create_pt_header(
        script_file: u16,
        text_archive: u16,
        event_file: u16,
        level_script_id: u16,
    ) -> MapHeader {
        MapHeader::Pt(MapHeaderPt {
            script_file_id: script_file,
            text_archive_id: text_archive,
            event_file_id: event_file,
            level_script_id,
            ..Default::default()
        })
    }

    #[test]
    fn test_is_common_script_id() {
        assert!(!is_common_script_id(0));
        assert!(!is_common_script_id(1));
        assert!(!is_common_script_id(1999));
        assert!(is_common_script_id(2000));
        assert!(is_common_script_id(2001));
        assert!(is_common_script_id(10000));
    }

    #[test]
    fn test_resolve_common_script() {
        let table = GlobalScriptTable::from_entries(vec![
            GlobalScriptEntry::new(2000, 211, 213),
            GlobalScriptEntry::new(2500, 212, 214),
        ]);

        let result = resolve_common_script(2018, &table);
        assert!(result.is_some());

        let resolution = result.unwrap();
        assert!(resolution.is_common_script());
        assert!(!resolution.is_map_script());
        assert_eq!(resolution.script_file_id(), 211);
        assert_eq!(resolution.text_archive_id(), 213);
    }

    #[test]
    fn test_resolve_map_script_without_map_id() {
        let provider = MockProvider { headers: vec![] };

        let result = resolve_map_script(5, None, &provider).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_map_script_with_map_id() {
        let provider = MockProvider {
            headers: vec![create_pt_header(100, 200, 50, 1)],
        };

        let result = resolve_map_script(5, Some(0), &provider).unwrap();
        assert!(result.is_some());

        let resolution = result.unwrap();
        assert!(resolution.is_map_script());
        assert!(!resolution.is_common_script());
        assert_eq!(resolution.script_file_id(), 100);
        assert_eq!(resolution.text_archive_id(), 200);
        assert_eq!(resolution.event_file_id(), Some(50));
        assert_eq!(resolution.map_id(), Some(0));
    }

    #[test]
    fn test_resolve_script_id_routes_correctly() {
        let table = GlobalScriptTable::from_entries(vec![GlobalScriptEntry::new(2000, 211, 213)]);
        let provider = MockProvider {
            headers: vec![create_pt_header(100, 200, 50, 1)],
        };

        let common_result = resolve_script_id(2050, None, &table, &provider).unwrap();
        assert!(common_result.is_some());
        assert!(common_result.unwrap().is_common_script());

        let map_result = resolve_script_id(5, Some(0), &table, &provider).unwrap();
        assert!(map_result.is_some());
        assert!(map_result.unwrap().is_map_script());

        let no_context = resolve_script_id(5, None, &table, &provider).unwrap();
        assert!(no_context.is_none());
    }

    #[test]
    fn test_find_maps_for_script_file() {
        let provider = MockProvider {
            headers: vec![
                create_pt_header(100, 200, 50, 1),
                create_pt_header(101, 201, 51, 1),
                create_pt_header(100, 202, 52, 1),
                create_pt_header(102, 203, 53, 1),
            ],
        };

        let maps = find_maps_for_script_file(100, &provider).unwrap();
        assert_eq!(maps, vec![0, 2]);

        let maps = find_maps_for_script_file(101, &provider).unwrap();
        assert_eq!(maps, vec![1]);

        let maps = find_maps_for_script_file(999, &provider).unwrap();
        assert!(maps.is_empty());
    }

    #[test]
    fn test_find_maps_for_level_script_file() {
        let provider = MockProvider {
            headers: vec![
                create_pt_header(100, 200, 50, 500),
                create_pt_header(101, 201, 51, 501),
                create_pt_header(102, 202, 52, 500),
            ],
        };

        let maps = find_maps_for_level_script_file(500, &provider).unwrap();
        assert_eq!(maps, vec![0, 2]);

        let maps = find_maps_for_level_script_file(501, &provider).unwrap();
        assert_eq!(maps, vec![1]);

        let maps = find_maps_for_level_script_file(999, &provider).unwrap();
        assert!(maps.is_empty());
    }

    #[test]
    fn test_get_common_script_info() {
        let table = GlobalScriptTable::from_entries(vec![GlobalScriptEntry::new(2000, 211, 213)]);

        let info = get_common_script_info(2050, &table);
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.script_file_id, 211);
        assert_eq!(info.text_archive_id, 213);

        let info = get_common_script_info(500, &table);
        assert!(info.is_none());
    }

    #[test]
    fn test_resolve_level_script_common() {
        let table = GlobalScriptTable::from_entries(vec![GlobalScriptEntry::new(2000, 211, 213)]);
        let provider = MockProvider {
            headers: vec![MapHeader::Pt(MapHeaderPt {
                script_file_id: 100,
                text_archive_id: 200,
                event_file_id: 50,
                level_script_id: 2050,
                ..Default::default()
            })],
        };

        let result = resolve_level_script(0, &table, &provider).unwrap();
        assert!(result.is_some());

        let resolution = result.unwrap();
        assert!(resolution.is_common_script());
        assert_eq!(resolution.script_file_id(), 211);
    }

    #[test]
    fn test_resolve_level_script_local() {
        let table = GlobalScriptTable::new();
        let provider = MockProvider {
            headers: vec![MapHeader::Pt(MapHeaderPt {
                script_file_id: 100,
                text_archive_id: 200,
                event_file_id: 50,
                level_script_id: 5,
                ..Default::default()
            })],
        };

        let result = resolve_level_script(0, &table, &provider).unwrap();
        assert!(result.is_some());

        let resolution = result.unwrap();
        assert!(resolution.is_map_script());
        assert_eq!(resolution.script_file_id(), 100);
    }

    #[test]
    fn test_resolve_script_id_by_file_common() {
        let table = GlobalScriptTable::from_entries(vec![GlobalScriptEntry::new(2000, 211, 213)]);
        let provider = MockProvider {
            headers: vec![create_pt_header(100, 200, 50, 1)],
        };

        let result = resolve_script_id_by_file(2000, 211, &table, &provider).unwrap();
        assert!(result.is_some());

        let resolution = result.unwrap();
        assert!(resolution.is_common_script());
        assert_eq!(resolution.script_file_id(), 211);
        assert_eq!(resolution.text_archive_id(), 213);
    }

    #[test]
    fn test_resolve_script_id_by_file_map() {
        let table = GlobalScriptTable::new();
        let provider = MockProvider {
            headers: vec![
                create_pt_header(100, 200, 50, 1),
                create_pt_header(101, 201, 51, 1),
            ],
        };

        let result = resolve_script_id_by_file(5, 101, &table, &provider).unwrap();
        assert!(result.is_some());

        let resolution = result.unwrap();
        assert!(resolution.is_map_script());
        assert_eq!(resolution.script_file_id(), 101);
        assert_eq!(resolution.text_archive_id(), 201);
        assert_eq!(resolution.event_file_id(), Some(51));
        assert_eq!(resolution.map_id(), Some(1));
    }

    #[test]
    fn test_resolve_script_id_by_file_not_found() {
        let table = GlobalScriptTable::new();
        let provider = MockProvider {
            headers: vec![create_pt_header(100, 200, 50, 1)],
        };

        let result = resolve_script_id_by_file(5, 999, &table, &provider).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_level_script_by_file_common() {
        let table = GlobalScriptTable::from_entries(vec![GlobalScriptEntry::new(2050, 211, 213)]);
        let provider = MockProvider {
            headers: vec![MapHeader::Pt(MapHeaderPt {
                script_file_id: 100,
                text_archive_id: 200,
                event_file_id: 50,
                level_script_id: 500,
                ..Default::default()
            })],
        };

        let result = resolve_level_script_by_file(500, &table, &provider).unwrap();
        assert!(result.is_some());

        let resolution = result.unwrap();
        assert!(resolution.is_map_script());
        assert_eq!(resolution.script_file_id(), 100);
        assert_eq!(resolution.text_archive_id(), 200);
    }

    #[test]
    fn test_resolve_level_script_by_file_local() {
        let table = GlobalScriptTable::new();
        let provider = MockProvider {
            headers: vec![MapHeader::Pt(MapHeaderPt {
                script_file_id: 100,
                text_archive_id: 200,
                event_file_id: 50,
                level_script_id: 500,
                ..Default::default()
            })],
        };

        let result = resolve_level_script_by_file(500, &table, &provider).unwrap();
        assert!(result.is_some());

        let resolution = result.unwrap();
        assert!(resolution.is_map_script());
        assert_eq!(resolution.script_file_id(), 100);
        assert_eq!(resolution.map_id(), Some(0));
    }

    #[test]
    fn test_resolve_level_script_by_file_not_found() {
        let table = GlobalScriptTable::new();
        let provider = MockProvider {
            headers: vec![MapHeader::Pt(MapHeaderPt {
                script_file_id: 100,
                text_archive_id: 200,
                event_file_id: 50,
                level_script_id: 500,
                ..Default::default()
            })],
        };

        let result = resolve_level_script_by_file(999, &table, &provider).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_level_script_by_file_level_script_is_common() {
        let table = GlobalScriptTable::from_entries(vec![GlobalScriptEntry::new(2050, 211, 213)]);
        let provider = MockProvider {
            headers: vec![MapHeader::Pt(MapHeaderPt {
                script_file_id: 100,
                text_archive_id: 200,
                event_file_id: 50,
                level_script_id: 2050,
                ..Default::default()
            })],
        };

        let result = resolve_level_script_by_file(2050, &table, &provider).unwrap();
        assert!(result.is_some());

        let resolution = result.unwrap();
        assert!(resolution.is_common_script());
        assert_eq!(resolution.script_file_id(), 211);
    }

    #[test]
    fn test_resolve_script_id_by_level_script_file_common() {
        let table = GlobalScriptTable::from_entries(vec![GlobalScriptEntry::new(2050, 211, 213)]);
        let provider = MockProvider {
            headers: vec![MapHeader::Pt(MapHeaderPt {
                script_file_id: 100,
                text_archive_id: 200,
                event_file_id: 50,
                level_script_id: 500,
                ..Default::default()
            })],
        };

        let result = resolve_script_id_by_level_script_file(2050, 500, &table, &provider).unwrap();
        assert!(result.is_some());

        let resolution = result.unwrap();
        assert!(resolution.is_common_script());
        assert_eq!(resolution.script_file_id(), 211);
        assert_eq!(resolution.text_archive_id(), 213);
    }

    #[test]
    fn test_resolve_script_id_by_level_script_file_local() {
        let table = GlobalScriptTable::new();
        let provider = MockProvider {
            headers: vec![MapHeader::Pt(MapHeaderPt {
                script_file_id: 100,
                text_archive_id: 200,
                event_file_id: 50,
                level_script_id: 500,
                ..Default::default()
            })],
        };

        let result = resolve_script_id_by_level_script_file(5, 500, &table, &provider).unwrap();
        assert!(result.is_some());

        let resolution = result.unwrap();
        assert!(resolution.is_map_script());
        assert_eq!(resolution.script_file_id(), 100);
        assert_eq!(resolution.text_archive_id(), 200);
        assert_eq!(resolution.event_file_id(), Some(50));
        assert_eq!(resolution.map_id(), Some(0));
    }

    #[test]
    fn test_resolve_script_id_by_level_script_file_not_found() {
        let table = GlobalScriptTable::new();
        let provider = MockProvider {
            headers: vec![MapHeader::Pt(MapHeaderPt {
                script_file_id: 100,
                text_archive_id: 200,
                event_file_id: 50,
                level_script_id: 500,
                ..Default::default()
            })],
        };

        let result = resolve_script_id_by_level_script_file(5, 999, &table, &provider).unwrap();
        assert!(result.is_none());
    }

    fn map_header_strategy() -> impl Strategy<Value = MapHeader> {
        (any::<u16>(), any::<u16>(), any::<u16>(), any::<u16>()).prop_map(
            |(script_file, text_archive, event_file, level_script_id)| {
                create_pt_header(script_file, text_archive, event_file, level_script_id)
            },
        )
    }

    fn headers_strategy() -> impl Strategy<Value = Vec<MapHeader>> {
        prop::collection::vec(map_header_strategy(), 0..64)
    }

    fn global_entries_strategy() -> impl Strategy<Value = Vec<GlobalScriptEntry>> {
        prop::collection::btree_map(
            COMMON_SCRIPT_THRESHOLD..=u16::MAX,
            (any::<u16>(), any::<u16>()),
            0..32,
        )
        .prop_map(|mapping| {
            mapping
                .into_iter()
                .map(|(min_script_id, (script_file_id, text_archive_id))| {
                    GlobalScriptEntry::new(min_script_id, script_file_id, text_archive_id)
                })
                .collect()
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_find_maps_for_script_file_matches_manual(
            headers in headers_strategy(),
            script_file_id in any::<u16>()
        ) {
            let provider = MockProvider {
                headers: headers.clone(),
            };
            let expected: Vec<u16> = headers
                .iter()
                .enumerate()
                .filter_map(|(idx, h)| (h.script_file_id() == script_file_id).then_some(idx as u16))
                .collect();
            let actual = find_maps_for_script_file(script_file_id, &provider).unwrap();
            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn prop_find_maps_for_level_script_file_matches_manual(
            headers in headers_strategy(),
            level_script_file_id in any::<u16>()
        ) {
            let provider = MockProvider {
                headers: headers.clone(),
            };
            let expected: Vec<u16> = headers
                .iter()
                .enumerate()
                .filter_map(|(idx, h)| (h.level_script_id() == level_script_file_id).then_some(idx as u16))
                .collect();
            let actual = find_maps_for_level_script_file(level_script_file_id, &provider).unwrap();
            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn prop_common_resolution_apis_consistent(
            headers in headers_strategy(),
            entries in global_entries_strategy(),
            common_script_id in COMMON_SCRIPT_THRESHOLD..=u16::MAX,
            script_file_id in any::<u16>(),
            level_script_file_id in any::<u16>()
        ) {
            let provider = MockProvider { headers };
            let table = GlobalScriptTable::from_entries(entries);

            let expected = resolve_common_script(common_script_id, &table);
            let direct = resolve_script_id(common_script_id, None, &table, &provider).unwrap();
            let by_file = resolve_script_id_by_file(common_script_id, script_file_id, &table, &provider).unwrap();
            let by_level_file = resolve_script_id_by_level_script_file(
                common_script_id,
                level_script_file_id,
                &table,
                &provider,
            ).unwrap();

            prop_assert_eq!(direct, expected.clone());
            prop_assert_eq!(by_file, expected.clone());
            prop_assert_eq!(by_level_file, expected);
        }

        #[test]
        fn prop_local_resolution_by_file_matches_direct_map_resolution(
            headers in headers_strategy(),
            entries in global_entries_strategy(),
            local_script_id in 0u16..COMMON_SCRIPT_THRESHOLD,
            script_file_id in any::<u16>()
        ) {
            let table = GlobalScriptTable::from_entries(entries);
            let map_id = headers
                .iter()
                .enumerate()
                .find_map(|(idx, h)| (h.script_file_id() == script_file_id).then_some(idx as u16));
            let provider = MockProvider { headers };

            let expected = resolve_script_id(local_script_id, map_id, &table, &provider).unwrap();
            let actual = resolve_script_id_by_file(local_script_id, script_file_id, &table, &provider).unwrap();
            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn prop_local_resolution_by_level_file_matches_direct_map_resolution(
            headers in headers_strategy(),
            entries in global_entries_strategy(),
            local_script_id in 0u16..COMMON_SCRIPT_THRESHOLD,
            level_script_file_id in any::<u16>()
        ) {
            let table = GlobalScriptTable::from_entries(entries);
            let map_id = headers
                .iter()
                .enumerate()
                .find_map(|(idx, h)| (h.level_script_id() == level_script_file_id).then_some(idx as u16));
            let provider = MockProvider { headers };

            let expected = resolve_script_id(local_script_id, map_id, &table, &provider).unwrap();
            let actual = resolve_script_id_by_level_script_file(
                local_script_id,
                level_script_file_id,
                &table,
                &provider,
            ).unwrap();
            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn prop_resolve_level_script_by_file_matches_resolve_level_script(
            headers in headers_strategy(),
            entries in global_entries_strategy(),
            level_script_file_id in any::<u16>()
        ) {
            let table = GlobalScriptTable::from_entries(entries);
            let map_id = headers
                .iter()
                .enumerate()
                .find_map(|(idx, h)| (h.level_script_id() == level_script_file_id).then_some(idx as u16));
            let provider = MockProvider { headers };

            let expected = match map_id {
                Some(id) => resolve_level_script(id, &table, &provider).unwrap(),
                None => None,
            };
            let actual = resolve_level_script_by_file(level_script_file_id, &table, &provider).unwrap();
            prop_assert_eq!(actual, expected);
        }
    }
}
