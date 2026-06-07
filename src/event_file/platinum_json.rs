//! Platinum-shaped decomp JSON for event files.

use super::{BgEvent, CoordEvent, EventFile, ObjectEvent, WarpEvent, invalid_data};
use crate::c_parser::{ConstantFamily, SymbolTable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;

/// Background event (spawnable)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BgEventJson {
    pub script: u16,
    #[serde(rename = "type")]
    pub event_type: Value,
    pub x: i32,
    pub z: i32,
    pub y: i32,
    #[serde(default, rename = "player_facing_dir")]
    pub player_facing_dir: Option<Value>,
}

/// Object event (NPC, items, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectEventJson {
    pub id: String,
    #[serde(rename = "graphics_id")]
    pub graphics_id: Value,
    #[serde(rename = "movement_type")]
    pub movement_type: Value,
    #[serde(rename = "trainer_type")]
    pub trainer_type: Value,
    #[serde(default, rename = "hidden_flag")]
    pub hidden_flag: Option<Value>,
    pub script: Value,
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
    pub clone_id: Option<u16>,
    #[serde(default, rename = "double_battle_id")]
    pub double_battle_id: Option<u8>,
}

/// Warp event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarpEventJson {
    pub x: u16,
    pub z: u16,
    #[serde(rename = "dest_header_id")]
    pub dest_header_id: Value,
    #[serde(rename = "dest_warp_id")]
    pub dest_warp_id: u16,
}

/// Coordinate event (triggers)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordEventJson {
    pub script: u16,
    pub x: i16,
    pub z: i16,
    pub width: u16,
    pub length: u16,
    pub y: u16,
    pub value: Value,
    pub var: Option<Value>,
}

/// Platinum decomp JSON event file container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatinumEventJson {
    #[serde(rename = "bg_events")]
    pub bg_events: Vec<BgEventJson>,
    #[serde(rename = "object_events")]
    pub object_events: Vec<ObjectEventJson>,
    #[serde(rename = "warp_events")]
    pub warp_events: Vec<WarpEventJson>,
    #[serde(rename = "coord_events")]
    pub coord_events: Vec<CoordEventJson>,
}

const PLATINUM_OBJECT_Y_SCALE: i32 = 0x10000;

fn platinum_json_y_from_binary(y: i32) -> io::Result<i32> {
    if y % PLATINUM_OBJECT_Y_SCALE != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("object y {y} is not a multiple of 0x10000"),
        ));
    }
    Ok(y / PLATINUM_OBJECT_Y_SCALE)
}

fn platinum_binary_y_from_json(y: i32) -> io::Result<i32> {
    let scaled = i64::from(y) * i64::from(PLATINUM_OBJECT_Y_SCALE);
    i32::try_from(scaled).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("object y {y} overflows when scaled for Platinum binary"),
        )
    })
}

impl EventFile {
    pub fn from_platinum_json(json: &PlatinumEventJson, symbols: &SymbolTable) -> io::Result<Self> {
        let mut bg_events = Vec::with_capacity(json.bg_events.len());
        for bg in &json.bg_events {
            bg_events.push(parse_bg_event(bg, symbols)?);
        }

        let mut object_events = Vec::with_capacity(json.object_events.len());
        for (index, obj) in json.object_events.iter().enumerate() {
            object_events.push(parse_object_event(obj, index, symbols)?);
        }

        let mut warp_events = Vec::with_capacity(json.warp_events.len());
        for warp in &json.warp_events {
            warp_events.push(parse_warp_event(warp, symbols)?);
        }

        let mut coord_events = Vec::with_capacity(json.coord_events.len());
        for coord in &json.coord_events {
            coord_events.push(parse_coord_event(coord, symbols)?);
        }

        Ok(Self {
            bg_events,
            object_events,
            warp_events,
            coord_events,
        })
    }

    pub fn to_platinum_json(&self, symbols: &SymbolTable) -> io::Result<PlatinumEventJson> {
        Ok(PlatinumEventJson {
            bg_events: self
                .bg_events
                .iter()
                .map(|bg| emit_bg_event(bg, symbols))
                .collect(),
            object_events: self
                .object_events
                .iter()
                .enumerate()
                .map(|(index, obj)| emit_object_event(obj, index, symbols))
                .collect::<io::Result<_>>()?,
            warp_events: self
                .warp_events
                .iter()
                .map(|warp| emit_warp_event(warp, symbols))
                .collect::<io::Result<_>>()?,
            coord_events: self
                .coord_events
                .iter()
                .map(|coord| emit_coord_event(coord, symbols))
                .collect(),
        })
    }
}

impl PlatinumEventJson {
    pub fn from_event_file(file: &EventFile, symbols: &SymbolTable) -> io::Result<Self> {
        file.to_platinum_json(symbols)
    }
}

