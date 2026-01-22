use crate::error::Result;
use crate::provider::DataProvider;
use crate::script_file::GlobalScriptTable;

pub const COMMON_SCRIPT_THRESHOLD: u16 = 2000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptFileInfo {
    pub script_file_id: u16,
    pub text_archive_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapScriptInfo {
    pub map_id: u16,
    pub script_file_id: u16,
    pub text_archive_id: u16,
    pub event_file_id: u16,
    pub level_script_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptResolution {
    CommonScript {
        script_id: u16,
        script_file_id: u16,
        text_archive_id: u16,
    },
    MapScript {
        script_id: u16,
        map_id: u16,
        script_file_id: u16,
        text_archive_id: u16,
        event_file_id: u16,
    },
}

impl ScriptResolution {
    pub fn script_file_id(&self) -> u16 {
        match self {
            ScriptResolution::CommonScript { script_file_id, .. } => *script_file_id,
            ScriptResolution::MapScript { script_file_id, .. } => *script_file_id,
        }
    }

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

    pub fn is_common_script(&self) -> bool {
        matches!(self, ScriptResolution::CommonScript { .. })
    }

    pub fn is_map_script(&self) -> bool {
        matches!(self, ScriptResolution::MapScript { .. })
    }

    pub fn event_file_id(&self) -> Option<u16> {
        match self {
            ScriptResolution::CommonScript { .. } => None,
            ScriptResolution::MapScript { event_file_id, .. } => Some(*event_file_id),
        }
    }

    pub fn map_id(&self) -> Option<u16> {
        match self {
            ScriptResolution::CommonScript { .. } => None,
            ScriptResolution::MapScript { map_id, .. } => Some(*map_id),
        }
    }
}

pub fn is_common_script_id(script_id: u16) -> bool {
    script_id >= COMMON_SCRIPT_THRESHOLD
}

pub fn resolve_script_id(
    script_id: u16,
    map_id: Option<u16>,
    global_table: &GlobalScriptTable,
    provider: &dyn DataProvider,
) -> Result<Option<ScriptResolution>> {
    if is_common_script_id(script_id) {
        resolve_common_script(script_id, global_table)
    } else {
        resolve_map_script(script_id, map_id, provider)
    }
}

pub fn resolve_script_id_by_file(
    script_id: u16,
    script_file_id: u16,
    global_table: &GlobalScriptTable,
    provider: &dyn DataProvider,
) -> Result<Option<ScriptResolution>> {
    if is_common_script_id(script_id) {
        resolve_common_script(script_id, global_table)
    } else {
        let map_id = provider.find_map_by_script_file_id(script_file_id)?;
        resolve_map_script(script_id, map_id, provider)
    }
}

pub fn resolve_script_id_by_level_script_file(
    script_id: u16,
    level_script_file_id: u16,
    global_table: &GlobalScriptTable,
    provider: &dyn DataProvider,
) -> Result<Option<ScriptResolution>> {
    if is_common_script_id(script_id) {
        resolve_common_script(script_id, global_table)
    } else {
        let map_id = provider.find_map_by_level_script_file_id(level_script_file_id)?;
        resolve_map_script(script_id, map_id, provider)
    }
}

fn resolve_common_script(
    script_id: u16,
    global_table: &GlobalScriptTable,
) -> Result<Option<ScriptResolution>> {
    let entry = global_table.lookup(script_id);
    Ok(entry.map(|e| ScriptResolution::CommonScript {
        script_id,
        script_file_id: e.script_file_id,
        text_archive_id: e.text_archive_id,
    }))
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

pub fn resolve_level_script(
    map_id: u16,
    global_table: &GlobalScriptTable,
    provider: &dyn DataProvider,
) -> Result<Option<ScriptResolution>> {
    let header = provider.get_map_header(map_id)?;
    let level_script_id = header.level_script_id();

    if is_common_script_id(level_script_id) {
        resolve_common_script(level_script_id, global_table)
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

pub fn resolve_level_script_by_file(
    level_script_file_id: u16,
    global_table: &GlobalScriptTable,
    provider: &dyn DataProvider,
) -> Result<Option<ScriptResolution>> {
    let map_id = provider.find_map_by_level_script_file_id(level_script_file_id)?;
    let map_id = match map_id {
        Some(id) => id,
        None => return Ok(None),
    };

    resolve_level_script(map_id, global_table, provider)
}

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

pub fn find_maps_for_script_file(
    script_file_id: u16,
    provider: &dyn DataProvider,
) -> Result<Vec<u16>> {
    let count = provider.get_map_header_count()?;
    let mut maps = Vec::new();

    for map_id in 0..count as u16 {
        let header = provider.get_map_header(map_id)?;
        if header.script_file_id() == script_file_id {
            maps.push(map_id);
        }
    }

    Ok(maps)
}

pub fn find_maps_for_level_script_file(
    level_script_file_id: u16,
    provider: &dyn DataProvider,
) -> Result<Vec<u16>> {
    let count = provider.get_map_header_count()?;
    let mut maps = Vec::new();

    for map_id in 0..count as u16 {
        let header = provider.get_map_header(map_id)?;
        if header.level_script_id() == level_script_file_id {
            maps.push(map_id);
        }
    }

    Ok(maps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_header::{MapHeader, MapHeaderPt};
    use crate::script_file::GlobalScriptEntry;

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

        fn get_text_archive_for_script(&self, script_file_id: u16) -> Result<Option<u16>> {
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
    }

    fn create_pt_header(script_file: u16, text_archive: u16, event_file: u16) -> MapHeader {
        MapHeader::Pt(MapHeaderPt {
            script_file_id: script_file,
            text_archive_id: text_archive,
            event_file_id: event_file,
            level_script_id: 1,
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

        let result = resolve_common_script(2018, &table).unwrap();
        assert!(result.is_some());

        let resolution = result.unwrap();
        assert!(resolution.is_common_script());
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
            headers: vec![create_pt_header(100, 200, 50)],
        };

        let result = resolve_map_script(5, Some(0), &provider).unwrap();
        assert!(result.is_some());

        let resolution = result.unwrap();
        assert!(resolution.is_map_script());
        assert_eq!(resolution.script_file_id(), 100);
        assert_eq!(resolution.text_archive_id(), 200);
        assert_eq!(resolution.event_file_id(), Some(50));
        assert_eq!(resolution.map_id(), Some(0));
    }

    #[test]
    fn test_resolve_script_id_routes_correctly() {
        let table = GlobalScriptTable::from_entries(vec![GlobalScriptEntry::new(2000, 211, 213)]);
        let provider = MockProvider {
            headers: vec![create_pt_header(100, 200, 50)],
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
                create_pt_header(100, 200, 50),
                create_pt_header(101, 201, 51),
                create_pt_header(100, 202, 52),
                create_pt_header(102, 203, 53),
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
            headers: vec![create_pt_header(100, 200, 50)],
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
                create_pt_header(100, 200, 50),
                create_pt_header(101, 201, 51),
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
            headers: vec![create_pt_header(100, 200, 50)],
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
}
