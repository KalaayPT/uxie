//! JSON event file structures matching pokeplatinum format

use serde::{Deserialize, Serialize};
use std::io;

pub use crate::c_parser::symbol_table::SymbolTable;

/// Background event (spawnable)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BgEventJson {
    pub script: u16,
    #[serde(rename = "type")]
    pub event_type: u16,
    pub x: i32,
    pub z: i32,
    pub y: i32,
    #[serde(default, rename = "player_facing_dir")]
    pub player_facing_dir: Option<String>,
}

/// Object event (NPC, items, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectEventJson {
    pub id: String,
    #[serde(rename = "graphics_id")]
    pub graphics_id: String,
    #[serde(rename = "movement_type")]
    pub movement_type: String,
    #[serde(rename = "trainer_type")]
    pub trainer_type: String,
    #[serde(default, rename = "hidden_flag")]
    pub hidden_flag: Option<String>,
    pub script: u16,
    pub initial_dir: i16,
    #[serde(default)]
    pub data: Vec<u16>,
    #[serde(default, rename = "movement_range_x")]
    pub movement_range_x: i16,
    #[serde(default, rename = "movement_range_z")]
    pub movement_range_z: i16,
    pub x: u16,
    pub z: u16,
    pub y: i32,
    #[serde(default, rename = "clone_id")]
    pub clone_id: Option<u8>,
}

/// Warp event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarpEventJson {
    pub x: u16,
    pub z: u16,
    #[serde(rename = "dest_header_id")]
    pub dest_header_id: String,
    #[serde(rename = "dest_warp_id")]
    pub dest_warp_id: u16,
}

/// Coordinate event (triggers)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordEventJson {
    pub script: u16,
    pub x: u16,
    pub z: u16,
    pub width: u16,
    pub length: u16,
    pub y: u16,
    pub value: u16,
    pub var: Option<String>,
}

/// JSON event file container
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonEventFile {
    #[serde(rename = "bg_events")]
    pub bg_events: Vec<BgEventJson>,
    #[serde(rename = "object_events")]
    pub object_events: Vec<ObjectEventJson>,
    #[serde(rename = "warp_events")]
    pub warp_events: Vec<WarpEventJson>,
    #[serde(rename = "coord_events")]
    pub coord_events: Vec<CoordEventJson>,
}

impl JsonEventFile {
    pub fn from_json_with_constants<R: io::Read>(
        reader: R,
        symbols: &SymbolTable,
    ) -> io::Result<Self> {
        let mut file: JsonEventFile = serde_json::from_reader(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        file.resolve_constants(symbols);
        Ok(file)
    }

    pub fn resolve_constants(&mut self, symbols: &SymbolTable) {
        for bg in &mut self.bg_events {
            if let Some(dir) = &bg.player_facing_dir {
                if let Some(val) = symbols.resolve_constant(dir) {
                    bg.player_facing_dir = Some(val.to_string());
                }
            }
        }
        for obj in &mut self.object_events {
            if let Some(gfx) = symbols.resolve_constant(&obj.graphics_id) {
                obj.graphics_id = gfx.to_string();
            }
            if let Some(mov) = symbols.resolve_constant(&obj.movement_type) {
                obj.movement_type = mov.to_string();
            }
            if let Some(trn) = symbols.resolve_constant(&obj.trainer_type) {
                obj.trainer_type = trn.to_string();
            }
            if let Some(flag) = &obj.hidden_flag {
                if flag == "0" {
                    obj.hidden_flag = Some("0".into());
                } else if let Some(val) = symbols.resolve_constant(flag) {
                    obj.hidden_flag = Some(val.to_string());
                }
            }
        }
        for warp in &mut self.warp_events {
            if let Some(val) = symbols.resolve_constant(&warp.dest_header_id) {
                warp.dest_header_id = val.to_string();
            }
        }
        for coord in &mut self.coord_events {
            if let Some(var) = &coord.var {
                if let Some(val) = symbols.resolve_constant(var) {
                    coord.var = Some(val.to_string());
                }
            }
        }
    }

    pub fn from_binary(bin: &crate::event_file::BinaryEventFile, symbols: &SymbolTable) -> Self {
        let mut bg_events = Vec::new();
        for bg in &bin.bg_events {
            bg_events.push(BgEventJson {
                script: bg.script,
                event_type: bg.event_type,
                x: (bg.x % 32) as i32,
                z: (bg.z % 32) as i32,
                y: bg.y,
                player_facing_dir: symbols.resolve_name(bg.player_facing_dir as i64, "BG_EVENT_DIR_"),
            });
        }

        let mut object_events = Vec::new();
        for obj in &bin.object_events {
            object_events.push(ObjectEventJson {
                id: format!("OBJ_{}", obj.local_id),
                graphics_id: symbols.resolve_name(obj.graphics_id as i64, "OBJ_EVENT_GFX_")
                    .unwrap_or_else(|| obj.graphics_id.to_string()),
                movement_type: symbols.resolve_name(obj.movement_type as i64, "MOVEMENT_TYPE_")
                    .unwrap_or_else(|| obj.movement_type.to_string()),
                trainer_type: symbols.resolve_name(obj.trainer_type as i64, "TRAINER_TYPE_")
                    .unwrap_or_else(|| obj.trainer_type.to_string()),
                hidden_flag: if obj.hidden_flag == 0 { Some("0".into()) } else {
                    symbols.resolve_name(obj.hidden_flag as i64, "FLAG_")
                },
                script: obj.script,
                initial_dir: obj.dir,
                data: obj.data.to_vec(),
                movement_range_x: obj.movement_range_x,
                movement_range_z: obj.movement_range_z,
                x: (obj.x % 32) as u16,
                z: (obj.z % 32) as u16,
                y: obj.y,
                clone_id: None,
            });
        }

        let mut warp_events = Vec::new();
        for warp in &bin.warp_events {
            warp_events.push(WarpEventJson {
                x: (warp.x % 32) as u16,
                z: (warp.z % 32) as u16,
                dest_header_id: symbols.resolve_name(warp.dest_header_id as i64, "MAP_HEADER_")
                    .unwrap_or_else(|| warp.dest_header_id.to_string()),
                dest_warp_id: warp.dest_warp_id,
            });
        }

        let mut coord_events = Vec::new();
        for coord in &bin.coord_events {
            coord_events.push(CoordEventJson {
                script: coord.script,
                x: (coord.x % 32) as u16,
                z: (coord.z % 32) as u16,
                width: coord.width,
                length: coord.length,
                y: coord.y,
                value: coord.value,
                var: symbols.resolve_name(coord.var as i64, "VAR_"),
            });
        }

        Self {
            bg_events,
            object_events,
            warp_events,
            coord_events,
        }
    }
}
