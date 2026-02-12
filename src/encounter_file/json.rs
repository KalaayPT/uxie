use crate::c_parser::SymbolTable;
use crate::encounter_file::binary::{BinaryEncounterFile, EncounterEntry, WaterEncounterEntry};
use crate::game::GameFamily;
use serde::{Deserialize, Serialize};

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
    pub music: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rock_smash_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rock_smash_encounters: Option<Vec<WaterEncounterEntryJson>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub morning: Option<Vec<EncounterEntryJson>>,
}

fn collect_encounter_array<const N: usize>(
    entries: &[EncounterEntryJson],
    resolve: &dyn Fn(&str) -> u32,
) -> [EncounterEntry; N] {
    let mut result = [EncounterEntry::default(); N];
    for (slot, entry) in result.iter_mut().zip(entries.iter()) {
        *slot = EncounterEntry {
            level: entry.level,
            species: resolve(&entry.species),
        };
    }
    result
}

fn collect_species_array<const N: usize>(
    names: &[String],
    resolve: &dyn Fn(&str) -> u32,
) -> [u32; N] {
    let mut result = [0u32; N];
    for (slot, name) in result.iter_mut().zip(names.iter()) {
        *slot = resolve(name);
    }
    result
}

fn collect_water_array<const N: usize>(
    entries: &[WaterEncounterEntryJson],
    resolve: &dyn Fn(&str) -> u32,
) -> [WaterEncounterEntry; N] {
    let mut result = [WaterEncounterEntry::default(); N];
    for (slot, entry) in result.iter_mut().zip(entries.iter()) {
        *slot = WaterEncounterEntry {
            min_level: entry.level_min,
            max_level: entry.level_max,
            species: resolve(&entry.species),
        };
    }
    result
}

fn encounter_entries_to_json(
    entries: &[EncounterEntry],
    resolve: &dyn Fn(u32) -> String,
) -> Vec<EncounterEntryJson> {
    entries
        .iter()
        .map(|e| EncounterEntryJson {
            level: e.level,
            species: resolve(e.species),
        })
        .collect()
}

fn species_to_json(ids: &[u32], resolve: &dyn Fn(u32) -> String) -> Vec<String> {
    ids.iter().map(|&s| resolve(s)).collect()
}

fn water_entries_to_json(
    entries: &[WaterEncounterEntry],
    resolve: &dyn Fn(u32) -> String,
) -> Vec<WaterEncounterEntryJson> {
    entries
        .iter()
        .map(|e| WaterEncounterEntryJson {
            level_min: e.min_level,
            level_max: e.max_level,
            species: resolve(e.species),
        })
        .collect()
}

