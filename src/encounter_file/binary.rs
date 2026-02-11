use crate::game::GameFamily;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Seek, SeekFrom, Write};

const DPPT_GRASS_COUNT: usize = 12;
const DPPT_SWARM_COUNT: usize = 2;
const DPPT_DAY_NIGHT_COUNT: usize = 2;
const DPPT_RADAR_COUNT: usize = 4;
const DPPT_FORM_RATE_COUNT: usize = 5;
const DPPT_DUAL_SLOT_COUNT: usize = 2;
const DPPT_WATER_SLOT_COUNT: usize = 5;

const HGSS_WALKING_COUNT: usize = 12;
const HGSS_SWARM_COUNT: usize = 4;
const HGSS_MUSIC_COUNT: usize = 4;
const HGSS_SURF_COUNT: usize = 5;
const HGSS_ROCK_SMASH_COUNT: usize = 2;
const HGSS_OLD_ROD_COUNT: usize = 5;
const HGSS_GOOD_ROD_COUNT: usize = 5;
const HGSS_SUPER_ROD_COUNT: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterEntry {
    pub level: u8,
    pub species: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterEncounterEntry {
    pub min_level: u8,
    pub max_level: u8,
    pub species: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryEncounterFile {
    pub walking_rate: u32,
    pub grass_encounters: Vec<EncounterEntry>,
    pub swarm_encounters: Vec<u32>,
    pub day_encounters: Vec<u32>,
    pub night_encounters: Vec<u32>,
    pub radar_encounters: Vec<u32>,
    pub form_encounter_rates: Vec<u32>,
    pub unown_table_id: u32,
    pub dual_slot_ruby: Vec<u32>,
    pub dual_slot_sapphire: Vec<u32>,
    pub dual_slot_emerald: Vec<u32>,
    pub dual_slot_firered: Vec<u32>,
    pub dual_slot_leafgreen: Vec<u32>,
    pub surf_rate: u32,
    pub surf_encounters: Vec<WaterEncounterEntry>,
    pub old_rod_rate: u32,
    pub old_rod_encounters: Vec<WaterEncounterEntry>,
    pub good_rod_rate: u32,
    pub good_rod_encounters: Vec<WaterEncounterEntry>,
    pub super_rod_rate: u32,
    pub super_rod_encounters: Vec<WaterEncounterEntry>,
    pub rock_smash_rate: u32,
    pub rock_smash_encounters: Vec<WaterEncounterEntry>,
    pub morning_encounters: Vec<EncounterEntry>,
}

impl BinaryEncounterFile {
    pub fn from_binary<R: Read + Seek>(reader: &mut R, family: GameFamily) -> io::Result<Self> {
        match family {
            GameFamily::DP | GameFamily::Platinum => Self::from_binary_dppt(reader),
            GameFamily::HGSS => Self::from_binary_hgss(reader),
        }
    }

    fn from_binary_dppt<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let walking_rate = reader.read_u32::<LittleEndian>()?;
        let mut grass_encounters = Vec::with_capacity(12);
        for _ in 0..12 {
            let level = reader.read_u32::<LittleEndian>()? as u8;
            let species = reader.read_u32::<LittleEndian>()?;
            grass_encounters.push(EncounterEntry { level, species });
        }

        let mut swarm_encounters = Vec::with_capacity(2);
        for _ in 0..2 {
            swarm_encounters.push(reader.read_u32::<LittleEndian>()?);
        }

        let mut day_encounters = Vec::with_capacity(2);
        for _ in 0..2 {
            day_encounters.push(reader.read_u32::<LittleEndian>()?);
        }

        let mut night_encounters = Vec::with_capacity(2);
        for _ in 0..2 {
            night_encounters.push(reader.read_u32::<LittleEndian>()?);
        }

        let mut radar_encounters = Vec::with_capacity(4);
        for _ in 0..4 {
            radar_encounters.push(reader.read_u32::<LittleEndian>()?);
        }

        let mut form_encounter_rates = Vec::with_capacity(5);
        for _ in 0..5 {
            form_encounter_rates.push(reader.read_u32::<LittleEndian>()?);
        }

        let unown_table_id = reader.read_u32::<LittleEndian>()?;

        let mut dual_slot_ruby = Vec::with_capacity(2);
        for _ in 0..2 {
            dual_slot_ruby.push(reader.read_u32::<LittleEndian>()?);
        }

        let mut dual_slot_sapphire = Vec::with_capacity(2);
        for _ in 0..2 {
            dual_slot_sapphire.push(reader.read_u32::<LittleEndian>()?);
        }

        let mut dual_slot_emerald = Vec::with_capacity(2);
        for _ in 0..2 {
            dual_slot_emerald.push(reader.read_u32::<LittleEndian>()?);
        }

        let mut dual_slot_firered = Vec::with_capacity(2);
        for _ in 0..2 {
            dual_slot_firered.push(reader.read_u32::<LittleEndian>()?);
        }

        let mut dual_slot_leafgreen = Vec::with_capacity(2);
        for _ in 0..2 {
            dual_slot_leafgreen.push(reader.read_u32::<LittleEndian>()?);
        }

        let surf_rate = reader.read_u32::<LittleEndian>()?;
        let mut surf_encounters = Vec::with_capacity(5);
        for _ in 0..5 {
            let max_level = reader.read_u8()?;
            let min_level = reader.read_u8()?;
            reader.seek(SeekFrom::Current(2))?;
            let species = reader.read_u32::<LittleEndian>()?;
            surf_encounters.push(WaterEncounterEntry {
                min_level,
                max_level,
                species,
            });
        }

        reader.seek(SeekFrom::Start(0x124))?;

        let old_rod_rate = reader.read_u32::<LittleEndian>()?;
        let mut old_rod_encounters = Vec::with_capacity(5);
        for _ in 0..5 {
            let max_level = reader.read_u8()?;
            let min_level = reader.read_u8()?;
            reader.seek(SeekFrom::Current(2))?;
            let species = reader.read_u32::<LittleEndian>()?;
            old_rod_encounters.push(WaterEncounterEntry {
                min_level,
                max_level,
                species,
            });
        }

        let good_rod_rate = reader.read_u32::<LittleEndian>()?;
        let mut good_rod_encounters = Vec::with_capacity(5);
        for _ in 0..5 {
            let max_level = reader.read_u8()?;
            let min_level = reader.read_u8()?;
            reader.seek(SeekFrom::Current(2))?;
            let species = reader.read_u32::<LittleEndian>()?;
            good_rod_encounters.push(WaterEncounterEntry {
                min_level,
                max_level,
                species,
            });
        }

        let super_rod_rate = reader.read_u32::<LittleEndian>()?;
        let mut super_rod_encounters = Vec::with_capacity(5);
        for _ in 0..5 {
            let max_level = reader.read_u8()?;
            let min_level = reader.read_u8()?;
            reader.seek(SeekFrom::Current(2))?;
            let species = reader.read_u32::<LittleEndian>()?;
            super_rod_encounters.push(WaterEncounterEntry {
                min_level,
                max_level,
                species,
            });
        }

        Ok(Self {
            walking_rate,
            grass_encounters,
            swarm_encounters,
            day_encounters,
            night_encounters,
            radar_encounters,
            form_encounter_rates,
            unown_table_id,
            dual_slot_ruby,
            dual_slot_sapphire,
            dual_slot_emerald,
            dual_slot_firered,
            dual_slot_leafgreen,
            surf_rate,
            surf_encounters,
            old_rod_rate,
            old_rod_encounters,
            good_rod_rate,
            good_rod_encounters,
            super_rod_rate,
            super_rod_encounters,
            rock_smash_rate: 0,
            rock_smash_encounters: Vec::new(),
            morning_encounters: Vec::new(),
        })
    }

    fn from_binary_hgss<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let walking_rate = reader.read_u8()? as u32;
        let surf_rate = reader.read_u8()? as u32;
        let rock_smash_rate = reader.read_u8()? as u32;
        let old_rod_rate = reader.read_u8()? as u32;
        let good_rod_rate = reader.read_u8()? as u32;
        let super_rod_rate = reader.read_u8()? as u32;
        reader.seek(SeekFrom::Current(2))?;

        let mut walking_levels = [0u8; 12];
        reader.read_exact(&mut walking_levels)?;

        let mut morning_encounters = Vec::with_capacity(12);
        for i in 0..12 {
            let species = reader.read_u16::<LittleEndian>()? as u32;
            morning_encounters.push(EncounterEntry {
                level: walking_levels[i],
                species,
            });
        }

        let mut day_encounters = Vec::with_capacity(12);
        for _ in 0..12 {
            let species = reader.read_u16::<LittleEndian>()? as u32;
            day_encounters.push(species);
        }

        let mut night_encounters = Vec::with_capacity(12);
        for _ in 0..12 {
            let species = reader.read_u16::<LittleEndian>()? as u32;
            night_encounters.push(species);
        }

        let mut swarm_encounters = Vec::with_capacity(4);
        for _ in 0..4 {
            swarm_encounters.push(reader.read_u16::<LittleEndian>()? as u32);
        }

        let mut hoenn_music = Vec::with_capacity(2);
        for _ in 0..2 {
            hoenn_music.push(reader.read_u16::<LittleEndian>()? as u32);
        }

        let mut sinnoh_music = Vec::with_capacity(2);
        for _ in 0..2 {
            sinnoh_music.push(reader.read_u16::<LittleEndian>()? as u32);
        }

        let mut surf_encounters = Vec::with_capacity(5);
        for _ in 0..5 {
            let min_level = reader.read_u8()?;
            let max_level = reader.read_u8()?;
            let species = reader.read_u16::<LittleEndian>()? as u32;
            surf_encounters.push(WaterEncounterEntry {
                min_level,
                max_level,
                species,
            });
        }

        let mut rock_smash_encounters = Vec::with_capacity(2);
        for _ in 0..2 {
            let min_level = reader.read_u8()?;
            let max_level = reader.read_u8()?;
            let species = reader.read_u16::<LittleEndian>()? as u32;
            rock_smash_encounters.push(WaterEncounterEntry {
                min_level,
                max_level,
                species,
            });
        }

        let mut old_rod_encounters = Vec::with_capacity(5);
        for _ in 0..5 {
            let min_level = reader.read_u8()?;
            let max_level = reader.read_u8()?;
            let species = reader.read_u16::<LittleEndian>()? as u32;
            old_rod_encounters.push(WaterEncounterEntry {
                min_level,
                max_level,
                species,
            });
        }

        let mut good_rod_encounters = Vec::with_capacity(5);
        for _ in 0..5 {
            let min_level = reader.read_u8()?;
            let max_level = reader.read_u8()?;
            let species = reader.read_u16::<LittleEndian>()? as u32;
            good_rod_encounters.push(WaterEncounterEntry {
                min_level,
                max_level,
                species,
            });
        }

        let mut super_rod_encounters = Vec::with_capacity(5);
        for _ in 0..5 {
            let min_level = reader.read_u8()?;
            let max_level = reader.read_u8()?;
            let species = reader.read_u16::<LittleEndian>()? as u32;
            super_rod_encounters.push(WaterEncounterEntry {
                min_level,
                max_level,
                species,
            });
        }

        let mut special = hoenn_music;
        special.extend(sinnoh_music);

        Ok(Self {
            walking_rate,
            grass_encounters: morning_encounters.clone(),
            swarm_encounters,
            day_encounters,
            night_encounters,
            radar_encounters: special,
            form_encounter_rates: Vec::new(),
            unown_table_id: 0,
            dual_slot_ruby: Vec::new(),
            dual_slot_sapphire: Vec::new(),
            dual_slot_emerald: Vec::new(),
            dual_slot_firered: Vec::new(),
            dual_slot_leafgreen: Vec::new(),
            surf_rate,
            surf_encounters,
            old_rod_rate,
            old_rod_encounters,
            good_rod_rate,
            good_rod_encounters,
            super_rod_rate,
            super_rod_encounters,
            rock_smash_rate,
            rock_smash_encounters,
            morning_encounters,
        })
    }

    pub fn to_binary<W: Write>(&self, writer: &mut W, family: GameFamily) -> io::Result<()> {
        self.validate_for_family(family)?;
        match family {
            GameFamily::DP | GameFamily::Platinum => self.to_binary_dppt(writer),
            GameFamily::HGSS => self.to_binary_hgss(writer),
        }
    }

    fn validate_for_family(&self, family: GameFamily) -> io::Result<()> {
        match family {
            GameFamily::DP | GameFamily::Platinum => self.validate_dppt_shape(),
            GameFamily::HGSS => self.validate_hgss_shape(),
        }
    }

    fn validate_dppt_shape(&self) -> io::Result<()> {
        ensure_len(
            "grass_encounters",
            self.grass_encounters.len(),
            DPPT_GRASS_COUNT,
        )?;
        ensure_len(
            "swarm_encounters",
            self.swarm_encounters.len(),
            DPPT_SWARM_COUNT,
        )?;
        ensure_len(
            "day_encounters",
            self.day_encounters.len(),
            DPPT_DAY_NIGHT_COUNT,
        )?;
        ensure_len(
            "night_encounters",
            self.night_encounters.len(),
            DPPT_DAY_NIGHT_COUNT,
        )?;
        ensure_len(
            "radar_encounters",
            self.radar_encounters.len(),
            DPPT_RADAR_COUNT,
        )?;
        ensure_len(
            "form_encounter_rates",
            self.form_encounter_rates.len(),
            DPPT_FORM_RATE_COUNT,
        )?;
        ensure_len(
            "dual_slot_ruby",
            self.dual_slot_ruby.len(),
            DPPT_DUAL_SLOT_COUNT,
        )?;
        ensure_len(
            "dual_slot_sapphire",
            self.dual_slot_sapphire.len(),
            DPPT_DUAL_SLOT_COUNT,
        )?;
        ensure_len(
            "dual_slot_emerald",
            self.dual_slot_emerald.len(),
            DPPT_DUAL_SLOT_COUNT,
        )?;
        ensure_len(
            "dual_slot_firered",
            self.dual_slot_firered.len(),
            DPPT_DUAL_SLOT_COUNT,
        )?;
        ensure_len(
            "dual_slot_leafgreen",
            self.dual_slot_leafgreen.len(),
            DPPT_DUAL_SLOT_COUNT,
        )?;
        ensure_len(
            "surf_encounters",
            self.surf_encounters.len(),
            DPPT_WATER_SLOT_COUNT,
        )?;
        ensure_len(
            "old_rod_encounters",
            self.old_rod_encounters.len(),
            DPPT_WATER_SLOT_COUNT,
        )?;
        ensure_len(
            "good_rod_encounters",
            self.good_rod_encounters.len(),
            DPPT_WATER_SLOT_COUNT,
        )?;
        ensure_len(
            "super_rod_encounters",
            self.super_rod_encounters.len(),
            DPPT_WATER_SLOT_COUNT,
        )?;

        Ok(())
    }

    fn validate_hgss_shape(&self) -> io::Result<()> {
        ensure_u8_range("walking_rate", self.walking_rate)?;
        ensure_u8_range("surf_rate", self.surf_rate)?;
        ensure_u8_range("rock_smash_rate", self.rock_smash_rate)?;
        ensure_u8_range("old_rod_rate", self.old_rod_rate)?;
        ensure_u8_range("good_rod_rate", self.good_rod_rate)?;
        ensure_u8_range("super_rod_rate", self.super_rod_rate)?;

        ensure_len(
            "morning_encounters",
            self.morning_encounters.len(),
            HGSS_WALKING_COUNT,
        )?;
        ensure_len(
            "day_encounters",
            self.day_encounters.len(),
            HGSS_WALKING_COUNT,
        )?;
        ensure_len(
            "night_encounters",
            self.night_encounters.len(),
            HGSS_WALKING_COUNT,
        )?;
        ensure_len(
            "swarm_encounters",
            self.swarm_encounters.len(),
            HGSS_SWARM_COUNT,
        )?;
        ensure_len(
            "radar_encounters",
            self.radar_encounters.len(),
            HGSS_MUSIC_COUNT,
        )?;
        ensure_len(
            "surf_encounters",
            self.surf_encounters.len(),
            HGSS_SURF_COUNT,
        )?;
        ensure_len(
            "rock_smash_encounters",
            self.rock_smash_encounters.len(),
            HGSS_ROCK_SMASH_COUNT,
        )?;
        ensure_len(
            "old_rod_encounters",
            self.old_rod_encounters.len(),
            HGSS_OLD_ROD_COUNT,
        )?;
        ensure_len(
            "good_rod_encounters",
            self.good_rod_encounters.len(),
            HGSS_GOOD_ROD_COUNT,
        )?;
        ensure_len(
            "super_rod_encounters",
            self.super_rod_encounters.len(),
            HGSS_SUPER_ROD_COUNT,
        )?;

        for (idx, e) in self.morning_encounters.iter().enumerate() {
            ensure_species_u16("morning_encounters", idx, e.species)?;
        }
        for (idx, species) in self.day_encounters.iter().enumerate() {
            ensure_species_u16("day_encounters", idx, *species)?;
        }
        for (idx, species) in self.night_encounters.iter().enumerate() {
            ensure_species_u16("night_encounters", idx, *species)?;
        }
        for (idx, species) in self.swarm_encounters.iter().enumerate() {
            ensure_species_u16("swarm_encounters", idx, *species)?;
        }
        for (idx, species) in self.radar_encounters.iter().enumerate() {
            ensure_species_u16("radar_encounters", idx, *species)?;
        }
        validate_water_species_u16("surf_encounters", &self.surf_encounters)?;
        validate_water_species_u16("rock_smash_encounters", &self.rock_smash_encounters)?;
        validate_water_species_u16("old_rod_encounters", &self.old_rod_encounters)?;
        validate_water_species_u16("good_rod_encounters", &self.good_rod_encounters)?;
        validate_water_species_u16("super_rod_encounters", &self.super_rod_encounters)?;

        Ok(())
    }

    fn to_binary_dppt<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u32::<LittleEndian>(self.walking_rate)?;
        for e in &self.grass_encounters {
            writer.write_u32::<LittleEndian>(e.level as u32)?;
            writer.write_u32::<LittleEndian>(e.species)?;
        }
        for &s in &self.swarm_encounters {
            writer.write_u32::<LittleEndian>(s)?;
        }
        for &s in &self.day_encounters {
            writer.write_u32::<LittleEndian>(s)?;
        }
        for &s in &self.night_encounters {
            writer.write_u32::<LittleEndian>(s)?;
        }
        for &s in &self.radar_encounters {
            writer.write_u32::<LittleEndian>(s)?;
        }
        for &r in &self.form_encounter_rates {
            writer.write_u32::<LittleEndian>(r)?;
        }
        writer.write_u32::<LittleEndian>(self.unown_table_id)?;
        for &s in &self.dual_slot_ruby {
            writer.write_u32::<LittleEndian>(s)?;
        }
        for &s in &self.dual_slot_sapphire {
            writer.write_u32::<LittleEndian>(s)?;
        }
        for &s in &self.dual_slot_emerald {
            writer.write_u32::<LittleEndian>(s)?;
        }
        for &s in &self.dual_slot_firered {
            writer.write_u32::<LittleEndian>(s)?;
        }
        for &s in &self.dual_slot_leafgreen {
            writer.write_u32::<LittleEndian>(s)?;
        }
        writer.write_u32::<LittleEndian>(self.surf_rate)?;
        for e in &self.surf_encounters {
            writer.write_u8(e.max_level)?;
            writer.write_u8(e.min_level)?;
            writer.write_all(&[0, 0])?;
            writer.write_u32::<LittleEndian>(e.species)?;
        }

        // Handle gap to 0x124
        // Current pos is 4 + 96 + 8 + 8 + 8 + 16 + 20 + 4 + 8 + 8 + 8 + 8 + 8 + 4 + 40 = 248
        // Need to pad 44 bytes
        writer.write_all(&[0u8; 44])?;

        writer.write_u32::<LittleEndian>(self.old_rod_rate)?;
        for e in &self.old_rod_encounters {
            writer.write_u8(e.max_level)?;
            writer.write_u8(e.min_level)?;
            writer.write_all(&[0, 0])?;
            writer.write_u32::<LittleEndian>(e.species)?;
        }
        writer.write_u32::<LittleEndian>(self.good_rod_rate)?;
        for e in &self.good_rod_encounters {
            writer.write_u8(e.max_level)?;
            writer.write_u8(e.min_level)?;
            writer.write_all(&[0, 0])?;
            writer.write_u32::<LittleEndian>(e.species)?;
        }
        writer.write_u32::<LittleEndian>(self.super_rod_rate)?;
        for e in &self.super_rod_encounters {
            writer.write_u8(e.max_level)?;
            writer.write_u8(e.min_level)?;
            writer.write_all(&[0, 0])?;
            writer.write_u32::<LittleEndian>(e.species)?;
        }

        Ok(())
    }

    fn to_binary_hgss<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u8(self.walking_rate as u8)?;
        writer.write_u8(self.surf_rate as u8)?;
        writer.write_u8(self.rock_smash_rate as u8)?;
        writer.write_u8(self.old_rod_rate as u8)?;
        writer.write_u8(self.good_rod_rate as u8)?;
        writer.write_u8(self.super_rod_rate as u8)?;
        writer.write_all(&[0, 0])?;

        for e in &self.morning_encounters {
            writer.write_u8(e.level)?;
        }
        for e in &self.morning_encounters {
            writer.write_u16::<LittleEndian>(e.species as u16)?;
        }
        for &s in &self.day_encounters {
            writer.write_u16::<LittleEndian>(s as u16)?;
        }
        for &s in &self.night_encounters {
            writer.write_u16::<LittleEndian>(s as u16)?;
        }
        for &s in &self.swarm_encounters {
            writer.write_u16::<LittleEndian>(s as u16)?;
        }

        // hoenn/sinnoh music
        for i in 0..4 {
            if let Some(&s) = self.radar_encounters.get(i) {
                writer.write_u16::<LittleEndian>(s as u16)?;
            } else {
                writer.write_u16::<LittleEndian>(0)?;
            }
        }

        for e in &self.surf_encounters {
            writer.write_u8(e.min_level)?;
            writer.write_u8(e.max_level)?;
            writer.write_u16::<LittleEndian>(e.species as u16)?;
        }
        for e in &self.rock_smash_encounters {
            writer.write_u8(e.min_level)?;
            writer.write_u8(e.max_level)?;
            writer.write_u16::<LittleEndian>(e.species as u16)?;
        }
        for e in &self.old_rod_encounters {
            writer.write_u8(e.min_level)?;
            writer.write_u8(e.max_level)?;
            writer.write_u16::<LittleEndian>(e.species as u16)?;
        }
        for e in &self.good_rod_encounters {
            writer.write_u8(e.min_level)?;
            writer.write_u8(e.max_level)?;
            writer.write_u16::<LittleEndian>(e.species as u16)?;
        }
        for e in &self.super_rod_encounters {
            writer.write_u8(e.min_level)?;
            writer.write_u8(e.max_level)?;
            writer.write_u16::<LittleEndian>(e.species as u16)?;
        }
        for &s in &self.swarm_encounters {
            writer.write_u16::<LittleEndian>(s as u16)?;
        }

        Ok(())
    }
}

fn ensure_len(field: &str, actual: usize, expected: usize) -> io::Result<()> {
    if actual != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Encounter field '{}' has {} entries, expected {}",
                field, actual, expected
            ),
        ));
    }
    Ok(())
}

fn ensure_u8_range(field: &str, value: u32) -> io::Result<()> {
    if value > u8::MAX as u32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Encounter field '{}' value {} exceeds u8 max {}",
                field,
                value,
                u8::MAX
            ),
        ));
    }
    Ok(())
}

fn ensure_species_u16(field: &str, index: usize, species: u32) -> io::Result<()> {
    if species > u16::MAX as u32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Encounter field '{}' index {} species {} exceeds u16 max {}",
                field,
                index,
                species,
                u16::MAX
            ),
        ));
    }
    Ok(())
}

fn validate_water_species_u16(field: &str, values: &[WaterEncounterEntry]) -> io::Result<()> {
    for (idx, entry) in values.iter().enumerate() {
        ensure_species_u16(field, idx, entry.species)?;
    }
    Ok(())
}
