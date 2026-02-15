//! Trainer data parser for decompilation JSON format
//!
//! Parses `res/trainers/data/{trainer}.json` files and converts to `TrainerData`.

use crate::trainer_data::{AiFlags, PartyPokemon, TrainerData, TrainerFlags, TrainerProperties};
use serde::Deserialize;
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
    #[serde(default)]
    pub power: u8,
    #[serde(default)]
    pub ball_seal: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrainerMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default, rename = "en_US")]
    pub en_us: Option<Vec<String>>,
}

impl DecompTrainerData {
    pub fn to_trainer_data<F>(&self, resolve_constant: F) -> TrainerData
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

        let trainer_class = resolve_constant(&self.trainer_class).unwrap_or(0) as u8;

        let mut items = [0u16; 4];
        for (i, item_name) in self.items.iter().take(4).enumerate() {
            items[i] = resolve_constant(item_name).unwrap_or(0) as u16;
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
            double_battle: if self.double_battle { 1 } else { 0 },
        };

        let party: Vec<PartyPokemon> = self
            .party
            .iter()
            .map(|p| p.to_party_pokemon(&resolve_constant))
            .collect();

        TrainerData { properties, party }
    }
}

impl DecompPartyMember {
    fn to_party_pokemon<F>(&self, resolve_constant: &F) -> PartyPokemon
    where
        F: Fn(&str) -> Option<i64>,
    {
        let species = resolve_constant(&self.species).unwrap_or(0) as u16;

        let held_item = self
            .item
            .as_ref()
            .map(|name| resolve_constant(name).unwrap_or(0) as u16);

        let moves = self.moves.as_ref().map(|move_list| {
            let mut arr = [0u16; 4];
            for (i, move_name) in move_list.iter().take(4).enumerate() {
                arr[i] = resolve_constant(move_name).unwrap_or(0) as u16;
            }
            arr
        });

        PartyPokemon {
            difficulty: self.power,
            gender_ability: 0,
            level: self.level,
            species,
            form: self.form,
            held_item,
            moves,
            ball_seal: if self.ball_seal > 0 {
                Some(self.ball_seal)
            } else {
                None
            },
        }
    }
}

pub fn load_trainer_data_from_json(path: impl AsRef<Path>) -> io::Result<DecompTrainerData> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
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
                match load_trainer_data_from_json(&path) {
                    Ok(data) => {
                        result.insert(trainer_name.to_lowercase(), data);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    Ok(result)
}
