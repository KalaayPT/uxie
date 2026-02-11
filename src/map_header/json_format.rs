//! JSON map header structures with constant resolution

use serde::{Deserialize, Serialize};
use std::io;

pub use crate::c_parser::symbol_table::SymbolTable;

/// Map header JSON format (pokeplatinum decompilation)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapHeaderJson {
    pub id: Option<String>,
    pub music_day_id: Option<String>,
    pub music_night_id: Option<String>,
    pub wild_pokemon: Option<String>,
    pub event_file_id: Option<String>,
    pub text_archive_id: Option<String>,
    pub location_name: Option<String>,
    pub area_icon: Option<String>,
    pub weather_id: Option<String>,
    pub camera_angle_id: Option<String>,
    pub battle_background: Option<String>,
    pub flags: Option<String>,
}

impl MapHeaderJson {
    pub fn from_json_with_constants<R: io::Read>(
        reader: R,
        symbols: &SymbolTable,
    ) -> io::Result<Self> {
        let mut header: MapHeaderJson = serde_json::from_reader(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        header.resolve_constants(symbols);
        Ok(header)
    }

    pub fn resolve_constants(&mut self, symbols: &SymbolTable) {
        macro_rules! resolve_field {
            ($field:ident) => {
                if let Some(val_str) = &self.$field {
                    if let Some(value) = symbols.resolve_constant(val_str) {
                        self.$field = Some(value.to_string());
                    }
                }
            };
        }

        resolve_field!(id);
        resolve_field!(music_day_id);
        resolve_field!(music_night_id);
        resolve_field!(wild_pokemon);
        resolve_field!(event_file_id);
        resolve_field!(text_archive_id);
        resolve_field!(location_name);
        resolve_field!(area_icon);
        resolve_field!(weather_id);
        resolve_field!(camera_angle_id);
        resolve_field!(battle_background);
        resolve_field!(flags);
    }

    pub fn from_binary(header: &crate::map_header::MapHeader, symbols: &SymbolTable) -> Self {
        macro_rules! resolve {
            ($val:expr, $prefix:expr) => {
                symbols
                    .resolve_name($val as i64, $prefix)
                    .unwrap_or_else(|| $val.to_string())
            };
        }

        match header {
            crate::map_header::MapHeader::DP(h) => Self {
                id: None,
                music_day_id: Some(resolve!(h.music_day_id, "SEQ_")),
                music_night_id: Some(resolve!(h.music_night_id, "SEQ_")),
                wild_pokemon: Some(h.wild_pokemon.to_string()),
                event_file_id: Some(h.event_file_id.to_string()),
                text_archive_id: Some(h.text_archive_id.to_string()),
                location_name: Some(resolve!(h.location_name, "MAPSEC_")),
                area_icon: None,
                weather_id: Some(resolve!(h.weather_id, "OVERWORLD_WEATHER_")),
                camera_angle_id: Some(resolve!(h.camera_angle_id, "CAMERA_TYPE_")),
                battle_background: Some(resolve!(h.battle_background, "BATTLE_BG_")),
                flags: Some(h.flags.to_string()),
            },
            crate::map_header::MapHeader::Pt(h) => Self {
                id: None,
                music_day_id: Some(resolve!(h.music_day_id, "SEQ_")),
                music_night_id: Some(resolve!(h.music_night_id, "SEQ_")),
                wild_pokemon: Some(h.wild_pokemon.to_string()),
                event_file_id: Some(h.event_file_id.to_string()),
                text_archive_id: Some(h.text_archive_id.to_string()),
                location_name: Some(resolve!(h.location_name, "MAPSEC_")),
                area_icon: Some(h.area_icon.to_string()),
                weather_id: Some(resolve!(h.weather_id, "OVERWORLD_WEATHER_")),
                camera_angle_id: Some(resolve!(h.camera_angle_id, "CAMERA_TYPE_")),
                battle_background: Some(resolve!(h.battle_background, "BATTLE_BG_")),
                flags: Some(h.flags.to_string()),
            },
            crate::map_header::MapHeader::HGSS(h) => Self {
                id: None,
                music_day_id: Some(resolve!(h.music_day_id, "SEQ_")),
                music_night_id: Some(resolve!(h.music_night_id, "SEQ_")),
                wild_pokemon: Some(h.wild_pokemon.to_string()),
                event_file_id: Some(h.event_file_id.to_string()),
                text_archive_id: Some(h.text_archive_id.to_string()),
                location_name: Some(resolve!(h.location_name, "MAPSEC_")),
                area_icon: Some(h.area_icon.to_string()),
                weather_id: Some(resolve!(h.weather_id, "OVERWORLD_WEATHER_")),
                camera_angle_id: Some(resolve!(h.camera_angle_id, "CAMERA_TYPE_")),
                battle_background: Some(resolve!(h.battle_background, "BATTLE_BG_")),
                flags: Some(h.flags.to_string()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_header_json_parse() {
        let json = r#"
        {
            "id": "MAP_HEADER_SANDGEM_TOWN",
            "music_day_id": "42",
            "music_night_id": "43",
            "text_archive_id": "15"
        }
        "#;

        let header: MapHeaderJson = serde_json::from_str(json).unwrap();
        assert_eq!(header.id, Some("MAP_HEADER_SANDGEM_TOWN".to_string()));
        assert_eq!(header.music_day_id, Some("42".into()));
    }

    #[test]
    fn test_map_header_resolution() {
        let mut symbols = SymbolTable::new();
        symbols.insert_define("MAPSEC_JUBILIFE_CITY".into(), 6);
        symbols.insert_define("SEQ_CITY01_D".into(), 1010);
        symbols.insert_define("CAMERA_TYPE_DEFAULT".into(), 0);

        let json = r#"{
            "location_name": "MAPSEC_JUBILIFE_CITY",
            "music_day_id": "SEQ_CITY01_D",
            "camera_angle_id": "CAMERA_TYPE_DEFAULT"
        }"#;

        let mut header: MapHeaderJson = serde_json::from_str(json).unwrap();
        header.resolve_constants(&symbols);

        assert_eq!(header.location_name, Some("6".into()));
        assert_eq!(header.music_day_id, Some("1010".into()));
        assert_eq!(header.camera_angle_id, Some("0".into()));
    }

    #[test]
    #[ignore]
    fn integration_dspre_map_header_jubilife() {
        use crate::GameFamily;
        use crate::provider::{Arm9Provider, DataProvider};
        let Some(dspre_path) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            "map header integration test",
        ) else {
            return;
        };
        let Some(decomp_path) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            "map header integration test",
        ) else {
            return;
        };
        let headers_path = decomp_path.join("build/generated");
        let include_path = decomp_path.join("include");

        if !dspre_path.exists() || !headers_path.exists() || !include_path.exists() {
            eprintln!("Skipping: test data not available at configured paths");
            return;
        }

        let mut symbols = SymbolTable::new();
        symbols.load_headers_from_dir(&headers_path).unwrap();
        symbols.load_headers_from_dir(&include_path).unwrap();

        let arm9_path = dspre_path.join("arm9.bin");
        let provider = Arm9Provider::new(&arm9_path, 0xE601C, 559, GameFamily::Platinum);

        let bin_header = provider.get_map_header(3).unwrap();
        let json_header = MapHeaderJson::from_binary(&bin_header, &symbols);

        assert_eq!(json_header.music_day_id, Some("SEQ_CITY01_D".into()));
        assert_eq!(json_header.music_night_id, Some("SEQ_CITY01_N".into()));
        assert_eq!(
            json_header.weather_id,
            Some("OVERWORLD_WEATHER_CLEAR".into())
        );
        assert_eq!(
            json_header.camera_angle_id,
            Some("CAMERA_TYPE_DEFAULT".into())
        );
    }
}
