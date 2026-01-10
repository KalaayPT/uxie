use regex::Regex;
use std::collections::HashMap;
use super::types::*;

#[derive(Debug, Clone)]
pub struct ParsedMapHeader {
    pub name: String,
    pub index: Option<usize>,
    pub fields: HashMap<String, String>,
}

pub fn parse_map_headers_from_c(source: &str) -> Vec<ParsedMapHeader> {
    let mut headers = Vec::new();
    
    let header_pattern = Regex::new(
        r"\[([A-Z_][A-Z0-9_]*)\]\s*=\s*\{([^}]+)\}"
    ).unwrap();
    
    let field_pattern = Regex::new(
        r"\.([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*([^,}]+)"
    ).unwrap();

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

pub fn parsed_to_pt_header(parsed: &ParsedMapHeader) -> MapHeaderPt {
    let mut h = MapHeaderPt::default();
    
    if let Some(v) = parsed.fields.get("areaDataArchiveID") {
        h.area_data_id = parse_int_or_hex(v) as u8;
    }
    if let Some(v) = parsed.fields.get("unk_01") {
        h.unknown1 = parse_int_or_hex(v) as u8;
    }
    if let Some(v) = parsed.fields.get("mapMatrixID") {
        h.matrix_id = parse_int_or_hex(v) as u16;
    }
    if let Some(v) = parsed.fields.get("scriptsArchiveID") {
        h.script_file_id = parse_int_or_hex(v) as u16;
    }
    if let Some(v) = parsed.fields.get("initScriptsArchiveID") {
        h.level_script_id = parse_int_or_hex(v) as u16;
    }
    if let Some(v) = parsed.fields.get("msgArchiveID") {
        h.text_archive_id = parse_int_or_hex(v) as u16;
    }
    if let Some(v) = parsed.fields.get("dayMusicID") {
        h.music_day_id = parse_int_or_hex(v) as u16;
    }
    if let Some(v) = parsed.fields.get("nightMusicID") {
        h.music_night_id = parse_int_or_hex(v) as u16;
    }
    if let Some(v) = parsed.fields.get("wildEncountersArchiveID") {
        h.wild_pokemon = parse_int_or_hex(v) as u16;
    }
    if let Some(v) = parsed.fields.get("eventsArchiveID") {
        h.event_file_id = parse_int_or_hex(v) as u16;
    }
    if let Some(v) = parsed.fields.get("mapLabelTextID") {
        h.location_name = parse_int_or_hex(v) as u8;
    }
    if let Some(v) = parsed.fields.get("mapLabelWindowID") {
        h.area_icon = parse_int_or_hex(v) as u8;
    }
    if let Some(v) = parsed.fields.get("weather") {
        h.weather_id = parse_int_or_hex(v) as u8;
    }
    if let Some(v) = parsed.fields.get("cameraType") {
        h.camera_angle_id = parse_int_or_hex(v) as u8;
    }
    if let Some(v) = parsed.fields.get("mapType") {
        h.location_specifier = parse_int_or_hex(v) as u8;
    }
    if let Some(v) = parsed.fields.get("battleBG") {
        h.battle_background = parse_int_or_hex(v) as u8;
    }
    
    let mut flags: u8 = 0;
    if parsed.fields.get("isBikeAllowed").map(|v| v == "TRUE").unwrap_or(false) {
        flags |= 0b0001;
    }
    if parsed.fields.get("isRunningAllowed").map(|v| v == "TRUE").unwrap_or(false) {
        flags |= 0b0010;
    }
    if parsed.fields.get("isEscapeRopeAllowed").map(|v| v == "TRUE").unwrap_or(false) {
        flags |= 0b0100;
    }
    if parsed.fields.get("isFlyAllowed").map(|v| v == "TRUE").unwrap_or(false) {
        flags |= 0b1000;
    }
    h.flags = flags;

    h
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
        assert_eq!(headers[0].fields.get("areaDataArchiveID"), Some(&"0x6".to_string()));
    }
}