fn eval_i64(symbols: &SymbolTable, value: &Value, field: &str) -> io::Result<i64> {
    match value {
        Value::Number(n) => n
            .as_i64()
            .ok_or_else(|| invalid_data(field, format!("invalid number {n}"))),
        Value::Bool(value) => Ok(i64::from(*value)),
        Value::String(expr) => {
            let expr = expr.trim();
            symbols.evaluate_expression(expr).ok_or_else(|| {
                invalid_data(field, format!("unknown symbol or expression `{expr}`"))
            })
        }
        _ => Err(invalid_data(
            field,
            format!("expected number, string, or boolean, got {value}"),
        )),
    }
}

fn eval_u16(symbols: &SymbolTable, value: &Value, field: &str) -> io::Result<u16> {
    let raw = eval_i64(symbols, value, field)?;
    u16::try_from(raw).map_err(|_| invalid_data(field, format!("value {raw} out of range for u16")))
}

fn eval_u16_str(symbols: &SymbolTable, expr: &str, field: &str) -> io::Result<u16> {
    let expr = expr.trim();
    let raw = symbols
        .evaluate_expression(expr)
        .ok_or_else(|| invalid_data(field, format!("unknown symbol or expression `{expr}`")))?;
    u16::try_from(raw)
        .map_err(|_| invalid_data(field, format!("`{expr}` = {raw} out of range for u16")))
}

fn resolve_platinum_script(
    symbols: &SymbolTable,
    value: &Value,
    double_battle_id: Option<u8>,
) -> io::Result<u16> {
    // A numeric script is used directly.
    if !matches!(value, Value::String(_)) {
        let raw = eval_i64(symbols, value, "script")?;
        return u16::try_from(raw)
            .map_err(|_| invalid_data("script", format!("script {raw} out of range for u16")));
    }
    let name = value
        .as_str()
        .ok_or_else(|| invalid_data("script", "expected number or string"))?;

    // TRAINER_* constants use a special base offset instead of their raw value.
    if name.starts_with("TRAINER_")
        && let Some(trainer_id) = symbols.resolve_constant(name)
    {
        let trainer_id = u16::try_from(trainer_id)
            .map_err(|_| invalid_data("script", format!("trainer constant {name} out of range")))?;
        let base: u16 = if double_battle_id == Some(2) {
            5000
        } else {
            3000
        };
        return trainer_id
            .checked_sub(1)
            .and_then(|offset| base.checked_add(offset))
            .ok_or_else(|| invalid_data("script", format!("trainer script overflow for {name}")));
    }

    eval_u16_str(symbols, name, "script")
}

fn parse_bg_event(bg: &BgEventJson, symbols: &SymbolTable) -> io::Result<BgEvent> {
    Ok(BgEvent {
        script: bg.script,
        event_type: eval_u16(symbols, &bg.event_type, "type")?,
        x: bg.x,
        z: bg.z,
        y: bg.y,
        player_facing_dir: match &bg.player_facing_dir {
            None => 0,
            Some(dir) => eval_u16(symbols, dir, "player_facing_dir")?,
        },
    })
}

fn parse_object_event(
    obj: &ObjectEventJson,
    index: usize,
    symbols: &SymbolTable,
) -> io::Result<ObjectEvent> {
    if obj.data.len() > 3 {
        return Err(invalid_data(
            "data",
            format!("object data has {} entries, maximum is 3", obj.data.len()),
        ));
    }
    let mut data = [0u16; 3];
    for (slot, value) in obj.data.iter().enumerate() {
        data[slot] = *value;
    }

    Ok(ObjectEvent {
        local_id: obj.clone_id.unwrap_or(
            u16::try_from(index).map_err(|_| {
                invalid_data("clone_id", format!("object index {index} exceeds u16"))
            })?,
        ),
        graphics_id: eval_u16(symbols, &obj.graphics_id, "graphics_id")?,
        movement_type: eval_u16(symbols, &obj.movement_type, "movement_type")?,
        trainer_type: eval_u16(symbols, &obj.trainer_type, "trainer_type")?,
        hidden_flag: match &obj.hidden_flag {
            None => 0,
            Some(flag) => eval_u16(symbols, flag, "hidden_flag")?,
        },
        script: resolve_platinum_script(symbols, &obj.script, obj.double_battle_id)?,
        dir: obj.initial_dir,
        data,
        movement_range_x: obj.movement_range_x,
        movement_range_z: obj.movement_range_z,
        x: obj.x,
        z: obj.z,
        y: platinum_binary_y_from_json(obj.y)?,
    })
}

fn parse_warp_event(warp: &WarpEventJson, symbols: &SymbolTable) -> io::Result<WarpEvent> {
    Ok(WarpEvent {
        x: warp.x,
        z: warp.z,
        dest_header_id: eval_u16(symbols, &warp.dest_header_id, "dest_header_id")?,
        dest_warp_id: warp.dest_warp_id,
        height: 0,
    })
}