impl JsonEncounterFile {
    pub fn to_binary(&self, symbols: &SymbolTable, family: GameFamily) -> BinaryEncounterFile {
        let resolve = |name: &str| {
            symbols
                .resolve_constant(name)
                .map(|v| v as u32)
                .unwrap_or_else(|| name.parse().unwrap_or(0))
        };

        let morning_encounters = self
            .morning
            .as_ref()
            .map(|m| collect_encounter_array(m, &resolve))
            .unwrap_or_default();

        let rock_smash_encounters = self
            .rock_smash_encounters
            .as_ref()
            .map(|r| collect_water_array(r, &resolve))
            .unwrap_or_default();

        let music_encounters = match family {
            GameFamily::HGSS => {
                if let Some(music) = self.music.as_ref() {
                    collect_species_array(music, &resolve)
                } else {
                    collect_species_array(&self.radar, &resolve)
                }
            }
            _ => Default::default(),
        };

        BinaryEncounterFile {
            walking_rate: self.land_rate,
            grass_encounters: collect_encounter_array(&self.land_encounters, &resolve),
            swarm_encounters: collect_species_array(&self.swarms, &resolve),
            day_encounters: collect_species_array(&self.day, &resolve),
            night_encounters: collect_species_array(&self.night, &resolve),
            radar_encounters: collect_species_array(&self.radar, &resolve),
            music_encounters,
            form_encounter_rates: [
                self.rate_form0,
                self.rate_form1,
                self.rate_form2,
                self.rate_form3,
                self.rate_form4,
            ],
            unown_table_id: self.unown_table,
            dual_slot_ruby: collect_species_array(&self.ruby, &resolve),
            dual_slot_sapphire: collect_species_array(&self.sapphire, &resolve),
            dual_slot_emerald: collect_species_array(&self.emerald, &resolve),
            dual_slot_firered: collect_species_array(&self.firered, &resolve),
            dual_slot_leafgreen: collect_species_array(&self.leafgreen, &resolve),
            surf_rate: self.surf_rate,
            surf_encounters: collect_water_array(&self.surf_encounters, &resolve),
            old_rod_rate: self.old_rod_rate,
            old_rod_encounters: collect_water_array(&self.old_rod_encounters, &resolve),
            good_rod_rate: self.good_rod_rate,
            good_rod_encounters: collect_water_array(&self.good_rod_encounters, &resolve),
            super_rod_rate: self.super_rod_rate,
            super_rod_encounters: collect_water_array(&self.super_rod_encounters, &resolve),
            rock_smash_rate: self.rock_smash_rate.unwrap_or(0),
            rock_smash_encounters,
            morning_encounters,
        }
    }

    pub fn from_binary(
        bin: &BinaryEncounterFile,
        symbols: &SymbolTable,
        family: GameFamily,
    ) -> Self {
        let resolve = |id: u32| {
            symbols
                .resolve_name(id as i64, "SPECIES_")
                .unwrap_or_else(|| id.to_string())
        };

        let mut res = Self {
            land_rate: bin.walking_rate,
            land_encounters: encounter_entries_to_json(&bin.grass_encounters, &resolve),
            swarms: species_to_json(&bin.swarm_encounters, &resolve),
            day: species_to_json(&bin.day_encounters, &resolve),
            night: species_to_json(&bin.night_encounters, &resolve),
            radar: species_to_json(&bin.radar_encounters, &resolve),
            rate_form0: bin.form_encounter_rates[0],
            rate_form1: bin.form_encounter_rates[1],
            rate_form2: bin.form_encounter_rates[2],
            rate_form3: bin.form_encounter_rates[3],
            rate_form4: bin.form_encounter_rates[4],
            unown_table: bin.unown_table_id,
            ruby: species_to_json(&bin.dual_slot_ruby, &resolve),
            sapphire: species_to_json(&bin.dual_slot_sapphire, &resolve),
            emerald: species_to_json(&bin.dual_slot_emerald, &resolve),
            firered: species_to_json(&bin.dual_slot_firered, &resolve),
            leafgreen: species_to_json(&bin.dual_slot_leafgreen, &resolve),
            surf_rate: bin.surf_rate,
            surf_encounters: water_entries_to_json(&bin.surf_encounters, &resolve),
            old_rod_rate: bin.old_rod_rate,
            old_rod_encounters: water_entries_to_json(&bin.old_rod_encounters, &resolve),
            good_rod_rate: bin.good_rod_rate,
            good_rod_encounters: water_entries_to_json(&bin.good_rod_encounters, &resolve),
            super_rod_rate: bin.super_rod_rate,
            super_rod_encounters: water_entries_to_json(&bin.super_rod_encounters, &resolve),
            music: None,
            rock_smash_rate: None,
            rock_smash_encounters: None,
            morning: None,
        };

        if family == GameFamily::HGSS {
            res.music = Some(species_to_json(&bin.music_encounters, &resolve));
            res.rock_smash_rate = Some(bin.rock_smash_rate);
            res.rock_smash_encounters =
                Some(water_entries_to_json(&bin.rock_smash_encounters, &resolve));
            res.morning = Some(encounter_entries_to_json(&bin.morning_encounters, &resolve));
        }

        res
    }
}
