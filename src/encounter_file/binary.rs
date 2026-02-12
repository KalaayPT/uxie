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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EncounterEntry {
    pub level: u8,
    pub species: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WaterEncounterEntry {
    pub min_level: u8,
    pub max_level: u8,
    pub species: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryEncounterFile {
    pub walking_rate: u32,
    pub grass_encounters: [EncounterEntry; DPPT_GRASS_COUNT],
    pub swarm_encounters: [u32; HGSS_SWARM_COUNT],
    pub day_encounters: [u32; HGSS_WALKING_COUNT],
    pub night_encounters: [u32; HGSS_WALKING_COUNT],
    pub radar_encounters: [u32; DPPT_RADAR_COUNT],
    pub music_encounters: [u32; HGSS_MUSIC_COUNT],
    pub form_encounter_rates: [u32; DPPT_FORM_RATE_COUNT],
    pub unown_table_id: u32,
    pub dual_slot_ruby: [u32; DPPT_DUAL_SLOT_COUNT],
    pub dual_slot_sapphire: [u32; DPPT_DUAL_SLOT_COUNT],
    pub dual_slot_emerald: [u32; DPPT_DUAL_SLOT_COUNT],
    pub dual_slot_firered: [u32; DPPT_DUAL_SLOT_COUNT],
    pub dual_slot_leafgreen: [u32; DPPT_DUAL_SLOT_COUNT],
    pub surf_rate: u32,
    pub surf_encounters: [WaterEncounterEntry; DPPT_WATER_SLOT_COUNT],
    pub old_rod_rate: u32,
    pub old_rod_encounters: [WaterEncounterEntry; DPPT_WATER_SLOT_COUNT],
    pub good_rod_rate: u32,
    pub good_rod_encounters: [WaterEncounterEntry; DPPT_WATER_SLOT_COUNT],
    pub super_rod_rate: u32,
    pub super_rod_encounters: [WaterEncounterEntry; DPPT_WATER_SLOT_COUNT],
    pub rock_smash_rate: u32,
    pub rock_smash_encounters: [WaterEncounterEntry; HGSS_ROCK_SMASH_COUNT],
    pub morning_encounters: [EncounterEntry; HGSS_WALKING_COUNT],
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

        let mut grass_encounters = [EncounterEntry {
            level: 0,
            species: 0,
        }; DPPT_GRASS_COUNT];
        for slot in &mut grass_encounters {
            let level = reader.read_u32::<LittleEndian>()? as u8;
            let species = reader.read_u32::<LittleEndian>()?;
            *slot = EncounterEntry { level, species };
        }

        let mut swarm_pair = [0u32; DPPT_SWARM_COUNT];
        for slot in &mut swarm_pair {
            *slot = reader.read_u32::<LittleEndian>()?;
        }

        let mut day_pair = [0u32; DPPT_DAY_NIGHT_COUNT];
        for slot in &mut day_pair {
            *slot = reader.read_u32::<LittleEndian>()?;
        }

        let mut night_pair = [0u32; DPPT_DAY_NIGHT_COUNT];
        for slot in &mut night_pair {
            *slot = reader.read_u32::<LittleEndian>()?;
        }

        let mut radar_encounters = [0u32; DPPT_RADAR_COUNT];
        for slot in &mut radar_encounters {
            *slot = reader.read_u32::<LittleEndian>()?;
        }

        let mut form_encounter_rates = [0u32; DPPT_FORM_RATE_COUNT];
        for slot in &mut form_encounter_rates {
            *slot = reader.read_u32::<LittleEndian>()?;
        }

        let unown_table_id = reader.read_u32::<LittleEndian>()?;

        let mut dual_slot_ruby = [0u32; DPPT_DUAL_SLOT_COUNT];
        for slot in &mut dual_slot_ruby {
            *slot = reader.read_u32::<LittleEndian>()?;
        }

        let mut dual_slot_sapphire = [0u32; DPPT_DUAL_SLOT_COUNT];
        for slot in &mut dual_slot_sapphire {
            *slot = reader.read_u32::<LittleEndian>()?;
        }

        let mut dual_slot_emerald = [0u32; DPPT_DUAL_SLOT_COUNT];
        for slot in &mut dual_slot_emerald {
            *slot = reader.read_u32::<LittleEndian>()?;
        }

        let mut dual_slot_firered = [0u32; DPPT_DUAL_SLOT_COUNT];
        for slot in &mut dual_slot_firered {
            *slot = reader.read_u32::<LittleEndian>()?;
        }

        let mut dual_slot_leafgreen = [0u32; DPPT_DUAL_SLOT_COUNT];
        for slot in &mut dual_slot_leafgreen {
            *slot = reader.read_u32::<LittleEndian>()?;
        }

        let surf_rate = reader.read_u32::<LittleEndian>()?;
        let mut surf_encounters = [WaterEncounterEntry {
            min_level: 0,
            max_level: 0,
            species: 0,
        }; DPPT_WATER_SLOT_COUNT];
        for slot in &mut surf_encounters {
            let max_level = reader.read_u8()?;
            let min_level = reader.read_u8()?;
            reader.seek(SeekFrom::Current(2))?;
            let species = reader.read_u32::<LittleEndian>()?;
            *slot = WaterEncounterEntry {
                min_level,
                max_level,
                species,
            };
        }

        reader.seek(SeekFrom::Start(0x124))?;

        let old_rod_rate = reader.read_u32::<LittleEndian>()?;
        let mut old_rod_encounters = [WaterEncounterEntry {
            min_level: 0,
            max_level: 0,
            species: 0,
        }; DPPT_WATER_SLOT_COUNT];
        for slot in &mut old_rod_encounters {
            let max_level = reader.read_u8()?;
            let min_level = reader.read_u8()?;
            reader.seek(SeekFrom::Current(2))?;
            let species = reader.read_u32::<LittleEndian>()?;
            *slot = WaterEncounterEntry {
                min_level,
                max_level,
                species,
            };
        }

        let good_rod_rate = reader.read_u32::<LittleEndian>()?;
        let mut good_rod_encounters = [WaterEncounterEntry {
            min_level: 0,
            max_level: 0,
            species: 0,
        }; DPPT_WATER_SLOT_COUNT];
        for slot in &mut good_rod_encounters {
            let max_level = reader.read_u8()?;
            let min_level = reader.read_u8()?;
            reader.seek(SeekFrom::Current(2))?;
            let species = reader.read_u32::<LittleEndian>()?;
            *slot = WaterEncounterEntry {
                min_level,
                max_level,
                species,
            };
        }

        let super_rod_rate = reader.read_u32::<LittleEndian>()?;
        let mut super_rod_encounters = [WaterEncounterEntry {
            min_level: 0,
            max_level: 0,
            species: 0,
        }; DPPT_WATER_SLOT_COUNT];
        for slot in &mut super_rod_encounters {
            let max_level = reader.read_u8()?;
            let min_level = reader.read_u8()?;
            reader.seek(SeekFrom::Current(2))?;
            let species = reader.read_u32::<LittleEndian>()?;
            *slot = WaterEncounterEntry {
                min_level,
                max_level,
                species,
            };
        }

        Ok(Self {
            walking_rate,
            grass_encounters,
            swarm_encounters: [swarm_pair[0], swarm_pair[1], 0, 0],
            day_encounters: [day_pair[0], day_pair[1], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            night_encounters: [night_pair[0], night_pair[1], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            radar_encounters,
            music_encounters: [0; HGSS_MUSIC_COUNT],
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
            rock_smash_encounters: [WaterEncounterEntry {
                min_level: 0,
                max_level: 0,
                species: 0,
            }; HGSS_ROCK_SMASH_COUNT],
            morning_encounters: [EncounterEntry {
                level: 0,
                species: 0,
            }; HGSS_WALKING_COUNT],
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

        let mut walking_levels = [0u8; HGSS_WALKING_COUNT];
        reader.read_exact(&mut walking_levels)?;

        let mut morning_encounters = [EncounterEntry {
            level: 0,
            species: 0,
        }; HGSS_WALKING_COUNT];
        for (index, slot) in morning_encounters.iter_mut().enumerate() {
            let species = reader.read_u16::<LittleEndian>()? as u32;
            *slot = EncounterEntry {
                level: walking_levels[index],
                species,
            };
        }

        let mut day_encounters = [0u32; HGSS_WALKING_COUNT];
        for slot in &mut day_encounters {
            *slot = reader.read_u16::<LittleEndian>()? as u32;
        }

        let mut night_encounters = [0u32; HGSS_WALKING_COUNT];
        for slot in &mut night_encounters {
            *slot = reader.read_u16::<LittleEndian>()? as u32;
        }

        let mut swarm_encounters = [0u32; HGSS_SWARM_COUNT];
        for slot in &mut swarm_encounters {
            *slot = reader.read_u16::<LittleEndian>()? as u32;
        }

        let mut music_encounters = [0u32; HGSS_MUSIC_COUNT];
        for slot in &mut music_encounters {
            *slot = reader.read_u16::<LittleEndian>()? as u32;
        }

        let mut surf_encounters = [WaterEncounterEntry {
            min_level: 0,
            max_level: 0,
            species: 0,
        }; HGSS_SURF_COUNT];
        for slot in &mut surf_encounters {
            let min_level = reader.read_u8()?;
            let max_level = reader.read_u8()?;
            let species = reader.read_u16::<LittleEndian>()? as u32;
            *slot = WaterEncounterEntry {
                min_level,
                max_level,
                species,
            };
        }

        let mut rock_smash_encounters = [WaterEncounterEntry {
            min_level: 0,
            max_level: 0,
            species: 0,
        }; HGSS_ROCK_SMASH_COUNT];
        for slot in &mut rock_smash_encounters {
            let min_level = reader.read_u8()?;
            let max_level = reader.read_u8()?;
            let species = reader.read_u16::<LittleEndian>()? as u32;
            *slot = WaterEncounterEntry {
                min_level,
                max_level,
                species,
            };
        }

        let mut old_rod_encounters = [WaterEncounterEntry {
            min_level: 0,
            max_level: 0,
            species: 0,
        }; HGSS_OLD_ROD_COUNT];
        for slot in &mut old_rod_encounters {
            let min_level = reader.read_u8()?;
            let max_level = reader.read_u8()?;
            let species = reader.read_u16::<LittleEndian>()? as u32;
            *slot = WaterEncounterEntry {
                min_level,
                max_level,
                species,
            };
        }

        let mut good_rod_encounters = [WaterEncounterEntry {
            min_level: 0,
            max_level: 0,
            species: 0,
        }; HGSS_GOOD_ROD_COUNT];
        for slot in &mut good_rod_encounters {
            let min_level = reader.read_u8()?;
            let max_level = reader.read_u8()?;
            let species = reader.read_u16::<LittleEndian>()? as u32;
            *slot = WaterEncounterEntry {
                min_level,
                max_level,
                species,
            };
        }

        let mut super_rod_encounters = [WaterEncounterEntry {
            min_level: 0,
            max_level: 0,
            species: 0,
        }; HGSS_SUPER_ROD_COUNT];
        for slot in &mut super_rod_encounters {
            let min_level = reader.read_u8()?;
            let max_level = reader.read_u8()?;
            let species = reader.read_u16::<LittleEndian>()? as u32;
            *slot = WaterEncounterEntry {
                min_level,
                max_level,
                species,
            };
        }

        Ok(Self {
            walking_rate,
            grass_encounters: morning_encounters,
            swarm_encounters,
            day_encounters,
            night_encounters,
            radar_encounters: [0; DPPT_RADAR_COUNT],
            music_encounters,
            form_encounter_rates: [0; DPPT_FORM_RATE_COUNT],
            unown_table_id: 0,
            dual_slot_ruby: [0; DPPT_DUAL_SLOT_COUNT],
            dual_slot_sapphire: [0; DPPT_DUAL_SLOT_COUNT],
            dual_slot_emerald: [0; DPPT_DUAL_SLOT_COUNT],
            dual_slot_firered: [0; DPPT_DUAL_SLOT_COUNT],
            dual_slot_leafgreen: [0; DPPT_DUAL_SLOT_COUNT],
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
        for (idx, entry) in self.grass_encounters.iter().enumerate() {
            ensure_level_u8("grass_encounters", idx, entry.level)?;
            ensure_species_u32("grass_encounters", idx, entry.species)?;
        }

        for (idx, species) in self.swarm_encounters[..DPPT_SWARM_COUNT].iter().enumerate() {
            ensure_species_u32("swarm_encounters", idx, *species)?;
        }
        for (idx, species) in self.day_encounters[..DPPT_DAY_NIGHT_COUNT]
            .iter()
            .enumerate()
        {
            ensure_species_u32("day_encounters", idx, *species)?;
        }
        for (idx, species) in self.night_encounters[..DPPT_DAY_NIGHT_COUNT]
            .iter()
            .enumerate()
        {
            ensure_species_u32("night_encounters", idx, *species)?;
        }

        validate_water_species_u32("surf_encounters", &self.surf_encounters)?;
        validate_water_species_u32("old_rod_encounters", &self.old_rod_encounters)?;
        validate_water_species_u32("good_rod_encounters", &self.good_rod_encounters)?;
        validate_water_species_u32("super_rod_encounters", &self.super_rod_encounters)?;
        Ok(())
    }

    fn validate_hgss_shape(&self) -> io::Result<()> {
        ensure_u8_range("walking_rate", self.walking_rate)?;
        ensure_u8_range("surf_rate", self.surf_rate)?;
        ensure_u8_range("rock_smash_rate", self.rock_smash_rate)?;
        ensure_u8_range("old_rod_rate", self.old_rod_rate)?;
        ensure_u8_range("good_rod_rate", self.good_rod_rate)?;
        ensure_u8_range("super_rod_rate", self.super_rod_rate)?;

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
        for (idx, species) in self.music_encounters.iter().enumerate() {
            ensure_species_u16("music_encounters", idx, *species)?;
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
        for &s in &self.swarm_encounters[..DPPT_SWARM_COUNT] {
            writer.write_u32::<LittleEndian>(s)?;
        }
        for &s in &self.day_encounters[..DPPT_DAY_NIGHT_COUNT] {
            writer.write_u32::<LittleEndian>(s)?;
        }
        for &s in &self.night_encounters[..DPPT_DAY_NIGHT_COUNT] {
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
        for &s in &self.music_encounters {
            writer.write_u16::<LittleEndian>(s as u16)?;
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

fn ensure_level_u8(field: &str, index: usize, level: u8) -> io::Result<()> {
    if level > u8::MAX {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Encounter field '{}' index {} level {} exceeds u8 max {}",
                field,
                index,
                level,
                u8::MAX
            ),
        ));
    }
    Ok(())
}

fn ensure_species_u32(_field: &str, _index: usize, _species: u32) -> io::Result<()> {
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

fn validate_water_species_u32(_field: &str, _values: &[WaterEncounterEntry]) -> io::Result<()> {
    Ok(())
}

fn validate_water_species_u16(field: &str, values: &[WaterEncounterEntry]) -> io::Result<()> {
    for (idx, entry) in values.iter().enumerate() {
        ensure_species_u16(field, idx, entry.species)?;
    }
    Ok(())
}