fn parse_coord_event(coord: &CoordEventJson, symbols: &SymbolTable) -> io::Result<CoordEvent> {
    Ok(CoordEvent {
        script: coord.script,
        x: coord.x,
        z: coord.z,
        width: coord.width,
        length: coord.length,
        y: coord.y,
        value: eval_u16(symbols, &coord.value, "value")?,
        var: match &coord.var {
            None => 0,
            Some(name) => eval_u16(symbols, name, "var")?,
        },
    })
}

fn emit_bg_event(bg: &BgEvent, symbols: &SymbolTable) -> BgEventJson {
    BgEventJson {
        script: bg.script,
        event_type: family_symbol_or_number(bg.event_type, symbols, ConstantFamily::BgEventType),
        x: bg.x,
        z: bg.z,
        y: bg.y,
        player_facing_dir: if bg.player_facing_dir == 0 {
            None
        } else {
            Some(family_symbol_or_number(
                bg.player_facing_dir,
                symbols,
                ConstantFamily::BgEventDir,
            ))
        },
    }
}

fn emit_object_event(
    obj: &ObjectEvent,
    index: usize,
    symbols: &SymbolTable,
) -> io::Result<ObjectEventJson> {
    let json_y = platinum_json_y_from_binary(obj.y)?;
    let data_len = obj
        .data
        .iter()
        .rposition(|value| *value != 0)
        .map_or(0, |index| index + 1);
    let data = obj.data[..data_len].to_vec();

    let clone_id = (obj.local_id as usize != index).then_some(obj.local_id);
    let (script, double_battle_id) = emit_platinum_script(obj.script, symbols);

    Ok(ObjectEventJson {
        id: format!("OBJ_{}", obj.local_id),
        graphics_id: family_symbol_or_number(
            obj.graphics_id,
            symbols,
            ConstantFamily::ObjectGraphics,
        ),
        movement_type: family_symbol_or_number(
            obj.movement_type,
            symbols,
            ConstantFamily::MovementType,
        ),
        trainer_type: family_symbol_or_number(
            obj.trainer_type,
            symbols,
            ConstantFamily::TrainerType,
        ),
        hidden_flag: Some(emit_hidden_flag(obj.hidden_flag, symbols)),
        script,
        initial_dir: obj.dir,
        data,
        movement_range_x: obj.movement_range_x,
        movement_range_z: obj.movement_range_z,
        x: obj.x,
        z: obj.z,
        y: json_y,
        clone_id,
        double_battle_id,
    })
}

fn emit_platinum_script(script: u16, symbols: &SymbolTable) -> (Value, Option<u8>) {
    for (base, double_battle_id) in [(5000u16, Some(2)), (3000u16, None)] {
        let Some(offset) = script.checked_sub(base) else {
            continue;
        };
        let trainer_id = u32::from(offset) + 1;
        if trainer_id > u16::MAX as u32 {
            continue;
        }
        if let Some(name) =
            symbols.resolve_name_in_family(i64::from(trainer_id), ConstantFamily::Trainer)
        {
            return (Value::String(name), double_battle_id);
        }
    }

    (Value::from(script), None)
}

fn emit_hidden_flag(value: u16, symbols: &SymbolTable) -> Value {
    if value == 0 {
        return Value::from(0);
    }
    symbols
        .resolve_name_in_family(value as i64, ConstantFamily::Flag)
        .or_else(|| symbols.resolve_name_in_family(value as i64, ConstantFamily::Variable))
        .or_else(|| symbols.resolve_name_in_family(value as i64, ConstantFamily::MapHeader))
        .map(Value::String)
        .unwrap_or_else(|| Value::from(value))
}

fn emit_warp_event(warp: &WarpEvent, symbols: &SymbolTable) -> io::Result<WarpEventJson> {
    if warp.height != 0 {
        return Err(invalid_data(
            "warp.height",
            format!(
                "Platinum JSON cannot represent non-zero warp height {}",
                warp.height
            ),
        ));
    }

    Ok(WarpEventJson {
        x: warp.x,
        z: warp.z,
        dest_header_id: family_symbol_or_number(
            warp.dest_header_id,
            symbols,
            ConstantFamily::MapHeader,
        ),
        dest_warp_id: warp.dest_warp_id,
    })
}

fn emit_coord_event(coord: &CoordEvent, symbols: &SymbolTable) -> CoordEventJson {
    CoordEventJson {
        script: coord.script,
        x: coord.x,
        z: coord.z,
        width: coord.width,
        length: coord.length,
        y: coord.y,
        value: Value::from(coord.value),
        var: if coord.var == 0 {
            None
        } else {
            Some(family_symbol_or_number(
                coord.var,
                symbols,
                ConstantFamily::Variable,
            ))
        },
    }
}

fn family_symbol_or_number(value: u16, symbols: &SymbolTable, family: ConstantFamily) -> Value {
    symbols
        .resolve_name_in_family(value as i64, family)
        .map(Value::String)
        .unwrap_or_else(|| Value::from(value))
}
