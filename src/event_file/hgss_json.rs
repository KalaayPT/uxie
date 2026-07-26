//! HeartGold/SoulSilver-shaped decomp JSON for event files.

use super::{BgEvent, CoordEvent, EventFile, ObjectEvent, WarpEvent, invalid_data};
use crate::c_parser::{ConstantFamily, SymbolTable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// HGSS background event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HgssBgEventJson {
    #[serde(rename = "scriptId")]
    pub script_id: Value,
    #[serde(rename = "type")]
    pub event_type: Value,
    pub x: Value,
    pub z: Value,
    pub y: Value,
    #[serde(rename = "playerFacingDir", alias = "dir", default)]
    pub player_facing_dir: Option<Value>,
}

/// HGSS object event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HgssObjectEventJson {
    pub id: Value,
    #[serde(rename = "spriteId")]
    pub sprite_id: Value,
    pub movement: Value,
    #[serde(rename = "type")]
    pub event_type: Value,
    #[serde(rename = "eventFlag")]
    pub event_flag: Value,
    #[serde(rename = "scriptId")]
    pub script_id: Value,
    #[serde(rename = "facingDirection")]
    pub facing_direction: Value,
    #[serde(rename = "param0")]
    pub param0: Value,
    #[serde(rename = "param1")]
    pub param1: Value,
    #[serde(rename = "param2")]
    pub param2: Value,
    #[serde(rename = "xRange")]
    pub x_range: Value,
    #[serde(rename = "yRange")]
    pub y_range: Value,
    pub x: Value,
    pub z: Value,
    pub y: Value,
}

/// HGSS warp event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HgssWarpEventJson {
    pub x: Value,
    pub z: Value,
    pub header: Value,
    pub anchor: Value,
    pub y: Value,
}

/// HGSS coordinate event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HgssCoordEventJson {
    #[serde(rename = "scriptId")]
    pub script_id: Value,
    pub x: Value,
    pub z: Value,
    pub w: Value,
    pub h: Value,
    pub y: Value,
    pub val: Value,
    pub var: Value,
}

/// HGSS decomp JSON event file container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HgssEventJson {
    #[serde(default)]
    pub header: Option<String>,
    #[serde(default)]
    pub bgs: Vec<HgssBgEventJson>,
    #[serde(default)]
    pub objects: Vec<HgssObjectEventJson>,
    #[serde(default)]
    pub warps: Vec<HgssWarpEventJson>,
    #[serde(default)]
    pub coords: Vec<HgssCoordEventJson>,
}

impl EventFile {
    pub fn from_hgss_json(
        json: &HgssEventJson,
        symbols: Arc<SymbolTable>,
        project_root: &Path,
    ) -> io::Result<Self> {
        let table = hgss_symbol_table(json, symbols, project_root)?;

        let mut bg_events = Vec::with_capacity(json.bgs.len());
        for bg in &json.bgs {
            bg_events.push(parse_hgss_bg(bg, &table)?);
        }

        let mut object_events = Vec::with_capacity(json.objects.len());
        for obj in &json.objects {
            object_events.push(parse_hgss_object(obj, &table)?);
        }

        let mut warp_events = Vec::with_capacity(json.warps.len());
        for warp in &json.warps {
            warp_events.push(parse_hgss_warp(warp, &table)?);
        }

        let mut coord_events = Vec::with_capacity(json.coords.len());
        for coord in &json.coords {
            coord_events.push(parse_hgss_coord(coord, &table)?);
        }

        Ok(Self {
            bg_events,
            object_events,
            warp_events,
            coord_events,
        })
    }

    pub fn to_hgss_json(&self, symbols: &SymbolTable, header: Option<String>) -> HgssEventJson {
        HgssEventJson {
            header,
            bgs: self
                .bg_events
                .iter()
                .map(|bg| emit_hgss_bg(bg, symbols))
                .collect(),
            objects: self
                .object_events
                .iter()
                .map(|obj| emit_hgss_object(obj, symbols))
                .collect(),
            warps: self
                .warp_events
                .iter()
                .map(|warp| emit_hgss_warp(warp, symbols))
                .collect(),
            coords: self
                .coord_events
                .iter()
                .map(|coord| emit_hgss_coord(coord, symbols))
                .collect(),
        }
    }
}

