use super::types::PersonalData;
use binrw::{BinRead, BinWrite};
use std::io::{self, Read, Seek, Write};

pub const PERSONAL_DATA_SIZE: usize = 44;

impl PersonalData {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Self::read_le(reader).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_binary<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        self.write_le(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::with_capacity(PERSONAL_DATA_SIZE));
        self.to_binary(&mut buf).unwrap();
        buf.into_inner()
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

    fn create_test_personal_data() -> PersonalData {
        PersonalData {
            hp: 45,
            attack: 49,
            defense: 49,
            speed: 45,
            sp_attack: 65,
            sp_defense: 65,
            type1: 12, // Grass
            type2: 3,  // Poison
            catch_rate: 45,
            base_exp: 64,
            ev_yield: 0b0000_0001_0000_0001, // bits 8-9 = Sp.Atk (1), bits 0-1 = HP (1)
            item1: 0,
            item2: 0,
            gender_ratio: 31,
            egg_cycles: 20,
            base_friendship: 70,
            growth_rate: 3,
            egg_group1: 7, // Monster
            egg_group2: 1, // Plant
            ability1: 65,  // Overgrow
            ability2: 34,  // Chlorophyll
            safari_flee_rate: 0,
            color_flip: 5,
            tm_compatibility: [0xFF; 16],
        }
    }

