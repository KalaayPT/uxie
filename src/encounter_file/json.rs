use serde::{Deserialize, Serialize};
use crate::c_parser::SymbolTable;
use crate::encounter_file::binary::{BinaryEncounterFile, EncounterEntry, WaterEncounterEntry};
use crate::game::GameFamily;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterEntryJson {
    pub level: u8,
    pub species: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterEncounterEntryJson {
    pub level_min: u8,
    pub level_max: u8,
    pub species: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonEncounterFile {
    pub land_rate: u32,
    pub land_encounters: Vec<EncounterEntryJson>,
    pub swarms: Vec<String>,
    pub day: Vec<String>,
    pub night: Vec<String>,
    pub radar: Vec<String>,
    pub rate_form0: u32,
    pub rate_form1: u32,
    pub rate_form2: u32,
    pub rate_form3: u32,
    pub rate_form4: u32,
    pub unown_table: u32,
    pub ruby: Vec<String>,
    pub sapphire: Vec<String>,
    pub emerald: Vec<String>,
    pub firered: Vec<String>,
    pub leafgreen: Vec<String>,
    pub surf_rate: u32,
    pub surf_encounters: Vec<WaterEncounterEntryJson>,
    pub old_rod_rate: u32,
    pub old_rod_encounters: Vec<WaterEncounterEntryJson>,
    pub good_rod_rate: u32,
    pub good_rod_encounters: Vec<WaterEncounterEntryJson>,
    pub super_rod_rate: u32,
    pub super_rod_encounters: Vec<WaterEncounterEntryJson>,
    
    // HGSS specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rock_smash_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rock_smash_encounters: Option<Vec<WaterEncounterEntryJson>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub morning: Option<Vec<EncounterEntryJson>>,
}

impl JsonEncounterFile {
    pub fn to_binary(&self, symbols: &SymbolTable, _family: GameFamily) -> BinaryEncounterFile {
        let resolve = |name: &str| {
            symbols.resolve_constant(name)
                .map(|v| v as u32)
                .unwrap_or_else(|| name.parse().unwrap_or(0))
        };

        let grass_encounters = self.land_encounters.iter()
            .map(|e| EncounterEntry { level: e.level, species: resolve(&e.species) })
            .collect();

        let swarm_encounters = self.swarms.iter().map(|s| resolve(s)).collect();
        let day_encounters = self.day.iter().map(|s| resolve(s)).collect();
        let night_encounters = self.night.iter().map(|s| resolve(s)).collect();
        let radar_encounters = self.radar.iter().map(|s| resolve(s)).collect();
        let dual_slot_ruby = self.ruby.iter().map(|s| resolve(s)).collect();
        let dual_slot_sapphire = self.sapphire.iter().map(|s| resolve(s)).collect();
        let dual_slot_emerald = self.emerald.iter().map(|s| resolve(s)).collect();
        let dual_slot_firered = self.firered.iter().map(|s| resolve(s)).collect();
        let dual_slot_leafgreen = self.leafgreen.iter().map(|s| resolve(s)).collect();

        let surf_encounters = self.surf_encounters.iter()
            .map(|e| WaterEncounterEntry { min_level: e.level_min, max_level: e.level_max, species: resolve(&e.species) })
            .collect();
        let old_rod_encounters = self.old_rod_encounters.iter()
            .map(|e| WaterEncounterEntry { min_level: e.level_min, max_level: e.level_max, species: resolve(&e.species) })
            .collect();
        let good_rod_encounters = self.good_rod_encounters.iter()
            .map(|e| WaterEncounterEntry { min_level: e.level_min, max_level: e.level_max, species: resolve(&e.species) })
            .collect();
        let super_rod_encounters = self.super_rod_encounters.iter()
            .map(|e| WaterEncounterEntry { min_level: e.level_min, max_level: e.level_max, species: resolve(&e.species) })
            .collect();

        BinaryEncounterFile {
            walking_rate: self.land_rate,
            grass_encounters,
            swarm_encounters,
            day_encounters,
            night_encounters,
            radar_encounters,
            form_encounter_rates: vec![self.rate_form0, self.rate_form1, self.rate_form2, self.rate_form3, self.rate_form4],
            unown_table_id: self.unown_table,
            dual_slot_ruby,
            dual_slot_sapphire,
            dual_slot_emerald,
            dual_slot_firered,
            dual_slot_leafgreen,
            surf_rate: self.surf_rate,
            surf_encounters,
            old_rod_rate: self.old_rod_rate,
            old_rod_encounters,
            good_rod_rate: self.good_rod_rate,
            good_rod_encounters,
            super_rod_rate: self.super_rod_rate,
            super_rod_encounters,
            rock_smash_rate: self.rock_smash_rate.unwrap_or(0),
            rock_smash_encounters: self.rock_smash_encounters.as_ref().map(|e| e.iter()
                .map(|e| WaterEncounterEntry { min_level: e.level_min, max_level: e.level_max, species: resolve(&e.species) })
                .collect()).unwrap_or_default(),
            morning_encounters: self.morning.as_ref().map(|e| e.iter()
                .map(|e| EncounterEntry { level: e.level, species: resolve(&e.species) })
                .collect()).unwrap_or_default(),
        }
    }

    pub fn from_binary(bin: &BinaryEncounterFile, symbols: &SymbolTable, family: GameFamily) -> Self {
        let resolve = |id: u32| {
            symbols.resolve_name(id as i64, "SPECIES_")
                .unwrap_or_else(|| id.to_string())
        };

        let land_encounters = bin.grass_encounters.iter()
            .map(|e| EncounterEntryJson { level: e.level, species: resolve(e.species) })
            .collect();

        let swarms = bin.swarm_encounters.iter().map(|&s| resolve(s)).collect();
        let day = bin.day_encounters.iter().map(|&s| resolve(s)).collect();
        let night = bin.night_encounters.iter().map(|&s| resolve(s)).collect();
        let radar = bin.radar_encounters.iter().map(|&s| resolve(s)).collect();
        let ruby = bin.dual_slot_ruby.iter().map(|&s| resolve(s)).collect();
        let sapphire = bin.dual_slot_sapphire.iter().map(|&s| resolve(s)).collect();
        let emerald = bin.dual_slot_emerald.iter().map(|&s| resolve(s)).collect();
        let firered = bin.dual_slot_firered.iter().map(|&s| resolve(s)).collect();
        let leafgreen = bin.dual_slot_leafgreen.iter().map(|&s| resolve(s)).collect();

        let surf_encounters = bin.surf_encounters.iter()
            .map(|e| WaterEncounterEntryJson { level_min: e.min_level, level_max: e.max_level, species: resolve(e.species) })
            .collect();
        let old_rod_encounters = bin.old_rod_encounters.iter()
            .map(|e| WaterEncounterEntryJson { level_min: e.min_level, level_max: e.max_level, species: resolve(e.species) })
            .collect();
        let good_rod_encounters = bin.good_rod_encounters.iter()
            .map(|e| WaterEncounterEntryJson { level_min: e.min_level, level_max: e.max_level, species: resolve(e.species) })
            .collect();
        let super_rod_encounters = bin.super_rod_encounters.iter()
            .map(|e| WaterEncounterEntryJson { level_min: e.min_level, level_max: e.max_level, species: resolve(e.species) })
            .collect();

        let mut res = Self {
            land_rate: bin.walking_rate,
            land_encounters,
            swarms,
            day,
            night,
            radar,
            rate_form0: bin.form_encounter_rates.get(0).cloned().unwrap_or(0),
            rate_form1: bin.form_encounter_rates.get(1).cloned().unwrap_or(0),
            rate_form2: bin.form_encounter_rates.get(2).cloned().unwrap_or(0),
            rate_form3: bin.form_encounter_rates.get(3).cloned().unwrap_or(0),
            rate_form4: bin.form_encounter_rates.get(4).cloned().unwrap_or(0),
            unown_table: bin.unown_table_id,
            ruby,
            sapphire,
            emerald,
            firered,
            leafgreen,
            surf_rate: bin.surf_rate,
            surf_encounters,
            old_rod_rate: bin.old_rod_rate,
            old_rod_encounters,
            good_rod_rate: bin.good_rod_rate,
            good_rod_encounters,
            super_rod_rate: bin.super_rod_rate,
            super_rod_encounters,
            rock_smash_rate: None,
            rock_smash_encounters: None,
            morning: None,
        };

        if family == GameFamily::HGSS {
            res.rock_smash_rate = Some(bin.rock_smash_rate);
            res.rock_smash_encounters = Some(bin.rock_smash_encounters.iter()
                .map(|e| WaterEncounterEntryJson { level_min: e.min_level, level_max: e.max_level, species: resolve(e.species) })
                .collect());
            res.morning = Some(bin.morning_encounters.iter()
                .map(|e| EncounterEntryJson { level: e.level, species: resolve(e.species) })
                .collect());
        }

        res
    }
}
