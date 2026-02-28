//! Trainer data parser for decompilation JSON format
//!
//! Parses `res/trainers/data/{trainer}.json` files and converts to `TrainerData`.

use super::util::{load_json_file, resolve_required_constant};
use crate::trainer_data::{AiFlags, PartyPokemon, TrainerData, TrainerFlags, TrainerProperties};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct DecompTrainerData {
    pub name: String,
    #[serde(rename = "class")]
    pub trainer_class: String,
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default)]
    pub ai_flags: Vec<String>,
    #[serde(default)]
    pub double_battle: bool,
    pub party: Vec<DecompPartyMember>,
    #[serde(default)]
    pub messages: Vec<TrainerMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DecompPartyMember {
    pub species: String,
    #[serde(default)]
    pub form: u8,
    pub level: u16,
    #[serde(default)]
    pub item: Option<String>,
    #[serde(default)]
    pub moves: Option<Vec<String>>,
    #[serde(default, alias = "iv_scale", alias = "power")]
    pub difficulty: u16,
    #[serde(default, rename = "genderOverride")]
    pub gender_override: Option<String>,
    #[serde(default, rename = "abilityOverride")]
    pub ability_override: Option<String>,
    #[serde(default, alias = "ball_seal", alias = "capsule")]
    pub capsule: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrainerMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(
        default,
        rename = "en_US",
        deserialize_with = "deserialize_optional_string_or_vec"
    )]
    pub en_us: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum OneOrManyStrings {
    One(String),
    Many(Vec<String>),
}

fn deserialize_optional_string_or_vec<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<OneOrManyStrings>::deserialize(deserializer)?;
    Ok(value.map(|value| match value {
        OneOrManyStrings::One(s) => vec![s],
        OneOrManyStrings::Many(v) => v,
    }))
}

fn resolve_override_symbol<F>(
    value: &str,
    resolve_constant: &F,
    known: &[(&str, u8)],
    field: &str,
) -> io::Result<u8>
where
    F: Fn(&str) -> Option<i64>,
{
    if let Some(resolved) = resolve_constant(value) {
        return Ok(resolved as u8);
    }
    if let Some(value_id) = known
        .iter()
        .find_map(|(name, value_id)| (*name == value).then_some(*value_id))
    {
        return Ok(value_id);
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Failed to resolve override constant '{}' for trainer field '{}'",
            value, field
        ),
    ))
}

fn pack_gender_ability_override<F>(
    gender_override: Option<&str>,
    ability_override: Option<&str>,
    resolve_constant: &F,
) -> io::Result<u8>
where
    F: Fn(&str) -> Option<i64>,
{
    let gender = match gender_override {
        Some(value) => resolve_override_symbol(
            value,
            resolve_constant,
            &[
                ("TRPOKE_GENDER_OVERRIDE_OFF", 0),
                ("TRPOKE_GENDER_OVERRIDE_MALE", 1),
                ("TRPOKE_GENDER_OVERRIDE_FEMALE", 2),
            ],
            "genderOverride",
        )?,
        None => 0,
    } & 0x0F;

    let ability = match ability_override {
        Some(value) => resolve_override_symbol(
            value,
            resolve_constant,
            &[
                ("TRPOKE_ABILITY_OVERRIDE_OFF", 0),
                ("TRPOKE_ABILITY_OVERRIDE_FIRST", 1),
                ("TRPOKE_ABILITY_OVERRIDE_SECOND", 2),
            ],
            "abilityOverride",
        )?,
        None => 0,
    } & 0x0F;

    Ok(gender | (ability << 4))
}

