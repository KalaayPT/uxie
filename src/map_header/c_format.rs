use super::types::*;
use crate::c_parser::SymbolTable;
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ParsedMapHeader {
    pub name: String,
    pub index: Option<usize>,
    pub fields: HashMap<String, String>,
}

pub fn parse_map_headers_from_c(source: &str) -> Vec<ParsedMapHeader> {
    let mut headers = Vec::new();

    let header_pattern = Regex::new(r"\[([A-Z_][A-Z0-9_]*)\]\s*=\s*\{([^}]+)\}").unwrap();

    let field_pattern = Regex::new(r"\.([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*([^,}]+)").unwrap();

    for cap in header_pattern.captures_iter(source) {
        let name = cap[1].to_string();
        let body = &cap[2];

        let mut fields = HashMap::new();
        for field_cap in field_pattern.captures_iter(body) {
            let field_name = field_cap[1].trim().to_string();
            let field_value = field_cap[2].trim().to_string();
            fields.insert(field_name, field_value);
        }

        headers.push(ParsedMapHeader {
            name,
            index: None,
            fields,
        });
    }

    headers
}

pub fn parsed_to_pt_header(parsed: &ParsedMapHeader, symbols: &SymbolTable) -> MapHeaderPt {
    let mut h = MapHeaderPt::default();

    let resolve = |v: &str| -> i64 { resolve_value(v, symbols) };

    if let Some(v) = parsed.fields.get("areaDataArchiveID") {
        h.area_data_id = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("unk_01") {
        h.unknown1 = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("mapMatrixID") {
        h.matrix_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("scriptsArchiveID") {
        h.script_file_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("initScriptsArchiveID") {
        h.level_script_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("msgArchiveID") {
        h.text_archive_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("dayMusicID") {
        h.music_day_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("nightMusicID") {
        h.music_night_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("wildEncountersArchiveID") {
        h.wild_pokemon = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("eventsArchiveID") {
        h.event_file_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("mapLabelTextID") {
        h.location_name = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("mapLabelWindowID") {
        h.area_icon = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("weather") {
        h.weather_id = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("cameraType") {
        h.camera_angle_id = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("mapType") {
        h.location_specifier = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("battleBG") {
        h.battle_background = resolve(v) as u8;
    }

    let mut flags: u8 = 0;
    if parsed
        .fields
        .get("isBikeAllowed")
        .map(|v| v == "TRUE")
        .unwrap_or(false)
    {
        flags |= 0b0001;
    }
    if parsed
        .fields
        .get("isRunningAllowed")
        .map(|v| v == "TRUE")
        .unwrap_or(false)
    {
        flags |= 0b0010;
    }
    if parsed
        .fields
        .get("isEscapeRopeAllowed")
        .map(|v| v == "TRUE")
        .unwrap_or(false)
    {
        flags |= 0b0100;
    }
    if parsed
        .fields
        .get("isFlyAllowed")
        .map(|v| v == "TRUE")
        .unwrap_or(false)
    {
        flags |= 0b1000;
    }
    h.flags = flags;

    h
}

pub fn parsed_to_hgss_header(parsed: &ParsedMapHeader, symbols: &SymbolTable) -> MapHeaderHGSS {
    let mut h = MapHeaderHGSS::default();

    let resolve = |v: &str| -> i64 { resolve_value(v, symbols) };
    let field_bool = |name: &str| -> bool {
        parsed
            .fields
            .get(name)
            .map(|v| parse_bool_with_symbols(v, symbols))
            .unwrap_or(false)
    };

    if let Some(v) = parsed.fields.get("wildEncounterBank") {
        h.wild_pokemon = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("areaDataBank") {
        h.area_data_id = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("moveModelBank") {
        h.unknown0 = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("worldMapX") {
        h.worldmap_x = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("worldMapY") {
        h.worldmap_y = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("matrixId") {
        h.matrix_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("scriptsBank") {
        h.script_file_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("scriptHeaderBank") {
        h.level_script_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("msgBank") {
        h.text_archive_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("dayMusicId") {
        h.music_day_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("nightMusicId") {
        h.music_night_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("eventsBank") {
        h.event_file_id = resolve(v) as u16;
    }
    if let Some(v) = parsed.fields.get("mapsec") {
        h.location_name = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("areaIcon") {
        h.area_icon = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("momCallIntroParam") {
        h.unknown1 = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("isKanto") {
        h.kanto_flag = parse_bool_with_symbols(v, symbols);
    }
    if let Some(v) = parsed.fields.get("weather") {
        h.weather_id = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("mapType") {
        h.location_type = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("cameraType") {
        h.camera_angle_id = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("followMode") {
        h.follow_mode = resolve(v) as u8;
    }
    if let Some(v) = parsed.fields.get("battleBg") {
        h.battle_background = resolve(v) as u8;
    }

    let mut flags: u8 = 0;
    if field_bool("bikeAllowed") {
        flags |= 1 << 0;
    }
    if field_bool("runningAllowed_Unused") {
        flags |= 1 << 1;
    }
    if field_bool("escapeRopeAllowed") {
        flags |= 1 << 2;
    }
    if field_bool("flyAllowed") {
        flags |= 1 << 3;
    }
    if field_bool("outgoingCalls") {
        flags |= 1 << 4;
    }
    if field_bool("incomingCalls") {
        flags |= 1 << 5;
    }
    if field_bool("radioSignal") {
        flags |= 1 << 6;
    }
    h.flags = flags;

    h
}

fn resolve_value(v: &str, symbols: &SymbolTable) -> i64 {
    symbols
        .resolve_constant(v)
        .unwrap_or_else(|| parse_int_or_hex(v))
}

fn parse_bool_with_symbols(value: &str, symbols: &SymbolTable) -> bool {
    match value.trim() {
        "TRUE" | "true" | "1" => true,
        "FALSE" | "false" | "0" => false,
        other => resolve_value(other, symbols) != 0,
    }
}

fn parse_int_or_hex(s: &str) -> i64 {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        i64::from_str_radix(&s[2..], 16).unwrap_or(0)
    } else {
        s.parse().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_header() {
        let source = r#"
    [MAP_HEADER_JUBILIFE_CITY] = {
        .areaDataArchiveID = 0x6,
        .unk_01 = 0x0,
        .mapMatrixID = 0x0,
        .scriptsArchiveID = scripts_jubilife_city,
        .msgArchiveID = TEXT_BANK_JUBILIFE_CITY,
        .dayMusicID = SEQ_CITY01_D,
        .isBikeAllowed = TRUE,
        .isRunningAllowed = TRUE,
    },
        "#;

        let headers = parse_map_headers_from_c(source);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].name, "MAP_HEADER_JUBILIFE_CITY");
        assert_eq!(
            headers[0].fields.get("areaDataArchiveID"),
            Some(&"0x6".to_string())
        );
    }

    #[test]
    fn test_parse_hgss_header_conversion() {
        let source = r"
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
        ";

        let headers = parse_map_headers_from_c(source);
        assert_eq!(headers.len(), 1);

        let symbols = SymbolTable::new();
        let h = parsed_to_hgss_header(&headers[0], &symbols);

        assert_eq!(h.wild_pokemon, 1);
        assert_eq!(h.area_data_id, 2);
        assert_eq!(h.unknown0, 3);
        assert_eq!(h.worldmap_x, 4);
        assert_eq!(h.worldmap_y, 5);
        assert_eq!(h.matrix_id, 6);
        assert_eq!(h.script_file_id, 7);
        assert_eq!(h.level_script_id, 8);
        assert_eq!(h.text_archive_id, 9);
        assert_eq!(h.event_file_id, 12);
        assert_eq!(h.location_name, 13);
        assert_eq!(h.area_icon, 14);
        assert_eq!(h.unknown1, 15);
        assert!(h.kanto_flag);
        assert_eq!(h.weather_id, 16);
        assert_eq!(h.location_type, 17);
        assert_eq!(h.camera_angle_id, 18);
        assert_eq!(h.follow_mode, 2);
        assert_eq!(h.battle_background, 19);
        assert_eq!(h.flags, 0x55);
    }
}