    #[test]
    fn test_roundtrip() {
        let original = create_test_personal_data();
        let bytes = original.to_bytes();
        assert_eq!(bytes.len(), PERSONAL_DATA_SIZE);

        let mut cursor = Cursor::new(bytes);
        let parsed = PersonalData::from_binary(&mut cursor).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_ev_yield_accessors() {
        let data = create_test_personal_data();
        assert_eq!(data.ev_yield_hp(), 1);
        assert_eq!(data.ev_yield_attack(), 0);
        assert_eq!(data.ev_yield_defense(), 0);
        assert_eq!(data.ev_yield_speed(), 0);
        assert_eq!(data.ev_yield_sp_attack(), 1);
        assert_eq!(data.ev_yield_sp_defense(), 0);
    }

    #[test]
    fn test_tm_compatibility() {
        let data = create_test_personal_data();
        for i in 0..128 {
            assert!(data.can_learn_tm(i));
        }
        assert!(!data.can_learn_tm(128));
    }

    fn personal_data_strategy() -> impl Strategy<Value = PersonalData> {
        let part1 = (
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u16>(),
            any::<u16>(),
        );
        let part2 = (
            any::<u16>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<[u8; 16]>(),
        );

        (part1, part2).prop_map(
            |(
                (
                    hp,
                    attack,
                    defense,
                    speed,
                    sp_attack,
                    sp_defense,
                    type1,
                    type2,
                    catch_rate,
                    base_exp,
                    ev_yield,
                    item1,
                ),
                (
                    item2,
                    gender_ratio,
                    egg_cycles,
                    base_friendship,
                    growth_rate,
                    egg_group1,
                    egg_group2,
                    ability1,
                    ability2,
                    safari_flee_rate,
                    color_flip,
                    tm_compatibility,
                ),
            )| PersonalData {
                hp,
                attack,
                defense,
                speed,
                sp_attack,
                sp_defense,
                type1,
                type2,
                catch_rate,
                base_exp,
                ev_yield,
                item1,
                item2,
                gender_ratio,
                egg_cycles,
                base_friendship,
                growth_rate,
                egg_group1,
                egg_group2,
                ability1,
                ability2,
                safari_flee_rate,
                color_flip,
                tm_compatibility,
            },
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_personal_data_roundtrip(data in personal_data_strategy()) {
            let bytes = data.to_bytes();
            prop_assert_eq!(bytes.len(), PERSONAL_DATA_SIZE);

            let mut cursor = Cursor::new(bytes);
            let parsed = PersonalData::from_binary(&mut cursor).unwrap();
            prop_assert_eq!(data, parsed);
        }

        #[test]
        fn prop_ev_yield_accessors_match_packed_bits(ev_yield in any::<u16>()) {
            let mut data = create_test_personal_data();
            data.ev_yield = ev_yield;

            prop_assert_eq!(data.ev_yield_hp(), (ev_yield & 0b11) as u8);
            prop_assert_eq!(data.ev_yield_attack(), ((ev_yield >> 2) & 0b11) as u8);
            prop_assert_eq!(data.ev_yield_defense(), ((ev_yield >> 4) & 0b11) as u8);
            prop_assert_eq!(data.ev_yield_speed(), ((ev_yield >> 6) & 0b11) as u8);
            prop_assert_eq!(data.ev_yield_sp_attack(), ((ev_yield >> 8) & 0b11) as u8);
            prop_assert_eq!(data.ev_yield_sp_defense(), ((ev_yield >> 10) & 0b11) as u8);
        }

        #[test]
        fn prop_can_learn_tm_single_bit_behavior(tm_index in 0u8..128, other_tm in 0u8..128) {
            let mut data = create_test_personal_data();
            data.tm_compatibility = [0; 16];
            let byte_idx = (tm_index / 8) as usize;
            let bit_idx = tm_index % 8;
            data.tm_compatibility[byte_idx] = 1 << bit_idx;

            prop_assert!(data.can_learn_tm(tm_index));
            prop_assert_eq!(data.can_learn_tm(other_tm), other_tm == tm_index);
        }

        #[test]
        fn prop_can_learn_tm_out_of_range_is_false(tm_index in 128u8..=u8::MAX) {
            let data = create_test_personal_data();
            prop_assert!(!data.can_learn_tm(tm_index));
        }
    }

    fn assert_real_personal_narc_roundtrip(narc_path: &Path) {
        let file = File::open(narc_path).expect("Failed to open personal NARC");
        let mut reader = BufReader::new(file);
        let narc = crate::Narc::from_binary(&mut reader).expect("Failed to load personal NARC");

        let mut roundtripped_members = 0usize;
        for (i, original_bytes) in narc.members.iter().enumerate().take(600) {
            if original_bytes.len() != PERSONAL_DATA_SIZE {
                continue;
            }

            roundtripped_members += 1;
            let mut cursor = Cursor::new(original_bytes.as_slice());
            let personal = PersonalData::from_binary(&mut cursor)
                .unwrap_or_else(|_| panic!("Failed to parse personal member {}", i));
            let serialized = personal.to_bytes();

            assert_eq!(
                original_bytes.as_slice(),
                serialized.as_slice(),
                "Roundtrip failed for personal member {}",
                i
            );
        }

        assert!(
            roundtripped_members > 0,
            "expected at least one personal entry with {} bytes in {}",
            PERSONAL_DATA_SIZE,
            narc_path.display()
        );
    }

    #[test]
    #[ignore = "requires a real Platinum DSPRE project via UXIE_TEST_PLATINUM_DSPRE_PATH"]
    fn integration_real_rom_roundtrip_platinum() {
        let Some(narc_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            &[
                "data/poketool/personal/pl_personal.narc",
                "data/poketool/personal/personal.narc",
                "data/pbr/personal.narc",
            ],
            "personal data real ROM roundtrip test (platinum)",
        ) else {
            return;
        };

        assert_real_personal_narc_roundtrip(&narc_path);
    }

    #[test]
    #[ignore = "requires a real HGSS DSPRE project via UXIE_TEST_HGSS_DSPRE_PATH"]
    fn integration_real_rom_roundtrip_hgss() {
        let Some(narc_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            &[
                "data/poketool/personal/personal.narc",
                "data/poketool/personal/pl_personal.narc",
                "data/pbr/personal.narc",
            ],
            "personal data real ROM roundtrip test (hgss)",
        ) else {
            return;
        };

        assert_real_personal_narc_roundtrip(&narc_path);
    }
}
