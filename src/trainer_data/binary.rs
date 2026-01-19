use super::types::{AiFlags, PartyPokemon, TrainerData, TrainerFlags, TrainerProperties};
use crate::game::GameFamily;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

pub const TRAINER_PROPERTIES_SIZE: usize = 20;

impl TrainerProperties {
    pub fn from_binary<R: Read>(reader: &mut R) -> io::Result<Self> {
        let flags_byte = reader.read_u8()?;
        let flags = TrainerFlags::from_bits_truncate(flags_byte);
        let trainer_class = reader.read_u8()?;
        let unknown = reader.read_u8()?;
        let party_count = reader.read_u8()?;

        let mut items = [0u16; 4];
        for item in &mut items {
            *item = reader.read_u16::<LittleEndian>()?;
        }

        let ai_flags = AiFlags::from_bits_truncate(reader.read_u32::<LittleEndian>()?);
        let double_battle = reader.read_u32::<LittleEndian>()?;

        Ok(Self {
            flags,
            trainer_class,
            unknown,
            party_count,
            items,
            ai_flags,
            double_battle,
        })
    }

    pub fn to_binary<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u8(self.flags.bits())?;
        writer.write_u8(self.trainer_class)?;
        writer.write_u8(self.unknown)?;
        writer.write_u8(self.party_count)?;
        for &item in &self.items {
            writer.write_u16::<LittleEndian>(item)?;
        }
        writer.write_u32::<LittleEndian>(self.ai_flags.bits())?;
        writer.write_u32::<LittleEndian>(self.double_battle)?;
        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::with_capacity(TRAINER_PROPERTIES_SIZE));
        self.to_binary(&mut buf).unwrap();
        buf.into_inner()
    }
}

impl PartyPokemon {
    pub fn from_binary<R: Read>(
        reader: &mut R,
        flags: TrainerFlags,
        family: GameFamily,
    ) -> io::Result<Self> {
        let difficulty = reader.read_u8()?;
        let gender_ability = reader.read_u8()?;
        let level = reader.read_u16::<LittleEndian>()?;
        let species_form = reader.read_u16::<LittleEndian>()?;
        let species = species_form & 0x3FF;
        let form = ((species_form >> 10) & 0x3F) as u8;

        let held_item = if flags.contains(TrainerFlags::HAS_ITEMS) {
            Some(reader.read_u16::<LittleEndian>()?)
        } else {
            None
        };

        let moves = if flags.contains(TrainerFlags::HAS_MOVES) {
            let mut m = [0u16; 4];
            for mv in &mut m {
                *mv = reader.read_u16::<LittleEndian>()?;
            }
            Some(m)
        } else {
            None
        };

        let ball_seal = if family != GameFamily::DP {
            Some(reader.read_u16::<LittleEndian>()?)
        } else {
            None
        };

        Ok(Self {
            difficulty,
            gender_ability,
            level,
            species,
            form,
            held_item,
            moves,
            ball_seal,
        })
    }

    pub fn to_binary<W: Write>(
        &self,
        writer: &mut W,
        flags: TrainerFlags,
        family: GameFamily,
    ) -> io::Result<()> {
        writer.write_u8(self.difficulty)?;
        writer.write_u8(self.gender_ability)?;
        writer.write_u16::<LittleEndian>(self.level)?;
        let species_form = (self.species & 0x3FF) | ((self.form as u16 & 0x3F) << 10);
        writer.write_u16::<LittleEndian>(species_form)?;

        if flags.contains(TrainerFlags::HAS_ITEMS) {
            writer.write_u16::<LittleEndian>(self.held_item.unwrap_or(0))?;
        }

        if flags.contains(TrainerFlags::HAS_MOVES) {
            let moves = self.moves.unwrap_or([0; 4]);
            for mv in moves {
                writer.write_u16::<LittleEndian>(mv)?;
            }
        }

        if family != GameFamily::DP {
            writer.write_u16::<LittleEndian>(self.ball_seal.unwrap_or(0))?;
        }

        Ok(())
    }

    pub fn binary_size(flags: TrainerFlags, family: GameFamily) -> usize {
        let mut size = 6; // base: difficulty(1) + gender_ability(1) + level(2) + species_form(2)
        if flags.contains(TrainerFlags::HAS_ITEMS) {
            size += 2;
        }
        if flags.contains(TrainerFlags::HAS_MOVES) {
            size += 8;
        }
        if family != GameFamily::DP {
            size += 2;
        }
        size
    }
}

impl TrainerData {
    pub fn from_binary_parts<R1: Read, R2: Read>(
        properties_reader: &mut R1,
        party_reader: &mut R2,
        family: GameFamily,
    ) -> io::Result<Self> {
        let properties = TrainerProperties::from_binary(properties_reader)?;
        let mut party = Vec::with_capacity(properties.party_count as usize);
        for _ in 0..properties.party_count {
            party.push(PartyPokemon::from_binary(
                party_reader,
                properties.flags,
                family,
            )?);
        }
        Ok(Self { properties, party })
    }

