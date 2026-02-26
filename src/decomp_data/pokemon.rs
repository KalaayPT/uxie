//! Pokemon data parser for decompilation JSON format
//!
//! Parses `res/pokemon/{species}/data.json` files and converts to `PersonalData`.

use super::util::{load_json_file, resolve_required_constant};
use crate::personal_data::PersonalData;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

/// JSON structure for decomp Pokemon data
#[derive(Debug, Clone, Deserialize)]
pub struct DecompPokemonData {
    pub base_stats: BaseStats,
    pub types: [String; 2],
    pub catch_rate: u8,
    pub base_exp_reward: u8,
    pub ev_yields: EvYields,
    pub held_items: HeldItems,
    pub gender_ratio: String,
    pub hatch_cycles: u8,
    pub base_friendship: u8,
    pub exp_rate: String,
    pub egg_groups: [String; 2],
    pub abilities: [String; 2],
    pub safari_flee_rate: u8,
    pub body_color: String,
    pub flip_sprite: bool,
    #[serde(default, deserialize_with = "deserialize_icon_palette")]
    pub icon_palette: u8,
    #[serde(default)]
    pub learnset: Option<Learnset>,
    #[serde(default)]
    pub evolutions: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub offspring: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BaseStats {
    pub hp: u8,
    pub attack: u8,
    pub defense: u8,
    pub speed: u8,
    pub special_attack: u8,
    pub special_defense: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EvYields {
    pub hp: u8,
    pub attack: u8,
    pub defense: u8,
    pub speed: u8,
    pub special_attack: u8,
    pub special_defense: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeldItems {
    pub common: String,
    pub rare: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Learnset {
    pub by_level: Vec<(u8, String)>,
    #[serde(default)]
    pub by_tm: Vec<String>,
    #[serde(default)]
    pub by_tutor: Vec<String>,
    #[serde(default)]
    pub egg_moves: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum IconPaletteField {
    Value(u8),
    Variant { base: u8 },
}

fn deserialize_icon_palette<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<IconPaletteField>::deserialize(deserializer)?;
    Ok(match value {
        Some(IconPaletteField::Value(value)) => value,
        Some(IconPaletteField::Variant { base }) => base,
        None => 0,
    })
}

impl DecompPokemonData {
    /// Convert decomp pokemon data into binary [`PersonalData`].
    ///
    /// The `resolve_constant` callback supplies numeric values for symbolic
    /// names used by decomp JSON fields.
    ///
    /// Typical usage is passing through workspace symbol resolution:
    /// `to_personal_data(|name| workspace.resolve_constant(name))`.
    ///
    /// Returns `InvalidData` when any required constant cannot be resolved.
    pub fn to_personal_data<F>(&self, resolve_constant: F) -> io::Result<PersonalData>
    where
        F: Fn(&str) -> Option<i64>,
    {
        // Pack EV yields into u16: 2 bits per stat
        let ev_yield = (self.ev_yields.hp as u16 & 0b11)
            | ((self.ev_yields.attack as u16 & 0b11) << 2)
            | ((self.ev_yields.defense as u16 & 0b11) << 4)
            | ((self.ev_yields.speed as u16 & 0b11) << 6)
            | ((self.ev_yields.special_attack as u16 & 0b11) << 8)
            | ((self.ev_yields.special_defense as u16 & 0b11) << 10);

        // color_flip: lower 7 bits = color, bit 7 = flip
        let color = resolve_required_constant(
            &resolve_constant,
            &self.body_color,
            "body_color",
            "pokemon",
        )? as u8;
        let color_flip = (color & 0x7F) | if self.flip_sprite { 0x80 } else { 0 };

        let type1 =
            resolve_required_constant(&resolve_constant, &self.types[0], "types[0]", "pokemon")?
                as u8;
        let type2 =
            resolve_required_constant(&resolve_constant, &self.types[1], "types[1]", "pokemon")?
                as u8;
        let item1 = resolve_required_constant(
            &resolve_constant,
            &self.held_items.common,
            "held_items.common",
            "pokemon",
        )? as u16;
        let item2 = resolve_required_constant(
            &resolve_constant,
            &self.held_items.rare,
            "held_items.rare",
            "pokemon",
        )? as u16;
        let ability1 = resolve_required_constant(
            &resolve_constant,
            &self.abilities[0],
            "abilities[0]",
            "pokemon",
        )? as u8;
        let ability2 = resolve_required_constant(
            &resolve_constant,
            &self.abilities[1],
            "abilities[1]",
            "pokemon",
        )? as u8;
        let egg_group1 = resolve_required_constant(
            &resolve_constant,
            &self.egg_groups[0],
            "egg_groups[0]",
            "pokemon",
        )? as u8;
        let egg_group2 = resolve_required_constant(
            &resolve_constant,
            &self.egg_groups[1],
            "egg_groups[1]",
            "pokemon",
        )? as u8;
        let gender_ratio = resolve_required_constant(
            &resolve_constant,
            &self.gender_ratio,
            "gender_ratio",
            "pokemon",
        )? as u8;
        let growth_rate =
            resolve_required_constant(&resolve_constant, &self.exp_rate, "exp_rate", "pokemon")?
                as u8;
        let mut tm_compatibility = [0u8; 16];
        if let Some(ref learnset) = self.learnset {
            for tm in &learnset.by_tm {
                if let Some(idx) = parse_tm_index(tm) {
                    if idx < 128 {
                        let byte_idx = (idx / 8) as usize;
                        let bit_idx = idx % 8;
                        tm_compatibility[byte_idx] |= 1 << bit_idx;
                    }
                }
            }
        }

        Ok(PersonalData {
            hp: self.base_stats.hp,
            attack: self.base_stats.attack,
            defense: self.base_stats.defense,
            speed: self.base_stats.speed,
            sp_attack: self.base_stats.special_attack,
            sp_defense: self.base_stats.special_defense,
            type1,
            type2,
            catch_rate: self.catch_rate,
            base_exp: self.base_exp_reward,
            ev_yield,
            item1,
            item2,
            gender_ratio,
            egg_cycles: self.hatch_cycles,
            base_friendship: self.base_friendship,
            growth_rate,
            egg_group1,
            egg_group2,
            ability1,
            ability2,
            safari_flee_rate: self.safari_flee_rate,
            color_flip,
            tm_compatibility,
        })
    }
}

/// Parse TM/HM index from string like "TM01", "TM92", "HM01"
fn parse_tm_index(tm: &str) -> Option<u8> {
    if tm.starts_with("TM") {
        // TM01-TM92 -> indices 0-91
        tm[2..].parse::<u8>().ok().map(|n| n.saturating_sub(1))
    } else if tm.starts_with("HM") {
        // HM01-HM08 -> indices 92-99
        tm[2..].parse::<u8>().ok().map(|n| 91 + n)
    } else {
        None
    }
}

/// Load Pokemon data from a decomp JSON file
pub fn load_pokemon_data_from_json(path: impl AsRef<Path>) -> io::Result<DecompPokemonData> {
    load_json_file(path)
}

/// Load all Pokemon data from a decomp source directory
///
/// Returns a map of species name (lowercase) to parsed data
pub fn load_all_pokemon_data(
    pokemon_dir: impl AsRef<Path>,
) -> io::Result<HashMap<String, DecompPokemonData>> {
    let mut result = HashMap::new();

    let dir = pokemon_dir.as_ref();
    if !dir.exists() {
        return Ok(result);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let data_file = path.join("data.json");
            if data_file.exists() {
                if let Some(species_name) = path.file_name().and_then(|n| n.to_str()) {
                    let data = load_pokemon_data_from_json(&data_file).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Failed to parse {}: {e}", data_file.display()),
                        )
                    })?;
                    result.insert(species_name.to_lowercase(), data);
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
    fn test_parse_tm_index() {
        assert_eq!(parse_tm_index("TM01"), Some(0));
        assert_eq!(parse_tm_index("TM25"), Some(24));
        assert_eq!(parse_tm_index("TM92"), Some(91));
        assert_eq!(parse_tm_index("HM01"), Some(92));
        assert_eq!(parse_tm_index("HM08"), Some(99));
    }

    #[test]
    fn test_load_all_pokemon_data_loads_valid_entries() {
        let dir = tempdir().unwrap();
        let species_dir = dir.path().join("chimchar");
        fs::create_dir_all(&species_dir).unwrap();
        fs::write(
            species_dir.join("data.json"),
            r#"{
  "base_stats": {
    "hp": 44, "attack": 58, "defense": 44, "speed": 61, "special_attack": 58, "special_defense": 44
  },
  "types": ["TYPE_FIRE", "TYPE_FIRE"],
  "catch_rate": 45,
  "base_exp_reward": 62,
  "ev_yields": { "hp": 0, "attack": 0, "defense": 0, "speed": 1, "special_attack": 0, "special_defense": 0 },
  "held_items": { "common": "ITEM_NONE", "rare": "ITEM_NONE" },
  "gender_ratio": "MON_RATIO_MALE_87_5",
  "hatch_cycles": 20,
  "base_friendship": 70,
  "exp_rate": "GROWTH_MEDIUM_SLOW",
  "egg_groups": ["EGG_GROUP_FIELD", "EGG_GROUP_HUMAN_LIKE"],
  "abilities": ["ABILITY_BLAZE", "ABILITY_NONE"],
  "safari_flee_rate": 0,
  "body_color": "BODY_COLOR_BROWN",
  "flip_sprite": false
}"#,
        )
        .unwrap();

        let loaded = load_all_pokemon_data(dir.path()).unwrap();
        assert!(loaded.contains_key("chimchar"));
    }

    #[test]
    fn test_load_all_pokemon_data_invalid_existing_file_returns_error() {
        let dir = tempdir().unwrap();
        let species_dir = dir.path().join("chimchar");
        fs::create_dir_all(&species_dir).unwrap();
        fs::write(species_dir.join("data.json"), "{").unwrap();

        let err = load_all_pokemon_data(dir.path()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Failed to parse"));
        assert!(err.to_string().contains("data.json"));
    }

    #[test]
    fn test_to_personal_data_resolves_required_constants() {
        let data: DecompPokemonData = serde_json::from_str(
            r#"{
  "base_stats": {
    "hp": 44, "attack": 58, "defense": 44, "speed": 61, "special_attack": 58, "special_defense": 44
  },
  "types": ["TYPE_FIRE", "TYPE_FIRE"],
  "catch_rate": 45,
  "base_exp_reward": 62,
  "ev_yields": { "hp": 0, "attack": 0, "defense": 0, "speed": 1, "special_attack": 0, "special_defense": 0 },
  "held_items": { "common": "ITEM_NONE", "rare": "ITEM_NONE" },
  "gender_ratio": "MON_RATIO_MALE_87_5",
  "hatch_cycles": 20,
  "base_friendship": 70,
  "exp_rate": "GROWTH_MEDIUM_SLOW",
  "egg_groups": ["EGG_GROUP_FIELD", "EGG_GROUP_HUMAN_LIKE"],
  "abilities": ["ABILITY_BLAZE", "ABILITY_NONE"],
  "safari_flee_rate": 0,
  "body_color": "BODY_COLOR_BROWN",
  "flip_sprite": false
}"#,
        )
        .unwrap();

        let personal = data
            .to_personal_data(|name| match name {
                "TYPE_FIRE" => Some(10),
                "ITEM_NONE" => Some(0),
                "MON_RATIO_MALE_87_5" => Some(31),
                "GROWTH_MEDIUM_SLOW" => Some(3),
                "EGG_GROUP_FIELD" => Some(5),
                "EGG_GROUP_HUMAN_LIKE" => Some(8),
                "ABILITY_BLAZE" => Some(66),
                "ABILITY_NONE" => Some(0),
                "BODY_COLOR_BROWN" => Some(4),
                _ => None,
            })
            .unwrap();

        assert_eq!(personal.type1, 10);
        assert_eq!(personal.type2, 10);
        assert_eq!(personal.gender_ratio, 31);
        assert_eq!(personal.ability1, 66);
        assert_eq!(personal.color_flip & 0x7F, 4);
    }

    #[test]
    fn test_to_personal_data_unresolved_required_constant_returns_error() {
        let data: DecompPokemonData = serde_json::from_str(
            r#"{
  "base_stats": {
    "hp": 44, "attack": 58, "defense": 44, "speed": 61, "special_attack": 58, "special_defense": 44
  },
  "types": ["TYPE_FIRE", "TYPE_FIRE"],
  "catch_rate": 45,
  "base_exp_reward": 62,
  "ev_yields": { "hp": 0, "attack": 0, "defense": 0, "speed": 1, "special_attack": 0, "special_defense": 0 },
  "held_items": { "common": "ITEM_NONE", "rare": "ITEM_NONE" },
  "gender_ratio": "MON_RATIO_MALE_87_5",
  "hatch_cycles": 20,
  "base_friendship": 70,
  "exp_rate": "GROWTH_MEDIUM_SLOW",
  "egg_groups": ["EGG_GROUP_FIELD", "EGG_GROUP_HUMAN_LIKE"],
  "abilities": ["ABILITY_BLAZE", "ABILITY_NONE"],
  "safari_flee_rate": 0,
  "body_color": "BODY_COLOR_BROWN",
  "flip_sprite": false
}"#,
        )
        .unwrap();

        let err = data
            .to_personal_data(|name| match name {
                "TYPE_FIRE" => Some(10),
                "ITEM_NONE" => Some(0),
                "MON_RATIO_MALE_87_5" => Some(31),
                "GROWTH_MEDIUM_SLOW" => Some(3),
                "EGG_GROUP_FIELD" => Some(5),
                "ABILITY_BLAZE" => Some(66),
                "ABILITY_NONE" => Some(0),
                "BODY_COLOR_BROWN" => Some(4),
                _ => None,
            })
            .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("EGG_GROUP_HUMAN_LIKE"));
        assert!(err.to_string().contains("egg_groups[1]"));
    }

    #[test]
    #[ignore = "requires local Platinum decomp fixture via UXIE_TEST_PLATINUM_DECOMP_PATH"]
    fn integration_load_all_pokemon_data_platinum_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            "decomp_data pokemon integration test",
        ) else {
            return;
        };