fn hgss_symbol_table(
    json: &HgssEventJson,
    symbols: Arc<SymbolTable>,
    project_root: &Path,
) -> io::Result<SymbolTable> {
    let mut local = SymbolTable::new();
    if let Some(header) = &json.header {
        let header_path = resolve_hgss_header_path(project_root, header).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "HGSS event header `{header}` not found under {}",
                    project_root.display()
                ),
            )
        })?;
        let include_dirs = [
            project_root.to_path_buf(),
            project_root.join("include"),
            project_root.join("files"),
        ];
        local.load_recursive(&header_path, &include_dirs)?;
        local.resolve_all();
    }

    let mut table = SymbolTable::with_parent(symbols);
    table.extend(local);
    table.resolve_all();
    Ok(table)
}

fn resolve_hgss_header_path(project_root: &Path, header: &str) -> Option<PathBuf> {
    [
        project_root.join("files").join(header),
        project_root.join(header),
        project_root.join("include").join(header),
    ]
    .into_iter()
    .find(|path| path.exists())
}

fn resolve_hgss_i64(table: &SymbolTable, value: &Value, field: &str) -> io::Result<i64> {
    match value {
        Value::Number(n) => n
            .as_i64()
            .ok_or_else(|| invalid_data(field, format!("invalid number {n}"))),
        Value::String(s) => {
            let expr = s.trim();
            if let Some(value) = try_expand_std_trainer_macro(table, expr) {
                return Ok(value);
            }
            table.evaluate_expression(expr).ok_or_else(|| {
                invalid_data(field, format!("unknown symbol or expression `{expr}`"))
            })
        }
        _ => Err(invalid_data(
            field,
            format!("expected number or string, got {value}"),
        )),
    }
}

fn try_expand_std_trainer_macro(table: &SymbolTable, expr: &str) -> Option<i64> {
    for (macro_name, base_symbol) in [
        ("std_trainer", "_std_npc_trainer"),
        ("std_trainer_2", "_std_npc_trainer_2"),
    ] {
        let prefix = format!("{macro_name}(");
        let Some(rest) = expr.strip_prefix(&prefix) else {
            continue;
        };
        let Some(arg) = rest.strip_suffix(')') else {
            continue;
        };
        let trainer = table.resolve_constant(arg.trim())?;
        let first = table.resolve_constant("FIRST_TRAINER_INDEX")?;
        let base = table.resolve_constant(base_symbol)?;
        return Some(trainer - first + base);
    }
    None
}

fn parse_hgss_bg(bg: &HgssBgEventJson, table: &SymbolTable) -> io::Result<BgEvent> {
    let script = resolve_hgss_i64(table, &bg.script_id, "scriptId")?;
    let event_type = resolve_hgss_i64(table, &bg.event_type, "type")?;
    let x = resolve_hgss_i64(table, &bg.x, "x")?;
    let z = resolve_hgss_i64(table, &bg.z, "z")?;
    let y = resolve_hgss_i64(table, &bg.y, "y")?;
    let player_facing_dir = match &bg.player_facing_dir {
        None => 0,
        Some(dir) => {
            let raw = resolve_hgss_i64(table, dir, "playerFacingDir")?;
            u16::try_from(raw).map_err(|_| {
                invalid_data(
                    "playerFacingDir",
                    format!("value {raw} out of range for u16"),
                )
            })?
        }
    };

    Ok(BgEvent {
        script: u16::try_from(script).map_err(|_| {
            invalid_data("scriptId", format!("value {script} out of range for u16"))
        })?,
        event_type: u16::try_from(event_type).map_err(|_| {
            invalid_data("type", format!("value {event_type} out of range for u16"))
        })?,
        x: i32::try_from(x)
            .map_err(|_| invalid_data("x", format!("value {x} out of range for i32")))?,
        z: i32::try_from(z)
            .map_err(|_| invalid_data("z", format!("value {z} out of range for i32")))?,
        y: i32::try_from(y)
            .map_err(|_| invalid_data("y", format!("value {y} out of range for i32")))?,
        player_facing_dir,
    })
}