impl DecompTrainerData {
    /// Convert decomp trainer data into binary [`TrainerData`].
    ///
    /// The `resolve_constant` callback supplies numeric values for symbolic
    /// names used by decomp JSON fields.
    ///
    /// Typical usage is passing through workspace symbol resolution:
    /// `to_trainer_data(|name| workspace.resolve_constant(name))`.
    ///
    /// Returns `InvalidData` when any required constant cannot be resolved.
    pub fn to_trainer_data<F>(&self, resolve_constant: F) -> io::Result<TrainerData>
    where
        F: Fn(&str) -> Option<i64>,
    {
        let has_moves = self.party.iter().any(|p| p.moves.is_some());
        let has_items = self.party.iter().any(|p| p.item.is_some());

        let mut flags = TrainerFlags::empty();
        if has_moves {
            flags |= TrainerFlags::HAS_MOVES;
        }
        if has_items {
            flags |= TrainerFlags::HAS_ITEMS;
        }

        let trainer_class = resolve_required_constant(
            &resolve_constant,
            &self.trainer_class,
            "trainer_class",
            "trainer",
        )? as u8;

        let mut items = [0u16; 4];
        for (i, item_name) in self.items.iter().take(4).enumerate() {
            let field = format!("items[{i}]");
            items[i] =
                resolve_required_constant(&resolve_constant, item_name, &field, "trainer")? as u16;
        }

        let mut ai_flags = AiFlags::empty();
        for flag in &self.ai_flags {
            match flag.as_str() {
                "AI_FLAG_BASIC" => ai_flags |= AiFlags::BASIC,
                "AI_FLAG_EVAL_ATTACK" => ai_flags |= AiFlags::EVAL_ATTACK,
                "AI_FLAG_EXPERT" => ai_flags |= AiFlags::EXPERT,
                "AI_FLAG_SETUP_FIRST_TURN" => ai_flags |= AiFlags::SETUP_FIRST_TURN,
                "AI_FLAG_RISKY" => ai_flags |= AiFlags::RISKY,
                "AI_FLAG_DAMAGE_PRIORITY" => ai_flags |= AiFlags::DAMAGE_PRIORITY,
                "AI_FLAG_BATON_PASS" => ai_flags |= AiFlags::BATON_PASS,
                "AI_FLAG_TAG_STRATEGY" => ai_flags |= AiFlags::TAG_STRATEGY,
                "AI_FLAG_CHECK_HP" => ai_flags |= AiFlags::CHECK_HP,
                "AI_FLAG_WEATHER" => ai_flags |= AiFlags::WEATHER,
                "AI_FLAG_HARRASSMENT" => ai_flags |= AiFlags::HARRASSMENT,
                "AI_FLAG_ROAMING_POKEMON" => ai_flags |= AiFlags::ROAMING_POKEMON,
                "AI_FLAG_SAFARI" => ai_flags |= AiFlags::SAFARI,
                "AI_FLAG_CATCH_TUTORIAL" => ai_flags |= AiFlags::CATCH_TUTORIAL,
                _ => {}
            }
        }

        let properties = TrainerProperties {
            flags,
            trainer_class,
            unknown: 0,
            party_count: self.party.len() as u8,
            items,
            ai_flags,
            double_battle: u32::from(self.double_battle),
        };

        let party: Vec<PartyPokemon> = self
            .party
            .iter()
            .map(|p| p.to_party_pokemon(&resolve_constant))
            .collect::<io::Result<Vec<_>>>()?;

        Ok(TrainerData { properties, party })
    }
}

impl DecompPartyMember {
    fn to_party_pokemon<F>(&self, resolve_constant: &F) -> io::Result<PartyPokemon>
    where
        F: Fn(&str) -> Option<i64>,
    {
        let species =
            resolve_required_constant(resolve_constant, &self.species, "party.species", "trainer")?
                as u16;

        let held_item = if let Some(name) = self.item.as_ref() {
            Some(resolve_required_constant(resolve_constant, name, "party.item", "trainer")? as u16)
        } else {
            None
        };

        let moves = if let Some(move_list) = self.moves.as_ref() {
            let mut arr = [0u16; 4];
            for (i, move_name) in move_list.iter().take(4).enumerate() {
                let field = format!("party.moves[{i}]");
                arr[i] = resolve_required_constant(resolve_constant, move_name, &field, "trainer")?
                    as u16;
            }
            Some(arr)
        } else {
            None
        };

        Ok(PartyPokemon {
            difficulty: self.difficulty,
            gender_ability_override: pack_gender_ability_override(
                self.gender_override.as_deref(),
                self.ability_override.as_deref(),
                resolve_constant,
            )?,
            level: self.level,
            species,
            form: self.form,
            held_item,
            moves,
            ball_seal: if self.capsule > 0 {
                Some(self.capsule)
            } else {
                None
            },
        })
    }
}