        let pokemon_dir = root.join("res/pokemon");
        if !pokemon_dir.exists() {
            eprintln!(
                "Skipping decomp_data pokemon integration test: pokemon directory does not exist: {}",
                pokemon_dir.display()
            );
            return;
        }

        let loaded = load_all_pokemon_data(&pokemon_dir).unwrap();
        assert!(
            !loaded.is_empty(),
            "expected at least one species from {}",
            pokemon_dir.display()
        );

        let expected_name = fs::read_dir(&pokemon_dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir() && path.join("data.json").exists())
            .find_map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_lowercase)
            })
            .expect("expected at least one species directory with data.json");

        assert!(
            loaded.contains_key(&expected_name),
            "expected loaded species table to contain '{}'",
            expected_name
        );
    }

    #[test]
    #[ignore = "requires local HGSS decomp fixture via UXIE_TEST_HGSS_DECOMP_PATH"]
    fn integration_load_all_pokemon_data_hgss_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DECOMP_PATH",
            "decomp_data pokemon integration test (hgss)",
        ) else {
            return;
        };

        let pokemon_dir = root.join("res/pokemon");
        let loaded = load_all_pokemon_data(&pokemon_dir).unwrap();

        if pokemon_dir.exists() {
            assert!(
                !loaded.is_empty(),
                "expected at least one species from {}",
                pokemon_dir.display()
            );
        } else {
            assert!(
                loaded.is_empty(),
                "expected empty species table when HGSS pokemon directory is missing: {}",
                pokemon_dir.display()
            );
        }
    }
}