fn parse_hgss_object(obj: &HgssObjectEventJson, table: &SymbolTable) -> io::Result<ObjectEvent> {
    let local_id = resolve_hgss_i64(table, &obj.id, "id")?;
    let graphics_id = resolve_hgss_i64(table, &obj.sprite_id, "spriteId")?;
    let movement_type = resolve_hgss_i64(table, &obj.movement, "movement")?;
    let trainer_type = resolve_hgss_i64(table, &obj.event_type, "type")?;
    let hidden_flag = resolve_hgss_i64(table, &obj.event_flag, "eventFlag")?;
    let script = resolve_hgss_i64(table, &obj.script_id, "scriptId")?;
    let dir = resolve_hgss_i64(table, &obj.facing_direction, "facingDirection")?;
    let param0 = resolve_hgss_i64(table, &obj.param0, "param0")?;
    let param1 = resolve_hgss_i64(table, &obj.param1, "param1")?;
    let param2 = resolve_hgss_i64(table, &obj.param2, "param2")?;
    let movement_range_x = resolve_hgss_i64(table, &obj.x_range, "xRange")?;
    let movement_range_z = resolve_hgss_i64(table, &obj.y_range, "yRange")?;
    let x = resolve_hgss_i64(table, &obj.x, "x")?;
    let z = resolve_hgss_i64(table, &obj.z, "z")?;
    let y = resolve_hgss_i64(table, &obj.y, "y")?;

    Ok(ObjectEvent {
        local_id: u16::try_from(local_id)
            .map_err(|_| invalid_data("id", format!("value {local_id} out of range for u16")))?,
        graphics_id: u16::try_from(graphics_id).map_err(|_| {
            invalid_data(
                "spriteId",
                format!("value {graphics_id} out of range for u16"),
            )
        })?,
        movement_type: u16::try_from(movement_type).map_err(|_| {
            invalid_data(
                "movement",
                format!("value {movement_type} out of range for u16"),
            )
        })?,
        trainer_type: u16::try_from(trainer_type).map_err(|_| {
            invalid_data("type", format!("value {trainer_type} out of range for u16"))
        })?,
        hidden_flag: u16::try_from(hidden_flag).map_err(|_| {
            invalid_data(
                "eventFlag",
                format!("value {hidden_flag} out of range for u16"),
            )
        })?,
        script: u16::try_from(script).map_err(|_| {
            invalid_data("scriptId", format!("value {script} out of range for u16"))
        })?,
        dir: i16::try_from(dir).map_err(|_| {
            invalid_data(
                "facingDirection",
                format!("value {dir} out of range for i16"),
            )
        })?,
        data: [
            u16::try_from(param0).map_err(|_| {
                invalid_data("param0", format!("value {param0} out of range for u16"))
            })?,
            u16::try_from(param1).map_err(|_| {
                invalid_data("param1", format!("value {param1} out of range for u16"))
            })?,
            u16::try_from(param2).map_err(|_| {
                invalid_data("param2", format!("value {param2} out of range for u16"))
            })?,
        ],
        movement_range_x: i16::try_from(movement_range_x).map_err(|_| {
            invalid_data(
                "xRange",
                format!("value {movement_range_x} out of range for i16"),
            )
        })?,
        movement_range_z: i16::try_from(movement_range_z).map_err(|_| {
            invalid_data(
                "yRange",
                format!("value {movement_range_z} out of range for i16"),
            )
        })?,
        x: u16::try_from(x)
            .map_err(|_| invalid_data("x", format!("value {x} out of range for u16")))?,
        z: u16::try_from(z)
            .map_err(|_| invalid_data("z", format!("value {z} out of range for u16")))?,
        y: i32::try_from(y)
            .map_err(|_| invalid_data("y", format!("value {y} out of range for i32")))?,
    })
}

fn parse_hgss_warp(warp: &HgssWarpEventJson, table: &SymbolTable) -> io::Result<WarpEvent> {
    let x = resolve_hgss_i64(table, &warp.x, "x")?;
    let z = resolve_hgss_i64(table, &warp.z, "z")?;
    let dest_header_id = resolve_hgss_i64(table, &warp.header, "header")?;
    let dest_warp_id = resolve_hgss_i64(table, &warp.anchor, "anchor")?;
    let height = resolve_hgss_i64(table, &warp.y, "y")?;

    Ok(WarpEvent {
        x: u16::try_from(x)
            .map_err(|_| invalid_data("x", format!("value {x} out of range for u16")))?,
        z: u16::try_from(z)
            .map_err(|_| invalid_data("z", format!("value {z} out of range for u16")))?,
        dest_header_id: u16::try_from(dest_header_id).map_err(|_| {
            invalid_data(
                "header",
                format!("value {dest_header_id} out of range for u16"),
            )
        })?,
        dest_warp_id: u16::try_from(dest_warp_id).map_err(|_| {
            invalid_data(
                "anchor",
                format!("value {dest_warp_id} out of range for u16"),
            )
        })?,
        height: u32::try_from(height)
            .map_err(|_| invalid_data("y", format!("value {height} out of range for u32")))?,
    })
}

