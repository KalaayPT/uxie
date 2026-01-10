use serde::{Deserialize, Serialize};

pub const MAP_HEADER_SIZE: usize = 24;
pub const ENCOUNTERS_NONE_DPPT: u16 = 0xFFFF;
pub const ENCOUNTERS_NONE_HGSS: u16 = 0xFF;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapHeader {
    DP(MapHeaderDP),
    Pt(MapHeaderPt),
    HGSS(MapHeaderHGSS),
}

impl MapHeader {
    pub fn script_file_id(&self) -> u16 {
        match self {
            MapHeader::DP(h) => h.script_file_id,
            MapHeader::Pt(h) => h.script_file_id,
            MapHeader::HGSS(h) => h.script_file_id,
        }
    }

    pub fn text_archive_id(&self) -> u16 {
        match self {
            MapHeader::DP(h) => h.text_archive_id,
            MapHeader::Pt(h) => h.text_archive_id,
            MapHeader::HGSS(h) => h.text_archive_id,
        }
    }

    pub fn level_script_id(&self) -> u16 {
        match self {
            MapHeader::DP(h) => h.level_script_id,
            MapHeader::Pt(h) => h.level_script_id,
            MapHeader::HGSS(h) => h.level_script_id,
        }
    }

    pub fn event_file_id(&self) -> u16 {
        match self {
            MapHeader::DP(h) => h.event_file_id,
            MapHeader::Pt(h) => h.event_file_id,
            MapHeader::HGSS(h) => h.event_file_id,
        }
    }

    pub fn matrix_id(&self) -> u16 {
        match self {
            MapHeader::DP(h) => h.matrix_id,
            MapHeader::Pt(h) => h.matrix_id,
            MapHeader::HGSS(h) => h.matrix_id,
        }
    }

    pub fn area_data_id(&self) -> u8 {
        match self {
            MapHeader::DP(h) => h.area_data_id,
            MapHeader::Pt(h) => h.area_data_id,
            MapHeader::HGSS(h) => h.area_data_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MapHeaderDP {
    pub area_data_id: u8,
    pub unknown1: u8,
    pub matrix_id: u16,
    pub script_file_id: u16,
    pub level_script_id: u16,
    pub text_archive_id: u16,
    pub music_day_id: u16,
    pub music_night_id: u16,
    pub wild_pokemon: u16,
    pub event_file_id: u16,
    pub location_name: u16,
    pub weather_id: u8,
    pub camera_angle_id: u8,
    pub location_specifier: u8,
    pub battle_background: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MapHeaderPt {
    pub area_data_id: u8,
    pub unknown1: u8,
    pub matrix_id: u16,
    pub script_file_id: u16,
    pub level_script_id: u16,
    pub text_archive_id: u16,
    pub music_day_id: u16,
    pub music_night_id: u16,
    pub wild_pokemon: u16,
    pub event_file_id: u16,
    pub location_name: u8,
    pub area_icon: u8,
    pub weather_id: u8,
    pub camera_angle_id: u8,
    pub location_specifier: u8,
    pub battle_background: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MapHeaderHGSS {
    pub wild_pokemon: u8,
    pub area_data_id: u8,
    pub unknown0: u8,
    pub worldmap_x: u8,
    pub worldmap_y: u8,
    pub matrix_id: u16,
    pub script_file_id: u16,
    pub level_script_id: u16,
    pub text_archive_id: u16,
    pub music_day_id: u16,
    pub music_night_id: u16,
    pub event_file_id: u16,
    pub location_name: u8,
    pub area_icon: u8,
    pub unknown1: u8,
    pub kanto_flag: bool,
    pub weather_id: u8,
    pub location_type: u8,
    pub camera_angle_id: u8,
    pub follow_mode: u8,
    pub battle_background: u8,
    pub flags: u8,
}
