use super::types::{AiFlags, PartyPokemon, TrainerData, TrainerFlags, TrainerProperties};
use crate::game::GameFamily;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

pub const TRAINER_PROPERTIES_SIZE: usize = 20;

impl TrainerProperties {
    pub fn from_binary<R: Read>(reader: &mut R) -> io::Result<Self> {
        let flags_byte = reader.read_u8()?;
        let flags = TrainerFlags::from_bits(flags_byte).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid trainer flags bits '{}'", flags_byte),
            )
        })?;
        let trainer_class = reader.read_u8()?;
        let unknown = reader.read_u8()?;
        let party_count = reader.read_u8()?;

        let mut items = [0u16; 4];
        for item in &mut items {
            *item = reader.read_u16::<LittleEndian>()?;
        }

        let ai_flags_bits = reader.read_u32::<LittleEndian>()?;
        let ai_flags = AiFlags::from_bits(ai_flags_bits).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid trainer AI flags bits '{:#010X}'", ai_flags_bits),
            )
        })?;
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
        let (difficulty, gender_ability_override) = match family {
            GameFamily::DP | GameFamily::Platinum => (reader.read_u16::<LittleEndian>()?, 0),
            GameFamily::HGSS => {
                let difficulty = reader.read_u8()?;
                let gender_ability_override = reader.read_u8()?;
                (u16::from(difficulty), gender_ability_override)
            }
        };
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
            gender_ability_override,
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
        match family {
            GameFamily::DP | GameFamily::Platinum => {
                writer.write_u16::<LittleEndian>(self.difficulty)?;
            }
            GameFamily::HGSS => {
                let difficulty = u8::try_from(self.difficulty).map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "HGSS trainer difficulty {} does not fit in u8",
                            self.difficulty
                        ),
                    )
                })?;
                writer.write_u8(difficulty)?;
                writer.write_u8(self.gender_ability_override)?;
            }
        }
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
        let mut size = 6; // base: DP/PT difficulty(2) or HGSS difficulty(1)+override(1), plus level(2) + species_form(2)
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
    use proptest::prelude::*;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Cursor;
    use std::path::Path;

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
    fn test_trainer_properties_invalid_flags_bits_returns_error() {
        let props = TrainerProperties {
            flags: TrainerFlags::HAS_MOVES,
            trainer_class: 0,
            unknown: 0,
            party_count: 0,
            items: [0; 4],
            ai_flags: AiFlags::BASIC,
            double_battle: 0,
        };
        let mut bytes = props.to_bytes();
        bytes[0] = 0b100;

        let mut cursor = Cursor::new(bytes);
        let err = TrainerProperties::from_binary(&mut cursor).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Invalid trainer flags bits"));
    }

    #[test]
    fn test_trainer_properties_invalid_ai_flags_bits_returns_error() {
        let props = TrainerProperties {
            flags: TrainerFlags::HAS_MOVES,
            trainer_class: 0,
            unknown: 0,
            party_count: 0,
            items: [0; 4],
            ai_flags: AiFlags::BASIC,
            double_battle: 0,
        };
        let mut bytes = props.to_bytes();
        let invalid_ai_bits = 1u32 << 11;
        bytes[12..16].copy_from_slice(&invalid_ai_bits.to_le_bytes());

        let mut cursor = Cursor::new(bytes);
        let err = TrainerProperties::from_binary(&mut cursor).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Invalid trainer AI flags bits"));
    }

    #[test]
    fn test_party_pokemon_base() {
        let flags = TrainerFlags::empty();
        let family = GameFamily::DP;

        let pokemon = PartyPokemon {
            difficulty: 8,
            gender_ability_override: 0,
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
            gender_ability_override: 0,
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
    fn test_party_pokemon_hgss_base_roundtrip_preserves_overrides() {
        let flags = TrainerFlags::empty();
        let family = GameFamily::HGSS;

        let pokemon = PartyPokemon {
            difficulty: 30,
            gender_ability_override: 0x21,
            level: 14,
            species: 92,
            form: 0,
            held_item: None,
            moves: None,
            ball_seal: Some(0),
        };

        let mut buf = Vec::new();
        pokemon.to_binary(&mut buf, flags, family).unwrap();
        assert_eq!(buf.len(), PartyPokemon::binary_size(flags, family));

        let mut cursor = Cursor::new(buf);
        let parsed = PartyPokemon::from_binary(&mut cursor, flags, family).unwrap();
        assert_eq!(pokemon, parsed);
        assert_eq!(parsed.gender_override(), 1);
        assert_eq!(parsed.ability_override(), 2);
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
                    gender_ability_override: 0,
                    level: 30,
                    species: 6,
                    form: 0,
                    held_item: None,
                    moves: Some([53, 126, 14, 0]),
                    ball_seal: Some(0),
                },
                PartyPokemon {
                    difficulty: 15,
                    gender_ability_override: 0,
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

    fn family_strategy() -> impl Strategy<Value = GameFamily> {
        prop_oneof![
            Just(GameFamily::DP),
            Just(GameFamily::Platinum),
            Just(GameFamily::HGSS),
        ]
    }

    fn trainer_flags_strategy() -> impl Strategy<Value = TrainerFlags> {
        prop_oneof![
            Just(TrainerFlags::empty()),
            Just(TrainerFlags::HAS_MOVES),
            Just(TrainerFlags::HAS_ITEMS),
            Just(TrainerFlags::HAS_MOVES | TrainerFlags::HAS_ITEMS),
        ]
    }

    fn ai_flags_strategy() -> impl Strategy<Value = AiFlags> {
        any::<u16>().prop_map(|mask| {
            let mut flags = AiFlags::empty();
            let all = [
                AiFlags::BASIC,
                AiFlags::EVAL_ATTACK,
                AiFlags::EXPERT,
                AiFlags::SETUP_FIRST_TURN,
                AiFlags::RISKY,
                AiFlags::DAMAGE_PRIORITY,
                AiFlags::BATON_PASS,
                AiFlags::TAG_STRATEGY,
                AiFlags::CHECK_HP,
                AiFlags::WEATHER,
                AiFlags::HARRASSMENT,
                AiFlags::ROAMING_POKEMON,
                AiFlags::SAFARI,
                AiFlags::CATCH_TUTORIAL,
            ];
            for (idx, flag) in all.iter().enumerate() {
                if (mask & (1 << idx)) != 0 {
                    flags |= *flag;
                }
            }
            flags
        })
    }

    fn trainer_properties_strategy() -> impl Strategy<Value = TrainerProperties> {
        (
            trainer_flags_strategy(),
            any::<u8>(),
            any::<u8>(),
            0u8..16,
            any::<[u16; 4]>(),
            ai_flags_strategy(),
            any::<u32>(),
        )
            .prop_map(
                |(flags, trainer_class, unknown, party_count, items, ai_flags, double_battle)| {
                    TrainerProperties {
                        flags,
                        trainer_class,
                        unknown,
                        party_count,
                        items,
                        ai_flags,
                        double_battle,
                    }
                },
            )
    }

    fn party_pokemon_strategy(
        flags: TrainerFlags,
        family: GameFamily,
    ) -> BoxedStrategy<PartyPokemon> {
        let difficulty = if family == GameFamily::HGSS {
            (0u16..=u16::from(u8::MAX)).boxed()
        } else {
            any::<u16>().boxed()
        };

        let gender_ability_override = if family == GameFamily::HGSS {
            any::<u8>().boxed()
        } else {
            Just(0u8).boxed()
        };

        let held_item = if flags.contains(TrainerFlags::HAS_ITEMS) {
            any::<u16>().prop_map(Some).boxed()
        } else {
            Just(None).boxed()
        };

        let moves = if flags.contains(TrainerFlags::HAS_MOVES) {
            any::<[u16; 4]>().prop_map(Some).boxed()
        } else {
            Just(None).boxed()
        };

        let ball_seal = if family != GameFamily::DP {
            any::<u16>().prop_map(Some).boxed()
        } else {
            Just(None).boxed()
        };

        (
            difficulty,
            gender_ability_override,
            any::<u16>(),
            0u16..1024,
            0u8..64,
            held_item,
            moves,
            ball_seal,
        )
            .prop_map(
                |(
                    difficulty,
                    gender_ability_override,
                    level,
                    species,
                    form,
                    held_item,
                    moves,
                    ball_seal,
                )| PartyPokemon {
                    difficulty,
                    gender_ability_override,
                    level,
                    species,
                    form,
                    held_item,
                    moves,
                    ball_seal,
                },
            )
            .boxed()
    }

    fn party_case_strategy() -> impl Strategy<Value = (TrainerFlags, GameFamily, PartyPokemon)> {
        (trainer_flags_strategy(), family_strategy()).prop_flat_map(|(flags, family)| {
            party_pokemon_strategy(flags, family).prop_map(move |pokemon| (flags, family, pokemon))
        })
    }

    fn trainer_data_case_strategy() -> impl Strategy<Value = (TrainerData, GameFamily)> {
        (
            trainer_flags_strategy(),
            family_strategy(),
            any::<u8>(),
            any::<u8>(),
            0u8..12,
            any::<[u16; 4]>(),
            ai_flags_strategy(),
            any::<u32>(),
        )
            .prop_flat_map(
                |(
                    flags,
                    family,
                    trainer_class,
                    unknown,
                    party_count,
                    items,
                    ai_flags,
                    double_battle,
                )| {
                    let properties = TrainerProperties {
                        flags,
                        trainer_class,
                        unknown,
                        party_count,
                        items,
                        ai_flags,
                        double_battle,
                    };
                    let party_len = usize::from(party_count);
                    prop::collection::vec(party_pokemon_strategy(flags, family), party_len)
                        .prop_map(move |party| {
                            (
                                TrainerData {
                                    properties: properties.clone(),
                                    party,
                                },
                                family,
                            )
                        })
                },
            )
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_trainer_properties_roundtrip(props in trainer_properties_strategy()) {
            let bytes = props.to_bytes();
            prop_assert_eq!(bytes.len(), TRAINER_PROPERTIES_SIZE);
            let mut cursor = Cursor::new(bytes);
            let parsed = TrainerProperties::from_binary(&mut cursor).unwrap();
            prop_assert_eq!(props, parsed);
        }

        #[test]
        fn prop_party_pokemon_roundtrip((flags, family, pokemon) in party_case_strategy()) {
            let mut buf = Vec::new();
            pokemon.to_binary(&mut buf, flags, family).unwrap();
            prop_assert_eq!(buf.len(), PartyPokemon::binary_size(flags, family));
            let mut cursor = Cursor::new(buf);
            let parsed = PartyPokemon::from_binary(&mut cursor, flags, family).unwrap();
            prop_assert_eq!(pokemon, parsed);
        }

        #[test]
        fn prop_trainer_data_roundtrip((trainer, family) in trainer_data_case_strategy()) {
            let mut properties_buf = Vec::new();
            let mut party_buf = Vec::new();
            trainer
                .to_binary_parts(&mut properties_buf, &mut party_buf, family)
                .unwrap();

            let mut properties_cursor = Cursor::new(properties_buf);
            let mut party_cursor = Cursor::new(party_buf);
            let parsed = TrainerData::from_binary_parts(
                &mut properties_cursor,
                &mut party_cursor,
                family,
            ).unwrap();

            prop_assert_eq!(trainer, parsed);
        }
    }

    fn assert_real_trainer_narcs_roundtrip(
        trdata_path: &Path,
        trpoke_path: &Path,
        family: GameFamily,
    ) {
        let trdata_file = File::open(trdata_path).expect("Failed to open trainer properties NARC");
        let trpoke_file = File::open(trpoke_path).expect("Failed to open trainer party NARC");
        let mut trdata_reader = BufReader::new(trdata_file);
        let mut trpoke_reader = BufReader::new(trpoke_file);
        let trdata_narc =
            crate::Narc::from_binary(&mut trdata_reader).expect("Failed to load trdata NARC");
        let trpoke_narc =
            crate::Narc::from_binary(&mut trpoke_reader).expect("Failed to load trpoke NARC");

        assert_eq!(
            trdata_narc.members.len(),
            trpoke_narc.members.len(),
            "trdata/trpoke member-count mismatch: {} vs {}",
            trdata_narc.members.len(),
            trpoke_narc.members.len()
        );
        assert!(
            !trdata_narc.members.is_empty(),
            "expected non-empty trainer NARCs: {} and {}",
            trdata_path.display(),
            trpoke_path.display()
        );

        for i in 0..trdata_narc.members.len().min(400) {
            let props_data = trdata_narc
                .members
                .get(i)
                .expect("trdata member index out of range");
            let party_data = trpoke_narc
                .members
                .get(i)
                .expect("trpoke member index out of range");

            let mut props_cursor = Cursor::new(props_data.as_slice());
            let mut party_cursor = Cursor::new(party_data.as_slice());
            let trainer =
                TrainerData::from_binary_parts(&mut props_cursor, &mut party_cursor, family)
                    .unwrap_or_else(|_| panic!("Failed to parse trainer member {}", i));

            let mut props_out = Vec::new();
            let mut party_out = Vec::new();
            trainer
                .to_binary_parts(&mut props_out, &mut party_out, family)
                .unwrap_or_else(|_| panic!("Failed to serialize trainer member {}", i));

            assert_eq!(
                props_data.as_slice(),
                props_out.as_slice(),
                "trdata roundtrip failed for trainer member {}",
                i
            );
            assert_eq!(
                party_data.as_slice(),
                party_out.as_slice(),
                "trpoke roundtrip failed for trainer member {}",
                i
            );
        }
    }

    #[test]
    #[ignore = "requires a real Platinum DSPRE project via UXIE_TEST_PLATINUM_DSPRE_PATH"]
    fn integration_real_rom_roundtrip_platinum() {
        let Some(trdata_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            &["data/poketool/trainer/trdata.narc", "data/a/0/5/5"],
            "trainer data real ROM roundtrip test (platinum trdata)",
        ) else {
            return;
        };
        let Some(trpoke_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            &["data/poketool/trainer/trpoke.narc", "data/a/0/5/6"],
            "trainer data real ROM roundtrip test (platinum trpoke)",
        ) else {
            return;
        };

        assert_real_trainer_narcs_roundtrip(&trdata_path, &trpoke_path, GameFamily::Platinum);
    }

    #[test]
    #[ignore = "requires a real HGSS DSPRE project via UXIE_TEST_HGSS_DSPRE_PATH"]
    fn integration_real_rom_roundtrip_hgss() {
        let Some(trdata_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            &["data/poketool/trainer/trdata.narc", "data/a/0/5/5"],
            "trainer data real ROM roundtrip test (hgss trdata)",
        ) else {
            return;
        };
        let Some(trpoke_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            &["data/poketool/trainer/trpoke.narc", "data/a/0/5/6"],
            "trainer data real ROM roundtrip test (hgss trpoke)",
        ) else {
            return;
        };

        assert_real_trainer_narcs_roundtrip(&trdata_path, &trpoke_path, GameFamily::HGSS);
    }
}