pub fn load_trainer_data_from_json(path: impl AsRef<Path>) -> io::Result<DecompTrainerData> {
    load_json_file(path)
}

pub fn load_all_trainer_data(
    trainers_dir: impl AsRef<Path>,
) -> io::Result<HashMap<String, DecompTrainerData>> {
    let mut result = HashMap::new();

    let dir = trainers_dir.as_ref();
    if !dir.exists() {
        return Ok(result);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(trainer_name) = path.file_stem().and_then(|n| n.to_str()) {
                let data = load_trainer_data_from_json(&path).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Failed to parse {}: {e}", path.display()),
                    )
                })?;
                result.insert(trainer_name.to_lowercase(), data);
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
    fn test_load_all_trainer_data_loads_valid_entries() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("rival.json"),
            r#"{
  "name": "Rival",
  "class": "TRAINER_CLASS_RIVAL",
  "items": [],
  "ai_flags": [],
  "double_battle": false,
  "party": [
    { "species": "SPECIES_CHIMCHAR", "level": 5, "form": 0, "iv_scale": 0, "ball_seal": 0 }
  ],
  "messages": []
}"#,
        )
        .unwrap();

        let loaded = load_all_trainer_data(dir.path()).unwrap();
        assert!(loaded.contains_key("rival"));
    }

    #[test]
    fn test_to_trainer_data_supports_platinum_party_shape() {
        let data: DecompTrainerData = serde_json::from_str(
            r#"{
  "name": "Ace Trainer",
  "class": "TRAINER_CLASS_ACE_TRAINER",
  "items": [],
  "ai_flags": [],
  "double_battle": false,
  "party": [
    {
      "species": "SPECIES_GLALIE",
      "form": 0,
      "level": 59,
      "item": null,
      "moves": null,
      "iv_scale": 50,
      "ball_seal": 7
    }
  ],
  "messages": []
}"#,
        )
        .unwrap();

        let trainer = data
            .to_trainer_data(|name| match name {
                "TRAINER_CLASS_ACE_TRAINER" => Some(12),
                "SPECIES_GLALIE" => Some(362),
                _ => None,
            })
            .unwrap();

        assert_eq!(trainer.party.len(), 1);
        let mon = &trainer.party[0];
        assert_eq!(mon.difficulty, 50);
        assert_eq!(mon.gender_ability_override, 0);
        assert_eq!(mon.level, 59);
        assert_eq!(mon.species, 362);
        assert_eq!(mon.ball_seal, Some(7));
    }

    #[test]
    fn test_to_trainer_data_supports_hgss_party_shape() {
        let data: DecompTrainerData = serde_json::from_str(
            r#"{
  "name": "Rival",
  "class": "TRAINER_CLASS_RIVAL",
  "items": [],
  "ai_flags": [],
  "double_battle": false,
  "party": [
    {
      "difficulty": 30,
      "genderOverride": "TRPOKE_GENDER_OVERRIDE_MALE",
      "abilityOverride": "TRPOKE_ABILITY_OVERRIDE_SECOND",
      "level": 14,
      "species": "SPECIES_GASTLY",
      "capsule": 3
    }
  ],
  "messages": []
}"#,
        )
        .unwrap();

        let trainer = data
            .to_trainer_data(|name| match name {
                "TRAINER_CLASS_RIVAL" => Some(7),
                "SPECIES_GASTLY" => Some(92),
                _ => None,
            })
            .unwrap();

        assert_eq!(trainer.party.len(), 1);
        let mon = &trainer.party[0];
        assert_eq!(mon.difficulty, 30);
        assert_eq!(mon.gender_ability_override, 0x21);
        assert_eq!(mon.level, 14);
        assert_eq!(mon.species, 92);
        assert_eq!(mon.ball_seal, Some(3));
    }

    #[test]
    fn test_to_trainer_data_unresolved_required_constant_returns_error() {
        let data: DecompTrainerData = serde_json::from_str(
            r#"{
  "name": "Rival",
  "class": "TRAINER_CLASS_RIVAL",
  "items": [],
  "ai_flags": [],
  "double_battle": false,
  "party": [
    { "species": "SPECIES_GASTLY", "level": 14, "difficulty": 30 }
  ],
  "messages": []
}"#,
        )
        .unwrap();

        let err = data
            .to_trainer_data(|name| match name {
                "TRAINER_CLASS_RIVAL" => Some(7),
                _ => None,
            })
            .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("SPECIES_GASTLY"));
        assert!(err.to_string().contains("party.species"));
    }

    #[test]
    fn test_to_trainer_data_unresolved_override_constant_returns_error() {
        let data: DecompTrainerData = serde_json::from_str(
            r#"{
  "name": "Rival",
  "class": "TRAINER_CLASS_RIVAL",
  "items": [],
  "ai_flags": [],
  "double_battle": false,
  "party": [
    {
      "difficulty": 30,
      "genderOverride": "TRPOKE_GENDER_OVERRIDE_UNKNOWN",
      "level": 14,
      "species": "SPECIES_GASTLY"
    }
  ],
  "messages": []
}"#,
        )
        .unwrap();

        let err = data
            .to_trainer_data(|name| match name {
                "TRAINER_CLASS_RIVAL" => Some(7),
                "SPECIES_GASTLY" => Some(92),
                _ => None,
            })
            .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("TRPOKE_GENDER_OVERRIDE_UNKNOWN"));
        assert!(err.to_string().contains("genderOverride"));
    }

    #[test]
    fn test_load_all_trainer_data_invalid_existing_file_returns_error() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("rival.json"), "{").unwrap();

        let err = load_all_trainer_data(dir.path()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Failed to parse"));
        assert!(err.to_string().contains("rival.json"));
    }

    #[test]
    #[ignore = "requires local Platinum decomp fixture via UXIE_TEST_PLATINUM_DECOMP_PATH"]
    fn integration_load_all_trainer_data_platinum_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            "decomp_data trainers integration test",
        ) else {
            return;
        };

        let trainers_dir = root.join("res/trainers/data");
        if !trainers_dir.exists() {
            eprintln!(
                "Skipping decomp_data trainers integration test: trainers directory does not exist: {}",
                trainers_dir.display()
            );
            return;
        }

        let loaded = load_all_trainer_data(&trainers_dir).unwrap();
        assert!(
            !loaded.is_empty(),
            "expected at least one trainer from {}",
            trainers_dir.display()
        );

        let expected_name = fs::read_dir(&trainers_dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .find_map(|path| {
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .map(str::to_lowercase)
            })
            .expect("expected at least one trainer json file");

        assert!(
            loaded.contains_key(&expected_name),
            "expected loaded trainer table to contain '{}'",
            expected_name
        );
    }

    #[test]
    #[ignore = "requires local HGSS decomp fixture via UXIE_TEST_HGSS_DECOMP_PATH"]
    fn integration_load_all_trainer_data_hgss_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DECOMP_PATH",
            "decomp_data trainers integration test (hgss)",
        ) else {
            return;
        };

        let trainers_dir = root.join("res/trainers/data");
        let loaded = load_all_trainer_data(&trainers_dir).unwrap();

        if trainers_dir.exists() {
            assert!(
                !loaded.is_empty(),
                "expected at least one trainer from {}",
                trainers_dir.display()
            );
        } else {
            assert!(
                loaded.is_empty(),
                "expected empty trainer table when HGSS trainers directory is missing: {}",
                trainers_dir.display()
            );
        }
    }
}
