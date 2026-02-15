//! Move data parser for decompilation JSON format
//!
//! Parses `res/battle/moves/{move}/data.json` files and converts to `MoveData`.

use crate::move_data::{MoveData, MoveFlags, MoveSplit};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct DecompMoveData {
    pub name: String,
    #[serde(rename = "class")]
    pub move_class: String,
    #[serde(rename = "type")]
    pub move_type: String,
    pub power: u8,
    pub accuracy: u8,
    pub pp: u8,
    pub effect: MoveEffect,
    pub range: String,
    pub priority: i8,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub contest: Option<ContestData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MoveEffect {
    #[serde(rename = "type")]
    pub effect_type: String,
    #[serde(default)]
    pub chance: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContestData {
    pub effect: String,
    #[serde(rename = "type")]
    pub contest_type: String,
}

impl DecompMoveData {
    pub fn to_move_data<F>(&self, resolve_constant: F) -> MoveData
    where
        F: Fn(&str) -> Option<i64>,
    {
        let battle_effect = resolve_constant(&self.effect.effect_type).unwrap_or(0) as u16;

        let split = match self.move_class.as_str() {
            "CLASS_PHYSICAL" => MoveSplit::Physical,
            "CLASS_SPECIAL" => MoveSplit::Special,
            "CLASS_STATUS" => MoveSplit::Status,
            _ => MoveSplit::Status,
        };

        let move_type = resolve_constant(&self.move_type).unwrap_or(0) as u8;
        let target = resolve_constant(&self.range).unwrap_or(0) as u16;

        let mut flags = MoveFlags::empty();
        for flag in &self.flags {
            match flag.as_str() {
                "MOVE_FLAG_MAKES_CONTACT" => flags |= MoveFlags::MAKES_CONTACT,
                "MOVE_FLAG_CAN_PROTECT" => flags |= MoveFlags::AFFECTED_BY_PROTECT,
                "MOVE_FLAG_CAN_MAGIC_COAT" => flags |= MoveFlags::AFFECTED_BY_MAGIC_COAT,
                "MOVE_FLAG_CAN_SNATCH" => flags |= MoveFlags::AFFECTED_BY_SNATCH,
                "MOVE_FLAG_CAN_MIRROR_MOVE" => flags |= MoveFlags::USABLE_BY_MIRROR_MOVE,
                "MOVE_FLAG_KINGS_ROCK" => flags |= MoveFlags::AFFECTED_BY_KINGS_ROCK,
                _ => {}
            }
        }

        let (contest_appeal, contest_condition) = if let Some(ref contest) = self.contest {
            let appeal = resolve_constant(&contest.effect).unwrap_or(0) as u8;
            let condition = resolve_constant(&contest.contest_type).unwrap_or(0) as u8;
            (appeal, condition)
        } else {
            (0, 0)
        };

        MoveData {
            battle_effect,
            split,
            power: self.power,
            move_type,
            accuracy: self.accuracy,
            pp: self.pp,
            side_effect_chance: self.effect.chance,
            target,
            priority: self.priority,
            flags,
            contest_appeal,
            contest_condition,
        }
    }
}

/// Load move data from a decomp JSON file
pub fn load_move_data_from_json(path: impl AsRef<Path>) -> io::Result<DecompMoveData> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Load all move data from a decomp source directory
///
/// Returns a map of move name (lowercase) to parsed data
pub fn load_all_move_data(
    moves_dir: impl AsRef<Path>,
) -> io::Result<HashMap<String, DecompMoveData>> {
    let mut result = HashMap::new();

    let dir = moves_dir.as_ref();
    if !dir.exists() {
        return Ok(result);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let data_file = path.join("data.json");
            if data_file.exists() {
                if let Some(move_name) = path.file_name().and_then(|n| n.to_str()) {
                    match load_move_data_from_json(&data_file) {
                        Ok(data) => {
                            result.insert(move_name.to_lowercase(), data);
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to parse {}: {}", data_file.display(), e);
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}
