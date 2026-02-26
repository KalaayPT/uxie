use super::types::*;
use crate::c_parser::SymbolTable;
use regex::Regex;
use std::collections::HashMap;
use std::io;
use std::sync::LazyLock;

static HEADER_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([A-Z_][A-Z0-9_]*)\]\s*=\s*\{([^}]+)\}").unwrap());

static FIELD_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*([^,}]+)").unwrap());

#[derive(Debug, Clone)]
pub struct ParsedMapHeader {
    pub name: String,
    pub index: Option<usize>,
    pub fields: HashMap<String, String>,
}

pub fn parse_map_headers_from_c(source: &str) -> Vec<ParsedMapHeader> {
    let mut headers = Vec::new();

    for cap in HEADER_PATTERN.captures_iter(source) {
        let name = cap[1].to_string();
        let body = &cap[2];

        let mut fields = HashMap::new();
        for field_cap in FIELD_PATTERN.captures_iter(body) {
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

pub fn parsed_to_pt_header(
    parsed: &ParsedMapHeader,
    symbols: &SymbolTable,
) -> io::Result<MapHeaderPt> {
    let mut h = MapHeaderPt::default();

    if let Some(v) = resolve_field_value(parsed, "areaDataArchiveID", symbols)? {
        h.area_data_id = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "unk_01", symbols)? {
        h.unknown1 = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "mapMatrixID", symbols)? {
        h.matrix_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "scriptsArchiveID", symbols)? {
        h.script_file_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "initScriptsArchiveID", symbols)? {
        h.level_script_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "msgArchiveID", symbols)? {
        h.text_archive_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "dayMusicID", symbols)? {
        h.music_day_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "nightMusicID", symbols)? {
        h.music_night_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "wildEncountersArchiveID", symbols)? {
        h.wild_pokemon = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "eventsArchiveID", symbols)? {
        h.event_file_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "mapLabelTextID", symbols)? {
        h.location_name = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "mapLabelWindowID", symbols)? {
        h.area_icon = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "weather", symbols)? {
        h.weather_id = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "cameraType", symbols)? {
        h.camera_angle_id = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "mapType", symbols)? {
        h.location_specifier = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "battleBG", symbols)? {
        h.battle_background = v as u8;
    }

    let mut flags: u8 = 0;
    if resolve_field_bool(parsed, "isBikeAllowed", symbols)?.unwrap_or(false) {
        flags |= 0b0001;
    }
    if resolve_field_bool(parsed, "isRunningAllowed", symbols)?.unwrap_or(false) {
        flags |= 0b0010;
    }
    if resolve_field_bool(parsed, "isEscapeRopeAllowed", symbols)?.unwrap_or(false) {
        flags |= 0b0100;
    }
    if resolve_field_bool(parsed, "isFlyAllowed", symbols)?.unwrap_or(false) {
        flags |= 0b1000;
    }
    h.flags = flags;

    Ok(h)
}

pub fn parsed_to_hgss_header(
    parsed: &ParsedMapHeader,
    symbols: &SymbolTable,
) -> io::Result<MapHeaderHGSS> {
    let mut h = MapHeaderHGSS::default();

    if let Some(v) = resolve_field_value(parsed, "wildEncounterBank", symbols)? {
        h.wild_pokemon = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "areaDataBank", symbols)? {
        h.area_data_id = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "moveModelBank", symbols)? {
        h.unknown0 = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "worldMapX", symbols)? {
        h.worldmap_x = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "worldMapY", symbols)? {
        h.worldmap_y = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "matrixId", symbols)? {
        h.matrix_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "scriptsBank", symbols)? {
        h.script_file_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "scriptHeaderBank", symbols)? {
        h.level_script_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "msgBank", symbols)? {
        h.text_archive_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "dayMusicId", symbols)? {
        h.music_day_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "nightMusicId", symbols)? {
        h.music_night_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "eventsBank", symbols)? {
        h.event_file_id = v as u16;
    }
    if let Some(v) = resolve_field_value(parsed, "mapsec", symbols)? {
        h.location_name = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "areaIcon", symbols)? {
        h.area_icon = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "momCallIntroParam", symbols)? {
        h.unknown1 = v as u8;
    }
    if let Some(v) = resolve_field_bool(parsed, "isKanto", symbols)? {
        h.kanto_flag = v;
    }
    if let Some(v) = resolve_field_value(parsed, "weather", symbols)? {
        h.weather_id = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "mapType", symbols)? {
        h.location_type = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "cameraType", symbols)? {
        h.camera_angle_id = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "followMode", symbols)? {
        h.follow_mode = v as u8;
    }
    if let Some(v) = resolve_field_value(parsed, "battleBg", symbols)? {
        h.battle_background = v as u8;
    }

    let mut flags: u8 = 0;
    if resolve_field_bool(parsed, "bikeAllowed", symbols)?.unwrap_or(false) {
        flags |= 1 << 0;
    }
    if resolve_field_bool(parsed, "runningAllowed_Unused", symbols)?.unwrap_or(false) {
        flags |= 1 << 1;
    }
    if resolve_field_bool(parsed, "escapeRopeAllowed", symbols)?.unwrap_or(false) {
        flags |= 1 << 2;
    }
    if resolve_field_bool(parsed, "flyAllowed", symbols)?.unwrap_or(false) {
        flags |= 1 << 3;
    }
    if resolve_field_bool(parsed, "outgoingCalls", symbols)?.unwrap_or(false) {
        flags |= 1 << 4;
    }
    if resolve_field_bool(parsed, "incomingCalls", symbols)?.unwrap_or(false) {
        flags |= 1 << 5;
    }
    if resolve_field_bool(parsed, "radioSignal", symbols)?.unwrap_or(false) {
        flags |= 1 << 6;
    }
    h.flags = flags;

    Ok(h)
}

fn resolve_field_value(
    parsed: &ParsedMapHeader,
    field_name: &str,
    symbols: &SymbolTable,
) -> io::Result<Option<i64>> {
    parsed
        .fields
        .get(field_name)
        .map(|value| {
            resolve_value(value, symbols).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Failed to resolve map header '{}' field '{}' value '{}': {}",
                        parsed.name, field_name, value, e
                    ),
                )
            })
        })
        .transpose()
}