fn parse_hgss_coord(coord: &HgssCoordEventJson, table: &SymbolTable) -> io::Result<CoordEvent> {
    let script = resolve_hgss_i64(table, &coord.script_id, "scriptId")?;
    let x = resolve_hgss_i64(table, &coord.x, "x")?;
    let z = resolve_hgss_i64(table, &coord.z, "z")?;
    let width = resolve_hgss_i64(table, &coord.w, "w")?;
    let length = resolve_hgss_i64(table, &coord.h, "h")?;
    let y = resolve_hgss_i64(table, &coord.y, "y")?;
    let value = resolve_hgss_i64(table, &coord.val, "val")?;
    let var = resolve_hgss_i64(table, &coord.var, "var")?;

    Ok(CoordEvent {
        script: u16::try_from(script).map_err(|_| {
            invalid_data("scriptId", format!("value {script} out of range for u16"))
        })?,
        x: i16::try_from(x)
            .map_err(|_| invalid_data("x", format!("value {x} out of range for i16")))?,
        z: i16::try_from(z)
            .map_err(|_| invalid_data("z", format!("value {z} out of range for i16")))?,
        width: u16::try_from(width)
            .map_err(|_| invalid_data("w", format!("value {width} out of range for u16")))?,
        length: u16::try_from(length)
            .map_err(|_| invalid_data("h", format!("value {length} out of range for u16")))?,
        y: u16::try_from(y)
            .map_err(|_| invalid_data("y", format!("value {y} out of range for u16")))?,
        value: u16::try_from(value)
            .map_err(|_| invalid_data("val", format!("value {value} out of range for u16")))?,
        var: u16::try_from(var)
            .map_err(|_| invalid_data("var", format!("value {var} out of range for u16")))?,
    })
}

fn family_symbol_or_number(value: u16, symbols: &SymbolTable, family: ConstantFamily) -> Value {
    symbols
        .resolve_name_in_family(value as i64, family)
        .map(Value::String)
        .unwrap_or_else(|| Value::from(value))
}

fn emit_hgss_bg(bg: &BgEvent, _symbols: &SymbolTable) -> HgssBgEventJson {
    HgssBgEventJson {
        script_id: Value::from(bg.script),
        event_type: Value::from(bg.event_type),
        x: Value::from(bg.x),
        z: Value::from(bg.z),
        y: Value::from(bg.y),
        player_facing_dir: (bg.player_facing_dir != 0).then(|| Value::from(bg.player_facing_dir)),
    }
}

fn emit_hgss_object(obj: &ObjectEvent, symbols: &SymbolTable) -> HgssObjectEventJson {
    HgssObjectEventJson {
        id: Value::from(obj.local_id),
        sprite_id: family_symbol_or_number(obj.graphics_id, symbols, ConstantFamily::Sprite),
        movement: Value::from(obj.movement_type),
        event_type: Value::from(obj.trainer_type),
        event_flag: family_symbol_or_number(obj.hidden_flag, symbols, ConstantFamily::Flag),
        script_id: Value::from(obj.script),
        facing_direction: Value::from(obj.dir),
        param0: Value::from(obj.data[0]),
        param1: Value::from(obj.data[1]),
        param2: Value::from(obj.data[2]),
        x_range: Value::from(obj.movement_range_x),
        y_range: Value::from(obj.movement_range_z),
        x: Value::from(obj.x),
        z: Value::from(obj.z),
        y: Value::from(obj.y),
    }
}

fn emit_hgss_warp(warp: &WarpEvent, symbols: &SymbolTable) -> HgssWarpEventJson {
    HgssWarpEventJson {
        x: Value::from(warp.x),
        z: Value::from(warp.z),
        header: family_symbol_or_number(warp.dest_header_id, symbols, ConstantFamily::Map),
        anchor: Value::from(warp.dest_warp_id),
        y: Value::from(warp.height),
    }
}

fn emit_hgss_coord(coord: &CoordEvent, symbols: &SymbolTable) -> HgssCoordEventJson {
    HgssCoordEventJson {
        script_id: Value::from(coord.script),
        x: Value::from(coord.x),
        z: Value::from(coord.z),
        w: Value::from(coord.width),
        h: Value::from(coord.length),
        y: Value::from(coord.y),
        val: Value::from(coord.value),
        var: family_symbol_or_number(coord.var, symbols, ConstantFamily::Variable),
    }
}