    pub fn to_binary_parts<W1: Write, W2: Write>(
        &self,
        properties_writer: &mut W1,
        party_writer: &mut W2,
        family: GameFamily,
    ) -> io::Result<()> {
        self.properties.to_binary(properties_writer)?;
        for pokemon in &self.party {
            pokemon.to_binary(party_writer, self.properties.flags, family)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_trainer_properties_roundtrip() {
        let props = TrainerProperties {
            flags: TrainerFlags::HAS_MOVES | TrainerFlags::HAS_ITEMS,
            trainer_class: 5,
            unknown: 0,
            party_count: 3,
            items: [17, 0, 0, 0],
            ai_flags: AiFlags::BASIC | AiFlags::EVAL_ATTACK | AiFlags::EXPERT,
            double_battle: 0,
        };

        let bytes = props.to_bytes();
        assert_eq!(bytes.len(), TRAINER_PROPERTIES_SIZE);

        let mut cursor = Cursor::new(bytes);
        let parsed = TrainerProperties::from_binary(&mut cursor).unwrap();
        assert_eq!(props, parsed);
    }

    #[test]
    fn test_party_pokemon_base() {
        let flags = TrainerFlags::empty();
        let family = GameFamily::DP;

        let pokemon = PartyPokemon {
            difficulty: 8,
            gender_ability: 0x10,
            level: 25,
            species: 25,
            form: 0,
            held_item: None,
            moves: None,
            ball_seal: None,
        };

        let mut buf = Vec::new();
        pokemon.to_binary(&mut buf, flags, family).unwrap();
        assert_eq!(buf.len(), 6);

        let mut cursor = Cursor::new(buf);
        let parsed = PartyPokemon::from_binary(&mut cursor, flags, family).unwrap();
        assert_eq!(pokemon.species, parsed.species);
        assert_eq!(pokemon.level, parsed.level);
    }

    #[test]
    fn test_party_pokemon_with_moves_items_platinum() {
        let flags = TrainerFlags::HAS_MOVES | TrainerFlags::HAS_ITEMS;
        let family = GameFamily::Platinum;

        let pokemon = PartyPokemon {
            difficulty: 31,
            gender_ability: 0x21,
            level: 50,
            species: 150,
            form: 1,
            held_item: Some(234),
            moves: Some([10, 20, 30, 40]),
            ball_seal: Some(0),
        };

        let mut buf = Vec::new();
        pokemon.to_binary(&mut buf, flags, family).unwrap();
        let expected_size = PartyPokemon::binary_size(flags, family);
        assert_eq!(buf.len(), expected_size);

        let mut cursor = Cursor::new(buf);
        let parsed = PartyPokemon::from_binary(&mut cursor, flags, family).unwrap();
        assert_eq!(pokemon.species, parsed.species);
        assert_eq!(pokemon.form, parsed.form);
        assert_eq!(pokemon.held_item, parsed.held_item);
        assert_eq!(pokemon.moves, parsed.moves);
    }

    #[test]
    fn test_full_trainer_roundtrip() {
        let trainer = TrainerData {
            properties: TrainerProperties {
                flags: TrainerFlags::HAS_MOVES,
                trainer_class: 10,
                unknown: 0,
                party_count: 2,
                items: [0; 4],
                ai_flags: AiFlags::BASIC
                    | AiFlags::EVAL_ATTACK
                    | AiFlags::EXPERT
                    | AiFlags::SETUP_FIRST_TURN,
                double_battle: 0,
            },
            party: vec![
                PartyPokemon {
                    difficulty: 15,
                    gender_ability: 0,
                    level: 30,
                    species: 6,
                    form: 0,
                    held_item: None,
                    moves: Some([53, 126, 14, 0]),
                    ball_seal: Some(0),
                },
                PartyPokemon {
                    difficulty: 15,
                    gender_ability: 0,
                    level: 32,
                    species: 9,
                    form: 0,
                    held_item: None,
                    moves: Some([56, 55, 110, 0]),
                    ball_seal: Some(0),
                },
            ],
        };

        let mut props_buf = Vec::new();
        let mut party_buf = Vec::new();
        trainer
            .to_binary_parts(&mut props_buf, &mut party_buf, GameFamily::Platinum)
            .unwrap();

        let mut props_cursor = Cursor::new(props_buf);
        let mut party_cursor = Cursor::new(party_buf);
        let parsed = TrainerData::from_binary_parts(
            &mut props_cursor,
            &mut party_cursor,
            GameFamily::Platinum,
        )
        .unwrap();

        assert_eq!(trainer.properties, parsed.properties);
        assert_eq!(trainer.party.len(), parsed.party.len());
        for (orig, parsed) in trainer.party.iter().zip(parsed.party.iter()) {
            assert_eq!(orig.species, parsed.species);
            assert_eq!(orig.level, parsed.level);
            assert_eq!(orig.moves, parsed.moves);
        }
    }
}