fn resolve_field_bool(
    parsed: &ParsedMapHeader,
    field_name: &str,
    symbols: &SymbolTable,
) -> io::Result<Option<bool>> {
    parsed
        .fields
        .get(field_name)
        .map(|value| {
            parse_bool_with_symbols(value, symbols).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Failed to resolve map header '{}' field '{}' value '{}': {}",
                        parsed.name, field_name, value, e
                    ),
                )
            })
        })
        .transpose()
}

fn resolve_value(v: &str, symbols: &SymbolTable) -> io::Result<i64> {
    if let Some(value) = symbols.resolve_constant(v) {
        return Ok(value);
    }
    parse_int_or_hex(v)
}

fn parse_bool_with_symbols(value: &str, symbols: &SymbolTable) -> io::Result<bool> {
    match value.trim() {
        "TRUE" | "true" | "1" => Ok(true),
        "FALSE" | "false" | "0" => Ok(false),
        other => Ok(resolve_value(other, symbols)? != 0),
    }
}

fn parse_int_or_hex(s: &str) -> io::Result<i64> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        i64::from_str_radix(&s[2..], 16).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid hexadecimal literal '{}': {}", s, e),
            )
        })
    } else {
        s.parse().map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid integer literal '{}': {}", s, e),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_header() {
        let source = r"
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
        ";

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
        let h = parsed_to_hgss_header(&headers[0], &symbols).unwrap();

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

    #[test]
    fn test_parsed_to_pt_header_invalid_literal_returns_error() {
        let source = r"
    [MAP_HEADER_BAD] = {
        .scriptsArchiveID = not-a-valid-literal,
    },
        ";
        let headers = parse_map_headers_from_c(source);
        let symbols = SymbolTable::new();

        let err = parsed_to_pt_header(&headers[0], &symbols).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("MAP_HEADER_BAD"));
        assert!(err.to_string().contains("scriptsArchiveID"));
    }

    #[test]
    fn test_parsed_to_hgss_header_invalid_boolean_symbol_returns_error() {
        let source = r"
    [MAP_HEADER_BAD_HG] = {
        .bikeAllowed = MAYBE_ENABLED,
    },
        ";
        let headers = parse_map_headers_from_c(source);
        let symbols = SymbolTable::new();

        let err = parsed_to_hgss_header(&headers[0], &symbols).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("MAP_HEADER_BAD_HG"));
        assert!(err.to_string().contains("bikeAllowed"));
    }
}
