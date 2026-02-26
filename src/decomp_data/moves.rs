//! Move data parser for decompilation JSON format
//!
//! Parses `res/battle/moves/{move}/data.json` files and converts to `MoveData`.

use super::util::{load_json_file, resolve_required_constant};
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
    /// Convert decomp move data into binary [`MoveData`].
    ///
    /// The `resolve_constant` callback supplies numeric values for symbolic
    /// names used by decomp JSON (for example `TYPE_NORMAL`).
    ///
    /// Typical usage is passing through workspace symbol resolution:
    /// `to_move_data(|name| workspace.resolve_constant(name))`.
    ///
    /// Returns `InvalidData` when any required constant cannot be resolved.
    pub fn to_move_data<F>(&self, resolve_constant: F) -> io::Result<MoveData>
    where
        F: Fn(&str) -> Option<i64>,
    {
        let battle_effect = resolve_required_constant(
            &resolve_constant,
            &self.effect.effect_type,
            "effect.type",
            "move",
        )? as u16;

        let split = match self.move_class.as_str() {
            "CLASS_PHYSICAL" => MoveSplit::Physical,
            "CLASS_SPECIAL" => MoveSplit::Special,
            "CLASS_STATUS" => MoveSplit::Status,
            _ => MoveSplit::Status,
        };

        let move_type =
            resolve_required_constant(&resolve_constant, &self.move_type, "type", "move")? as u8;
        let target =
            resolve_required_constant(&resolve_constant, &self.range, "range", "move")? as u16;

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
            let appeal = resolve_required_constant(
                &resolve_constant,
                &contest.effect,
                "contest.effect",
                "move",
            )? as u8;
            let condition = resolve_required_constant(
                &resolve_constant,
                &contest.contest_type,
                "contest.type",
                "move",
            )? as u8;
            (appeal, condition)
        } else {
            (0, 0)
        };

        Ok(MoveData {
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
        })
    }
}

/// Load move data from a decomp JSON file
pub fn load_move_data_from_json(path: impl AsRef<Path>) -> io::Result<DecompMoveData> {
    load_json_file(path)
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
                    let data = load_move_data_from_json(&data_file).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Failed to parse {}: {e}", data_file.display()),
                        )
                    })?;
                    result.insert(move_name.to_lowercase(), data);
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_all_move_data_loads_valid_entries() {
        let dir = tempdir().unwrap();
        let move_dir = dir.path().join("tackle");
        fs::create_dir_all(&move_dir).unwrap();
        fs::write(
            move_dir.join("data.json"),
            r#"{
  "name": "Tackle",
  "class": "CLASS_PHYSICAL",
  "type": "TYPE_NORMAL",
  "power": 40,
  "accuracy": 100,
  "pp": 35,
  "effect": { "type": "MOVE_EFFECT_HIT", "chance": 0 },
  "range": "RANGE_ADJACENT_OPPONENTS",
  "priority": 0,
  "flags": []
}"#,
        )
        .unwrap();

        let loaded = load_all_move_data(dir.path()).unwrap();
        assert!(loaded.contains_key("tackle"));
    }

    #[test]
    fn test_load_all_move_data_invalid_existing_file_returns_error() {
        let dir = tempdir().unwrap();
        let move_dir = dir.path().join("tackle");
        fs::create_dir_all(&move_dir).unwrap();
        fs::write(move_dir.join("data.json"), "{").unwrap();

        let err = load_all_move_data(dir.path()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Failed to parse"));
        assert!(err.to_string().contains("data.json"));
    }

    #[test]
    fn test_to_move_data_resolves_required_constants() {
        let data = DecompMoveData {
            name: "Tackle".to_string(),
            move_class: "CLASS_PHYSICAL".to_string(),
            move_type: "TYPE_NORMAL".to_string(),
            power: 40,
            accuracy: 100,
            pp: 35,
            effect: MoveEffect {
                effect_type: "MOVE_EFFECT_HIT".to_string(),
                chance: 0,
            },
            range: "RANGE_ADJACENT_OPPONENTS".to_string(),
            priority: 0,
            flags: vec!["MOVE_FLAG_MAKES_CONTACT".to_string()],
            contest: Some(ContestData {
                effect: "CONTEST_EFFECT_NONE".to_string(),
                contest_type: "CONTEST_TYPE_COOL".to_string(),
            }),
        };

        let move_data = data
            .to_move_data(|name| match name {
                "MOVE_EFFECT_HIT" => Some(1),
                "TYPE_NORMAL" => Some(2),
                "RANGE_ADJACENT_OPPONENTS" => Some(3),
                "CONTEST_EFFECT_NONE" => Some(4),
                "CONTEST_TYPE_COOL" => Some(5),
                _ => None,
            })
            .unwrap();

        assert_eq!(move_data.battle_effect, 1);
        assert_eq!(move_data.move_type, 2);
        assert_eq!(move_data.target, 3);
        assert_eq!(move_data.contest_appeal, 4);
        assert_eq!(move_data.contest_condition, 5);
    }

    #[test]
    fn test_to_move_data_unresolved_required_constant_returns_error() {
        let data = DecompMoveData {
            name: "Tackle".to_string(),
            move_class: "CLASS_PHYSICAL".to_string(),
            move_type: "TYPE_NORMAL".to_string(),
            power: 40,
            accuracy: 100,
            pp: 35,
            effect: MoveEffect {
                effect_type: "MOVE_EFFECT_HIT".to_string(),
                chance: 0,
            },
            range: "RANGE_ADJACENT_OPPONENTS".to_string(),
            priority: 0,
            flags: vec![],
            contest: None,
        };

        let err = data
            .to_move_data(|name| match name {
                "MOVE_EFFECT_HIT" => Some(1),
                "RANGE_ADJACENT_OPPONENTS" => Some(3),
                _ => None,
            })
            .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("TYPE_NORMAL"));
        assert!(err.to_string().contains("type"));
    }

    #[test]
    #[ignore = "requires local Platinum decomp fixture via UXIE_TEST_PLATINUM_DECOMP_PATH"]
    fn integration_load_all_move_data_platinum_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            "decomp_data moves integration test",
        ) else {
            return;
        };

        let moves_dir = root.join("res/battle/moves");
        if !moves_dir.exists() {
            eprintln!(
                "Skipping decomp_data moves integration test: moves directory does not exist: {}",
                moves_dir.display()
            );
            return;
        }

        let loaded = load_all_move_data(&moves_dir).unwrap();
        assert!(
            !loaded.is_empty(),
            "expected at least one move from {}",
            moves_dir.display()
        );

        let expected_name = fs::read_dir(&moves_dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.is_dir() && path.join("data.json").exists())
            .and_then(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_lowercase)
            })
            .expect("expected at least one move directory with data.json");

        assert!(
            loaded.contains_key(&expected_name),
            "expected loaded move table to contain '{}'",
            expected_name
        );
    }

    #[test]
    #[ignore = "requires local HGSS decomp fixture via UXIE_TEST_HGSS_DECOMP_PATH"]
    fn integration_load_all_move_data_hgss_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DECOMP_PATH",
            "decomp_data moves integration test (hgss)",
        ) else {
            return;
        };

        let moves_dir = root.join("res/battle/moves");
        let loaded = load_all_move_data(&moves_dir).unwrap();

        if moves_dir.exists() {
            assert!(
                !loaded.is_empty(),
                "expected at least one move from {}",
                moves_dir.display()
            );
        } else {
            assert!(
                loaded.is_empty(),
                "expected empty move table when HGSS moves directory is missing: {}",
                moves_dir.display()
            );
        }
    }
}
